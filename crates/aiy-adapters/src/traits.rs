use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    // Full trait will be implemented in Phase 2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReview {
    pub agent_id: String,
    pub verdict: Verdict,
    pub confidence: f64,
    pub issues: Vec<Issue>,
    pub suggestions: Vec<String>,
    pub sign_off: bool,
    pub reasoning: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Issue,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub category: String,
    pub description: String,
    pub location: Option<String>,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Nit,
}
