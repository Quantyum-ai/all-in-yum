//! Verification state management.
//!
//! This module provides the `VerificationState` struct that tracks repair attempts,
//! enforces limits, and manages the verification pipeline state.

use super::error::VerificationError;
use super::types::{RepairSummary, StageResult};
use aiy_core::config::VerificationConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// State machine for tracking verification progress and repair limits.
///
/// This struct enforces both per-stage and global repair limits as defined
/// in the `VerificationConfig`.
#[derive(Debug)]
pub struct VerificationState {
    /// Configuration for repair limits
    config: VerificationConfig,
    /// Per-stage repair attempt counts
    stage_repairs: HashMap<String, usize>,
    /// Total repairs across all stages
    global_repairs: usize,
    /// Stage results collected so far
    stage_results: Vec<StageResult>,
    /// Start time of verification
    start_time: Instant,
    /// Current stage being executed (if any)
    current_stage: Option<String>,
    /// Files modified during repair
    modified_files: Vec<PathBuf>,
    /// Whether verification was cancelled
    cancelled: bool,
}

impl VerificationState {
    /// Create a new verification state with the given configuration.
    pub fn new(config: VerificationConfig) -> Self {
        Self {
            config,
            stage_repairs: HashMap::new(),
            global_repairs: 0,
            stage_results: Vec::new(),
            start_time: Instant::now(),
            current_stage: None,
            modified_files: Vec::new(),
            cancelled: false,
        }
    }

    /// Create a new verification state with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(VerificationConfig::default())
    }

    /// Get the current configuration.
    pub fn config(&self) -> &VerificationConfig {
        &self.config
    }

    /// Get the repair limit for a specific stage.
    pub fn stage_limit(&self, stage_name: &str) -> usize {
        match stage_name {
            "fmt" => self.config.max_fmt_repairs,
            "clippy" => self.config.max_clippy_repairs,
            "test" => self.config.max_test_repairs,
            _ => 0, // Unknown stages get no repairs
        }
    }

    /// Get the global repair limit.
    pub fn global_limit(&self) -> usize {
        self.config.max_global_repairs
    }

    /// Get the number of repairs for a specific stage.
    pub fn stage_repair_count(&self, stage_name: &str) -> usize {
        *self.stage_repairs.get(stage_name).unwrap_or(&0)
    }

    /// Get the total number of global repairs.
    pub fn global_repair_count(&self) -> usize {
        self.global_repairs
    }

    /// Check if a repair can be attempted for a stage.
    ///
    /// Returns `Ok(())` if repair is allowed, or `Err` with the appropriate limit error.
    pub fn can_repair(&self, stage_name: &str) -> Result<(), VerificationError> {
        // Check global limit first
        if self.global_repairs >= self.config.max_global_repairs {
            return Err(VerificationError::global_limit_exceeded(
                self.global_repairs,
                self.config.max_global_repairs,
            ));
        }

        // Check stage-specific limit
        let stage_count = self.stage_repair_count(stage_name);
        let stage_limit = self.stage_limit(stage_name);

        if stage_count >= stage_limit {
            return Err(VerificationError::stage_limit_exceeded(
                stage_name,
                stage_count,
                stage_limit,
            ));
        }

        Ok(())
    }

    /// Record a repair attempt for a stage.
    ///
    /// Returns `Err` if the repair would exceed limits (call `can_repair` first).
    pub fn record_repair(&mut self, stage_name: &str) -> Result<(), VerificationError> {
        self.can_repair(stage_name)?;

        *self.stage_repairs.entry(stage_name.to_string()).or_insert(0) += 1;
        self.global_repairs += 1;

        Ok(())
    }

    /// Record a file that was modified during repair.
    pub fn record_modified_file(&mut self, path: PathBuf) {
        if !self.modified_files.contains(&path) {
            self.modified_files.push(path);
        }
    }

    /// Get the list of modified files.
    pub fn modified_files(&self) -> &[PathBuf] {
        &self.modified_files
    }

    /// Set the current stage being executed.
    pub fn enter_stage(&mut self, stage_name: &str) {
        self.current_stage = Some(stage_name.to_string());
    }

    /// Clear the current stage.
    pub fn exit_stage(&mut self) {
        self.current_stage = None;
    }

    /// Get the current stage name.
    pub fn current_stage(&self) -> Option<&str> {
        self.current_stage.as_deref()
    }

    /// Record a stage result.
    pub fn record_stage_result(&mut self, result: StageResult) {
        self.stage_results.push(result);
        self.exit_stage();
    }

    /// Get all stage results.
    pub fn stage_results(&self) -> &[StageResult] {
        &self.stage_results
    }

    /// Take ownership of stage results.
    pub fn take_stage_results(&mut self) -> Vec<StageResult> {
        std::mem::take(&mut self.stage_results)
    }

    /// Get the elapsed time since verification started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Cancel the verification.
    pub fn cancel(&mut self, reason: &str) {
        self.cancelled = true;
        tracing::info!("Verification cancelled: {}", reason);
    }

    /// Check if verification was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Get remaining repairs for a stage.
    pub fn remaining_stage_repairs(&self, stage_name: &str) -> usize {
        let limit = self.stage_limit(stage_name);
        let used = self.stage_repair_count(stage_name);
        limit.saturating_sub(used)
    }

    /// Get remaining global repairs.
    pub fn remaining_global_repairs(&self) -> usize {
        self.config
            .max_global_repairs
            .saturating_sub(self.global_repairs)
    }

    /// Build a repair summary.
    pub fn repair_summary(&self) -> RepairSummary {
        RepairSummary {
            by_stage: self.stage_repairs.clone(),
            total: self.global_repairs,
            modified_files: self.modified_files.clone(),
        }
    }

    /// Check if all stages passed.
    pub fn all_passed(&self) -> bool {
        self.stage_results.iter().all(|r| r.success)
    }

    /// Get the first failed stage, if any.
    pub fn first_failure(&self) -> Option<&StageResult> {
        self.stage_results.iter().find(|r| !r.success)
    }

    /// Reset state for a new verification run (for testing).
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.stage_repairs.clear();
        self.global_repairs = 0;
        self.stage_results.clear();
        self.start_time = Instant::now();
        self.current_stage = None;
        self.modified_files.clear();
        self.cancelled = false;
    }
}

impl Default for VerificationState {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    fn test_config() -> VerificationConfig {
        VerificationConfig {
            max_fmt_repairs: 2,
            max_clippy_repairs: 2,
            max_test_repairs: 1,
            max_global_repairs: 8,
        }
    }

    #[test]
    fn test_new_state() {
        let state = VerificationState::new(test_config());
        assert_eq!(state.global_repair_count(), 0);
        assert_eq!(state.stage_repair_count("fmt"), 0);
        assert!(state.stage_results().is_empty());
    }

    #[test]
    fn test_stage_limits() {
        let state = VerificationState::new(test_config());
        assert_eq!(state.stage_limit("fmt"), 2);
        assert_eq!(state.stage_limit("clippy"), 2);
        assert_eq!(state.stage_limit("test"), 1);
        assert_eq!(state.stage_limit("unknown"), 0);
    }

    #[test]
    fn test_can_repair_allowed() {
        let state = VerificationState::new(test_config());
        assert!(state.can_repair("fmt").is_ok());
        assert!(state.can_repair("clippy").is_ok());
        assert!(state.can_repair("test").is_ok());
    }

    #[test]
    fn test_record_repair() {
        let mut state = VerificationState::new(test_config());

        assert!(state.record_repair("fmt").is_ok());
        assert_eq!(state.stage_repair_count("fmt"), 1);
        assert_eq!(state.global_repair_count(), 1);

        assert!(state.record_repair("fmt").is_ok());
        assert_eq!(state.stage_repair_count("fmt"), 2);
        assert_eq!(state.global_repair_count(), 2);
    }

    #[test]
    fn test_stage_limit_exceeded() {
        let mut state = VerificationState::new(test_config());

        // Use up fmt repairs
        state.record_repair("fmt").unwrap();
        state.record_repair("fmt").unwrap();

        // Third should fail
        let result = state.record_repair("fmt");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VerificationError::StageLimitExceeded { stage, attempts, limit }
            if stage == "fmt" && attempts == 2 && limit == 2
        ));
    }

    #[test]
    fn test_global_limit_exceeded() {
        let config = VerificationConfig {
            max_fmt_repairs: 5,
            max_clippy_repairs: 5,
            max_test_repairs: 5,
            max_global_repairs: 3, // Low global limit
        };
        let mut state = VerificationState::new(config);

        state.record_repair("fmt").unwrap();
        state.record_repair("clippy").unwrap();
        state.record_repair("test").unwrap();

        // Fourth should fail globally
        let result = state.record_repair("fmt");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VerificationError::GlobalLimitExceeded { attempts, limit }
            if attempts == 3 && limit == 3
        ));
    }

    #[test]
    fn test_remaining_repairs() {
        let mut state = VerificationState::new(test_config());
        assert_eq!(state.remaining_stage_repairs("fmt"), 2);
        assert_eq!(state.remaining_global_repairs(), 8);

        state.record_repair("fmt").unwrap();
        assert_eq!(state.remaining_stage_repairs("fmt"), 1);
        assert_eq!(state.remaining_global_repairs(), 7);
    }

    #[test]
    fn test_stage_lifecycle() {
        let mut state = VerificationState::new(test_config());

        assert!(state.current_stage().is_none());

        state.enter_stage("fmt");
        assert_eq!(state.current_stage(), Some("fmt"));

        state.exit_stage();
        assert!(state.current_stage().is_none());
    }

    #[test]
    fn test_record_stage_result() {
        let mut state = VerificationState::new(test_config());

        state.enter_stage("fmt");
        let result = StageResult::success("fmt", StdDuration::from_millis(100), String::new());
        state.record_stage_result(result);

        assert!(state.current_stage().is_none());
        assert_eq!(state.stage_results().len(), 1);
        assert!(state.all_passed());
    }

    #[test]
    fn test_first_failure() {
        let mut state = VerificationState::new(test_config());

        state.record_stage_result(StageResult::success(
            "fmt",
            StdDuration::from_millis(100),
            String::new(),
        ));
        state.record_stage_result(StageResult::failure(
            "clippy",
            vec![],
            1,
            false,
            StdDuration::from_millis(200),
            String::new(),
        ));

        assert!(!state.all_passed());
        let failure = state.first_failure().unwrap();
        assert_eq!(failure.stage_name, "clippy");
    }

    #[test]
    fn test_modified_files() {
        let mut state = VerificationState::new(test_config());

        state.record_modified_file(PathBuf::from("src/main.rs"));
        state.record_modified_file(PathBuf::from("src/lib.rs"));
        state.record_modified_file(PathBuf::from("src/main.rs")); // Duplicate

        assert_eq!(state.modified_files().len(), 2);
    }

    #[test]
    fn test_cancellation() {
        let mut state = VerificationState::new(test_config());

        assert!(!state.is_cancelled());
        state.cancel("user request");
        assert!(state.is_cancelled());
    }

    #[test]
    fn test_repair_summary() {
        let mut state = VerificationState::new(test_config());

        state.record_repair("fmt").unwrap();
        state.record_repair("fmt").unwrap();
        state.record_repair("clippy").unwrap();
        state.record_modified_file(PathBuf::from("src/main.rs"));

        let summary = state.repair_summary();
        assert_eq!(summary.total, 3);
        assert_eq!(*summary.by_stage.get("fmt").unwrap(), 2);
        assert_eq!(*summary.by_stage.get("clippy").unwrap(), 1);
        assert_eq!(summary.modified_files.len(), 1);
    }

    #[test]
    fn test_elapsed() {
        let state = VerificationState::new(test_config());
        std::thread::sleep(StdDuration::from_millis(10));
        assert!(state.elapsed() >= StdDuration::from_millis(10));
    }

    #[test]
    fn test_take_stage_results() {
        let mut state = VerificationState::new(test_config());
        state.record_stage_result(StageResult::success(
            "fmt",
            StdDuration::from_millis(100),
            String::new(),
        ));

        let results = state.take_stage_results();
        assert_eq!(results.len(), 1);
        assert!(state.stage_results().is_empty());
    }

    #[test]
    fn test_default_state() {
        let state = VerificationState::default();
        // Uses VerificationConfig::default() which has 2/2/1/8 limits
        assert_eq!(state.stage_limit("fmt"), 2);
        assert_eq!(state.global_limit(), 8);
    }
}
