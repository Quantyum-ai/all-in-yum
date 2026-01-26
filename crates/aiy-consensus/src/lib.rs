//! # aiy-consensus
//!
//! Consensus engine for All-in-Yum multi-agent code review pipeline.
//!
//! This crate provides the core consensus mechanism for orchestrating multiple
//! AI agents to review code artifacts and reach a collective decision.
//!
//! ## Features
//!
//! - **Parallel Execution**: Run multiple agents concurrently with configurable timeouts
//! - **Voting Strategies**: Choose from Unanimous, Majority, Any, or Weighted voting
//! - **Issue Aggregation**: Merge and deduplicate issues from multiple agent reviews
//! - **Disagreement Detection**: Identify and classify conflicts between agents
//! - **Reasoning Generation**: Generate human-readable explanations of consensus decisions
//! - **Robust Error Handling**: Graceful handling of agent failures and timeouts
//!
//! ## Quick Start
//!
//! ```ignore
//! use aiy_consensus::{ConsensusEngine, VotingStrategy};
//! use std::time::Duration;
//!
//! // Create engine with adapters
//! let engine = ConsensusEngine::new(adapters)
//!     .with_strategy(VotingStrategy::Majority)
//!     .with_timeout(Duration::from_secs(30));
//!
//! // Run consensus review
//! let result = engine.review("fn main() { }").await?;
//! println!("Verdict: {:?}", result.final_verdict);
//! ```
//!
//! ## Voting Strategies
//!
//! - **Unanimous**: All agents must pass for consensus to pass
//! - **Majority**: More than 50% of agents must pass
//! - **Any**: At least one agent passing is sufficient
//! - **Weighted**: Votes are weighted by agent confidence or explicit weights
//!
//! ## Architecture
//!
//! The consensus engine operates in multiple phases:
//!
//! 1. **Parallel Execution Phase**: All agent adapters are invoked concurrently
//!    with individual timeout handling. Failed or timed-out agents are tracked.
//!
//! 2. **Voting Phase**: The configured voting strategy is applied to determine
//!    the final verdict based on collected reviews.
//!
//! 3. **Aggregation Phase**: Issues from all agent reviews are collected, merged,
//!    and deduplicated. Similar issues are combined, and severity is determined
//!    by agent agreement.
//!
//! 4. **Disagreement Analysis Phase**: The engine identifies where agents disagree,
//!    classifies the severity of disagreements (Minor, Moderate, Severe), and
//!    determines if manual review is needed.
//!
//! 5. **Reasoning Generation Phase**: A human-readable summary is generated
//!    explaining the consensus decision, key points from reviews, and any
//!    areas requiring attention.

#![warn(missing_docs)]

// Core consensus modules (Agent 7 - Consensus Core)
pub mod engine;
pub mod error;
pub mod parallel;
pub mod strategies;
pub mod types;

// Aggregation modules (Agent 8 - Consensus Aggregation)
pub mod aggregation;
pub mod disagreement;
pub mod reasoning;

// Re-export main types from consensus core
pub use engine::{ConsensusEngine, ConsensusEngineBuilder};
pub use error::ConsensusError;
pub use parallel::{
    calculate_success_rate, collect_errors, execute_agent_with_timeout, execute_agents_parallel,
    execute_agents_sequential, extract_successful_reviews, DEFAULT_TIMEOUT_MS,
};
pub use strategies::{VotingStrategy, WeightedConfig};
pub use types::{AgentOutcome, ConsensusResult};

// Re-export main types from aggregation
pub use aggregation::{
    calculate_average_confidence, determine_overall_verdict, AggregatedIssue, IssueAggregator,
};
pub use disagreement::{
    AgentPosition, Disagreement, DisagreementAnalyzer, DisagreementLevel, DisagreementSummary,
};
pub use reasoning::{
    format_aggregated_issues, format_confidence, format_verdict, ReasoningGenerator,
};

// Re-export commonly used types from aiy-adapters for convenience
pub use aiy_adapters::{AgentAdapter, AgentReview, Issue, Severity, Verdict};

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_adapters::{AgentReview, Issue, Severity, Verdict};

    fn make_issue(severity: Severity, category: &str, desc: &str, location: Option<&str>) -> Issue {
        Issue {
            severity,
            category: category.to_string(),
            description: desc.to_string(),
            location: location.map(|s| s.to_string()),
            suggested_fix: None,
        }
    }

    fn make_review(
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
            reasoning: "Test reasoning".to_string(),
        }
    }

    #[test]
    fn test_integration_full_consensus_flow() {
        // Simulate a multi-agent review scenario
        let reviews = vec![
            make_review(
                "grok",
                Verdict::Pass,
                0.95,
                vec![make_issue(
                    Severity::Nit,
                    "style",
                    "Consider adding documentation",
                    Some("lib.rs:10"),
                )],
            ),
            make_review(
                "claude",
                Verdict::Issue,
                0.88,
                vec![
                    make_issue(
                        Severity::Minor,
                        "security",
                        "Input validation could be stronger",
                        Some("api.rs:45"),
                    ),
                    make_issue(
                        Severity::Nit,
                        "style",
                        "Add documentation",
                        Some("lib.rs:12"),
                    ),
                ],
            ),
            make_review("gemini", Verdict::Pass, 0.92, vec![]),
        ];

        // Step 1: Aggregate issues
        let aggregated_issues = IssueAggregator::merge_issues(&reviews);
        assert!(!aggregated_issues.is_empty());

        // The style issue should be merged between grok and claude
        let style_issues: Vec<_> = aggregated_issues
            .iter()
            .filter(|i| i.category == "style")
            .collect();
        assert!(!style_issues.is_empty());

        // Step 2: Detect disagreements
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);

        // Should have verdict disagreement (grok/gemini pass vs claude issue)
        let verdict_disagreements: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Overall verdict")
            .collect();
        assert!(!verdict_disagreements.is_empty());

        // Step 3: Determine overall verdict
        let overall_verdict = determine_overall_verdict(&reviews, 0.5);
        assert_eq!(overall_verdict, Verdict::Pass); // 2/3 pass

        // Step 4: Calculate average confidence
        let avg_confidence = calculate_average_confidence(&reviews);
        assert!(avg_confidence > 0.9);

        // Step 5: Generate reasoning
        let reasoning = ReasoningGenerator::generate(&reviews, overall_verdict, &disagreements);
        assert!(reasoning.contains("PASS"));
        assert!(reasoning.contains("grok"));
        assert!(reasoning.contains("claude"));
        assert!(reasoning.contains("gemini"));
    }

    #[test]
    fn test_integration_unanimous_block() {
        let reviews = vec![
            make_review(
                "grok",
                Verdict::Block,
                0.98,
                vec![make_issue(
                    Severity::Critical,
                    "security",
                    "SQL injection vulnerability",
                    Some("db.rs:42"),
                )],
            ),
            make_review(
                "claude",
                Verdict::Block,
                0.96,
                vec![make_issue(
                    Severity::Critical,
                    "security",
                    "SQL injection risk",
                    Some("db.rs:44"),
                )],
            ),
        ];

        // Issues should be merged
        let aggregated_issues = IssueAggregator::merge_issues(&reviews);
        assert_eq!(aggregated_issues.len(), 1); // Both report the same issue
        assert_eq!(aggregated_issues[0].reporting_agents.len(), 2);
        assert_eq!(aggregated_issues[0].agreed_severity, Severity::Critical);

        // No verdict disagreement
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let verdict_disagreements: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Overall verdict")
            .collect();
        assert!(verdict_disagreements.is_empty());

        // Overall verdict should be Block
        let overall_verdict = determine_overall_verdict(&reviews, 0.5);
        assert_eq!(overall_verdict, Verdict::Block);

        // Reasoning should indicate DO NOT MERGE
        let reasoning = ReasoningGenerator::generate(&reviews, overall_verdict, &disagreements);
        assert!(reasoning.contains("BLOCK"));
        assert!(reasoning.contains("DO NOT MERGE"));
    }

    #[test]
    fn test_integration_severe_disagreement() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.95, vec![]),
            make_review(
                "claude",
                Verdict::Block,
                0.92,
                vec![make_issue(
                    Severity::Critical,
                    "security",
                    "Possible data leak",
                    None,
                )],
            ),
        ];

        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);

        // Should have severe disagreement
        let severe: Vec<_> = disagreements
            .iter()
            .filter(|d| d.level == DisagreementLevel::Severe)
            .collect();
        assert!(!severe.is_empty());

        // Summary should recommend manual review
        let summary = DisagreementSummary::from_disagreements(&disagreements);
        assert!(summary.requires_manual_review);

        // Reasoning should highlight the disagreement
        let overall_verdict = determine_overall_verdict(&reviews, 0.5);
        let reasoning = ReasoningGenerator::generate(&reviews, overall_verdict, &disagreements);
        assert!(reasoning.contains("MANUAL REVIEW RECOMMENDED"));
    }
}
