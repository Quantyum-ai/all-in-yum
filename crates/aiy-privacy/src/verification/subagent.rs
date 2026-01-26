//! Subagent coordination for parallel verification.

use super::error::VerificationError;
use super::stages::VerificationStage;
use super::types::StageResult;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Coordinates parallel execution of verification stages.
pub struct SubagentCoordinator {
    max_parallelism: usize,
    semaphore: Arc<Semaphore>,
}

impl SubagentCoordinator {
    /// Create a new coordinator with specified max parallelism
    pub fn new(max_parallelism: usize) -> Self {
        Self {
            max_parallelism,
            semaphore: Arc::new(Semaphore::new(max_parallelism)),
        }
    }

    /// Get the max parallelism setting
    pub fn max_parallelism(&self) -> usize {
        self.max_parallelism
    }

    /// Run multiple verification stages in parallel
    pub async fn run_stages_parallel(
        &self,
        stages: Vec<Box<dyn VerificationStage>>,
        working_dir: &Path,
    ) -> Vec<Result<StageResult, VerificationError>> {
        let mut handles = Vec::new();
        let working_dir = working_dir.to_path_buf();

        for stage in stages {
            let sem = self.semaphore.clone();
            let wd = working_dir.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                stage.run(&wd).await
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(VerificationError::cancelled(e.to_string()))),
            }
        }
        results
    }
}

impl Default for SubagentCoordinator {
    fn default() -> Self {
        Self::new(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_new() {
        let coord = SubagentCoordinator::new(8);
        assert_eq!(coord.max_parallelism(), 8);
    }

    #[test]
    fn test_coordinator_default() {
        let coord = SubagentCoordinator::default();
        assert_eq!(coord.max_parallelism(), 4);
    }
}
