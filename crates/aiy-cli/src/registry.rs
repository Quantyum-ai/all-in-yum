//! Centralized Agent Registry
//!
//! This module provides a single source of truth for all agent metadata.
//! Adding or removing an agent requires changing only the AGENTS constant.

/// Agent metadata information
#[derive(Debug, Clone, Copy)]
pub struct AgentInfo {
    /// Unique agent identifier (e.g., "grok")
    pub id: &'static str,
    /// Human-readable display name (e.g., "Grok (xAI)")
    pub display_name: &'static str,
    /// Credential provider identifier (e.g., "xai")
    pub credential_provider: &'static str,
    /// Whether this agent is enabled by default
    pub enabled_by_default: bool,
}

/// Static registry of all available agents.
/// This is the single source of truth for agent definitions.
pub const AGENTS: &[AgentInfo] = &[
    AgentInfo {
        id: "grok",
        display_name: "Grok (xAI)",
        credential_provider: "xai",
        enabled_by_default: false,
    },
    AgentInfo {
        id: "claude",
        display_name: "Claude (Anthropic)",
        credential_provider: "anthropic",
        enabled_by_default: false,
    },
    AgentInfo {
        id: "gemini",
        display_name: "Gemini (Google)",
        credential_provider: "google",
        enabled_by_default: false,
    },
    AgentInfo {
        id: "codex",
        display_name: "Codex (OpenAI)",
        credential_provider: "openai",
        enabled_by_default: false,
    },
];

/// Get agent info by ID
///
/// # Example
/// ```
/// use aiy_cli::registry::get_agent;
/// let agent = get_agent("grok");
/// assert!(agent.is_some());
/// assert_eq!(agent.unwrap().display_name, "Grok (xAI)");
/// ```
pub fn get_agent(id: &str) -> Option<&'static AgentInfo> {
    AGENTS.iter().find(|a| a.id == id)
}

/// Check if an agent ID is valid
///
/// # Example
/// ```
/// use aiy_cli::registry::is_valid_agent;
/// assert!(is_valid_agent("grok"));
/// assert!(!is_valid_agent("unknown"));
/// ```
pub fn is_valid_agent(id: &str) -> bool {
    AGENTS.iter().any(|a| a.id == id)
}

/// Get an iterator over all agent IDs
///
/// # Example
/// ```
/// use aiy_cli::registry::all_agent_ids;
/// let ids: Vec<_> = all_agent_ids().collect();
/// assert!(ids.contains(&"grok"));
/// ```
pub fn all_agent_ids() -> impl Iterator<Item = &'static str> {
    AGENTS.iter().map(|a| a.id)
}

/// Get a comma-separated string of all valid agent IDs.
///
/// Useful for error messages and help text that need to display
/// all available agents.
///
/// # Returns
///
/// A comma-separated string like "grok, claude, gemini, codex".
pub fn valid_agents_string() -> String {
    all_agent_ids().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agents_count() {
        assert_eq!(AGENTS.len(), 4);
    }

    #[test]
    fn test_get_agent() {
        let grok = get_agent("grok");
        assert!(grok.is_some());
        let grok = grok.unwrap();
        assert_eq!(grok.id, "grok");
        assert_eq!(grok.display_name, "Grok (xAI)");
        assert_eq!(grok.credential_provider, "xai");
        assert!(!grok.enabled_by_default);
    }

    #[test]
    fn test_get_agent_not_found() {
        assert!(get_agent("unknown").is_none());
    }

    #[test]
    fn test_is_valid_agent() {
        assert!(is_valid_agent("grok"));
        assert!(is_valid_agent("claude"));
        assert!(is_valid_agent("gemini"));
        assert!(is_valid_agent("codex"));
        assert!(!is_valid_agent("unknown"));
        assert!(!is_valid_agent(""));
    }

    #[test]
    fn test_all_agent_ids() {
        let ids: Vec<_> = all_agent_ids().collect();
        assert_eq!(ids.len(), 4);
        assert!(ids.contains(&"grok"));
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"codex"));
    }

    #[test]
    fn test_valid_agents_string() {
        let s = valid_agents_string();
        assert!(s.contains("grok"));
        assert!(s.contains("claude"));
        assert!(s.contains("gemini"));
        assert!(s.contains("codex"));
    }

    #[test]
    fn test_all_agents_have_required_fields() {
        for agent in AGENTS {
            assert!(!agent.id.is_empty(), "Agent ID should not be empty");
            assert!(
                !agent.display_name.is_empty(),
                "Display name should not be empty"
            );
            assert!(
                !agent.credential_provider.is_empty(),
                "Credential provider should not be empty"
            );
        }
    }

    #[test]
    fn test_unique_agent_ids() {
        let ids: Vec<_> = all_agent_ids().collect();
        let unique_count = {
            let mut sorted = ids.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(ids.len(), unique_count, "Agent IDs should be unique");
    }
}
