//! ConsensusEngine - the main orchestrator for multi-agent consensus.

use crate::error::ConsensusError;
use crate::parallel::{
    execute_agents_parallel, execute_agents_sequential, extract_successful_reviews,
    DEFAULT_TIMEOUT_MS,
};
use crate::strategies::VotingStrategy;
use crate::types::ConsensusResult;
use aiy_adapters::AgentAdapter;
use std::time::Duration;

/// The main consensus engine that orchestrates multi-agent code reviews.
///
/// The engine runs multiple AI agents in parallel, collects their reviews,
/// and applies a voting strategy to determine the final consensus verdict.
///
/// # Security Guarantees
///
/// The engine provides defense-in-depth protection against empty review scenarios:
///
/// 1. **No adapters configured**: Returns `ConsensusError::NoReviews` immediately.
/// 2. **All agents fail/timeout**: Returns `ConsensusError::InsufficientReviews`,
///    never allows a Pass verdict with zero successful reviews.
/// 3. **Voting strategy layer**: Even if reviews somehow bypass the engine checks,
///    `VotingStrategy::decide()` will return `Block` for empty review slices.
///
/// This multi-layer approach ensures that code cannot be approved without at least
/// one successful agent review.
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
    ///
    /// # Arguments
    ///
    /// * `strategy` - The voting strategy to use for determining consensus
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_strategy(mut self, strategy: VotingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the per-agent timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for each agent's review
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the timeout in milliseconds.
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Maximum time in milliseconds to wait for each agent's review
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = Duration::from_millis(timeout_ms);
        self
    }

    /// Enable or disable parallel execution.
    ///
    /// When enabled (default), all agents run concurrently.
    /// When disabled, agents run one at a time in sequence.
    ///
    /// # Arguments
    ///
    /// * `parallel` - `true` for parallel execution, `false` for sequential
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set the minimum number of successful reviews required.
    ///
    /// If fewer than this many agents complete successfully, the review
    /// will fail with `ConsensusError::InsufficientReviews`.
    ///
    /// # Arguments
    ///
    /// * `min_reviews` - Minimum successful reviews needed (default: 1)
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_min_reviews(mut self, min_reviews: usize) -> Self {
        self.min_reviews = min_reviews;
        self
    }

    /// Get the number of configured adapters.
    ///
    /// # Returns
    ///
    /// The count of agent adapters registered with this engine.
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }

    /// Get the configured timeout duration.
    ///
    /// # Returns
    ///
    /// The maximum time allowed for each agent's review.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get a reference to the voting strategy.
    ///
    /// # Returns
    ///
    /// Reference to the configured voting strategy.
    pub fn strategy(&self) -> &VotingStrategy {
        &self.strategy
    }

    /// Check if parallel execution is enabled.
    ///
    /// # Returns
    ///
    /// `true` if agents will run concurrently, `false` if sequentially.
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
    ///
    /// # Arguments
    ///
    /// * `artifact` - The code or diff content to review
    ///
    /// # Returns
    ///
    /// A `ConsensusResult` containing the final verdict and all agent reviews.
    ///
    /// # Errors
    ///
    /// * `ConsensusError::NoReviews` - No adapters configured
    /// * `ConsensusError::InsufficientReviews` - Fewer than `min_reviews` succeeded
    ///
    /// # Security
    ///
    /// Empty or insufficient reviews will **never** result in a `Pass` verdict.
    /// This is enforced at multiple layers (engine, voting strategy) as
    /// defense-in-depth against accidental approval of unreviewed code.
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
    ///
    /// # Arguments
    ///
    /// * `artifact` - The code or diff content to review
    ///
    /// # Returns
    ///
    /// A `ConsensusResult` with a note about partial agent participation.
    ///
    /// # Errors
    ///
    /// * `ConsensusError::NoReviews` - No adapters configured or all failed
    ///
    /// # Security
    ///
    /// Even in best-effort mode, at least one agent must succeed.
    /// Zero successful reviews will always return an error, never `Pass`.
    pub async fn review_best_effort(
        &self,
        artifact: &str,
    ) -> Result<ConsensusResult, ConsensusError> {
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
///
/// # Example
///
/// ```ignore
/// use aiy_consensus::{ConsensusEngineBuilder, VotingStrategy};
/// use std::time::Duration;
///
/// let engine = ConsensusEngineBuilder::new()
///     .adapter(Box::new(claude_adapter))
///     .adapter(Box::new(grok_adapter))
///     .strategy(VotingStrategy::Majority)
///     .timeout(Duration::from_secs(60))
///     .min_reviews(2)
///     .build();
/// ```
pub struct ConsensusEngineBuilder {
    /// Registered agent adapters.
    adapters: Vec<Box<dyn AgentAdapter>>,
    /// Optional voting strategy override.
    strategy: Option<VotingStrategy>,
    /// Optional timeout override.
    timeout: Option<Duration>,
    /// Optional parallel execution flag.
    parallel: Option<bool>,
    /// Optional minimum reviews requirement.
    min_reviews: Option<usize>,
}

impl ConsensusEngineBuilder {
    /// Create a new builder with default settings.
    ///
    /// # Returns
    ///
    /// An empty builder ready for configuration.
    pub fn new() -> Self {
        Self {
            adapters: vec![],
            strategy: None,
            timeout: None,
            parallel: None,
            min_reviews: None,
        }
    }

    /// Add an adapter to the engine.
    ///
    /// # Arguments
    ///
    /// * `adapter` - The agent adapter to add
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn adapter(mut self, adapter: Box<dyn AgentAdapter>) -> Self {
        self.adapters.push(adapter);
        self
    }

    /// Add multiple adapters at once.
    ///
    /// # Arguments
    ///
    /// * `adapters` - Vector of agent adapters to add
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn adapters(mut self, adapters: Vec<Box<dyn AgentAdapter>>) -> Self {
        self.adapters.extend(adapters);
        self
    }

    /// Set the voting strategy.
    ///
    /// # Arguments
    ///
    /// * `strategy` - The voting strategy for consensus decisions
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn strategy(mut self, strategy: VotingStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Set the per-agent timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for each agent
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the timeout in milliseconds.
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Maximum time in milliseconds per agent
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(Duration::from_millis(timeout_ms));
        self
    }

    /// Enable or disable parallel execution.
    ///
    /// # Arguments
    ///
    /// * `parallel` - `true` for parallel, `false` for sequential
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = Some(parallel);
        self
    }

    /// Set minimum required reviews.
    ///
    /// # Arguments
    ///
    /// * `min_reviews` - Minimum successful reviews needed
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn min_reviews(mut self, min_reviews: usize) -> Self {
        self.min_reviews = Some(min_reviews);
        self
    }

    /// Build the ConsensusEngine with configured settings.
    ///
    /// # Returns
    ///
    /// A fully configured `ConsensusEngine` ready for use.
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
    use aiy_adapters::{AdapterError, AdapterErrorKind, AgentReview, Verdict};
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
            Self { id: id.to_string() }
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
            Err(AdapterError::new(
                AdapterErrorKind::Unknown,
                "Simulated failure",
            ))
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
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(MockAdapter::with_delay(
            "slow",
            Verdict::Pass,
            500,
        ))];

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

    // =========================================================================
    // SECURITY REGRESSION TESTS
    //
    // These tests document critical security invariants for the ConsensusEngine.
    // They ensure that missing or failed reviews can NEVER result in a Pass.
    //
    // Defense-in-depth layers:
    // 1. Engine checks for no adapters -> NoReviews error
    // 2. Engine checks for insufficient reviews -> InsufficientReviews error
    // 3. VotingStrategy.decide() blocks on empty reviews (final safety net)
    // =========================================================================

    /// Security test: Engine with no adapters must return NoReviews error.
    ///
    /// An engine with no adapters configured cannot possibly produce reviews,
    /// so it must fail rather than proceeding with an empty review list.
    #[tokio::test]
    async fn test_security_engine_no_adapters_returns_no_reviews_error() {
        let engine = ConsensusEngine::new(vec![]);
        let result = engine.review("test artifact").await;

        assert!(
            matches!(result, Err(ConsensusError::NoReviews)),
            "SECURITY: Engine with no adapters must return NoReviews error, got: {:?}",
            result
        );
    }

    /// Security test: Engine where all agents fail must return InsufficientReviews.
    ///
    /// If every configured agent fails, we have zero successful reviews.
    /// This must produce an error, never a Pass verdict.
    #[tokio::test]
    async fn test_security_engine_all_agents_fail_returns_insufficient_reviews() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(FailingAdapter::new("failing1")),
            Box::new(FailingAdapter::new("failing2")),
            Box::new(FailingAdapter::new("failing3")),
        ];

        let engine = ConsensusEngine::new(adapters);
        let result = engine.review("test artifact").await;

        assert!(
            matches!(result, Err(ConsensusError::InsufficientReviews { required: 1, provided: 0 })),
            "SECURITY: Engine with all failing agents must return InsufficientReviews error, got: {:?}",
            result
        );
    }

    /// Security test: Engine where all agents timeout must return InsufficientReviews.
    ///
    /// If every configured agent times out, we have zero successful reviews.
    /// This must produce an error, never a Pass verdict.
    #[tokio::test]
    async fn test_security_engine_all_agents_timeout_returns_insufficient_reviews() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(MockAdapter::with_delay("slow1", Verdict::Pass, 1000)),
            Box::new(MockAdapter::with_delay("slow2", Verdict::Pass, 1000)),
        ];

        // Very short timeout to force timeouts
        let engine = ConsensusEngine::new(adapters).with_timeout_ms(10);
        let result = engine.review("test artifact").await;

        assert!(
            matches!(result, Err(ConsensusError::InsufficientReviews { .. })),
            "SECURITY: Engine with all timed-out agents must return InsufficientReviews error, got: {:?}",
            result
        );
    }

    /// Security test: Engine review_best_effort with all agents failing returns NoReviews.
    ///
    /// Even the best-effort method must not produce a Pass when there are zero reviews.
    #[tokio::test]
    async fn test_security_engine_best_effort_all_fail_returns_no_reviews() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(FailingAdapter::new("failing1")),
            Box::new(FailingAdapter::new("failing2")),
        ];

        let engine = ConsensusEngine::new(adapters);
        let result = engine.review_best_effort("test artifact").await;

        assert!(
            matches!(result, Err(ConsensusError::NoReviews)),
            "SECURITY: Best-effort with all failing agents must return NoReviews error, got: {:?}",
            result
        );
    }

    /// Security test: Verify engine checks adapters before executing reviews.
    ///
    /// The no-adapters check should be the first thing the engine does,
    /// preventing any resource usage or side effects.
    #[tokio::test]
    async fn test_security_engine_checks_adapters_first() {
        let engine = ConsensusEngine::new(vec![]);

        // Both review methods should fail immediately with NoReviews
        let result1 = engine.review("test").await;
        let result2 = engine.review_best_effort("test").await;

        assert!(matches!(result1, Err(ConsensusError::NoReviews)));
        assert!(matches!(result2, Err(ConsensusError::NoReviews)));
    }

    /// Security test: Mixed failing and timing-out agents with insufficient successes.
    ///
    /// Combination scenario where some fail, some timeout, none succeed.
    #[tokio::test]
    async fn test_security_engine_mixed_failures_never_passes() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(FailingAdapter::new("failing")),
            Box::new(MockAdapter::with_delay("slow", Verdict::Pass, 1000)),
        ];

        let engine = ConsensusEngine::new(adapters).with_timeout_ms(10);
        let result = engine.review("test artifact").await;

        // Should fail because: 1 fails immediately, 1 times out
        assert!(
            matches!(result, Err(ConsensusError::InsufficientReviews { .. })),
            "SECURITY: Mixed failures/timeouts with no success must error, got: {:?}",
            result
        );
    }
}
