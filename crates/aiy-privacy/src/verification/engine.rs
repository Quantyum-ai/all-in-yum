//! Verification engine for the privacy mode.
//!
//! This module provides the `VerificationEngine` that orchestrates the
//! verification pipeline, running stages and repair loops.

use super::error::VerificationError;
use super::repair::{RepairConfig, RepairGenerator};
use super::stages::{default_stage_order, VerificationStage};
use super::state::VerificationState;
use super::types::{StageResult, VerificationResult};
use aiy_adapter_ollama::OllamaAdapter;
#[cfg(test)]
use aiy_adapter_ollama::OllamaClient;
use aiy_core::config::VerificationConfig;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Configuration for the verification engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Verification limits configuration
    pub verification: VerificationConfig,
    /// Repair generation configuration
    pub repair: RepairConfig,
    /// Whether to attempt repairs
    pub enable_repairs: bool,
    /// Backoff configuration for retries
    pub backoff: BackoffConfig,
    /// Working directory for verification
    pub working_dir: PathBuf,
}

impl EngineConfig {
    /// Create a new engine config with the given working directory.
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            verification: VerificationConfig::default(),
            repair: RepairConfig::default(),
            enable_repairs: true,
            backoff: BackoffConfig::default(),
            working_dir: working_dir.into(),
        }
    }

    /// Disable repairs.
    pub fn without_repairs(mut self) -> Self {
        self.enable_repairs = false;
        self
    }

    /// Set verification configuration.
    pub fn with_verification_config(mut self, config: VerificationConfig) -> Self {
        self.verification = config;
        self
    }

    /// Set repair configuration.
    pub fn with_repair_config(mut self, config: RepairConfig) -> Self {
        self.repair = config;
        self
    }

    /// Set backoff configuration.
    pub fn with_backoff(mut self, backoff: BackoffConfig) -> Self {
        self.backoff = backoff;
        self
    }
}

/// Configuration for exponential backoff between retries.
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub multiplier: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 500,
            max_delay_ms: 10_000,
            multiplier: 2.0,
        }
    }
}

impl BackoffConfig {
    /// Calculate delay for given attempt (0-indexed).
    pub fn delay(&self, attempt: usize) -> Duration {
        let delay_ms = self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32);
        let capped_ms = delay_ms.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(capped_ms)
    }
}

/// Verification engine that orchestrates stages and repair loops.
///
/// The engine runs verification stages (fmt, clippy, test) in order,
/// attempting repairs for failures using a local Ollama model.
pub struct VerificationEngine {
    /// Engine configuration
    config: EngineConfig,
    /// Verification stages to run
    stages: Vec<Box<dyn VerificationStage>>,
    /// Repair generator (lazy initialized)
    repair_generator: Option<RepairGenerator>,
    /// Ollama adapter for repair generation
    ollama_adapter: Option<OllamaAdapter>,
}

impl VerificationEngine {
    /// Create a new verification engine with the given configuration.
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            stages: default_stage_order(),
            repair_generator: None,
            ollama_adapter: None,
        }
    }

    /// Create an engine with custom stages.
    pub fn with_stages(mut self, stages: Vec<Box<dyn VerificationStage>>) -> Self {
        self.stages = stages;
        self
    }

    /// Set the Ollama adapter for repair generation.
    pub fn with_ollama_adapter(mut self, adapter: OllamaAdapter) -> Self {
        self.ollama_adapter = Some(adapter);
        self
    }

    /// Create an engine for testing with a mock Ollama transport.
    #[cfg(test)]
    pub fn with_mock_ollama(
        config: EngineConfig,
        transport: Arc<dyn aiy_adapter_ollama::transport::HttpTransport>,
    ) -> Self {
        let client = OllamaClient::new_with_mock(transport);
        let adapter = OllamaAdapter::new(client);
        Self {
            config,
            stages: default_stage_order(),
            repair_generator: None,
            ollama_adapter: Some(adapter),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Get the stages.
    pub fn stages(&self) -> &[Box<dyn VerificationStage>] {
        &self.stages
    }

    /// Run the complete verification pipeline.
    ///
    /// Executes each stage in order, attempting repairs for failures.
    /// Returns the final verification result.
    pub async fn run(&mut self) -> Result<VerificationResult, VerificationError> {
        info!("Starting verification pipeline");

        // Validate working directory
        if !self.config.working_dir.exists() {
            return Err(VerificationError::invalid_working_directory(
                &self.config.working_dir,
            ));
        }

        // Check for stages
        if self.stages.is_empty() {
            return Err(VerificationError::NoStagesConfigured);
        }

        // Initialize state
        let mut state = VerificationState::new(self.config.verification.clone());

        // Initialize repair generator if repairs are enabled
        if self.config.enable_repairs && self.repair_generator.is_none() {
            self.initialize_repair_generator()?;
        }

        // Run each stage using index to avoid borrow issues
        let num_stages = self.stages.len();
        for i in 0..num_stages {
            if state.is_cancelled() {
                break;
            }

            // Run stage with repairs (working_dir is borrowed but stages aren't)
            let stage_name = self.stages[i].name().to_string();
            let stage_display = self.stages[i].display_name().to_string();
            let supports_repair = self.stages[i].supports_repair();

            info!("Running stage: {}", stage_display);
            state.enter_stage(&stage_name);

            // Initial run
            let mut result = self.stages[i].run(&self.config.working_dir).await?;

            // If successful or repairs disabled, skip repair loop
            if !result.success && self.config.enable_repairs && supports_repair {
                result = self.run_repair_loop(i, result, &mut state).await?;
            }

            state.record_stage_result(result.clone());

            // Stop on failure
            if !result.success {
                let failure_reason = if result.repair_limit_exceeded {
                    format!("Stage '{}' repair limit exceeded", result.stage_name)
                } else {
                    format!("Stage '{}' failed", result.stage_name)
                };

                return Ok(VerificationResult::failure(
                    state.take_stage_results(),
                    state.global_repair_count(),
                    state.elapsed(),
                    &failure_reason,
                ));
            }
        }

        // All stages passed
        Ok(VerificationResult::success(
            state.take_stage_results(),
            state.global_repair_count(),
            state.elapsed(),
        ))
    }

    /// Run the repair loop for a stage.
    async fn run_repair_loop(
        &mut self,
        stage_idx: usize,
        initial_result: StageResult,
        state: &mut VerificationState,
    ) -> Result<StageResult, VerificationError> {
        let stage_name = self.stages[stage_idx].name().to_string();
        let mut result = initial_result;
        let mut attempt = 0;

        while !result.success {
            // Check if we can repair
            if let Err(e) = state.can_repair(&stage_name) {
                warn!("Repair limit reached for stage '{}': {}", stage_name, e);
                return Ok(StageResult::failure(
                    &stage_name,
                    result.failures.clone(),
                    state.stage_repair_count(&stage_name),
                    true, // repair_limit_exceeded
                    result.duration,
                    result.raw_output.clone(),
                ));
            }

            // Attempt repair
            info!(
                "Attempting repair {} for stage '{}'",
                attempt + 1,
                stage_name
            );

            let repair_success = self.attempt_repair(stage_idx, &result, state).await?;

            if !repair_success {
                debug!("Repair generation failed or produced no changes");
                break;
            }

            // Record the repair
            state.record_repair(&stage_name)?;

            // Backoff before rerunning
            let delay = self.config.backoff.delay(attempt);
            debug!("Backoff: {:?}", delay);
            sleep(delay).await;

            // Re-run the stage
            result = self.stages[stage_idx].run(&self.config.working_dir).await?;
            attempt += 1;

            if result.success {
                info!("Stage '{}' passed after {} repair(s)", stage_name, attempt);
                return Ok(StageResult::repaired(
                    &stage_name,
                    attempt,
                    result.duration,
                    result.raw_output,
                ));
            }
        }

        // Failed after repair attempts
        Ok(StageResult::failure(
            &stage_name,
            result.failures,
            state.stage_repair_count(&stage_name),
            false,
            result.duration,
            result.raw_output,
        ))
    }

    /// Run a single stage with repair loop.
    #[allow(dead_code)] // Will be used when repair loop is fully integrated
    async fn run_stage_with_repairs(
        &mut self,
        stage_idx: usize,
        state: &mut VerificationState,
    ) -> Result<StageResult, VerificationError> {
        let stage_name = self.stages[stage_idx].name().to_string();
        info!("Running stage: {}", self.stages[stage_idx].display_name());
        state.enter_stage(&stage_name);

        // Initial run
        let mut result = self.stages[stage_idx].run(&self.config.working_dir).await?;

        // If successful or repairs disabled, return immediately
        if result.success || !self.config.enable_repairs || !self.stages[stage_idx].supports_repair() {
            return Ok(result);
        }

        // Repair loop
        let mut attempt = 0;
        while !result.success {
            // Check if we can repair
            if let Err(e) = state.can_repair(&stage_name) {
                warn!("Repair limit reached for stage '{}': {}", stage_name, e);
                return Ok(StageResult::failure(
                    &stage_name,
                    result.failures.clone(),
                    state.stage_repair_count(&stage_name),
                    true, // repair_limit_exceeded
                    result.duration,
                    result.raw_output.clone(),
                ));
            }

            // Attempt repair
            info!(
                "Attempting repair {} for stage '{}'",
                attempt + 1,
                stage_name
            );

            let repair_success = self.attempt_repair(stage_idx, &result, state).await?;

            if !repair_success {
                debug!("Repair generation failed or produced no changes");
                break;
            }

            // Record the repair
            state.record_repair(&stage_name)?;

            // Backoff before rerunning
            let delay = self.config.backoff.delay(attempt);
            debug!("Backoff: {:?}", delay);
            sleep(delay).await;

            // Re-run the stage
            result = self.stages[stage_idx].run(&self.config.working_dir).await?;
            attempt += 1;

            if result.success {
                info!("Stage '{}' passed after {} repair(s)", stage_name, attempt);
                return Ok(StageResult::repaired(
                    &stage_name,
                    attempt,
                    result.duration,
                    result.raw_output,
                ));
            }
        }

        // Failed after repair attempts
        Ok(StageResult::failure(
            &stage_name,
            result.failures,
            state.stage_repair_count(&stage_name),
            false,
            result.duration,
            result.raw_output,
        ))
    }

    /// Attempt to repair failures from a stage result.
    async fn attempt_repair(
        &mut self,
        stage_idx: usize,
        result: &StageResult,
        state: &mut VerificationState,
    ) -> Result<bool, VerificationError> {
        let generator = match &mut self.repair_generator {
            Some(g) => g,
            None => return Ok(false),
        };

        // Get repair hints from the stage
        let hints = self.stages[stage_idx].repair_hints(&result.failures);
        debug!("Repair hints: {:?}", hints);

        // Attempt to generate and apply repairs for each failure
        let mut any_applied = false;

        for failure in &result.failures {
            match generator.generate_repair(failure, &self.config.working_dir).await {
                Ok(Some(repair)) => {
                    debug!(
                        "Generated repair for {:?} at {:?}",
                        failure.failure_type, failure.location
                    );

                    // Apply the repair
                    match generator.apply_repair(&repair).await {
                        Ok(()) => {
                            info!("Applied repair to {}", repair.file.display());
                            state.record_modified_file(repair.file.clone());
                            any_applied = true;
                        }
                        Err(e) => {
                            error!("Failed to apply repair: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    debug!("No repair generated for failure");
                }
                Err(e) => {
                    warn!("Repair generation failed: {}", e);
                }
            }
        }

        Ok(any_applied)
    }

    /// Initialize the repair generator.
    fn initialize_repair_generator(&mut self) -> Result<(), VerificationError> {
        let adapter = match self.ollama_adapter.take() {
            Some(a) => a,
            None => {
                // Try to create a default adapter
                warn!("No Ollama adapter provided, repairs will be skipped");
                return Ok(());
            }
        };

        self.repair_generator = Some(RepairGenerator::with_config(
            adapter,
            self.config.repair.clone(),
        ));

        Ok(())
    }

    /// Run a single stage without repairs (for testing).
    pub async fn run_stage(&self, stage: &dyn VerificationStage) -> Result<StageResult, VerificationError> {
        stage.run(&self.config.working_dir).await
    }

    /// Cancel the current verification.
    pub fn cancel(&mut self) {
        info!("Verification cancelled");
        // The actual cancellation is handled through the state
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::stages::{ClippyStage, FmtStage};
    use aiy_adapter_ollama::transport::MockTransport;
    use tempfile::TempDir;

    fn create_temp_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        // Create minimal Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();
        // Create src directory
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        dir
    }

    #[test]
    fn test_engine_config_default() {
        let config = EngineConfig::new("/tmp");
        assert!(config.enable_repairs);
        assert_eq!(config.working_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_engine_config_without_repairs() {
        let config = EngineConfig::new("/tmp").without_repairs();
        assert!(!config.enable_repairs);
    }

    #[test]
    fn test_backoff_config_default() {
        let config = BackoffConfig::default();
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10_000);
        assert!((config.multiplier - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_backoff_delay_calculation() {
        let config = BackoffConfig::default();
        assert_eq!(config.delay(0), Duration::from_millis(500));
        assert_eq!(config.delay(1), Duration::from_millis(1000));
        assert_eq!(config.delay(2), Duration::from_millis(2000));
        assert_eq!(config.delay(3), Duration::from_millis(4000));
        assert_eq!(config.delay(4), Duration::from_millis(8000));
        assert_eq!(config.delay(5), Duration::from_millis(10_000)); // Capped
    }

    #[test]
    fn test_engine_creation() {
        let config = EngineConfig::new("/tmp");
        let engine = VerificationEngine::new(config);
        assert_eq!(engine.stages().len(), 3); // fmt, clippy, test
    }

    #[test]
    fn test_engine_custom_stages() {
        let config = EngineConfig::new("/tmp");
        let stages: Vec<Box<dyn VerificationStage>> = vec![
            Box::new(FmtStage::new()),
            Box::new(ClippyStage::new()),
        ];
        let engine = VerificationEngine::new(config).with_stages(stages);
        assert_eq!(engine.stages().len(), 2);
    }

    #[tokio::test]
    async fn test_engine_no_stages() {
        let config = EngineConfig::new("/tmp").without_repairs();
        let mut engine = VerificationEngine::new(config).with_stages(vec![]);

        let result = engine.run().await;
        assert!(matches!(result, Err(VerificationError::NoStagesConfigured)));
    }

    #[tokio::test]
    async fn test_engine_invalid_working_dir() {
        let config = EngineConfig::new("/nonexistent/path/12345").without_repairs();
        let mut engine = VerificationEngine::new(config);

        let result = engine.run().await;
        assert!(matches!(
            result,
            Err(VerificationError::InvalidWorkingDirectory { .. })
        ));
    }

    #[test]
    fn test_verification_result_success() {
        let stages = vec![StageResult::success(
            "fmt",
            Duration::from_millis(100),
            String::new(),
        )];
        let result = VerificationResult::success(stages, 0, Duration::from_millis(100));
        assert!(result.success);
        assert!(result.failed_stage().is_none());
    }

    #[test]
    fn test_verification_result_failure() {
        let stages = vec![
            StageResult::success("fmt", Duration::from_millis(100), String::new()),
            StageResult::failure(
                "clippy",
                vec![],
                2,
                true,
                Duration::from_millis(200),
                String::new(),
            ),
        ];
        let result = VerificationResult::failure(
            stages,
            2,
            Duration::from_millis(300),
            "clippy failed",
        );
        assert!(!result.success);
        assert!(result.failed_stage().is_some());
        assert_eq!(result.failed_stage().unwrap().stage_name, "clippy");
        assert!(result.repair_limit_exceeded());
    }

    #[tokio::test]
    async fn test_engine_with_mock_ollama() {
        let temp = create_temp_project();
        let config = EngineConfig::new(temp.path());
        let transport = Arc::new(MockTransport::new());
        let engine = VerificationEngine::with_mock_ollama(config, transport);

        assert!(engine.ollama_adapter.is_some());
    }
}
