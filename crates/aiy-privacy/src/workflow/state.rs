//! Workflow state - serializable metadata only
//!
//! This module provides workflow state that can be safely persisted.
//!
//! # Security
//!
//! WorkflowState contains ONLY metadata:
//! - Counts (tasks, repairs)
//! - Timestamps
//! - State names
//!
//! It NEVER contains:
//! - File paths (real or opaque)
//! - Code content
//! - Identifier mappings
//! - Any information that could leak code structure

use super::error::{WorkflowError, WorkflowResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

/// Workflow state names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStateName {
    /// Not yet initialized
    Uninitialized,
    /// Indexed and ready
    Ready,
    /// Currently executing
    Executing,
    /// Paused mid-execution
    Paused,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

impl Default for WorkflowStateName {
    fn default() -> Self {
        Self::Uninitialized
    }
}

/// Serializable workflow state
///
/// Contains ONLY metadata - no identifiers, paths, or code.
///
/// # Security
///
/// This structure is designed to be safe to persist to disk.
/// It contains only:
/// - Aggregate counts
/// - Timestamps
/// - State flags
///
/// The following are NEVER stored:
/// - File paths (even opaque ones like FILE_001)
/// - Code snippets
/// - Redaction mappings
/// - Task descriptions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Current state
    pub state: WorkflowStateName,

    /// When workflow was created (Unix timestamp)
    #[serde(default)]
    pub created_at: u64,

    /// When workflow was last updated (Unix timestamp)
    #[serde(default)]
    pub updated_at: u64,

    /// Total tasks in plan
    #[serde(default)]
    pub total_tasks: usize,

    /// Tasks completed
    #[serde(default)]
    pub tasks_completed: usize,

    /// Tasks failed
    #[serde(default)]
    pub tasks_failed: usize,

    /// Total repair attempts
    #[serde(default)]
    pub total_repairs: usize,

    /// Files modified count (not paths!)
    #[serde(default)]
    pub files_modified_count: usize,

    /// Chunks indexed
    #[serde(default)]
    pub chunks_indexed: usize,

    /// Cloud requests made
    #[serde(default)]
    pub cloud_requests: usize,

    /// Local executions
    #[serde(default)]
    pub local_executions: usize,

    /// Total duration in milliseconds
    #[serde(default)]
    pub duration_ms: u64,

    /// Error message (if failed) - generic, no identifiers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl WorkflowState {
    /// Create a new workflow state
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            state: WorkflowStateName::Uninitialized,
            created_at: now,
            updated_at: now,
            ..Default::default()
        }
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Update from session stats
    pub fn update_from_stats(&mut self, stats: &crate::orchestration::SessionStats) {
        self.tasks_completed = stats.tasks_succeeded;
        self.tasks_failed = stats.tasks_failed;
        self.total_repairs = stats.total_repairs;
        self.files_modified_count = stats.files_modified_count;
        self.chunks_indexed = stats.chunks_indexed;
        self.cloud_requests = stats.cloud_requests;
        self.local_executions = stats.local_executions;
        self.duration_ms = stats.duration_ms;
        self.touch();
    }

    /// Mark as ready
    pub fn mark_ready(&mut self, chunks_indexed: usize) {
        self.state = WorkflowStateName::Ready;
        self.chunks_indexed = chunks_indexed;
        self.touch();
    }

    /// Mark as executing with task count
    pub fn mark_executing(&mut self, total_tasks: usize) {
        self.state = WorkflowStateName::Executing;
        self.total_tasks = total_tasks;
        self.touch();
    }

    /// Mark as paused
    pub fn mark_paused(&mut self) {
        self.state = WorkflowStateName::Paused;
        self.touch();
    }

    /// Mark as completed
    pub fn mark_completed(&mut self) {
        self.state = WorkflowStateName::Completed;
        self.touch();
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.state = WorkflowStateName::Failed;
        // Sanitize error message to remove any potential identifiers
        let error_msg = error.into();
        self.error_message = Some(Self::sanitize_error(&error_msg));
        self.touch();
    }

    /// Mark as cancelled
    pub fn mark_cancelled(&mut self) {
        self.state = WorkflowStateName::Cancelled;
        self.touch();
    }

    /// Check if workflow can be resumed
    pub fn can_resume(&self) -> bool {
        matches!(self.state, WorkflowStateName::Paused)
    }

    /// Check if workflow is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            WorkflowStateName::Ready | WorkflowStateName::Executing | WorkflowStateName::Paused
        )
    }

    /// Check if workflow is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            WorkflowStateName::Completed
                | WorkflowStateName::Failed
                | WorkflowStateName::Cancelled
        )
    }

    /// Calculate progress percentage
    pub fn progress_percent(&self) -> u8 {
        if self.total_tasks == 0 {
            if self.state == WorkflowStateName::Completed {
                100
            } else {
                0
            }
        } else {
            ((self.tasks_completed * 100) / self.total_tasks).min(100) as u8
        }
    }

    /// Sanitize error message to remove potential identifiers
    fn sanitize_error(error: &str) -> String {
        // Remove anything that looks like a file path
        let path_regex = regex::Regex::new(r"[/\\][\w/\\.-]+\.\w+").unwrap();
        let sanitized = path_regex.replace_all(error, "[PATH]");

        // Remove anything that looks like an identifier (snake_case or camelCase)
        let ident_regex = regex::Regex::new(r"\b[a-z_][a-z0-9_]*[A-Z][a-zA-Z0-9]*\b").unwrap();
        let sanitized = ident_regex.replace_all(&sanitized, "[IDENT]");

        // Truncate to prevent very long messages
        if sanitized.len() > 200 {
            format!("{}...", &sanitized[..197])
        } else {
            sanitized.to_string()
        }
    }

    /// Load from file
    pub fn load(path: &Path) -> WorkflowResult<Self> {
        let contents = std::fs::read_to_string(path)?;
        let state: Self = serde_json::from_str(&contents)
            .map_err(|e| WorkflowError::serialization(e.to_string()))?;
        Ok(state)
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> WorkflowResult<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| WorkflowError::serialization(e.to_string()))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Get state file path for a directory
    pub fn state_path(dir: &Path) -> std::path::PathBuf {
        dir.join(".aiy").join("workflow-state.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workflow_state_new() {
        let state = WorkflowState::new();
        assert_eq!(state.state, WorkflowStateName::Uninitialized);
        assert!(state.created_at > 0);
    }

    #[test]
    fn test_workflow_state_transitions() {
        let mut state = WorkflowState::new();

        state.mark_ready(100);
        assert_eq!(state.state, WorkflowStateName::Ready);
        assert_eq!(state.chunks_indexed, 100);

        state.mark_executing(5);
        assert_eq!(state.state, WorkflowStateName::Executing);
        assert_eq!(state.total_tasks, 5);

        state.mark_paused();
        assert_eq!(state.state, WorkflowStateName::Paused);
        assert!(state.can_resume());

        state.mark_completed();
        assert_eq!(state.state, WorkflowStateName::Completed);
        assert!(state.is_terminal());
    }

    #[test]
    fn test_workflow_state_failed() {
        let mut state = WorkflowState::new();
        state.mark_failed("Task failed");
        assert_eq!(state.state, WorkflowStateName::Failed);
        assert!(state.error_message.is_some());
    }

    #[test]
    fn test_progress_percent() {
        let mut state = WorkflowState::new();
        state.total_tasks = 10;
        state.tasks_completed = 5;
        assert_eq!(state.progress_percent(), 50);

        state.tasks_completed = 10;
        assert_eq!(state.progress_percent(), 100);
    }

    #[test]
    fn test_sanitize_error() {
        let error = "Error in /home/user/project/src/main.rs with someFunction";
        let sanitized = WorkflowState::sanitize_error(error);
        assert!(!sanitized.contains("/home/user"));
        assert!(sanitized.contains("[PATH]"));
    }

    #[test]
    fn test_save_load() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("state.json");

        let mut state = WorkflowState::new();
        state.mark_ready(50);
        state.save(&path).unwrap();

        let loaded = WorkflowState::load(&path).unwrap();
        assert_eq!(loaded.state, WorkflowStateName::Ready);
        assert_eq!(loaded.chunks_indexed, 50);
    }

    #[test]
    fn test_state_path() {
        let path = WorkflowState::state_path(Path::new("/project"));
        assert!(path.ends_with(".aiy/workflow-state.json"));
    }
}
