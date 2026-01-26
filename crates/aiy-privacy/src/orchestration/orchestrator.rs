//! Privacy orchestrator - main coordinator
//!
//! Coordinates cloud planning and local execution for privacy mode.

use super::cloud::CloudCommunicator;
use super::error::{OrchestrationError, OrchestrationResult};
use super::local::{LocalExecutor, LocalExecutorConfig};
use super::session::{OrchestrationSession, SessionState};
use super::types::SessionStats;
use crate::rag::{RagSystem, RagSystemConfig};
use aiy_core::config::PrivacyModeConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Privacy orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Working directory (repository root)
    pub working_dir: PathBuf,
    /// RAG system configuration
    pub rag_config: RagSystemConfig,
    /// Local executor configuration
    pub local_config: LocalExecutorConfig,
    /// Enable cloud planning (false = local-only mode)
    pub enable_cloud: bool,
}

impl OrchestratorConfig {
    /// Create from privacy mode configuration
    pub fn from_privacy_config(
        privacy: &PrivacyModeConfig,
        working_dir: impl Into<PathBuf>,
    ) -> OrchestrationResult<Self> {
        let rag_config = RagSystemConfig::from_privacy_config(privacy)
            .map_err(|e| OrchestrationError::config(e.to_string()))?;

        let local_config = LocalExecutorConfig {
            ollama_url: privacy.local_executor.ollama_url.clone(),
            model: privacy.local_executor.model.clone(),
            context_size: privacy.local_executor.context_size,
            temperature: privacy.local_executor.temperature,
            timeout_ms: privacy.local_executor.timeout_ms,
            verify_after_task: true,
            max_repairs: privacy.verification.max_global_repairs,
        };

        Ok(Self {
            working_dir: working_dir.into(),
            rag_config,
            local_config,
            enable_cloud: true,
        })
    }

    /// Disable cloud planning (local-only mode)
    pub fn without_cloud(mut self) -> Self {
        self.enable_cloud = false;
        self
    }
}

/// Privacy orchestrator
///
/// Coordinates the privacy mode workflow:
/// 1. Index codebase locally
/// 2. Send redacted context to cloud for planning
/// 3. Execute plan locally with full code access
/// 4. Verify results
pub struct PrivacyOrchestrator {
    /// Configuration
    config: OrchestratorConfig,
    /// Current session (protected by RwLock for async access)
    session: Arc<RwLock<Option<OrchestrationSession>>>,
    /// Cloud communicator
    cloud: CloudCommunicator,
    /// Local executor
    local: LocalExecutor,
    /// RAG system (initialized on first use)
    rag: Option<RagSystem>,
}

impl PrivacyOrchestrator {
    /// Create a new orchestrator
    pub fn new(config: OrchestratorConfig) -> Self {
        let local = LocalExecutor::new(config.local_config.clone());

        Self {
            config,
            session: Arc::new(RwLock::new(None)),
            cloud: CloudCommunicator::new(),
            local,
            rag: None,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Initialize the orchestrator (index codebase)
    pub async fn init(&mut self) -> OrchestrationResult<SessionStats> {
        info!("Initializing privacy orchestrator");

        // Check if session already exists
        {
            let session = self.session.read().await;
            if session.is_some() {
                return Err(OrchestrationError::SessionAlreadyExists);
            }
        }

        // Create new session
        let mut new_session = OrchestrationSession::new(&self.config.working_dir);

        // Initialize RAG system
        info!("Initializing RAG system");
        self.init_rag_system().await?;

        // Index directory
        if let Some(ref mut rag) = self.rag {
            info!("Indexing directory: {:?}", self.config.working_dir);
            let index_result = rag
                .index_directory(&self.config.working_dir)
                .await
                .map_err(|e| OrchestrationError::rag(e.to_string()))?;

            new_session.stats_mut().chunks_indexed = index_result.chunks_indexed;
            info!(
                "Indexed {} chunks, {} tokens",
                index_result.chunks_indexed, index_result.tokens_indexed
            );
        }

        // Mark session as ready
        new_session.mark_ready()?;

        let stats = new_session.stats().clone();

        // Store session
        {
            let mut session_guard = self.session.write().await;
            *session_guard = Some(new_session);
        }

        Ok(stats)
    }

    /// Execute a user request
    pub async fn execute(&mut self, request: &str) -> OrchestrationResult<SessionStats> {
        info!("Executing request");

        // Get session
        let session_guard = self.session.read().await;
        let session = session_guard
            .as_ref()
            .ok_or(OrchestrationError::SessionNotInitialized)?;

        // Verify session is ready
        if session.state() != SessionState::Ready && session.state() != SessionState::Paused {
            return Err(OrchestrationError::InvalidState(format!(
                "Session is {:?}, expected Ready or Paused",
                session.state()
            )));
        }

        drop(session_guard);

        // Redact the request
        let mut session_guard = self.session.write().await;
        let session = session_guard.as_mut().unwrap();
        let redacted_request = self.redact_request(request, session);

        // Get RAG context (already uses opaque IDs internally)
        let rag_context = if let Some(ref rag) = self.rag {
            match rag.query(&redacted_request).await {
                Ok(result) => Some(result),
                Err(e) => {
                    warn!("RAG query failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Request plan from cloud
        session.stats_mut().record_cloud_request();
        let plan = self
            .cloud
            .request_plan(
                &redacted_request,
                rag_context.as_ref(),
                session.redactor_mut(),
            )
            .await?;

        info!("Received plan with {} tasks", plan.task_count());

        // Start execution
        session.start_execution(plan)?;

        drop(session_guard);

        // Execute tasks
        self.execute_plan().await?;

        // Get final stats
        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().unwrap();
        Ok(session.stats().clone())
    }

    /// Get current session state
    pub async fn state(&self) -> Option<SessionState> {
        let session = self.session.read().await;
        session.as_ref().map(|s| s.state())
    }

    /// Get current session statistics
    pub async fn stats(&self) -> Option<SessionStats> {
        let session = self.session.read().await;
        session.as_ref().map(|s| s.stats().clone())
    }

    /// Resume a paused session
    pub async fn resume(&mut self) -> OrchestrationResult<SessionStats> {
        {
            let mut session_guard = self.session.write().await;
            let session = session_guard
                .as_mut()
                .ok_or(OrchestrationError::SessionNotInitialized)?;
            session.resume()?;
        }

        self.execute_plan().await?;

        let session_guard = self.session.read().await;
        let session = session_guard.as_ref().unwrap();
        Ok(session.stats().clone())
    }

    /// Cancel the current session
    pub async fn cancel(&mut self) -> OrchestrationResult<()> {
        let mut session_guard = self.session.write().await;
        if let Some(ref mut session) = *session_guard {
            session.cancel()?;
            session.clear();
        }
        *session_guard = None;
        Ok(())
    }

    /// Redact user request
    fn redact_request(&self, request: &str, session: &mut OrchestrationSession) -> String {
        // Find and redact file paths in the request
        let path_regex = regex::Regex::new(r"[/\\][\w/\\.-]+\.\w+").unwrap();
        let mut redacted = request.to_string();

        for cap in path_regex.captures_iter(request) {
            let path = &cap[0];
            let opaque = session.redactor_mut().map_mut().redact_file(path);
            redacted = redacted.replace(path, &opaque);
        }

        // Find and redact function-like identifiers
        let func_regex = regex::Regex::new(r"\b([a-z_][a-z0-9_]*)\s*\(").unwrap();
        for cap in func_regex.captures_iter(&redacted.clone()) {
            if let Some(name) = cap.get(1) {
                let name_str = name.as_str();
                // Skip common words
                if !["if", "for", "while", "match", "return", "fn", "let", "mut"].contains(&name_str) {
                    let opaque = session.redactor_mut().map_mut().redact_function(name_str);
                    redacted = redacted.replace(name_str, &opaque);
                }
            }
        }

        redacted
    }

    /// Initialize RAG system
    async fn init_rag_system(&mut self) -> OrchestrationResult<()> {
        // Create mock embedder for now (real HTTP embedder requires feature flag)
        use crate::rag::{LocalEmbedder, MockEmbeddingTransport};
        use std::sync::Arc;

        let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
        let embedder = Arc::new(LocalEmbedder::with_mock_transport(
            &self.config.rag_config.ollama_url,
            &self.config.rag_config.embedding_model,
            transport,
        ));

        self.rag = Some(RagSystem::with_mock_embedder(
            self.config.rag_config.clone(),
            embedder,
        ));

        Ok(())
    }

    /// Execute the current plan
    async fn execute_plan(&mut self) -> OrchestrationResult<()> {
        loop {
            // Get next task
            let (task_index, task) = {
                let session_guard = self.session.read().await;
                let session = session_guard.as_ref().unwrap();

                if session.state() == SessionState::Cancelled {
                    return Err(OrchestrationError::Cancelled);
                }

                if session.all_tasks_completed() {
                    break;
                }

                let idx = match session.next_task_index() {
                    Some(i) => i,
                    None => break,
                };

                let plan = session.plan().unwrap();
                (idx, plan.tasks[idx].clone())
            };

            // Execute task
            debug!("Executing task {}: {}", task.id, task.description);

            let rag = self.rag.as_ref().ok_or(OrchestrationError::SessionNotInitialized)?;

            let result = {
                let session_guard = self.session.read().await;
                let session = session_guard.as_ref().unwrap();
                self.local
                    .execute_task(&task, rag, session.redactor(), &self.config.working_dir)
                    .await?
            };

            // Record result
            {
                let mut session_guard = self.session.write().await;
                let session = session_guard.as_mut().unwrap();
                session.stats_mut().record_local_execution();
                session.record_task_result(result.clone());
                session.mark_task_completed(task_index);

                if !result.success {
                    session.fail()?;
                    return Err(OrchestrationError::task_failed(
                        &task.id,
                        result.error.unwrap_or_default(),
                    ));
                }
            }
        }

        // Mark as completed
        {
            let mut session_guard = self.session.write().await;
            let session = session_guard.as_mut().unwrap();
            session.complete()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> OrchestratorConfig {
        let temp_dir = std::env::temp_dir();
        OrchestratorConfig {
            working_dir: temp_dir,
            rag_config: RagSystemConfig::default(),
            local_config: LocalExecutorConfig::default(),
            enable_cloud: false,
        }
    }

    #[test]
    fn test_orchestrator_creation() {
        let config = create_test_config();
        let orchestrator = PrivacyOrchestrator::new(config.clone());
        assert_eq!(orchestrator.config().working_dir, config.working_dir);
    }

    #[tokio::test]
    async fn test_orchestrator_state_before_init() {
        let config = create_test_config();
        let orchestrator = PrivacyOrchestrator::new(config);
        assert!(orchestrator.state().await.is_none());
    }

    #[test]
    fn test_config_without_cloud() {
        let config = create_test_config().without_cloud();
        assert!(!config.enable_cloud);
    }
}
