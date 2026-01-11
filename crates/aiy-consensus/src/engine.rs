//! ConsensusEngine - the main orchestrator for multi-agent consensus.

use crate::error::ConsensusError;
use crate::parallel::{
    execute_agents_parallel, execute_agents_sequential, extract_successful_reviews, DEFAULT_TIMEOUT_MS,
};
use crate::strategies::VotingStrategy;
use crate::types::ConsensusResult;
use aiy_adapters::AgentAdapter;
use std::time::Duration;

/// The main consensus engine that orchestrates multi-agent code reviews.
///
/// The engine runs multiple AI agents in parallel, collects their reviews,
/// and applies a voting strategy to determine the final consensus verdict.
pub struct ConsensusEngine {
    /// The agent adapters to use for reviews.
    adapters: Vec<Box<dyn AgentAdapter>>,

    /// The voting strategy for determining consensus.
    strategy: VotingStrategy,

    /// Timeout duration for each agent's review.
    timeout: Duration,

    /// Whether to run agents in parallel (true) or sequentially (false).
    parallel: bool,

    /// Minimum number of successful reviews required.
    min_reviews: usize,
}

impl ConsensusEngine {
    /// Create a new ConsensusEngine with the given adapters.
    ///
    /// Uses default settings:
    /// - Majority voting strategy
    /// - 30 second timeout per agent
    /// - Parallel execution
    /// - Minimum 1 review required
    pub fn new(adapters: Vec<Box<dyn AgentAdapter>>) -> Self {
        Self {
            adapters,
            strategy: VotingStrategy::Majority,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            parallel: true,
            min_reviews: 1,
        }
    }

    /// Set the voting strategy.
    pub fn with_strategy(mut self, strategy: VotingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the per-agent timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the timeout in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = Duration::from_millis(timeout_ms);
        self
    }

    /// Enable or disable parallel execution.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set the minimum number of successful reviews required.
    pub fn with_min_reviews(mut self, min_reviews: usize) -> Self {
        self.min_reviews = min_reviews;
        self
    }

    /// Get the number of configured adapters.
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }

    /// Get the configured timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get a reference to the voting strategy.
    pub fn strategy(&self) -> &VotingStrategy {
        &self.strategy
    }

    /// Check if parallel execution is enabled.
    pub fn is_parallel(&self) -> bool {
        self.parallel
    }

    /// Run all agents and aggregate results into a consensus.
    ///
    /// This is the main entry point for conducting a multi-agent review.
    /// It will:
    /// 1. Execute all agent reviews (in parallel or sequentially)
    /// 2. Collect successful reviews
    /// 3. Apply the voting strategy
    /// 4. Generate a ConsensusResult with the final verdict
    pub async fn review(&self, artifact: &str) -> Result<ConsensusResult, ConsensusError> {
        if self.adapters.is_empty() {
            return Err(ConsensusError::NoReviews);
        }

        // Execute all agents
        let outcomes = if self.parallel {
            execute_agents_parallel(&self.adapters, artifact, self.timeout).await
        } else {
            execute_agents_sequential(&self.adapters, artifact, self.timeout).await
        };

        // Extract successful reviews
        let reviews = extract_successful_reviews(&outcomes);

        // Check minimum review requirement
        if reviews.len() < self.min_reviews {
            return Err(ConsensusError::InsufficientReviews {
                required: self.min_reviews,
                provided: reviews.len(),
            });
        }

        // Apply voting strategy
        let verdict = self.strategy.decide(&reviews);

        // Generate reasoning
        let reasoning = self.strategy.generate_reasoning(&reviews, verdict);

        // Build consensus result
        Ok(ConsensusResult::new(verdict, reviews, reasoning))
    }

    /// Run a review but allow partial results even if some agents fail.
    ///
    /// Unlike `review()`, this method will return a result even if
    /// fewer than `min_reviews` agents succeed, as long as at least
    /// one agent returns a review.
    pub async fn review_best_effort(&self, artifact: &str) -> Result<ConsensusResult, ConsensusError> {
        if self.adapters.is_empty() {
            return Err(ConsensusError::NoReviews);
        }

        // Execute all agents
        let outcomes = if self.parallel {
            execute_agents_parallel(&self.adapters, artifact, self.timeout).await
        } else {
            execute_agents_sequential(&self.adapters, artifact, self.timeout).await
        };

        // Extract successful reviews
        let reviews = extract_successful_reviews(&outcomes);

        if reviews.is_empty() {
            return Err(ConsensusError::NoReviews);
        }

        // Apply voting strategy
        let verdict = self.strategy.decide(&reviews);

        // Generate reasoning with note about partial results
        let mut reasoning = self.strategy.generate_reasoning(&reviews, verdict);
        let success_count = reviews.len();
        let total_count = self.adapters.len();

        if success_count < total_count {
            reasoning = format!(
                "{} Note: {} of {} agent(s) failed or timed out.",
                reasoning,
                total_count - success_count,
                total_count
            );
        }

        // Build consensus result
        Ok(ConsensusResult::new(verdict, reviews, reasoning))
    }
}

/// Builder pattern for more ergonomic ConsensusEngine construction.
pub struct ConsensusEngineBuilder {
    adapters: Vec<Box<dyn AgentAdapter>>,
    strategy: Option<VotingStrategy>,
    timeout: Option<Duration>,
    parallel: Option<bool>,
    min_reviews: Option<usize>,
}

impl ConsensusEngineBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            adapters: vec![],
            strategy: None,
            timeout: None,
            parallel: None,
            min_reviews: None,
        }
    }

    /// Add an adapter.
    pub fn adapter(mut self, adapter: Box<dyn AgentAdapter>) -> Self {
        self.adapters.push(adapter);
        self
    }

    /// Add multiple adapters.
    pub fn adapters(mut self, adapters: Vec<Box<dyn AgentAdapter>>) -> Self {
        self.adapters.extend(adapters);
        self
    }

    /// Set the voting strategy.
    pub fn strategy(mut self, strategy: VotingStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the timeout in milliseconds.
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(Duration::from_millis(timeout_ms));
        self
    }

    /// Enable or disable parallel execution.
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = Some(parallel);
        self
    }

    /// Set minimum required reviews.
    pub fn min_reviews(mut self, min_reviews: usize) -> Self {
        self.min_reviews = Some(min_reviews);
        self
    }

    /// Build the ConsensusEngine.
    pub fn build(self) -> ConsensusEngine {
        let mut engine = ConsensusEngine::new(self.adapters);

        if let Some(strategy) = self.strategy {
            engine = engine.with_strategy(strategy);
        }
        if let Some(timeout) = self.timeout {
            engine = engine.with_timeout(timeout);
        }
        if let Some(parallel) = self.parallel {
            engine = engine.with_parallel(parallel);
        }
        if let Some(min_reviews) = self.min_reviews {
            engine = engine.with_min_reviews(min_reviews);
        }

        engine
    }
}

impl Default for ConsensusEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_adapters::{AdapterError, AgentReview, Verdict};
    use async_trait::async_trait;

    /// Mock adapter for testing.
    struct MockAdapter {
        id: String,
        verdict: Verdict,
        delay_ms: u64,
    }

    impl MockAdapter {
        fn new(id: &str, verdict: Verdict) -> Self {
            Self {
                id: id.to_string(),
                verdict,
                delay_ms: 0,
            }
        }

        fn with_delay(id: &str, verdict: Verdict, delay_ms: u64) -> Self {
            Self {
                id: id.to_string(),
                verdict,
                delay_ms,
            }
        }
    }

    #[async_trait]
    impl AgentAdapter for MockAdapter {
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
                reasoning: format!("{} review", self.id),
            })
        }
    }

    /// Failing mock adapter.
    struct FailingAdapter {
        id: String,
    }

    impl FailingAdapter {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
            }
        }
    }

    #[async_trait]
    impl AgentAdapter for FailingAdapter {
        fn id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.id
        }

        async fn review_artifact(&self, _artifact: &str) -> Result<AgentReview, AdapterError> {
            Err(AdapterError::new("Simulated failure"))
        }
    }

    #[tokio::test]
    async fn test_engine_basic_review() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("grok", Verdict::Pass)),
            Box::new(MockAdapter::new("claude", Verdict::Pass)),
        ];

        let engine = ConsensusEngine::new(adapters);
        let result = engine.review("test code").await.unwrap();

        assert_eq!(result.final_verdict, Verdict::Pass);
        assert_eq!(result.agent_count(), 2);
        assert!(result.is_unanimous());
    }

    #[tokio::test]
    async fn test_engine_majority_strategy() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("grok", Verdict::Pass)),
            Box::new(MockAdapter::new("claude", Verdict::Pass)),
            Box::new(MockAdapter::new("gemini", Verdict::Block)),
        ];

        let engine = ConsensusEngine::new(adapters).with_strategy(VotingStrategy::Majority);
        let result = engine.review("test code").await.unwrap();

        assert_eq!(result.final_verdict, Verdict::Pass);
        assert!(!result.is_unanimous());
        assert_eq!(result.dissenting_agents.len(), 1);
    }

    #[tokio::test]
    async fn test_engine_unanimous_strategy() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("grok", Verdict::Pass)),
            Box::new(MockAdapter::new("claude", Verdict::Issue)),
        ];

        let engine = ConsensusEngine::new(adapters).with_strategy(VotingStrategy::Unanimous);
        let result = engine.review("test code").await.unwrap();

        assert_eq!(result.final_verdict, Verdict::Issue);
    }

    #[tokio::test]
    async fn test_engine_any_strategy() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("grok", Verdict::Block)),
            Box::new(MockAdapter::new("claude", Verdict::Block)),
            Box::new(MockAdapter::new("gemini", Verdict::Pass)),
        ];

        let engine = ConsensusEngine::new(adapters).with_strategy(VotingStrategy::Any);
        let result = engine.review("test code").await.unwrap();

        assert_eq!(result.final_verdict, Verdict::Pass);
    }

    #[tokio::test]
    async fn test_engine_no_adapters() {
        let engine = ConsensusEngine::new(vec![]);
        let result = engine.review("test code").await;

        assert!(matches!(result, Err(ConsensusError::NoReviews)));
    }

    #[tokio::test]
    async fn test_engine_insufficient_reviews() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(FailingAdapter::new("failing"))];

        let engine = ConsensusEngine::new(adapters).with_min_reviews(1);
        let result = engine.review("test code").await;

        assert!(matches!(
            result,
            Err(ConsensusError::InsufficientReviews { .. })
        ));
    }

    #[tokio::test]
    async fn test_engine_timeout() {
        let adapters: Vec<Box<dyn AgentAdapter>> =
            vec![Box::new(MockAdapter::with_delay("slow", Verdict::Pass, 500))];

        let engine = ConsensusEngine::new(adapters).with_timeout_ms(100);
        let result = engine.review("test code").await;

        // Should fail because the only adapter timed out
        assert!(matches!(
            result,
            Err(ConsensusError::InsufficientReviews { .. })
        ));
    }

    #[tokio::test]
    async fn test_engine_best_effort_partial() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("success", Verdict::Pass)),
            Box::new(FailingAdapter::new("failing")),
        ];

        let engine = ConsensusEngine::new(adapters);
        let result = engine.review_best_effort("test code").await.unwrap();

        assert_eq!(result.agent_count(), 1);
        assert!(result.reasoning.contains("failed or timed out"));
    }

    #[tokio::test]
    async fn test_engine_sequential() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("grok", Verdict::Pass)),
            Box::new(MockAdapter::new("claude", Verdict::Pass)),
        ];

        let engine = ConsensusEngine::new(adapters).with_parallel(false);

        assert!(!engine.is_parallel());

        let result = engine.review("test code").await.unwrap();
        assert_eq!(result.final_verdict, Verdict::Pass);
    }

    #[test]
    fn test_engine_builder() {
        let engine = ConsensusEngineBuilder::new()
            .strategy(VotingStrategy::Unanimous)
            .timeout_ms(60000)
            .parallel(false)
            .min_reviews(2)
            .build();

        assert_eq!(engine.timeout(), Duration::from_millis(60000));
        assert!(!engine.is_parallel());
        assert!(matches!(engine.strategy(), VotingStrategy::Unanimous));
    }

    #[test]
    fn test_engine_accessors() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::new("grok", Verdict::Pass)),
            Box::new(MockAdapter::new("claude", Verdict::Pass)),
        ];

        let engine = ConsensusEngine::new(adapters)
            .with_timeout_ms(45000)
            .with_parallel(true);

        assert_eq!(engine.adapter_count(), 2);
        assert_eq!(engine.timeout(), Duration::from_millis(45000));
        assert!(engine.is_parallel());
    }
}
