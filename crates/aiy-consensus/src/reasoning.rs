//! Reasoning generation for consensus decisions.
//!
//! This module generates human-readable explanations of consensus decisions,
//! summarizing key points from all reviews and highlighting disagreements.

use aiy_adapters::{AgentReview, Severity, Verdict};

use crate::aggregation::AggregatedIssue;
use crate::disagreement::{Disagreement, DisagreementLevel, DisagreementSummary};

/// Generator for human-readable consensus explanations.
pub struct ReasoningGenerator;

impl ReasoningGenerator {
    /// Generate a comprehensive explanation of the consensus decision.
    ///
    /// This produces a human-readable summary that includes:
    /// - The final verdict and confidence
    /// - Key points from all reviews
    /// - Areas of agreement
    /// - Areas of disagreement
    /// - Recommendations for human reviewers
    pub fn generate(
        reviews: &[AgentReview],
        final_verdict: Verdict,
        disagreements: &[Disagreement],
    ) -> String {
        if reviews.is_empty() {
            return "No reviews provided.".to_string();
        }

        let mut sections = Vec::new();

        // Header with verdict
        sections.push(Self::generate_verdict_summary(reviews, final_verdict));

        // Agent breakdown
        sections.push(Self::generate_agent_breakdown(reviews));

        // Key points
        let key_points = Self::summarize_key_points(reviews);
        if !key_points.is_empty() {
            sections.push(Self::format_key_points(&key_points));
        }

        // Disagreements section
        if !disagreements.is_empty() {
            sections.push(Self::generate_disagreement_section(disagreements));
        }

        // Recommendations
        let recommendations = Self::generate_recommendations(reviews, final_verdict, disagreements);
        if !recommendations.is_empty() {
            sections.push(format!("Recommendations:\n{}", recommendations));
        }

        sections.join("\n\n")
    }

    /// Summarize key points from all reviews.
    ///
    /// This extracts and deduplicates the most important insights from each agent.
    pub fn summarize_key_points(reviews: &[AgentReview]) -> Vec<String> {
        let mut points = Vec::new();

        // Extract unique suggestions
        let mut seen_suggestions = std::collections::HashSet::new();
        for review in reviews {
            for suggestion in &review.suggestions {
                let normalized = suggestion.to_lowercase();
                if !seen_suggestions.contains(&normalized) {
                    seen_suggestions.insert(normalized);
                    points.push(format!("[{}] {}", review.agent_id, suggestion));
                }
            }
        }

        // Extract significant issues
        for review in reviews {
            for issue in &review.issues {
                if matches!(issue.severity, Severity::Critical | Severity::Major) {
                    let issue_point = match &issue.location {
                        Some(loc) => format!(
                            "[{}] {:?} issue at {}: {}",
                            review.agent_id, issue.severity, loc, issue.description
                        ),
                        None => format!(
                            "[{}] {:?} issue: {}",
                            review.agent_id, issue.severity, issue.description
                        ),
                    };
                    points.push(issue_point);
                }
            }
        }

        // Add high-confidence reasoning snippets
        for review in reviews {
            if review.confidence >= 0.9 && !review.reasoning.is_empty() {
                let snippet = Self::extract_reasoning_snippet(&review.reasoning);
                if !snippet.is_empty() {
                    points.push(format!("[{}] {}", review.agent_id, snippet));
                }
            }
        }

        // Limit to most important points
        points.truncate(10);
        points
    }

    /// Generate the verdict summary header.
    fn generate_verdict_summary(reviews: &[AgentReview], final_verdict: Verdict) -> String {
        let avg_confidence: f64 = if reviews.is_empty() {
            0.0
        } else {
            reviews.iter().map(|r| r.confidence).sum::<f64>() / reviews.len() as f64
        };

        let verdict_str = match final_verdict {
            Verdict::Pass => "PASS",
            Verdict::Issue => "ISSUE",
            Verdict::Block => "BLOCK",
        };

        let pass_count = reviews
            .iter()
            .filter(|r| r.verdict == Verdict::Pass)
            .count();
        let total = reviews.len();

        format!(
            "Consensus Verdict: {} ({}/{} approve, {:.0}% average confidence)",
            verdict_str,
            pass_count,
            total,
            avg_confidence * 100.0
        )
    }

    /// Generate a breakdown of each agent's verdict.
    fn generate_agent_breakdown(reviews: &[AgentReview]) -> String {
        let mut lines = vec!["Agent Breakdown:".to_string()];

        for review in reviews {
            let verdict_icon = match review.verdict {
                Verdict::Pass => "[+]",
                Verdict::Issue => "[!]",
                Verdict::Block => "[X]",
            };
            let issue_count = review.issues.len();
            let issue_text = if issue_count == 0 {
                String::new()
            } else if issue_count == 1 {
                " (1 issue)".to_string()
            } else {
                format!(" ({} issues)", issue_count)
            };

            lines.push(format!(
                "  {} {}: {:?} ({:.0}% confidence){}",
                verdict_icon,
                review.agent_id,
                review.verdict,
                review.confidence * 100.0,
                issue_text
            ));
        }

        lines.join("\n")
    }

    /// Format key points into a readable section.
    fn format_key_points(points: &[String]) -> String {
        let mut lines = vec!["Key Points:".to_string()];
        for (i, point) in points.iter().enumerate() {
            lines.push(format!("  {}. {}", i + 1, point));
        }
        lines.join("\n")
    }

    /// Generate the disagreement section.
    fn generate_disagreement_section(disagreements: &[Disagreement]) -> String {
        let summary = DisagreementSummary::from_disagreements(disagreements);

        let mut lines = vec![format!(
            "Disagreements ({} total, {} severe, {} moderate, {} minor):",
            summary.total, summary.severe_count, summary.moderate_count, summary.minor_count
        )];

        for (i, d) in disagreements.iter().enumerate().take(5) {
            let level_indicator = match d.level {
                DisagreementLevel::Severe => "[!!!]",
                DisagreementLevel::Moderate => "[!!]",
                DisagreementLevel::Minor => "[!]",
            };

            lines.push(format!("  {} {}. {}", level_indicator, i + 1, d.topic));
            lines.push(format!("     Summary: {}", d.summary));

            if !d.positions.is_empty() {
                let positions: Vec<String> = d
                    .positions
                    .iter()
                    .map(|p| format!("{}: {:?}", p.agent_id, p.verdict))
                    .collect();
                lines.push(format!("     Positions: {}", positions.join(", ")));
            }
        }

        if disagreements.len() > 5 {
            lines.push(format!(
                "  ... and {} more disagreements",
                disagreements.len() - 5
            ));
        }

        if summary.requires_manual_review {
            lines.push(String::new());
            lines.push("  *** MANUAL REVIEW RECOMMENDED ***".to_string());
        }

        lines.join("\n")
    }

    /// Generate recommendations based on the consensus results.
    fn generate_recommendations(
        reviews: &[AgentReview],
        final_verdict: Verdict,
        disagreements: &[Disagreement],
    ) -> String {
        let mut recommendations = Vec::new();

        // Based on verdict
        match final_verdict {
            Verdict::Pass => {
                if disagreements.is_empty() {
                    recommendations
                        .push("- All agents agree. Code is ready for merge.".to_string());
                } else {
                    recommendations.push(
                        "- Majority approve, but review flagged disagreements before merging."
                            .to_string(),
                    );
                }
            }
            Verdict::Issue => {
                recommendations
                    .push("- Address the reported issues before proceeding.".to_string());

                // Summarize issue categories
                let mut categories: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for review in reviews {
                    for issue in &review.issues {
                        categories.insert(issue.category.clone());
                    }
                }
                if !categories.is_empty() {
                    recommendations.push(format!(
                        "- Focus areas: {}",
                        categories.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            Verdict::Block => {
                recommendations
                    .push("- DO NOT MERGE. Critical issues require resolution.".to_string());

                // Find blocking agents and their reasons
                let blockers: Vec<&AgentReview> = reviews
                    .iter()
                    .filter(|r| r.verdict == Verdict::Block)
                    .collect();
                for blocker in blockers {
                    if !blocker.reasoning.is_empty() {
                        let snippet = Self::extract_reasoning_snippet(&blocker.reasoning);
                        recommendations.push(format!("- {} blocks: {}", blocker.agent_id, snippet));
                    }
                }
            }
        }

        // Based on disagreements
        let summary = DisagreementSummary::from_disagreements(disagreements);
        if summary.requires_manual_review {
            recommendations.push(
                "- Significant disagreements detected. Human review strongly recommended."
                    .to_string(),
            );
        }

        // Based on confidence spread
        if !reviews.is_empty() {
            let confidences: Vec<f64> = reviews.iter().map(|r| r.confidence).collect();
            let min_conf = confidences.iter().cloned().fold(f64::INFINITY, f64::min);
            if min_conf < 0.7 {
                recommendations.push(format!(
                    "- Low confidence from some agents ({:.0}%). Consider additional review.",
                    min_conf * 100.0
                ));
            }
        }

        recommendations.join("\n")
    }

    /// Extract a short snippet from reasoning text.
    fn extract_reasoning_snippet(reasoning: &str) -> String {
        let trimmed = reasoning.trim();

        // Take first sentence or first N characters
        if let Some(period_idx) = trimmed.find(". ") {
            if period_idx <= 150 {
                return trimmed[..=period_idx].to_string();
            }
        }

        // Truncate if too long
        if trimmed.len() > 100 {
            format!("{}...", &trimmed[..100])
        } else {
            trimmed.to_string()
        }
    }
}

/// Format aggregated issues into a human-readable summary.
///
/// Creates a numbered list of issues with severity, description, location,
/// and the agents that reported each issue.
///
/// # Arguments
///
/// * `issues` - Slice of aggregated issues to format
///
/// # Returns
///
/// A formatted multi-line string. Returns "No issues found." if empty.
pub fn format_aggregated_issues(issues: &[AggregatedIssue]) -> String {
    if issues.is_empty() {
        return "No issues found.".to_string();
    }

    let mut lines = vec![format!("Aggregated Issues ({} total):", issues.len())];

    for (i, issue) in issues.iter().enumerate() {
        let severity_str = format!("{:?}", issue.agreed_severity);
        let agents_str = issue.reporting_agents.join(", ");
        let location_str = issue
            .location
            .as_ref()
            .map(|l| format!(" at {}", l))
            .unwrap_or_default();

        lines.push(format!(
            "  {}. [{}] {}{} ({} agents, {:.0}% confidence)",
            i + 1,
            severity_str,
            issue.merged_description,
            location_str,
            issue.reporting_agents.len(),
            issue.confidence * 100.0
        ));
        lines.push(format!("     Reported by: {}", agents_str));
    }

    lines.join("\n")
}

/// Format a verdict for display.
///
/// Converts a verdict enum to a human-readable status string.
///
/// # Arguments
///
/// * `verdict` - The verdict to format
///
/// # Returns
///
/// A static string describing the verdict status.
pub fn format_verdict(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "PASS - Approved",
        Verdict::Issue => "ISSUE - Needs Attention",
        Verdict::Block => "BLOCK - Do Not Merge",
    }
}

/// Format confidence as a percentage string.
///
/// Converts a confidence value (0.0 to 1.0) to a percentage string.
///
/// # Arguments
///
/// * `confidence` - Confidence value between 0.0 and 1.0
///
/// # Returns
///
/// A string like "95%" representing the confidence as a percentage.
pub fn format_confidence(confidence: f64) -> String {
    format!("{:.0}%", confidence * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_adapters::Issue;

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
        suggestions: Vec<String>,
        reasoning: &str,
    ) -> AgentReview {
        AgentReview {
            agent_id: agent_id.to_string(),
            verdict,
            confidence,
            issues,
            suggestions,
            sign_off: verdict == Verdict::Pass,
            reasoning: reasoning.to_string(),
        }
    }

    #[test]
    fn test_generate_empty_reviews() {
        let result = ReasoningGenerator::generate(&[], Verdict::Pass, &[]);
        assert_eq!(result, "No reviews provided.");
    }

    #[test]
    fn test_generate_single_passing_review() {
        let reviews = vec![make_review(
            "grok",
            Verdict::Pass,
            0.95,
            vec![],
            vec![],
            "Code looks good.",
        )];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Pass, &[]);

        assert!(result.contains("PASS"));
        assert!(result.contains("1/1 approve"));
        assert!(result.contains("95%"));
        assert!(result.contains("grok"));
    }

    #[test]
    fn test_generate_with_disagreements() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![], vec![], "Looks fine."),
            make_review(
                "claude",
                Verdict::Block,
                0.9,
                vec![],
                vec![],
                "Security issue found.",
            ),
        ];

        let disagreements = vec![Disagreement {
            topic: "Overall verdict".to_string(),
            positions: vec![],
            level: DisagreementLevel::Severe,
            location: None,
            summary: "grok approves; claude blocks".to_string(),
        }];

        let result = ReasoningGenerator::generate(&reviews, Verdict::Block, &disagreements);

        assert!(result.contains("BLOCK"));
        assert!(result.contains("Disagreements"));
        assert!(result.contains("severe"));
        assert!(result.contains("MANUAL REVIEW RECOMMENDED"));
    }

    #[test]
    fn test_summarize_key_points_empty() {
        let points = ReasoningGenerator::summarize_key_points(&[]);
        assert!(points.is_empty());
    }

    #[test]
    fn test_summarize_key_points_with_suggestions() {
        let reviews = vec![
            make_review(
                "grok",
                Verdict::Pass,
                0.9,
                vec![],
                vec!["Consider adding error handling.".to_string()],
                "",
            ),
            make_review(
                "claude",
                Verdict::Pass,
                0.9,
                vec![],
                vec!["Add documentation.".to_string()],
                "",
            ),
        ];

        let points = ReasoningGenerator::summarize_key_points(&reviews);
        assert_eq!(points.len(), 2);
        assert!(points[0].contains("[grok]"));
        assert!(points[1].contains("[claude]"));
    }

    #[test]
    fn test_summarize_key_points_with_critical_issues() {
        let reviews = vec![make_review(
            "grok",
            Verdict::Issue,
            0.9,
            vec![make_issue(
                Severity::Critical,
                "security",
                "SQL injection",
                Some("db.rs:42"),
            )],
            vec![],
            "",
        )];

        let points = ReasoningGenerator::summarize_key_points(&reviews);
        assert!(!points.is_empty());
        assert!(points[0].contains("Critical"));
        assert!(points[0].contains("SQL injection"));
    }

    #[test]
    fn test_summarize_key_points_limits_to_10() {
        let mut suggestions = Vec::new();
        for i in 0..15 {
            suggestions.push(format!("Suggestion {}", i));
        }

        let reviews = vec![make_review(
            "grok",
            Verdict::Pass,
            0.9,
            vec![],
            suggestions,
            "",
        )];

        let points = ReasoningGenerator::summarize_key_points(&reviews);
        assert!(points.len() <= 10);
    }

    #[test]
    fn test_format_verdict() {
        assert_eq!(format_verdict(Verdict::Pass), "PASS - Approved");
        assert_eq!(format_verdict(Verdict::Issue), "ISSUE - Needs Attention");
        assert_eq!(format_verdict(Verdict::Block), "BLOCK - Do Not Merge");
    }

    #[test]
    fn test_format_confidence() {
        assert_eq!(format_confidence(0.95), "95%");
        assert_eq!(format_confidence(0.0), "0%");
        assert_eq!(format_confidence(1.0), "100%");
    }

    #[test]
    fn test_format_aggregated_issues_empty() {
        let result = format_aggregated_issues(&[]);
        assert_eq!(result, "No issues found.");
    }

    #[test]
    fn test_format_aggregated_issues() {
        let issues = vec![AggregatedIssue {
            original_issues: vec![],
            merged_description: "Test issue".to_string(),
            agreed_severity: Severity::Major,
            reporting_agents: vec!["grok".to_string(), "claude".to_string()],
            confidence: 1.0,
            category: "security".to_string(),
            location: Some("file.rs:10".to_string()),
        }];

        let result = format_aggregated_issues(&issues);
        assert!(result.contains("1 total"));
        assert!(result.contains("Major"));
        assert!(result.contains("Test issue"));
        assert!(result.contains("file.rs:10"));
        assert!(result.contains("grok, claude"));
    }

    #[test]
    fn test_extract_reasoning_snippet_short() {
        let snippet = ReasoningGenerator::extract_reasoning_snippet("Short reason.");
        assert_eq!(snippet, "Short reason.");
    }

    #[test]
    fn test_extract_reasoning_snippet_first_sentence() {
        let snippet = ReasoningGenerator::extract_reasoning_snippet(
            "First sentence. Second sentence. Third sentence.",
        );
        assert_eq!(snippet, "First sentence.");
    }

    #[test]
    fn test_extract_reasoning_snippet_truncates_long() {
        let long_text = "A".repeat(200);
        let snippet = ReasoningGenerator::extract_reasoning_snippet(&long_text);
        assert!(snippet.len() <= 103); // 100 + "..."
        assert!(snippet.ends_with("..."));
    }

    #[test]
    fn test_generate_recommendations_pass_no_disagreements() {
        let reviews = vec![make_review("grok", Verdict::Pass, 0.9, vec![], vec![], "")];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Pass, &[]);
        assert!(result.contains("ready for merge"));
    }

    #[test]
    fn test_generate_recommendations_pass_with_disagreements() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![], vec![], ""),
            make_review("claude", Verdict::Issue, 0.9, vec![], vec![], ""),
        ];
        let disagreements = vec![Disagreement {
            topic: "test".to_string(),
            positions: vec![],
            level: DisagreementLevel::Minor,
            location: None,
            summary: "test".to_string(),
        }];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Pass, &disagreements);
        assert!(result.contains("review flagged disagreements"));
    }

    #[test]
    fn test_generate_recommendations_issue() {
        let reviews = vec![make_review(
            "grok",
            Verdict::Issue,
            0.9,
            vec![make_issue(Severity::Minor, "style", "formatting", None)],
            vec![],
            "",
        )];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Issue, &[]);
        assert!(result.contains("Address the reported issues"));
    }

    #[test]
    fn test_generate_recommendations_block() {
        let reviews = vec![make_review(
            "grok",
            Verdict::Block,
            0.9,
            vec![],
            vec![],
            "Critical security flaw.",
        )];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Block, &[]);
        assert!(result.contains("DO NOT MERGE"));
        assert!(result.contains("grok blocks"));
    }

    #[test]
    fn test_generate_recommendations_low_confidence() {
        let reviews = vec![make_review("grok", Verdict::Pass, 0.5, vec![], vec![], "")];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Pass, &[]);
        assert!(result.contains("Low confidence"));
    }

    #[test]
    fn test_agent_breakdown_formatting() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![], vec![], ""),
            make_review(
                "claude",
                Verdict::Issue,
                0.85,
                vec![make_issue(Severity::Minor, "test", "desc", None)],
                vec![],
                "",
            ),
            make_review("gemini", Verdict::Block, 0.95, vec![], vec![], ""),
        ];
        let result = ReasoningGenerator::generate(&reviews, Verdict::Block, &[]);

        assert!(result.contains("[+]")); // Pass
        assert!(result.contains("[!]")); // Issue
        assert!(result.contains("[X]")); // Block
        assert!(result.contains("(1 issue)"));
    }
}
