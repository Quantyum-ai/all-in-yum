//! Core types for orchestration
//!
//! Defines task, result, and session types for the privacy mode orchestrator.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// A single task in an execution plan
///
/// Tasks are identified by opaque IDs - never real file paths or names.
#[derive(Debug, Clone)]
pub struct PlanTask {
    /// Opaque task identifier (e.g., "TASK_001")
    pub id: String,
    /// Task type
    pub task_type: TaskType,
    /// Description (redacted - no real identifiers)
    pub description: String,
    /// Dependencies (other task IDs that must complete first)
    pub dependencies: Vec<String>,
    /// Priority (lower = higher priority)
    pub priority: u8,
    /// Estimated effort (abstract units)
    pub estimated_effort: u8,
}

/// Types of tasks that can be executed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Create a new file
    CreateFile,
    /// Modify an existing file
    ModifyFile,
    /// Delete a file
    DeleteFile,
    /// Run tests
    RunTests,
    /// Run verification
    Verify,
    /// Refactor code
    Refactor,
    /// Add documentation
    Document,
    /// Generic task
    Generic,
}

impl Default for TaskType {
    fn default() -> Self {
        Self::Generic
    }
}

/// Result of executing a single task
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Whether task succeeded
    pub success: bool,
    /// Duration taken
    pub duration: Duration,
    /// Number of repair attempts (if any)
    pub repair_attempts: usize,
    /// Error message if failed (redacted)
    pub error: Option<String>,
}

impl TaskResult {
    /// Create a success result
    pub fn success(task_id: impl Into<String>, duration: Duration) -> Self {
        Self {
            task_id: task_id.into(),
            success: true,
            duration,
            repair_attempts: 0,
            error: None,
        }
    }

    /// Create a success result after repairs
    pub fn success_after_repair(
        task_id: impl Into<String>,
        duration: Duration,
        repairs: usize,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            success: true,
            duration,
            repair_attempts: repairs,
            error: None,
        }
    }

    /// Create a failure result
    pub fn failure(task_id: impl Into<String>, duration: Duration, error: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            success: false,
            duration,
            repair_attempts: 0,
            error: Some(error.into()),
        }
    }

    /// Create a failure result after repair attempts
    pub fn failure_after_repair(
        task_id: impl Into<String>,
        duration: Duration,
        repairs: usize,
        error: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            success: false,
            duration,
            repair_attempts: repairs,
            error: Some(error.into()),
        }
    }
}

/// Execution plan from cloud planner
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Plan identifier
    pub plan_id: String,
    /// Tasks to execute (in dependency order)
    pub tasks: Vec<PlanTask>,
    /// Overall description (redacted)
    pub description: String,
    /// Created timestamp
    pub created_at: Instant,
}

impl ExecutionPlan {
    /// Create a new execution plan
    pub fn new(plan_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            tasks: Vec::new(),
            description: description.into(),
            created_at: Instant::now(),
        }
    }

    /// Add a task to the plan
    pub fn add_task(&mut self, task: PlanTask) {
        self.tasks.push(task);
    }

    /// Get task count
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Check if plan is empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get tasks sorted by dependency order
    pub fn sorted_tasks(&self) -> Vec<&PlanTask> {
        // Simple topological sort
        let mut result = Vec::new();
        let mut completed: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut remaining: Vec<&PlanTask> = self.tasks.iter().collect();

        while !remaining.is_empty() {
            let before_len = remaining.len();

            remaining.retain(|task| {
                let deps_satisfied = task
                    .dependencies
                    .iter()
                    .all(|dep| completed.contains(dep.as_str()));

                if deps_satisfied {
                    completed.insert(&task.id);
                    result.push(*task);
                    false // Remove from remaining
                } else {
                    true // Keep in remaining
                }
            });

            // Detect circular dependencies
            if remaining.len() == before_len && !remaining.is_empty() {
                // Add remaining tasks in order (circular dep detected)
                result.extend(remaining.drain(..));
            }
        }

        result
    }
}

/// Statistics about orchestration session (SAFE to persist)
///
/// Contains ONLY metadata - no identifiers, file paths, or sensitive info.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total tasks executed
    pub tasks_executed: usize,
    /// Tasks that succeeded
    pub tasks_succeeded: usize,
    /// Tasks that failed
    pub tasks_failed: usize,
    /// Total repair attempts
    pub total_repairs: usize,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Files modified count
    pub files_modified_count: usize,
    /// Chunks indexed
    pub chunks_indexed: usize,
    /// Cloud requests made
    pub cloud_requests: usize,
    /// Local executions
    pub local_executions: usize,
}

impl SessionStats {
    /// Record a successful task
    pub fn record_success(&mut self, repairs: usize) {
        self.tasks_executed += 1;
        self.tasks_succeeded += 1;
        self.total_repairs += repairs;
    }

    /// Record a failed task
    pub fn record_failure(&mut self, repairs: usize) {
        self.tasks_executed += 1;
        self.tasks_failed += 1;
        self.total_repairs += repairs;
    }

    /// Record duration
    pub fn record_duration(&mut self, duration: Duration) {
        self.duration_ms += duration.as_millis() as u64;
    }

    /// Record cloud request
    pub fn record_cloud_request(&mut self) {
        self.cloud_requests += 1;
    }

    /// Record local execution
    pub fn record_local_execution(&mut self) {
        self.local_executions += 1;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.tasks_executed == 0 {
            1.0
        } else {
            self.tasks_succeeded as f64 / self.tasks_executed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_task() {
        let task = PlanTask {
            id: "TASK_001".to_string(),
            task_type: TaskType::ModifyFile,
            description: "Update FILE_001".to_string(),
            dependencies: vec![],
            priority: 1,
            estimated_effort: 2,
        };
        assert_eq!(task.id, "TASK_001");
        assert_eq!(task.task_type, TaskType::ModifyFile);
    }

    #[test]
    fn test_task_result_success() {
        let result = TaskResult::success("TASK_001", Duration::from_secs(1));
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.repair_attempts, 0);
    }

    #[test]
    fn test_task_result_failure() {
        let result = TaskResult::failure("TASK_001", Duration::from_secs(1), "compile error");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_execution_plan() {
        let mut plan = ExecutionPlan::new("PLAN_001", "Implement feature");
        assert!(plan.is_empty());

        plan.add_task(PlanTask {
            id: "TASK_001".to_string(),
            task_type: TaskType::CreateFile,
            description: "Create FILE_001".to_string(),
            dependencies: vec![],
            priority: 1,
            estimated_effort: 2,
        });

        plan.add_task(PlanTask {
            id: "TASK_002".to_string(),
            task_type: TaskType::ModifyFile,
            description: "Modify FILE_002".to_string(),
            dependencies: vec!["TASK_001".to_string()],
            priority: 2,
            estimated_effort: 3,
        });

        assert_eq!(plan.task_count(), 2);

        // Check dependency ordering
        let sorted = plan.sorted_tasks();
        assert_eq!(sorted[0].id, "TASK_001");
        assert_eq!(sorted[1].id, "TASK_002");
    }

    #[test]
    fn test_session_stats() {
        let mut stats = SessionStats::default();
        stats.record_success(0);
        stats.record_success(2);
        stats.record_failure(1);

        assert_eq!(stats.tasks_executed, 3);
        assert_eq!(stats.tasks_succeeded, 2);
        assert_eq!(stats.tasks_failed, 1);
        assert_eq!(stats.total_repairs, 3);

        // Success rate should be 2/3
        assert!((stats.success_rate() - 0.666).abs() < 0.01);
    }
}
