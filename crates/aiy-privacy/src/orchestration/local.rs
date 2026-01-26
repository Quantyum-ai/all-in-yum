//! Local executor for privacy mode
//!
//! Executes tasks locally using Ollama, receiving full code context.

use super::error::{OrchestrationError, OrchestrationResult};
use super::types::{PlanTask, TaskResult, TaskType};
use crate::enforcement::Redactor;
use crate::rag::RagSystem;
use crate::verification::VerificationEngine;
use aiy_adapter_ollama::OllamaAdapter;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

/// System prompt for local code generator
const LOCAL_CODE_PROMPT: &str = r#"You are a code implementation assistant running locally in privacy mode.

Your task is to implement code changes based on the execution plan and local codebase context.
You have FULL ACCESS to the actual code, file paths, and identifiers.

For each task, respond with:
1. The file path to modify/create
2. The complete code changes (unified diff format or full file)
3. Any additional files that need updating

Be precise and complete. Your changes will be verified by automated tools (fmt, clippy, tests)."#;

/// Local executor configuration
#[derive(Debug, Clone)]
pub struct LocalExecutorConfig {
    /// Ollama URL
    pub ollama_url: String,
    /// Model name
    pub model: String,
    /// Context size
    pub context_size: usize,
    /// Temperature
    pub temperature: f32,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable verification after each task
    pub verify_after_task: bool,
    /// Max repair attempts per task
    pub max_repairs: usize,
}

impl Default for LocalExecutorConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            model: "codellama:7b-instruct".to_string(),
            context_size: 8192,
            temperature: 0.1,
            timeout_ms: 300_000,
            verify_after_task: true,
            max_repairs: 3,
        }
    }
}

/// Local executor for code implementation
pub struct LocalExecutor {
    /// Configuration
    config: LocalExecutorConfig,
    /// Ollama adapter (optional - for testing)
    adapter: Option<OllamaAdapter>,
    /// Verification engine (optional)
    verification_engine: Option<VerificationEngine>,
}

impl LocalExecutor {
    /// Create a new local executor
    pub fn new(config: LocalExecutorConfig) -> Self {
        Self {
            config,
            adapter: None,
            verification_engine: None,
        }
    }

    /// Set the Ollama adapter
    pub fn with_adapter(mut self, adapter: OllamaAdapter) -> Self {
        self.adapter = Some(adapter);
        self
    }

    /// Set the verification engine
    pub fn with_verification(mut self, engine: VerificationEngine) -> Self {
        self.verification_engine = Some(engine);
        self
    }

    /// Get configuration
    pub fn config(&self) -> &LocalExecutorConfig {
        &self.config
    }

    /// Execute a single task
    ///
    /// Takes the task, RAG system for context, redactor for unredacting plan descriptions.
    pub async fn execute_task(
        &mut self,
        task: &PlanTask,
        rag: &RagSystem,
        redactor: &Redactor,
        working_dir: &Path,
    ) -> OrchestrationResult<TaskResult> {
        let start = Instant::now();
        info!("Executing task: {} - {}", task.id, task.description);

        // Unredact the task description to get actual identifiers
        let unredacted_desc = redactor.unredact_content(&task.description);
        debug!("Unredacted task: {}", unredacted_desc);

        // Get relevant code context from RAG
        let context = self.get_task_context(task, rag).await?;

        // Generate implementation
        let implementation = self.generate_implementation(task, &context, &unredacted_desc).await;

        match implementation {
            Ok(code_changes) => {
                // Apply changes
                if let Err(e) = self.apply_changes(&code_changes, working_dir).await {
                    return Ok(TaskResult::failure(
                        &task.id,
                        start.elapsed(),
                        format!("Failed to apply changes: {}", e),
                    ));
                }

                // Verify if enabled
                if self.config.verify_after_task {
                    match self.verify_changes(working_dir).await {
                        Ok(repairs) => Ok(TaskResult::success_after_repair(
                            &task.id,
                            start.elapsed(),
                            repairs,
                        )),
                        Err(e) => Ok(TaskResult::failure(
                            &task.id,
                            start.elapsed(),
                            format!("Verification failed: {}", e),
                        )),
                    }
                } else {
                    Ok(TaskResult::success(&task.id, start.elapsed()))
                }
            }
            Err(e) => Ok(TaskResult::failure(
                &task.id,
                start.elapsed(),
                format!("Implementation failed: {}", e),
            )),
        }
    }

    /// Get context for a task from RAG system
    async fn get_task_context(
        &self,
        task: &PlanTask,
        rag: &RagSystem,
    ) -> OrchestrationResult<String> {
        if !rag.is_indexed() {
            return Ok(String::new());
        }

        // Query based on task description
        let query = match task.task_type {
            TaskType::CreateFile => format!("similar files to: {}", task.description),
            TaskType::ModifyFile => format!("file to modify: {}", task.description),
            TaskType::RunTests => "test files and test functions".to_string(),
            TaskType::Refactor => format!("code to refactor: {}", task.description),
            _ => task.description.clone(),
        };

        match rag.query(&query).await {
            Ok(result) => {
                let mut context = String::new();
                for ranked in &result.chunks {
                    context.push_str(&format!(
                        "--- {} (similarity: {:.2}) ---\n{}\n\n",
                        ranked.id, ranked.similarity, ranked.content
                    ));
                }
                Ok(context)
            }
            Err(e) => {
                warn!("RAG query failed: {}", e);
                Ok(String::new())
            }
        }
    }

    /// Generate implementation using local model
    async fn generate_implementation(
        &self,
        task: &PlanTask,
        context: &str,
        unredacted_desc: &str,
    ) -> OrchestrationResult<String> {
        // Build prompt
        let prompt = format!(
            "Task: {} ({})\nDescription: {}\n\nRelevant code context:\n{}\n\n\
             Please implement the changes needed.",
            task.id, task.task_type_str(), unredacted_desc, context
        );

        if let Some(ref adapter) = self.adapter {
            adapter
                .generate_with_system(LOCAL_CODE_PROMPT, &prompt)
                .await
                .map_err(|e| OrchestrationError::local(e.to_string()))
        } else {
            // Mock implementation for testing
            Ok(format!("// Implementation for task {}\n", task.id))
        }
    }

    /// Apply code changes to working directory
    async fn apply_changes(
        &self,
        _code_changes: &str,
        _working_dir: &Path,
    ) -> OrchestrationResult<()> {
        // TODO: Parse and apply changes
        // For now, this is a placeholder
        Ok(())
    }

    /// Verify changes using verification engine
    async fn verify_changes(&mut self, _working_dir: &Path) -> OrchestrationResult<usize> {
        if let Some(ref mut engine) = self.verification_engine {
            match engine.run().await {
                Ok(result) => {
                    if result.success {
                        Ok(result.total_repairs)
                    } else {
                        Err(OrchestrationError::verification_failed(result.total_repairs))
                    }
                }
                Err(e) => Err(OrchestrationError::local(format!(
                    "Verification error: {}",
                    e
                ))),
            }
        } else {
            // No verification engine - skip
            Ok(0)
        }
    }
}

impl PlanTask {
    /// Get task type as string
    pub fn task_type_str(&self) -> &'static str {
        match self.task_type {
            TaskType::CreateFile => "create_file",
            TaskType::ModifyFile => "modify_file",
            TaskType::DeleteFile => "delete_file",
            TaskType::RunTests => "run_tests",
            TaskType::Verify => "verify",
            TaskType::Refactor => "refactor",
            TaskType::Document => "document",
            TaskType::Generic => "generic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_executor_config_default() {
        let config = LocalExecutorConfig::default();
        assert_eq!(config.ollama_url, "http://127.0.0.1:11434");
        assert!(config.verify_after_task);
        assert_eq!(config.max_repairs, 3);
    }

    #[test]
    fn test_local_executor_creation() {
        let config = LocalExecutorConfig::default();
        let executor = LocalExecutor::new(config.clone());
        assert_eq!(executor.config().ollama_url, config.ollama_url);
    }

    #[test]
    fn test_task_type_str() {
        let task = PlanTask {
            id: "TASK_001".to_string(),
            task_type: TaskType::ModifyFile,
            description: "Update FILE_001".to_string(),
            dependencies: vec![],
            priority: 1,
            estimated_effort: 2,
        };
        assert_eq!(task.task_type_str(), "modify_file");
    }
}
