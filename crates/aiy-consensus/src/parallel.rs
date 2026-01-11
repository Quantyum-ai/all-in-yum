//! Parallel execution utilities for running agents concurrently.

use crate::error::ConsensusError;
use crate::types::AgentOutcome;
use aiy_adapters::{AgentAdapter, AgentReview};
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for agent reviews (30 seconds).
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Execute a single agent review with timeout handling.
pub async fn execute_agent_with_timeout(
    adapter: &dyn AgentAdapter,
    artifact: &str,
    timeout_duration: Duration,
) -> AgentOutcome {
    let agent_id = adapter.id().to_string();

    match timeout(timeout_duration, adapter.review_artifact(artifact)).await {
        Ok(Ok(review)) => AgentOutcome::Success(review),
        Ok(Err(e)) => AgentOutcome::Failure {
            agent_id,
            error: e.message().to_string(),
        },
        Err(_) => AgentOutcome::Timeout {
            agent_id,
            timeout_ms: timeout_duration.as_millis() as u64,
        },
    }
}

/// Execute multiple agents in parallel and collect results.
///
/// This function spawns concurrent tasks for each adapter and waits for all
/// to complete (or timeout). Results are returned in completion order, not
/// submission order.
pub async fn execute_agents_parallel(
    adapters: &[Box<dyn AgentAdapter>],
    artifact: &str,
    timeout_duration: Duration,
) -> Vec<AgentOutcome> {
    use futures::future::join_all;

    let futures = adapters.iter().map(|adapter| {
        let artifact = artifact.to_string();
        let timeout_dur = timeout_duration;
        async move { execute_agent_with_timeout(adapter.as_ref(), &artifact, timeout_dur).await }
    });

    join_all(futures).await
}

/// Execute multiple agents sequentially (useful for testing/debugging).
pub async fn execute_agents_sequential(
    adapters: &[Box<dyn AgentAdapter>],
    artifact: &str,
    timeout_duration: Duration,
) -> Vec<AgentOutcome> {
    let mut outcomes = Vec::with_capacity(adapters.len());

    for adapter in adapters {
        let outcome = execute_agent_with_timeout(adapter.as_ref(), artifact, timeout_duration).await;
        outcomes.push(outcome);
    }

    outcomes
}

/// Filter successful outcomes and extract reviews.
pub fn extract_successful_reviews(outcomes: &[AgentOutcome]) -> Vec<AgentReview> {
    outcomes
        .iter()
        .filter_map(|o| o.review().cloned())
        .collect()
}

/// Collect errors from failed outcomes.
pub fn collect_errors(outcomes: &[AgentOutcome]) -> Vec<ConsensusError> {
    outcomes
        .iter()
        .filter_map(|o| match o {
            AgentOutcome::Failure { agent_id, error } => {
                Some(ConsensusError::agent_failure(agent_id, error))
            }
            AgentOutcome::Timeout {
                agent_id,
                timeout_ms,
            } => Some(ConsensusError::agent_timeout(agent_id, *timeout_ms)),
            AgentOutcome::Success(_) => None,
        })
        .collect()
}

/// Calculate success rate from outcomes.
pub fn calculate_success_rate(outcomes: &[AgentOutcome]) -> f64 {
    if outcomes.is_empty() {
        return 0.0;
    }

    let success_count = outcomes.iter().filter(|o| o.is_success()).count();
    success_count as f64 / outcomes.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_adapters::{AdapterError, AdapterErrorKind, Verdict};
    use async_trait::async_trait;

    /// Mock adapter that returns a predetermined review.
    struct MockSuccessAdapter {
        id: String,
        delay_ms: u64,
        verdict: Verdict,
    }

    impl MockSuccessAdapter {
        fn new(id: &str, delay_ms: u64, verdict: Verdict) -> Self {
            Self {
                id: id.to_string(),
                delay_ms,
                verdict,
            }
        }
    }

    #[async_trait]
    impl AgentAdapter for MockSuccessAdapter {
        fn id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.id
        }

        async fn review_artifact(&self, _artifact: &str) -> Result<AgentReview, AdapterError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            }

            Ok(AgentReview {
                agent_id: self.id.clone(),
                verdict: self.verdict,
                confidence: 0.9,
                issues: vec![],
                suggestions: vec![],
                sign_off: self.verdict == Verdict::Pass,
                reasoning: "Mock review".to_string(),
            })
        }
    }

    /// Mock adapter that always fails.
    struct MockFailureAdapter {
        id: String,
        error_message: String,
    }

    impl MockFailureAdapter {
        fn new(id: &str, error_message: &str) -> Self {
            Self {
                id: id.to_string(),
                error_message: error_message.to_string(),
            }
        }
    }

    #[async_trait]
    impl AgentAdapter for MockFailureAdapter {
        fn id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.id
        }

        async fn review_artifact(&self, _artifact: &str) -> Result<AgentReview, AdapterError> {
            Err(AdapterError::new(AdapterErrorKind::Unknown, &self.error_message))
        }
    }

    /// Mock adapter that takes longer than timeout.
    struct MockSlowAdapter {
        id: String,
        delay_ms: u64,
    }

    impl MockSlowAdapter {
        fn new(id: &str, delay_ms: u64) -> Self {
            Self {
                id: id.to_string(),
                delay_ms,
            }
        }
    }

    #[async_trait]
    impl AgentAdapter for MockSlowAdapter {
        fn id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.id
        }

        async fn review_artifact(&self, _artifact: &str) -> Result<AgentReview, AdapterError> {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            Ok(AgentReview {
                agent_id: self.id.clone(),
                verdict: Verdict::Pass,
                confidence: 0.9,
                issues: vec![],
                suggestions: vec![],
                sign_off: true,
                reasoning: "Slow review".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_execute_agent_success() {
        let adapter = MockSuccessAdapter::new("test", 0, Verdict::Pass);
        let timeout = Duration::from_millis(1000);

        let outcome = execute_agent_with_timeout(&adapter, "test artifact", timeout).await;

        assert!(outcome.is_success());
        assert_eq!(outcome.agent_id(), "test");
    }

    #[tokio::test]
    async fn test_execute_agent_failure() {
        let adapter = MockFailureAdapter::new("test", "Connection refused");
        let timeout = Duration::from_millis(1000);

        let outcome = execute_agent_with_timeout(&adapter, "test artifact", timeout).await;

        assert!(!outcome.is_success());
        match outcome {
            AgentOutcome::Failure { agent_id, error } => {
                assert_eq!(agent_id, "test");
                assert!(error.contains("Connection refused"));
            }
            _ => panic!("Expected Failure outcome"),
        }
    }

    #[tokio::test]
    async fn test_execute_agent_timeout() {
        let adapter = MockSlowAdapter::new("slow", 500);
        let timeout = Duration::from_millis(100);

        let outcome = execute_agent_with_timeout(&adapter, "test artifact", timeout).await;

        assert!(!outcome.is_success());
        match outcome {
            AgentOutcome::Timeout {
                agent_id,
                timeout_ms,
            } => {
                assert_eq!(agent_id, "slow");
                assert_eq!(timeout_ms, 100);
            }
            _ => panic!("Expected Timeout outcome"),
        }
    }

    #[tokio::test]
    async fn test_execute_agents_parallel_all_success() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockSuccessAdapter::new("grok", 10, Verdict::Pass)),
            Box::new(MockSuccessAdapter::new("claude", 10, Verdict::Pass)),
            Box::new(MockSuccessAdapter::new("gemini", 10, Verdict::Issue)),
        ];

        let timeout = Duration::from_millis(1000);
        let outcomes = execute_agents_parallel(&adapters, "test artifact", timeout).await;

        assert_eq!(outcomes.len(), 3);
        assert!(outcomes.iter().all(|o| o.is_success()));
    }

    #[tokio::test]
    async fn test_execute_agents_parallel_mixed() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockSuccessAdapter::new("success", 0, Verdict::Pass)),
            Box::new(MockFailureAdapter::new("failure", "Error")),
            Box::new(MockSlowAdapter::new("slow", 500)),
        ];

        let timeout = Duration::from_millis(100);
        let outcomes = execute_agents_parallel(&adapters, "test artifact", timeout).await;

        assert_eq!(outcomes.len(), 3);

        let success_count = outcomes.iter().filter(|o| o.is_success()).count();
        assert_eq!(success_count, 1);
    }

    #[tokio::test]
    async fn test_execute_agents_sequential() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockSuccessAdapter::new("first", 0, Verdict::Pass)),
            Box::new(MockSuccessAdapter::new("second", 0, Verdict::Pass)),
        ];

        let timeout = Duration::from_millis(1000);
        let outcomes = execute_agents_sequential(&adapters, "test artifact", timeout).await;

        assert_eq!(outcomes.len(), 2);
        assert!(outcomes.iter().all(|o| o.is_success()));
    }

    #[test]
    fn test_extract_successful_reviews() {
        let outcomes = vec![
            AgentOutcome::Success(AgentReview {
                agent_id: "grok".to_string(),
                verdict: Verdict::Pass,
                confidence: 0.9,
                issues: vec![],
                suggestions: vec![],
                sign_off: true,
                reasoning: "test".to_string(),
            }),
            AgentOutcome::Failure {
                agent_id: "claude".to_string(),
                error: "Error".to_string(),
            },
            AgentOutcome::Success(AgentReview {
                agent_id: "gemini".to_string(),
                verdict: Verdict::Issue,
                confidence: 0.8,
                issues: vec![],
                suggestions: vec![],
                sign_off: false,
                reasoning: "test".to_string(),
            }),
        ];

        let reviews = extract_successful_reviews(&outcomes);

        assert_eq!(reviews.len(), 2);
        assert_eq!(reviews[0].agent_id, "grok");
        assert_eq!(reviews[1].agent_id, "gemini");
    }

    #[test]
    fn test_collect_errors() {
        let outcomes = vec![
            AgentOutcome::Success(AgentReview {
                agent_id: "grok".to_string(),
                verdict: Verdict::Pass,
                confidence: 0.9,
                issues: vec![],
                suggestions: vec![],
                sign_off: true,
                reasoning: "test".to_string(),
            }),
            AgentOutcome::Failure {
                agent_id: "claude".to_string(),
                error: "Connection error".to_string(),
            },
            AgentOutcome::Timeout {
                agent_id: "gemini".to_string(),
                timeout_ms: 30000,
            },
        ];

        let errors = collect_errors(&outcomes);

        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_calculate_success_rate() {
        let outcomes = vec![
            AgentOutcome::Success(AgentReview {
                agent_id: "grok".to_string(),
                verdict: Verdict::Pass,
                confidence: 0.9,
                issues: vec![],
                suggestions: vec![],
                sign_off: true,
                reasoning: "test".to_string(),
            }),
            AgentOutcome::Failure {
                agent_id: "claude".to_string(),
                error: "Error".to_string(),
            },
            AgentOutcome::Success(AgentReview {
                agent_id: "gemini".to_string(),
                verdict: Verdict::Issue,
                confidence: 0.8,
                issues: vec![],
                suggestions: vec![],
                sign_off: false,
                reasoning: "test".to_string(),
            }),
            AgentOutcome::Timeout {
                agent_id: "codex".to_string(),
                timeout_ms: 30000,
            },
        ];

        let rate = calculate_success_rate(&outcomes);

        // 2 out of 4 succeeded
        assert!((rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_calculate_success_rate_empty() {
        let outcomes: Vec<AgentOutcome> = vec![];
        let rate = calculate_success_rate(&outcomes);
        assert_eq!(rate, 0.0);
    }
}
