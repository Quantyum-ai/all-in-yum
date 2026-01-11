//! Voting strategies for consensus decisions.

use aiy_adapters::{AgentReview, Verdict};
use std::collections::HashMap;

/// Voting strategy for determining consensus verdict.
#[derive(Debug, Clone)]
pub enum VotingStrategy {
    /// All agents must pass for consensus to pass.
    Unanimous,

    /// Majority (> 50%) must pass for consensus to pass.
    Majority,

    /// Any single agent passing is sufficient for consensus to pass.
    Any,

    /// Weighted voting based on agent confidence or explicit weights.
    Weighted(WeightedConfig),
}

/// Configuration for weighted voting.
#[derive(Debug, Clone)]
pub struct WeightedConfig {
    /// Explicit weights per agent ID. If not specified, uses confidence scores.
    pub weights: HashMap<String, f64>,

    /// Threshold for weighted pass (0.0 - 1.0). Default is 0.5.
    pub threshold: f64,

    /// If true, use agent confidence scores when no explicit weight is set.
    /// If false, treat missing weights as 1.0.
    pub use_confidence_as_weight: bool,
}

impl Default for WeightedConfig {
    fn default() -> Self {
        Self {
            weights: HashMap::new(),
            threshold: 0.5,
            use_confidence_as_weight: true,
        }
    }
}

impl WeightedConfig {
    /// Create a new weighted config with custom threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            threshold,
            ..Default::default()
        }
    }

    /// Create a new weighted config with explicit agent weights.
    pub fn with_weights(weights: HashMap<String, f64>) -> Self {
        Self {
            weights,
            ..Default::default()
        }
    }
}

impl VotingStrategy {
    /// Decide the final verdict based on the voting strategy.
    pub fn decide(&self, reviews: &[AgentReview]) -> Verdict {
        if reviews.is_empty() {
            return Verdict::Block;
        }

        match self {
            VotingStrategy::Unanimous => Self::decide_unanimous(reviews),
            VotingStrategy::Majority => Self::decide_majority(reviews),
            VotingStrategy::Any => Self::decide_any(reviews),
            VotingStrategy::Weighted(config) => Self::decide_weighted(reviews, config),
        }
    }

    /// Generate reasoning string explaining the decision.
    pub fn generate_reasoning(&self, reviews: &[AgentReview], verdict: Verdict) -> String {
        if reviews.is_empty() {
            return "No reviews provided - cannot reach consensus.".to_string();
        }

        let pass_count = reviews.iter().filter(|r| r.verdict == Verdict::Pass).count();
        let total = reviews.len();

        match self {
            VotingStrategy::Unanimous => {
                if verdict == Verdict::Pass {
                    format!(
                        "Unanimous consensus reached: all {} agent(s) approved.",
                        total
                    )
                } else {
                    let blockers: Vec<&str> = reviews
                        .iter()
                        .filter(|r| r.verdict != Verdict::Pass)
                        .map(|r| r.agent_id.as_str())
                        .collect();
                    format!(
                        "Unanimous consensus not reached: {} of {} agent(s) did not approve. \
                         Blocking agents: {}",
                        total - pass_count,
                        total,
                        blockers.join(", ")
                    )
                }
            }
            VotingStrategy::Majority => {
                let threshold = total / 2 + 1;
                if verdict == Verdict::Pass {
                    format!(
                        "Majority consensus reached: {} of {} agent(s) approved (threshold: {}).",
                        pass_count, total, threshold
                    )
                } else {
                    format!(
                        "Majority consensus not reached: {} of {} agent(s) approved \
                         (needed: {} for majority).",
                        pass_count, total, threshold
                    )
                }
            }
            VotingStrategy::Any => {
                if verdict == Verdict::Pass {
                    let approvers: Vec<&str> = reviews
                        .iter()
                        .filter(|r| r.verdict == Verdict::Pass)
                        .map(|r| r.agent_id.as_str())
                        .collect();
                    format!(
                        "Consensus reached (any-pass strategy): {} agent(s) approved: {}",
                        pass_count,
                        approvers.join(", ")
                    )
                } else {
                    format!(
                        "No agent approved: all {} agent(s) rejected or blocked.",
                        total
                    )
                }
            }
            VotingStrategy::Weighted(config) => {
                let weighted_score = Self::calculate_weighted_score(reviews, config);
                match verdict {
                    Verdict::Pass => format!(
                        "Weighted consensus reached: score {:.2} exceeds threshold {:.2}.",
                        weighted_score, config.threshold
                    ),
                    Verdict::Block => {
                        let block_score = Self::calculate_weighted_block_score(reviews, config);
                        format!(
                            "Weighted consensus blocked: block score {:.2} exceeds threshold {:.2} (pass score {:.2}).",
                            block_score, config.threshold, weighted_score
                        )
                    }
                    Verdict::Issue => format!(
                        "Weighted consensus not reached: score {:.2} below threshold {:.2}.",
                        weighted_score, config.threshold
                    ),
                }
            }
        }
    }

    /// Unanimous: all agents must pass.
    fn decide_unanimous(reviews: &[AgentReview]) -> Verdict {
        let all_pass = reviews.iter().all(|r| r.verdict == Verdict::Pass);
        if all_pass {
            Verdict::Pass
        } else if reviews.iter().any(|r| r.verdict == Verdict::Block) {
            Verdict::Block
        } else {
            Verdict::Issue
        }
    }

    /// Majority: more than 50% must pass.
    fn decide_majority(reviews: &[AgentReview]) -> Verdict {
        let pass_count = reviews.iter().filter(|r| r.verdict == Verdict::Pass).count();
        let total = reviews.len();

        if pass_count > total / 2 {
            Verdict::Pass
        } else {
            // Check if there are any blocking verdicts
            let block_count = reviews.iter().filter(|r| r.verdict == Verdict::Block).count();
            if block_count > total / 2 {
                Verdict::Block
            } else {
                Verdict::Issue
            }
        }
    }

    /// Any: at least one agent must pass.
    fn decide_any(reviews: &[AgentReview]) -> Verdict {
        if reviews.iter().any(|r| r.verdict == Verdict::Pass) {
            Verdict::Pass
        } else if reviews.iter().all(|r| r.verdict == Verdict::Block) {
            Verdict::Block
        } else {
            Verdict::Issue
        }
    }

    /// Weighted: based on agent weights or confidence scores.
    fn decide_weighted(reviews: &[AgentReview], config: &WeightedConfig) -> Verdict {
        let score = Self::calculate_weighted_score(reviews, config);

        if score >= config.threshold {
            Verdict::Pass
        } else if score == 0.0 {
            // Only produce a blocking verdict when there are no weighted passes at all.
            let block_score = Self::calculate_weighted_block_score(reviews, config);
            if block_score >= config.threshold {
                Verdict::Block
            } else {
                Verdict::Issue
            }
        } else {
            Verdict::Issue
        }
    }

    /// Calculate weighted pass score.
    fn calculate_weighted_score(reviews: &[AgentReview], config: &WeightedConfig) -> f64 {
        let mut total_weight = 0.0;
        let mut pass_weight = 0.0;

        for review in reviews {
            let weight = config
                .weights
                .get(&review.agent_id)
                .copied()
                .unwrap_or_else(|| {
                    if config.use_confidence_as_weight {
                        review.confidence
                    } else {
                        1.0
                    }
                });

            total_weight += weight;
            if review.verdict == Verdict::Pass {
                pass_weight += weight;
            }
        }

        if total_weight == 0.0 {
            return 0.0;
        }

        pass_weight / total_weight
    }

    /// Calculate weighted block score.
    fn calculate_weighted_block_score(reviews: &[AgentReview], config: &WeightedConfig) -> f64 {
        let mut total_weight = 0.0;
        let mut block_weight = 0.0;

        for review in reviews {
            let weight = config
                .weights
                .get(&review.agent_id)
                .copied()
                .unwrap_or_else(|| {
                    if config.use_confidence_as_weight {
                        review.confidence
                    } else {
                        1.0
                    }
                });

            total_weight += weight;
            if review.verdict == Verdict::Block {
                block_weight += weight;
            }
        }

        if total_weight == 0.0 {
            return 0.0;
        }

        block_weight / total_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_review(agent_id: &str, verdict: Verdict, confidence: f64) -> AgentReview {
        AgentReview {
            agent_id: agent_id.to_string(),
            verdict,
            confidence,
            issues: vec![],
            suggestions: vec![],
            sign_off: verdict == Verdict::Pass,
            reasoning: "test".to_string(),
        }
    }

    // Unanimous strategy tests
    #[test]
    fn test_unanimous_all_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Pass, 0.85),
            create_review("gemini", Verdict::Pass, 0.8),
        ];

        let strategy = VotingStrategy::Unanimous;
        assert_eq!(strategy.decide(&reviews), Verdict::Pass);
    }

    #[test]
    fn test_unanimous_one_block() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Block, 0.85),
            create_review("gemini", Verdict::Pass, 0.8),
        ];

        let strategy = VotingStrategy::Unanimous;
        assert_eq!(strategy.decide(&reviews), Verdict::Block);
    }

    #[test]
    fn test_unanimous_one_issue() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Issue, 0.85),
            create_review("gemini", Verdict::Pass, 0.8),
        ];

        let strategy = VotingStrategy::Unanimous;
        assert_eq!(strategy.decide(&reviews), Verdict::Issue);
    }

    // Majority strategy tests
    #[test]
    fn test_majority_two_of_three_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Pass, 0.85),
            create_review("gemini", Verdict::Block, 0.8),
        ];

        let strategy = VotingStrategy::Majority;
        assert_eq!(strategy.decide(&reviews), Verdict::Pass);
    }

    #[test]
    fn test_majority_one_of_three_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Issue, 0.85),
            create_review("gemini", Verdict::Block, 0.8),
        ];

        let strategy = VotingStrategy::Majority;
        assert_eq!(strategy.decide(&reviews), Verdict::Issue);
    }

    #[test]
    fn test_majority_three_of_five_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Pass, 0.85),
            create_review("gemini", Verdict::Pass, 0.8),
            create_review("codex", Verdict::Issue, 0.75),
            create_review("other", Verdict::Block, 0.7),
        ];

        let strategy = VotingStrategy::Majority;
        assert_eq!(strategy.decide(&reviews), Verdict::Pass);
    }

    #[test]
    fn test_majority_two_of_four_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Pass, 0.85),
            create_review("gemini", Verdict::Issue, 0.8),
            create_review("codex", Verdict::Block, 0.75),
        ];

        let strategy = VotingStrategy::Majority;
        // 2 of 4 is not > 50%, so not pass
        assert_eq!(strategy.decide(&reviews), Verdict::Issue);
    }

    // Any strategy tests
    #[test]
    fn test_any_one_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Block, 0.9),
            create_review("claude", Verdict::Pass, 0.85),
            create_review("gemini", Verdict::Block, 0.8),
        ];

        let strategy = VotingStrategy::Any;
        assert_eq!(strategy.decide(&reviews), Verdict::Pass);
    }

    #[test]
    fn test_any_none_pass_all_block() {
        let reviews = vec![
            create_review("grok", Verdict::Block, 0.9),
            create_review("claude", Verdict::Block, 0.85),
        ];

        let strategy = VotingStrategy::Any;
        assert_eq!(strategy.decide(&reviews), Verdict::Block);
    }

    #[test]
    fn test_any_none_pass_mixed() {
        let reviews = vec![
            create_review("grok", Verdict::Block, 0.9),
            create_review("claude", Verdict::Issue, 0.85),
        ];

        let strategy = VotingStrategy::Any;
        assert_eq!(strategy.decide(&reviews), Verdict::Issue);
    }

    // Weighted strategy tests
    #[test]
    fn test_weighted_with_confidence() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),   // weight 0.9
            create_review("claude", Verdict::Block, 0.1), // weight 0.1
        ];

        let config = WeightedConfig::with_threshold(0.5);
        let strategy = VotingStrategy::Weighted(config);

        // pass_weight = 0.9, total = 1.0, score = 0.9
        assert_eq!(strategy.decide(&reviews), Verdict::Pass);
    }

    #[test]
    fn test_weighted_with_explicit_weights() {
        let mut weights = HashMap::new();
        weights.insert("grok".to_string(), 1.0);
        weights.insert("claude".to_string(), 2.0);

        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Block, 0.1),
        ];

        let config = WeightedConfig::with_weights(weights);
        let strategy = VotingStrategy::Weighted(config);

        // pass_weight = 1.0, total = 3.0, score = 0.333
        assert_eq!(strategy.decide(&reviews), Verdict::Issue);
    }

    #[test]
    fn test_weighted_high_threshold() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.8),
            create_review("claude", Verdict::Pass, 0.8),
            create_review("gemini", Verdict::Issue, 0.8),
        ];

        let config = WeightedConfig::with_threshold(0.8);
        let strategy = VotingStrategy::Weighted(config);

        // pass_weight = 1.6, total = 2.4, score = 0.666
        assert_eq!(strategy.decide(&reviews), Verdict::Issue);
    }

    // Empty reviews tests
    #[test]
    fn test_empty_reviews() {
        let reviews: Vec<AgentReview> = vec![];

        assert_eq!(VotingStrategy::Unanimous.decide(&reviews), Verdict::Block);
        assert_eq!(VotingStrategy::Majority.decide(&reviews), Verdict::Block);
        assert_eq!(VotingStrategy::Any.decide(&reviews), Verdict::Block);
        assert_eq!(
            VotingStrategy::Weighted(WeightedConfig::default()).decide(&reviews),
            Verdict::Block
        );
    }

    // Reasoning tests
    #[test]
    fn test_unanimous_reasoning_pass() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Pass, 0.85),
        ];

        let strategy = VotingStrategy::Unanimous;
        let reasoning = strategy.generate_reasoning(&reviews, Verdict::Pass);

        assert!(reasoning.contains("Unanimous consensus reached"));
        assert!(reasoning.contains("2 agent(s) approved"));
    }

    #[test]
    fn test_majority_reasoning_fail() {
        let reviews = vec![
            create_review("grok", Verdict::Pass, 0.9),
            create_review("claude", Verdict::Block, 0.85),
            create_review("gemini", Verdict::Block, 0.8),
        ];

        let strategy = VotingStrategy::Majority;
        let reasoning = strategy.generate_reasoning(&reviews, Verdict::Issue);

        assert!(reasoning.contains("Majority consensus not reached"));
        assert!(reasoning.contains("1 of 3"));
    }
}
