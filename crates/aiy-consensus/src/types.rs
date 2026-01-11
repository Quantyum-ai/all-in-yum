//! Core types for the consensus engine.

use aiy_adapters::{AgentReview, Issue, Verdict};
use serde::{Deserialize, Serialize};

/// Result of a consensus operation across multiple agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// The final verdict after applying the voting strategy.
    pub final_verdict: Verdict,

    /// All reviews collected from participating agents.
    pub agent_reviews: Vec<AgentReview>,

    /// Overall confidence score (0.0 - 1.0), aggregated from all agents.
    pub consensus_confidence: f64,

    /// Agent IDs that disagreed with the final verdict.
    pub dissenting_agents: Vec<String>,

    /// All issues aggregated from all agent reviews.
    pub aggregated_issues: Vec<Issue>,

    /// Human-readable reasoning explaining the consensus decision.
    pub reasoning: String,
}

impl ConsensusResult {
    /// Create a new ConsensusResult from agent reviews and a voting decision.
    ///
    /// # Arguments
    ///
    /// * `final_verdict` - The consensus verdict determined by the voting strategy
    /// * `agent_reviews` - All successful reviews from participating agents
    /// * `reasoning` - Human-readable explanation of the consensus decision
    ///
    /// # Returns
    ///
    /// A fully populated `ConsensusResult` with calculated confidence,
    /// dissenting agents, and aggregated issues.
    pub fn new(
        final_verdict: Verdict,
        agent_reviews: Vec<AgentReview>,
        reasoning: String,
    ) -> Self {
        let consensus_confidence = Self::calculate_confidence(&agent_reviews);
        let dissenting_agents = Self::find_dissenting_agents(&agent_reviews, final_verdict);
        let aggregated_issues = Self::aggregate_issues(&agent_reviews);

        Self {
            final_verdict,
            agent_reviews,
            consensus_confidence,
            dissenting_agents,
            aggregated_issues,
            reasoning,
        }
    }

    /// Calculate average confidence from all agent reviews.
    fn calculate_confidence(reviews: &[AgentReview]) -> f64 {
        if reviews.is_empty() {
            return 0.0;
        }
        let sum: f64 = reviews.iter().map(|r| r.confidence).sum();
        sum / reviews.len() as f64
    }

    /// Find agents whose verdict differs from the final verdict.
    fn find_dissenting_agents(reviews: &[AgentReview], final_verdict: Verdict) -> Vec<String> {
        reviews
            .iter()
            .filter(|r| r.verdict != final_verdict)
            .map(|r| r.agent_id.clone())
            .collect()
    }

    /// Aggregate all issues from all agent reviews.
    fn aggregate_issues(reviews: &[AgentReview]) -> Vec<Issue> {
        reviews
            .iter()
            .flat_map(|r| r.issues.clone())
            .collect()
    }

    /// Check if consensus was unanimous (no dissenting agents).
    ///
    /// # Returns
    ///
    /// `true` if all participating agents agreed with the final verdict.
    pub fn is_unanimous(&self) -> bool {
        self.dissenting_agents.is_empty()
    }

    /// Get the number of agents that participated.
    ///
    /// # Returns
    ///
    /// The total count of agents that provided successful reviews.
    pub fn agent_count(&self) -> usize {
        self.agent_reviews.len()
    }

    /// Get the number of agents that approved (Pass verdict).
    ///
    /// # Returns
    ///
    /// Count of agents whose verdict was `Verdict::Pass`.
    pub fn approval_count(&self) -> usize {
        self.agent_reviews
            .iter()
            .filter(|r| r.verdict == Verdict::Pass)
            .count()
    }

    /// Get the approval ratio (0.0 - 1.0).
    ///
    /// # Returns
    ///
    /// Ratio of approving agents to total agents. Returns 0.0 if no agents participated.
    pub fn approval_ratio(&self) -> f64 {
        if self.agent_reviews.is_empty() {
            return 0.0;
        }
        self.approval_count() as f64 / self.agent_count() as f64
    }
}

/// Outcome of a single agent's review attempt.
#[derive(Debug, Clone)]
pub enum AgentOutcome {
    /// Agent completed review successfully.
    Success(AgentReview),

    /// Agent failed to complete review.
    Failure {
        /// The ID of the agent that failed.
        agent_id: String,
        /// Error message (sanitized).
        error: String,
    },

    /// Agent timed out before completing review.
    Timeout {
        /// The ID of the agent that timed out.
        agent_id: String,
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },
}

impl AgentOutcome {
    /// Get the agent ID from any outcome variant.
    ///
    /// # Returns
    ///
    /// The agent identifier regardless of whether the review succeeded or failed.
    pub fn agent_id(&self) -> &str {
        match self {
            AgentOutcome::Success(review) => &review.agent_id,
            AgentOutcome::Failure { agent_id, .. } => agent_id,
            AgentOutcome::Timeout { agent_id, .. } => agent_id,
        }
    }

    /// Check if this outcome represents a successful review.
    ///
    /// # Returns
    ///
    /// `true` if the agent completed its review without error or timeout.
    pub fn is_success(&self) -> bool {
        matches!(self, AgentOutcome::Success(_))
    }

    /// Extract the review if successful, None otherwise.
    ///
    /// # Returns
    ///
    /// `Some(&AgentReview)` if the outcome is `Success`, `None` otherwise.
    pub fn review(&self) -> Option<&AgentReview> {
        match self {
            AgentOutcome::Success(review) => Some(review),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_adapters::Severity;

    fn create_test_review(agent_id: &str, verdict: Verdict, confidence: f64) -> AgentReview {
        AgentReview {
            agent_id: agent_id.to_string(),
            verdict,
            confidence,
            issues: vec![],
            suggestions: vec![],
            sign_off: verdict == Verdict::Pass,
            reasoning: format!("{} review reasoning", agent_id),
        }
    }

    fn create_test_review_with_issues(
        agent_id: &str,
        verdict: Verdict,
        confidence: f64,
        issues: Vec<Issue>,
    ) -> AgentReview {
        AgentReview {
            agent_id: agent_id.to_string(),
            verdict,
            confidence,
            issues,
            suggestions: vec![],
            sign_off: verdict == Verdict::Pass,
            reasoning: format!("{} review reasoning", agent_id),
        }
    }

    fn create_test_issue(severity: Severity, description: &str) -> Issue {
        Issue {
            severity,
            category: "test".to_string(),
            description: description.to_string(),
            location: None,
            suggested_fix: None,
        }
    }

    #[test]
    fn test_consensus_result_new() {
        let reviews = vec![
            create_test_review("grok", Verdict::Pass, 0.9),
            create_test_review("claude", Verdict::Pass, 0.85),
        ];

        let result = ConsensusResult::new(
            Verdict::Pass,
            reviews,
            "All agents approved".to_string(),
        );

        assert_eq!(result.final_verdict, Verdict::Pass);
        assert_eq!(result.agent_reviews.len(), 2);
        assert!(result.is_unanimous());
    }

    #[test]
    fn test_consensus_confidence_calculation() {
        let reviews = vec![
            create_test_review("grok", Verdict::Pass, 0.9),
            create_test_review("claude", Verdict::Pass, 0.8),
            create_test_review("gemini", Verdict::Pass, 0.7),
        ];

        let result = ConsensusResult::new(Verdict::Pass, reviews, "test".to_string());

        // (0.9 + 0.8 + 0.7) / 3 = 0.8
        assert!((result.consensus_confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_dissenting_agents_detection() {
        let reviews = vec![
            create_test_review("grok", Verdict::Pass, 0.9),
            create_test_review("claude", Verdict::Block, 0.85),
            create_test_review("gemini", Verdict::Pass, 0.8),
        ];

        let result = ConsensusResult::new(Verdict::Pass, reviews, "test".to_string());

        assert!(!result.is_unanimous());
        assert_eq!(result.dissenting_agents.len(), 1);
        assert_eq!(result.dissenting_agents[0], "claude");
    }

    #[test]
    fn test_issue_aggregation() {
        let reviews = vec![
            create_test_review_with_issues(
                "grok",
                Verdict::Issue,
                0.9,
                vec![create_test_issue(Severity::Minor, "Issue 1")],
            ),
            create_test_review_with_issues(
                "claude",
                Verdict::Issue,
                0.85,
                vec![
                    create_test_issue(Severity::Major, "Issue 2"),
                    create_test_issue(Severity::Critical, "Issue 3"),
                ],
            ),
        ];

        let result = ConsensusResult::new(Verdict::Issue, reviews, "test".to_string());

        assert_eq!(result.aggregated_issues.len(), 3);
    }

    #[test]
    fn test_approval_count_and_ratio() {
        let reviews = vec![
            create_test_review("grok", Verdict::Pass, 0.9),
            create_test_review("claude", Verdict::Pass, 0.85),
            create_test_review("gemini", Verdict::Block, 0.8),
        ];

        let result = ConsensusResult::new(Verdict::Pass, reviews, "test".to_string());

        assert_eq!(result.approval_count(), 2);
        assert_eq!(result.agent_count(), 3);
        assert!((result.approval_ratio() - 0.666666).abs() < 0.01);
    }

    #[test]
    fn test_empty_reviews() {
        let result = ConsensusResult::new(Verdict::Pass, vec![], "No reviews".to_string());

        assert_eq!(result.consensus_confidence, 0.0);
        assert_eq!(result.approval_ratio(), 0.0);
        assert!(result.is_unanimous());
    }

    #[test]
    fn test_agent_outcome_success() {
        let review = create_test_review("grok", Verdict::Pass, 0.9);
        let outcome = AgentOutcome::Success(review);

        assert!(outcome.is_success());
        assert_eq!(outcome.agent_id(), "grok");
        assert!(outcome.review().is_some());
    }

    #[test]
    fn test_agent_outcome_failure() {
        let outcome = AgentOutcome::Failure {
            agent_id: "claude".to_string(),
            error: "Connection refused".to_string(),
        };

        assert!(!outcome.is_success());
        assert_eq!(outcome.agent_id(), "claude");
        assert!(outcome.review().is_none());
    }

    #[test]
    fn test_agent_outcome_timeout() {
        let outcome = AgentOutcome::Timeout {
            agent_id: "gemini".to_string(),
            timeout_ms: 30000,
        };

        assert!(!outcome.is_success());
        assert_eq!(outcome.agent_id(), "gemini");
        assert!(outcome.review().is_none());
    }
}
