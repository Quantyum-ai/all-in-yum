//! Issue aggregation and merging logic for consensus results.
//!
//! This module provides functionality to:
//! - Merge similar issues from multiple agent reviews
//! - Deduplicate issues based on location and description
//! - Calculate severity based on agent agreement

use aiy_adapters::{AgentReview, Issue, Severity, Verdict};
use std::collections::HashMap;

/// Aggregator for merging issues from multiple agent reviews.
pub struct IssueAggregator;

/// An issue that has been aggregated from multiple agent reviews.
#[derive(Debug, Clone)]
pub struct AggregatedIssue {
    /// The original issues that were merged into this aggregated issue.
    pub original_issues: Vec<Issue>,
    /// A merged description combining insights from all reporting agents.
    pub merged_description: String,
    /// The agreed-upon severity (most severe wins, modulated by agreement).
    pub agreed_severity: Severity,
    /// The IDs of agents that reported this issue.
    pub reporting_agents: Vec<String>,
    /// Confidence score based on agent agreement (0.0 to 1.0).
    pub confidence: f64,
    /// The category of the issue.
    pub category: String,
    /// The location if applicable (file:line or similar).
    pub location: Option<String>,
}

impl AggregatedIssue {
    /// Create a new aggregated issue from a single issue.
    pub fn from_single(issue: Issue, agent_id: String) -> Self {
        Self {
            merged_description: issue.description.clone(),
            agreed_severity: issue.severity,
            reporting_agents: vec![agent_id],
            category: issue.category.clone(),
            location: issue.location.clone(),
            confidence: 1.0, // Single agent, full confidence in their own report
            original_issues: vec![issue],
        }
    }

    /// Merge another issue into this aggregated issue.
    pub fn merge(&mut self, issue: Issue, agent_id: String, total_agents: usize) {
        // Add to original issues
        self.original_issues.push(issue.clone());

        // Add agent if not already present
        if !self.reporting_agents.contains(&agent_id) {
            self.reporting_agents.push(agent_id);
        }

        // Update severity - take the most severe
        self.agreed_severity = most_severe(self.agreed_severity, issue.severity);

        // Update confidence based on agreement ratio
        self.confidence = self.reporting_agents.len() as f64 / total_agents as f64;

        // Merge descriptions if they differ
        if !self.merged_description.contains(&issue.description) {
            self.merged_description = format!(
                "{}; Additional perspective: {}",
                self.merged_description, issue.description
            );
        }
    }
}

impl IssueAggregator {
    /// Merge similar issues from multiple reviews into aggregated issues.
    ///
    /// Issues are considered similar if they have matching:
    /// - Category
    /// - Location (if specified)
    /// - Similar description (normalized)
    pub fn merge_issues(reviews: &[AgentReview]) -> Vec<AggregatedIssue> {
        if reviews.is_empty() {
            return Vec::new();
        }

        let total_agents = reviews.len();
        let mut aggregated: Vec<AggregatedIssue> = Vec::new();

        for review in reviews {
            for issue in &review.issues {
                // Try to find an existing aggregated issue to merge with
                let matching_idx = aggregated.iter().position(|agg| {
                    Self::issues_similar(issue, &agg.original_issues[0])
                });

                if let Some(idx) = matching_idx {
                    aggregated[idx].merge(issue.clone(), review.agent_id.clone(), total_agents);
                } else {
                    aggregated.push(AggregatedIssue::from_single(
                        issue.clone(),
                        review.agent_id.clone(),
                    ));
                }
            }
        }

        // Sort by severity (most severe first) then by confidence
        aggregated.sort_by(|a, b| {
            let severity_cmp = severity_rank(b.agreed_severity).cmp(&severity_rank(a.agreed_severity));
            if severity_cmp == std::cmp::Ordering::Equal {
                b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                severity_cmp
            }
        });

        aggregated
    }

    /// Deduplicate issues based on location and description.
    ///
    /// Returns a list of unique issues, preferring more severe versions
    /// when duplicates are found.
    pub fn deduplicate(issues: &[Issue]) -> Vec<Issue> {
        if issues.is_empty() {
            return Vec::new();
        }

        let mut unique: Vec<Issue> = Vec::new();

        for issue in issues {
            let matching_idx = unique.iter().position(|existing| {
                Self::issues_similar(issue, existing)
            });

            if let Some(idx) = matching_idx {
                // Keep the more severe one
                if severity_rank(issue.severity) > severity_rank(unique[idx].severity) {
                    unique[idx] = issue.clone();
                }
            } else {
                unique.push(issue.clone());
            }
        }

        unique
    }

    /// Calculate overall severity based on agent agreement.
    ///
    /// The severity is determined by:
    /// 1. Finding the most common severity level
    /// 2. If there's a tie, preferring the more severe option
    /// 3. Upgrading if high-confidence agents report more severe issues
    pub fn calculate_severity(issues: &[Issue]) -> Severity {
        if issues.is_empty() {
            return Severity::Nit;
        }

        // Count occurrences of each severity
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for issue in issues {
            *counts.entry(severity_rank(issue.severity)).or_insert(0) += 1;
        }

        // Find the most common, preferring more severe in ties
        let mut max_count = 0;
        let mut result_rank = 0u8;

        for (&rank, &count) in &counts {
            if count > max_count || (count == max_count && rank > result_rank) {
                max_count = count;
                result_rank = rank;
            }
        }

        rank_to_severity(result_rank)
    }

    /// Check if two issues are similar enough to be merged.
    fn issues_similar(a: &Issue, b: &Issue) -> bool {
        // Same category is required
        if a.category.to_lowercase() != b.category.to_lowercase() {
            return false;
        }

        // If both have locations, they must match
        match (&a.location, &b.location) {
            (Some(loc_a), Some(loc_b)) => {
                if !locations_similar(loc_a, loc_b) {
                    return false;
                }
            }
            (None, Some(_)) | (Some(_), None) => {
                // One has location, one doesn't - may still be similar
            }
            (None, None) => {
                // Neither has location, check description similarity
            }
        }

        // Check description similarity using normalized comparison
        descriptions_similar(&a.description, &b.description)
    }
}

/// Determine if two locations are similar.
fn locations_similar(a: &str, b: &str) -> bool {
    // Exact match
    if a == b {
        return true;
    }

    // Extract file and line if in "file:line" format
    let parts_a: Vec<&str> = a.split(':').collect();
    let parts_b: Vec<&str> = b.split(':').collect();

    // Same file
    if !parts_a.is_empty() && !parts_b.is_empty() && parts_a[0] == parts_b[0] {
        // If we have line numbers, check if they're close (within 5 lines)
        if parts_a.len() > 1 && parts_b.len() > 1 {
            if let (Ok(line_a), Ok(line_b)) = (parts_a[1].parse::<i64>(), parts_b[1].parse::<i64>()) {
                return (line_a - line_b).abs() <= 5;
            }
        }
        return true;
    }

    false
}

/// Check if two descriptions are similar.
fn descriptions_similar(a: &str, b: &str) -> bool {
    let norm_a = normalize_text(a);
    let norm_b = normalize_text(b);

    // Exact match after normalization
    if norm_a == norm_b {
        return true;
    }

    // Check if one contains the other
    if norm_a.contains(&norm_b) || norm_b.contains(&norm_a) {
        return true;
    }

    // Calculate word overlap similarity.
    let words_a: std::collections::HashSet<&str> = norm_a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = norm_b.split_whitespace().collect();

    if words_a.is_empty() || words_b.is_empty() {
        return false;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        return false;
    }

    let jaccard = intersection as f64 / union as f64;
    if jaccard >= 0.5 {
        return true;
    }

    // Jaccard can be harsh when one description is a strict superset of the other.
    // Fall back to overlap coefficient, but require at least 2 shared words to avoid
    // over-merging on generic single-word matches.
    let min_len = words_a.len().min(words_b.len());
    if min_len == 0 {
        return false;
    }

    let overlap = intersection as f64 / min_len as f64;
    overlap >= 0.66 && intersection >= 2
}

/// Normalize text for comparison.
fn normalize_text(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Get the numeric rank of a severity level (higher = more severe).
fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Nit => 0,
        Severity::Minor => 1,
        Severity::Major => 2,
        Severity::Critical => 3,
    }
}

/// Convert a numeric rank back to a severity level.
fn rank_to_severity(rank: u8) -> Severity {
    match rank {
        0 => Severity::Nit,
        1 => Severity::Minor,
        2 => Severity::Major,
        _ => Severity::Critical,
    }
}

/// Return the more severe of two severity levels.
fn most_severe(a: Severity, b: Severity) -> Severity {
    if severity_rank(a) >= severity_rank(b) {
        a
    } else {
        b
    }
}

/// Calculate the average confidence from a list of agent reviews.
pub fn calculate_average_confidence(reviews: &[AgentReview]) -> f64 {
    if reviews.is_empty() {
        return 0.0;
    }
    reviews.iter().map(|r| r.confidence).sum::<f64>() / reviews.len() as f64
}

/// Determine the overall verdict based on individual verdicts and a threshold.
pub fn determine_overall_verdict(reviews: &[AgentReview], pass_threshold: f64) -> Verdict {
    if reviews.is_empty() {
        return Verdict::Block;
    }

    let total = reviews.len() as f64;
    let pass_count = reviews.iter().filter(|r| r.verdict == Verdict::Pass).count() as f64;
    let block_count = reviews.iter().filter(|r| r.verdict == Verdict::Block).count() as f64;

    // Any block means the overall verdict is Block
    if block_count > 0.0 {
        // Unless pass_count significantly outweighs block_count
        if pass_count / total >= pass_threshold && block_count / total < 0.25 {
            return Verdict::Issue;
        }
        return Verdict::Block;
    }

    // Check if enough agents passed
    if pass_count / total >= pass_threshold {
        Verdict::Pass
    } else {
        Verdict::Issue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(severity: Severity, category: &str, desc: &str, location: Option<&str>) -> Issue {
        Issue {
            severity,
            category: category.to_string(),
            description: desc.to_string(),
            location: location.map(|s| s.to_string()),
            suggested_fix: None,
        }
    }

    fn make_review(agent_id: &str, verdict: Verdict, issues: Vec<Issue>) -> AgentReview {
        AgentReview {
            agent_id: agent_id.to_string(),
            verdict,
            confidence: 0.9,
            issues,
            suggestions: vec![],
            sign_off: verdict == Verdict::Pass,
            reasoning: "Test reasoning".to_string(),
        }
    }

    #[test]
    fn test_merge_empty_reviews() {
        let result = IssueAggregator::merge_issues(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_merge_single_review() {
        let issue = make_issue(Severity::Minor, "security", "SQL injection risk", Some("db.rs:42"));
        let review = make_review("grok", Verdict::Issue, vec![issue.clone()]);

        let result = IssueAggregator::merge_issues(&[review]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].reporting_agents, vec!["grok"]);
        assert_eq!(result[0].agreed_severity, Severity::Minor);
        assert_eq!(result[0].confidence, 1.0);
    }

    #[test]
    fn test_merge_similar_issues_from_multiple_agents() {
        let issue1 = make_issue(Severity::Minor, "security", "SQL injection vulnerability", Some("db.rs:42"));
        let issue2 = make_issue(Severity::Major, "security", "SQL injection risk detected", Some("db.rs:44"));

        let review1 = make_review("grok", Verdict::Issue, vec![issue1]);
        let review2 = make_review("claude", Verdict::Issue, vec![issue2]);

        let result = IssueAggregator::merge_issues(&[review1, review2]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].reporting_agents.len(), 2);
        assert!(result[0].reporting_agents.contains(&"grok".to_string()));
        assert!(result[0].reporting_agents.contains(&"claude".to_string()));
        // Should take the more severe rating
        assert_eq!(result[0].agreed_severity, Severity::Major);
        assert_eq!(result[0].confidence, 1.0); // 2/2 agents agree
    }

    #[test]
    fn test_merge_different_issues_stay_separate() {
        let issue1 = make_issue(Severity::Minor, "security", "SQL injection", Some("db.rs:42"));
        let issue2 = make_issue(Severity::Minor, "performance", "Slow loop", Some("engine.rs:100"));

        let review1 = make_review("grok", Verdict::Issue, vec![issue1]);
        let review2 = make_review("claude", Verdict::Issue, vec![issue2]);

        let result = IssueAggregator::merge_issues(&[review1, review2]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplicate_empty() {
        let result = IssueAggregator::deduplicate(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplicate_keeps_most_severe() {
        let issue1 = make_issue(Severity::Minor, "security", "SQL injection risk", Some("db.rs:42"));
        let issue2 = make_issue(Severity::Critical, "security", "SQL injection vulnerability", Some("db.rs:42"));

        let result = IssueAggregator::deduplicate(&[issue1, issue2]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, Severity::Critical);
    }

    #[test]
    fn test_calculate_severity_empty() {
        assert_eq!(IssueAggregator::calculate_severity(&[]), Severity::Nit);
    }

    #[test]
    fn test_calculate_severity_majority() {
        let issues = vec![
            make_issue(Severity::Minor, "test", "desc", None),
            make_issue(Severity::Minor, "test", "desc", None),
            make_issue(Severity::Major, "test", "desc", None),
        ];
        assert_eq!(IssueAggregator::calculate_severity(&issues), Severity::Minor);
    }

    #[test]
    fn test_calculate_severity_tie_prefers_severe() {
        let issues = vec![
            make_issue(Severity::Minor, "test", "desc", None),
            make_issue(Severity::Major, "test", "desc", None),
        ];
        assert_eq!(IssueAggregator::calculate_severity(&issues), Severity::Major);
    }

    #[test]
    fn test_locations_similar_exact() {
        assert!(locations_similar("file.rs:42", "file.rs:42"));
    }

    #[test]
    fn test_locations_similar_close_lines() {
        assert!(locations_similar("file.rs:42", "file.rs:45")); // Within 5 lines
    }

    #[test]
    fn test_locations_not_similar_far_lines() {
        assert!(!locations_similar("file.rs:42", "file.rs:100")); // More than 5 lines apart
    }

    #[test]
    fn test_locations_similar_same_file() {
        assert!(locations_similar("file.rs", "file.rs"));
    }

    #[test]
    fn test_descriptions_similar_exact() {
        assert!(descriptions_similar("SQL injection risk", "SQL injection risk"));
    }

    #[test]
    fn test_descriptions_similar_case_insensitive() {
        assert!(descriptions_similar("SQL Injection Risk", "sql injection risk"));
    }

    #[test]
    fn test_descriptions_similar_subset() {
        assert!(descriptions_similar("SQL injection", "potential SQL injection vulnerability"));
    }

    #[test]
    fn test_descriptions_not_similar() {
        assert!(!descriptions_similar("SQL injection", "memory leak"));
    }

    #[test]
    fn test_calculate_average_confidence() {
        let reviews = vec![
            make_review("a", Verdict::Pass, vec![]),
            make_review("b", Verdict::Pass, vec![]),
        ];
        let avg = calculate_average_confidence(&reviews);
        assert!((avg - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_calculate_average_confidence_empty() {
        assert_eq!(calculate_average_confidence(&[]), 0.0);
    }

    #[test]
    fn test_determine_overall_verdict_all_pass() {
        let reviews = vec![
            make_review("a", Verdict::Pass, vec![]),
            make_review("b", Verdict::Pass, vec![]),
        ];
        assert_eq!(determine_overall_verdict(&reviews, 0.5), Verdict::Pass);
    }

    #[test]
    fn test_determine_overall_verdict_any_block() {
        let reviews = vec![
            make_review("a", Verdict::Pass, vec![]),
            make_review("b", Verdict::Block, vec![]),
        ];
        assert_eq!(determine_overall_verdict(&reviews, 0.5), Verdict::Block);
    }

    #[test]
    fn test_determine_overall_verdict_empty() {
        assert_eq!(determine_overall_verdict(&[], 0.5), Verdict::Block);
    }

    #[test]
    fn test_determine_overall_verdict_majority_issue() {
        let reviews = vec![
            make_review("a", Verdict::Pass, vec![]),
            make_review("b", Verdict::Issue, vec![]),
            make_review("c", Verdict::Issue, vec![]),
        ];
        assert_eq!(determine_overall_verdict(&reviews, 0.5), Verdict::Issue);
    }

    #[test]
    fn test_severity_rank_ordering() {
        assert!(severity_rank(Severity::Nit) < severity_rank(Severity::Minor));
        assert!(severity_rank(Severity::Minor) < severity_rank(Severity::Major));
        assert!(severity_rank(Severity::Major) < severity_rank(Severity::Critical));
    }

    #[test]
    fn test_aggregated_issue_from_single() {
        let issue = make_issue(Severity::Major, "security", "test", Some("file.rs:10"));
        let agg = AggregatedIssue::from_single(issue.clone(), "agent1".to_string());

        assert_eq!(agg.reporting_agents, vec!["agent1"]);
        assert_eq!(agg.agreed_severity, Severity::Major);
        assert_eq!(agg.confidence, 1.0);
        assert_eq!(agg.category, "security");
        assert_eq!(agg.location, Some("file.rs:10".to_string()));
    }

    #[test]
    fn test_aggregated_issue_merge() {
        let issue1 = make_issue(Severity::Minor, "security", "issue one", Some("file.rs:10"));
        let issue2 = make_issue(Severity::Major, "security", "issue two", Some("file.rs:12"));

        let mut agg = AggregatedIssue::from_single(issue1, "agent1".to_string());
        agg.merge(issue2, "agent2".to_string(), 3);

        assert_eq!(agg.reporting_agents.len(), 2);
        assert_eq!(agg.agreed_severity, Severity::Major); // More severe
        assert!((agg.confidence - 2.0/3.0).abs() < 0.001); // 2/3 agents
        assert!(agg.merged_description.contains("issue one"));
        assert!(agg.merged_description.contains("issue two"));
    }
}
