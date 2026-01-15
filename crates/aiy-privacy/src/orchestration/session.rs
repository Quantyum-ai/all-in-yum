//! Session management for orchestration
//!
//! Manages in-memory state for orchestration sessions.
//! Session data is NOT persisted - only metadata can be saved.

use super::error::{OrchestrationError, OrchestrationResult};
use super::types::{ExecutionPlan, SessionStats, TaskResult};
use crate::enforcement::{RedactionMap, Redactor};
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

/// Session state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session created but not initialized
    Created,
    /// Directory indexed, ready to execute
    Ready,
    /// Currently executing a plan
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

impl Default for SessionState {
    fn default() -> Self {
        Self::Created
    }
}

impl SessionState {
    /// Check if session can transition to a new state
    pub fn can_transition_to(&self, target: SessionState) -> bool {
        use SessionState::*;

        match (self, target) {
            // From Created
            (Created, Ready) => true,
            (Created, Failed) => true,
            (Created, Cancelled) => true,

            // From Ready
            (Ready, Executing) => true,
            (Ready, Failed) => true,
            (Ready, Cancelled) => true,

            // From Executing
            (Executing, Paused) => true,
            (Executing, Completed) => true,
            (Executing, Failed) => true,
            (Executing, Cancelled) => true,

            // From Paused
            (Paused, Executing) => true,
            (Paused, Failed) => true,
            (Paused, Cancelled) => true,

            // Terminal states cannot transition
            (Completed, _) => false,
            (Failed, _) => false,
            (Cancelled, _) => false,

            // Same state is not a valid transition
            _ => false,
        }
    }
}

/// Orchestration session
///
/// Contains all in-memory state for a privacy mode orchestration session.
///
/// # Security
///
/// - The [`RedactionMap`] inside this session is NEVER serialized
/// - Only [`SessionStats`] (metadata) can be persisted
/// - Session ID and state can be persisted, but not task details
pub struct OrchestrationSession {
    /// Unique session identifier
    id: Uuid,
    /// Current state
    state: SessionState,
    /// Working directory
    working_dir: PathBuf,
    /// Redactor with session-scoped map
    redactor: Redactor,
    /// Current execution plan (if any)
    plan: Option<ExecutionPlan>,
    /// Task results
    task_results: Vec<TaskResult>,
    /// Session statistics (safe to persist)
    stats: SessionStats,
    /// Session start time
    started_at: Instant,
    /// Completed task indices (for resumption)
    completed_task_indices: Vec<usize>,
}

impl OrchestrationSession {
    /// Create a new orchestration session
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: SessionState::Created,
            working_dir: working_dir.into(),
            redactor: Redactor::new(),
            plan: None,
            task_results: Vec::new(),
            stats: SessionStats::default(),
            started_at: Instant::now(),
            completed_task_indices: Vec::new(),
        }
    }

    /// Get session ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get current state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get working directory
    pub fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Get redactor
    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// Get mutable redactor
    pub fn redactor_mut(&mut self) -> &mut Redactor {
        &mut self.redactor
    }

    /// Get redaction map
    pub fn redaction_map(&self) -> &RedactionMap {
        self.redactor.map()
    }

    /// Get current plan
    pub fn plan(&self) -> Option<&ExecutionPlan> {
        self.plan.as_ref()
    }

    /// Get task results
    pub fn task_results(&self) -> &[TaskResult] {
        &self.task_results
    }

    /// Get session statistics
    pub fn stats(&self) -> &SessionStats {
        &self.stats
    }

    /// Get mutable statistics
    pub fn stats_mut(&mut self) -> &mut SessionStats {
        &mut self.stats
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Transition to a new state
    pub fn transition(&mut self, target: SessionState) -> OrchestrationResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(OrchestrationError::InvalidState(format!(
                "Cannot transition from {:?} to {:?}",
                self.state, target
            )));
        }
        self.state = target;
        Ok(())
    }

    /// Mark as ready (after indexing)
    pub fn mark_ready(&mut self) -> OrchestrationResult<()> {
        self.transition(SessionState::Ready)
    }

    /// Set execution plan and start executing
    pub fn start_execution(&mut self, plan: ExecutionPlan) -> OrchestrationResult<()> {
        self.transition(SessionState::Executing)?;
        self.plan = Some(plan);
        Ok(())
    }

    /// Pause execution
    pub fn pause(&mut self) -> OrchestrationResult<()> {
        self.transition(SessionState::Paused)
    }

    /// Resume execution
    pub fn resume(&mut self) -> OrchestrationResult<()> {
        self.transition(SessionState::Executing)
    }

    /// Mark as completed
    pub fn complete(&mut self) -> OrchestrationResult<()> {
        self.transition(SessionState::Completed)
    }

    /// Mark as failed
    pub fn fail(&mut self) -> OrchestrationResult<()> {
        self.transition(SessionState::Failed)
    }

    /// Mark as cancelled
    pub fn cancel(&mut self) -> OrchestrationResult<()> {
        self.transition(SessionState::Cancelled)
    }

    /// Record a task result
    pub fn record_task_result(&mut self, result: TaskResult) {
        if result.success {
            self.stats.record_success(result.repair_attempts);
        } else {
            self.stats.record_failure(result.repair_attempts);
        }
        self.stats.record_duration(result.duration);
        self.task_results.push(result);
    }

    /// Mark task as completed (for resumption)
    pub fn mark_task_completed(&mut self, index: usize) {
        if !self.completed_task_indices.contains(&index) {
            self.completed_task_indices.push(index);
        }
    }

    /// Get next task index to execute
    pub fn next_task_index(&self) -> Option<usize> {
        let plan = self.plan.as_ref()?;
        for i in 0..plan.tasks.len() {
            if !self.completed_task_indices.contains(&i) {
                return Some(i);
            }
        }
        None
    }

    /// Check if all tasks are completed
    pub fn all_tasks_completed(&self) -> bool {
        if let Some(plan) = &self.plan {
            self.completed_task_indices.len() >= plan.tasks.len()
        } else {
            true
        }
    }

    /// Clear session (for security - zeros sensitive data)
    pub fn clear(&mut self) {
        self.redactor.clear();
        self.plan = None;
        self.task_results.clear();
        self.completed_task_indices.clear();
    }
}

impl Drop for OrchestrationSession {
    fn drop(&mut self) {
        // Ensure redaction map is cleared on drop
        self.redactor.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = OrchestrationSession::new("/tmp/test");
        assert_eq!(session.state(), SessionState::Created);
        assert!(session.plan().is_none());
    }

    #[test]
    fn test_session_state_transitions() {
        let mut session = OrchestrationSession::new("/tmp/test");

        // Created -> Ready
        assert!(session.mark_ready().is_ok());
        assert_eq!(session.state(), SessionState::Ready);

        // Ready -> Executing
        let plan = ExecutionPlan::new("PLAN_001", "Test plan");
        assert!(session.start_execution(plan).is_ok());
        assert_eq!(session.state(), SessionState::Executing);

        // Executing -> Paused
        assert!(session.pause().is_ok());
        assert_eq!(session.state(), SessionState::Paused);

        // Paused -> Executing
        assert!(session.resume().is_ok());
        assert_eq!(session.state(), SessionState::Executing);

        // Executing -> Completed
        assert!(session.complete().is_ok());
        assert_eq!(session.state(), SessionState::Completed);

        // Cannot transition from Completed
        assert!(session.fail().is_err());
    }

    #[test]
    fn test_invalid_transition() {
        let mut session = OrchestrationSession::new("/tmp/test");

        // Cannot go from Created directly to Executing
        let plan = ExecutionPlan::new("PLAN_001", "Test");
        assert!(session.start_execution(plan).is_err());
    }

    #[test]
    fn test_task_result_recording() {
        let mut session = OrchestrationSession::new("/tmp/test");
        session.mark_ready().unwrap();

        let plan = ExecutionPlan::new("PLAN_001", "Test");
        session.start_execution(plan).unwrap();

        session.record_task_result(TaskResult::success(
            "TASK_001",
            std::time::Duration::from_secs(1),
        ));
        session.record_task_result(TaskResult::failure(
            "TASK_002",
            std::time::Duration::from_secs(2),
            "error",
        ));

        assert_eq!(session.stats().tasks_executed, 2);
        assert_eq!(session.stats().tasks_succeeded, 1);
        assert_eq!(session.stats().tasks_failed, 1);
        assert_eq!(session.task_results().len(), 2);
    }

    #[test]
    fn test_session_state_can_transition() {
        use SessionState::*;

        assert!(Created.can_transition_to(Ready));
        assert!(Ready.can_transition_to(Executing));
        assert!(Executing.can_transition_to(Completed));

        assert!(!Created.can_transition_to(Executing));
        assert!(!Completed.can_transition_to(Ready));
    }
}
