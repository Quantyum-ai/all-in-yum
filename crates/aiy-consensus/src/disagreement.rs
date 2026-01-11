//! Disagreement detection and analysis for consensus engine.
//!
//! This module identifies and classifies disagreements between agents,
//! helping to highlight areas where human review may be needed.

use aiy_adapters::{AgentReview, Issue, Severity, Verdict};
use std::collections::{HashMap, HashSet};

/// Analyzer for detecting disagreements between agent reviews.
pub struct DisagreementAnalyzer;

/// Represents a disagreement between agents on a specific topic.
#[derive(Debug, Clone)]
pub struct Disagreement {
    /// The topic or issue under disagreement.
    pub topic: String,
    /// The positions held by different agents.
    pub positions: Vec<AgentPosition>,
    /// The severity level of the disagreement.
    pub level: DisagreementLevel,
    /// Optional location in the code where the disagreement centers.
    pub location: Option<String>,
    /// A summary of why agents disagree.
    pub summary: String,
}

/// A single agent's position on an issue.
#[derive(Debug, Clone)]
pub struct AgentPosition {
    /// The ID of the agent holding this position.
    pub agent_id: String,
    /// The agent's verdict.
    pub verdict: Verdict,
    /// The severity they assigned (if applicable).
    pub severity: Option<Severity>,
    /// Their reasoning or stance.
    pub reasoning: String,
}

/// The severity level of a disagreement between agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisagreementLevel {
    /// Minor disagreement: Nit vs no issue, or minor stylistic differences.
    Minor,
    /// Moderate disagreement: Minor vs Major severity, or Pass vs Issue verdicts.
    Moderate,
    /// Severe disagreement: Pass vs Block, or Critical severity disputes.
    Severe,
}

impl DisagreementLevel {
    /// Returns a human-readable description of the disagreement level.
    pub fn description(&self) -> &'static str {
        match self {
            DisagreementLevel::Minor => "Minor disagreement on stylistic or trivial matters",
            DisagreementLevel::Moderate => "Moderate disagreement requiring attention",
            DisagreementLevel::Severe => "Severe disagreement requiring manual review",
        }
    }
}

impl DisagreementAnalyzer {
    /// Find all disagreements between agent reviews.
    ///
    /// This analyzes:
    /// - Verdict disagreements (Pass vs Issue vs Block)
    /// - Severity disagreements on the same issue
    /// - Issue presence disagreements (one agent reports, others don't)
    pub fn find_disagreements(reviews: &[AgentReview]) -> Vec<Disagreement> {
        if reviews.len() < 2 {
            return Vec::new();
        }

        let mut disagreements = Vec::new();

        // Check for verdict-level disagreements
        if let Some(d) = Self::check_verdict_disagreement(reviews) {
            disagreements.push(d);
        }

        // Check for issue-specific disagreements
        disagreements.extend(Self::check_issue_disagreements(reviews));

        // Check for confidence disagreements
        if let Some(d) = Self::check_confidence_disagreement(reviews) {
            disagreements.push(d);
        }

        // Sort by severity (most severe first)
        disagreements.sort_by(|a, b| b.level.cmp(&a.level));

        disagreements
    }

    /// Classify the severity of a disagreement.
    pub fn classify(disagreement: &Disagreement) -> DisagreementLevel {
        disagreement.level
    }

    /// Check for disagreement in overall verdicts.
    fn check_verdict_disagreement(reviews: &[AgentReview]) -> Option<Disagreement> {
        let verdicts: HashSet<Verdict> = reviews.iter().map(|r| r.verdict).collect();

        if verdicts.len() < 2 {
            return None; // All agree
        }

        let positions: Vec<AgentPosition> = reviews
            .iter()
            .map(|r| AgentPosition {
                agent_id: r.agent_id.clone(),
                verdict: r.verdict,
                severity: None,
                reasoning: r.reasoning.clone(),
            })
            .collect();

        let has_pass = verdicts.contains(&Verdict::Pass);
        let has_block = verdicts.contains(&Verdict::Block);

        let level = if has_pass && has_block {
            DisagreementLevel::Severe
        } else if has_pass || has_block {
            DisagreementLevel::Moderate
        } else {
            DisagreementLevel::Minor
        };

        let summary = Self::summarize_verdict_disagreement(reviews);

        Some(Disagreement {
            topic: "Overall verdict".to_string(),
            positions,
            level,
            location: None,
            summary,
        })
    }

    /// Generate a summary of verdict disagreement.
    fn summarize_verdict_disagreement(reviews: &[AgentReview]) -> String {
        let pass_agents: Vec<&str> = reviews
            .iter()
            .filter(|r| r.verdict == Verdict::Pass)
            .map(|r| r.agent_id.as_str())
            .collect();
        let issue_agents: Vec<&str> = reviews
            .iter()
            .filter(|r| r.verdict == Verdict::Issue)
            .map(|r| r.agent_id.as_str())
            .collect();
        let block_agents: Vec<&str> = reviews
            .iter()
            .filter(|r| r.verdict == Verdict::Block)
            .map(|r| r.agent_id.as_str())
            .collect();

        let mut parts = Vec::new();
        if !pass_agents.is_empty() {
            parts.push(format!("{} approve", pass_agents.join(", ")));
        }
        if !issue_agents.is_empty() {
            parts.push(format!("{} flag issues", issue_agents.join(", ")));
        }
        if !block_agents.is_empty() {
            parts.push(format!("{} recommend blocking", block_agents.join(", ")));
        }

        parts.join("; ")
    }

    /// Check for disagreements on specific issues.
    fn check_issue_disagreements(reviews: &[AgentReview]) -> Vec<Disagreement> {
        let mut disagreements = Vec::new();

        // Build a map of issue categories/locations to which agents report them
        let mut issue_map: HashMap<String, Vec<(&AgentReview, &Issue)>> = HashMap::new();

        for review in reviews {
            for issue in &review.issues {
                let key = Self::issue_key(issue);
                issue_map.entry(key).or_default().push((review, issue));
            }
        }

        // Find issues where agents disagree on severity
        for (key, reporters) in &issue_map {
            let severities: HashSet<Severity> = reporters.iter().map(|(_, i)| i.severity).collect();

            if severities.len() > 1 {
                let level = Self::severity_disagreement_level(&severities);

                let positions: Vec<AgentPosition> = reporters
                    .iter()
                    .map(|(r, i)| AgentPosition {
                        agent_id: r.agent_id.clone(),
                        verdict: r.verdict,
                        severity: Some(i.severity),
                        reasoning: i.description.clone(),
                    })
                    .collect();

                let location = reporters.first().and_then(|(_, i)| i.location.clone());

                disagreements.push(Disagreement {
                    topic: format!("Issue: {}", key),
                    positions,
                    level,
                    location,
                    summary: format!(
                        "Agents disagree on severity of this issue: {:?}",
                        severities.iter().collect::<Vec<_>>()
                    ),
                });
            }
        }

        // Find issues only reported by some agents
        let agent_ids: Vec<&str> = reviews.iter().map(|r| r.agent_id.as_str()).collect();
        let agent_count = agent_ids.len();

        for (key, reporters) in &issue_map {
            let reporter_ids: HashSet<&str> = reporters.iter().map(|(r, _)| r.agent_id.as_str()).collect();

            // If less than half the agents report this issue, it might be a disagreement
            if reporter_ids.len() < (agent_count + 1) / 2 && reporter_ids.len() < agent_count {
                let non_reporters: Vec<&str> = agent_ids
                    .iter()
                    .filter(|id| !reporter_ids.contains(*id))
                    .copied()
                    .collect();

                if !non_reporters.is_empty() {
                    let max_severity = reporters.iter().map(|(_, i)| i.severity).max();

                    let level = match max_severity {
                        Some(Severity::Critical) | Some(Severity::Major) => DisagreementLevel::Moderate,
                        _ => DisagreementLevel::Minor,
                    };

                    let mut positions: Vec<AgentPosition> = reporters
                        .iter()
                        .map(|(r, i)| AgentPosition {
                            agent_id: r.agent_id.clone(),
                            verdict: r.verdict,
                            severity: Some(i.severity),
                            reasoning: format!("Reports: {}", i.description),
                        })
                        .collect();

                    // Add non-reporters
                    for id in non_reporters {
                        if let Some(review) = reviews.iter().find(|r| r.agent_id == id) {
                            positions.push(AgentPosition {
                                agent_id: id.to_string(),
                                verdict: review.verdict,
                                severity: None,
                                reasoning: "Does not report this issue".to_string(),
                            });
                        }
                    }

                    let location = reporters.first().and_then(|(_, i)| i.location.clone());

                    disagreements.push(Disagreement {
                        topic: format!("Issue detection: {}", key),
                        positions,
                        level,
                        location,
                        summary: format!(
                            "Only {} of {} agents report this issue",
                            reporter_ids.len(),
                            agent_count
                        ),
                    });
                }
            }
        }

        disagreements
    }

    /// Check for significant confidence disagreements.
    fn check_confidence_disagreement(reviews: &[AgentReview]) -> Option<Disagreement> {
        if reviews.len() < 2 {
            return None;
        }

        let confidences: Vec<f64> = reviews.iter().map(|r| r.confidence).collect();
        let min_conf = confidences.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_conf = confidences.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Significant disagreement if confidence varies by more than 0.3
        if (max_conf - min_conf) > 0.3 {
            let positions: Vec<AgentPosition> = reviews
                .iter()
                .map(|r| AgentPosition {
                    agent_id: r.agent_id.clone(),
                    verdict: r.verdict,
                    severity: None,
                    reasoning: format!("Confidence: {:.0}%", r.confidence * 100.0),
                })
                .collect();

            return Some(Disagreement {
                topic: "Confidence levels".to_string(),
                positions,
                level: DisagreementLevel::Minor,
                location: None,
                summary: format!(
                    "Confidence varies from {:.0}% to {:.0}%",
                    min_conf * 100.0,
                    max_conf * 100.0
                ),
            });
        }

        None
    }

    /// Generate a key for an issue based on category and location.
    fn issue_key(issue: &Issue) -> String {
        match &issue.location {
            Some(loc) => format!("{}@{}", issue.category, loc),
            None => issue.category.clone(),
        }
    }

    /// Determine the disagreement level based on severity variance.
    fn severity_disagreement_level(severities: &HashSet<Severity>) -> DisagreementLevel {
        let has_critical = severities.contains(&Severity::Critical);
        let has_major = severities.contains(&Severity::Major);
        let has_minor = severities.contains(&Severity::Minor);
        let has_nit = severities.contains(&Severity::Nit);

        // Critical vs Nit/Minor is severe
        if has_critical && (has_nit || has_minor) {
            return DisagreementLevel::Severe;
        }

        // Major vs Nit is moderate
        if has_major && has_nit {
            return DisagreementLevel::Moderate;
        }

        // Critical vs Major, or Major vs Minor, etc. is minor
        DisagreementLevel::Minor
    }
}

/// Summary statistics about disagreements.
#[derive(Debug, Clone, Default)]
pub struct DisagreementSummary {
    /// Total number of disagreements.
    pub total: usize,
    /// Count of minor disagreements.
    pub minor_count: usize,
    /// Count of moderate disagreements.
    pub moderate_count: usize,
    /// Count of severe disagreements.
    pub severe_count: usize,
    /// Whether manual review is recommended.
    pub requires_manual_review: bool,
}

impl DisagreementSummary {
    /// Create a summary from a list of disagreements.
    pub fn from_disagreements(disagreements: &[Disagreement]) -> Self {
        let mut summary = Self {
            total: disagreements.len(),
            ..Default::default()
        };

        for d in disagreements {
            match d.level {
                DisagreementLevel::Minor => summary.minor_count += 1,
                DisagreementLevel::Moderate => summary.moderate_count += 1,
                DisagreementLevel::Severe => summary.severe_count += 1,
            }
        }

        // Recommend manual review if there are any severe disagreements
        // or more than 2 moderate ones
        summary.requires_manual_review =
            summary.severe_count > 0 || summary.moderate_count > 2;

        summary
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

    fn make_review(agent_id: &str, verdict: Verdict, confidence: f64, issues: Vec<Issue>) -> AgentReview {
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
    fn test_no_disagreements_single_review() {
        let review = make_review("grok", Verdict::Pass, 0.9, vec![]);
        let disagreements = DisagreementAnalyzer::find_disagreements(&[review]);
        assert!(disagreements.is_empty());
    }

    #[test]
    fn test_no_disagreements_all_agree() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![]),
            make_review("claude", Verdict::Pass, 0.85, vec![]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        // May have confidence disagreement but no verdict disagreement
        let verdict_disagreements: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Overall verdict")
            .collect();
        assert!(verdict_disagreements.is_empty());
    }

    #[test]
    fn test_verdict_disagreement_pass_vs_block() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![]),
            make_review("claude", Verdict::Block, 0.9, vec![]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let verdict_d: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Overall verdict")
            .collect();
        assert_eq!(verdict_d.len(), 1);
        assert_eq!(verdict_d[0].level, DisagreementLevel::Severe);
    }

    #[test]
    fn test_verdict_disagreement_pass_vs_issue() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![]),
            make_review("claude", Verdict::Issue, 0.9, vec![make_issue(Severity::Minor, "test", "issue", None)]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let verdict_d: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Overall verdict")
            .collect();
        assert_eq!(verdict_d.len(), 1);
        assert_eq!(verdict_d[0].level, DisagreementLevel::Moderate);
    }

    #[test]
    fn test_severity_disagreement_same_issue() {
        let reviews = vec![
            make_review("grok", Verdict::Issue, 0.9, vec![
                make_issue(Severity::Minor, "security", "SQL injection", Some("db.rs:42"))
            ]),
            make_review("claude", Verdict::Issue, 0.9, vec![
                make_issue(Severity::Critical, "security", "SQL injection", Some("db.rs:42"))
            ]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let issue_d: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic.starts_with("Issue:"))
            .collect();
        assert!(!issue_d.is_empty());
        assert_eq!(issue_d[0].level, DisagreementLevel::Severe);
    }

    #[test]
    fn test_issue_detection_disagreement() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![]),
            make_review("claude", Verdict::Issue, 0.9, vec![
                make_issue(Severity::Minor, "performance", "Slow loop", Some("engine.rs:100"))
            ]),
            make_review("gemini", Verdict::Pass, 0.85, vec![]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let detection_d: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic.starts_with("Issue detection:"))
            .collect();
        // Claude reports an issue that grok and gemini don't see
        assert!(!detection_d.is_empty());
    }

    #[test]
    fn test_confidence_disagreement() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![]),
            make_review("claude", Verdict::Pass, 0.5, vec![]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let conf_d: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Confidence levels")
            .collect();
        assert_eq!(conf_d.len(), 1);
        assert_eq!(conf_d[0].level, DisagreementLevel::Minor);
    }

    #[test]
    fn test_no_confidence_disagreement_similar() {
        let reviews = vec![
            make_review("grok", Verdict::Pass, 0.9, vec![]),
            make_review("claude", Verdict::Pass, 0.85, vec![]),
        ];
        let disagreements = DisagreementAnalyzer::find_disagreements(&reviews);
        let conf_d: Vec<_> = disagreements
            .iter()
            .filter(|d| d.topic == "Confidence levels")
            .collect();
        assert!(conf_d.is_empty());
    }

    #[test]
    fn test_disagreement_level_description() {
        assert!(!DisagreementLevel::Minor.description().is_empty());
        assert!(!DisagreementLevel::Moderate.description().is_empty());
        assert!(!DisagreementLevel::Severe.description().is_empty());
    }

    #[test]
    fn test_disagreement_level_ordering() {
        assert!(DisagreementLevel::Minor < DisagreementLevel::Moderate);
        assert!(DisagreementLevel::Moderate < DisagreementLevel::Severe);
    }

    #[test]
    fn test_classify_returns_level() {
        let disagreement = Disagreement {
            topic: "test".to_string(),
            positions: vec![],
            level: DisagreementLevel::Moderate,
            location: None,
            summary: "test".to_string(),
        };
        assert_eq!(DisagreementAnalyzer::classify(&disagreement), DisagreementLevel::Moderate);
    }

    #[test]
    fn test_disagreement_summary_empty() {
        let summary = DisagreementSummary::from_disagreements(&[]);
        assert_eq!(summary.total, 0);
        assert!(!summary.requires_manual_review);
    }

    #[test]
    fn test_disagreement_summary_with_severe() {
        let disagreements = vec![
            Disagreement {
                topic: "test".to_string(),
                positions: vec![],
                level: DisagreementLevel::Severe,
                location: None,
                summary: "test".to_string(),
            },
        ];
        let summary = DisagreementSummary::from_disagreements(&disagreements);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.severe_count, 1);
        assert!(summary.requires_manual_review);
    }

    #[test]
    fn test_disagreement_summary_multiple_moderate() {
        let disagreements = vec![
            Disagreement {
                topic: "test1".to_string(),
                positions: vec![],
                level: DisagreementLevel::Moderate,
                location: None,
                summary: "test".to_string(),
            },
            Disagreement {
                topic: "test2".to_string(),
                positions: vec![],
                level: DisagreementLevel::Moderate,
                location: None,
                summary: "test".to_string(),
            },
            Disagreement {
                topic: "test3".to_string(),
                positions: vec![],
                level: DisagreementLevel::Moderate,
                location: None,
                summary: "test".to_string(),
            },
        ];
        let summary = DisagreementSummary::from_disagreements(&disagreements);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.moderate_count, 3);
        assert!(summary.requires_manual_review);
    }

    #[test]
    fn test_disagreement_summary_minor_only() {
        let disagreements = vec![
            Disagreement {
                topic: "test".to_string(),
                positions: vec![],
                level: DisagreementLevel::Minor,
                location: None,
                summary: "test".to_string(),
            },
        ];
        let summary = DisagreementSummary::from_disagreements(&disagreements);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.minor_count, 1);
        assert!(!summary.requires_manual_review);
    }

    #[test]
    fn test_severity_disagreement_level_critical_vs_nit() {
        let mut severities = HashSet::new();
        severities.insert(Severity::Critical);
        severities.insert(Severity::Nit);
        let level = DisagreementAnalyzer::severity_disagreement_level(&severities);
        assert_eq!(level, DisagreementLevel::Severe);
    }

    #[test]
    fn test_severity_disagreement_level_major_vs_nit() {
        let mut severities = HashSet::new();
        severities.insert(Severity::Major);
        severities.insert(Severity::Nit);
        let level = DisagreementAnalyzer::severity_disagreement_level(&severities);
        assert_eq!(level, DisagreementLevel::Moderate);
    }

    #[test]
    fn test_severity_disagreement_level_minor_vs_nit() {
        let mut severities = HashSet::new();
        severities.insert(Severity::Minor);
        severities.insert(Severity::Nit);
        let level = DisagreementAnalyzer::severity_disagreement_level(&severities);
        assert_eq!(level, DisagreementLevel::Minor);
    }
}
