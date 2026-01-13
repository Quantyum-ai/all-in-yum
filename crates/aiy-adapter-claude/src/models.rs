//! Claude model configurations
//!
//! Model data based on Anthropic's Claude model offerings.

use serde::{Deserialize, Serialize};

/// Claude model identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaudeModel {
    /// Claude 4 Opus - Most capable model
    Claude4Opus,
    /// Claude Sonnet 4 - Balanced performance (DEFAULT)
    ClaudeSonnet4,
    /// Claude 3.5 Sonnet - Previous generation balanced model
    Claude35Sonnet,
    /// Claude 3.5 Haiku - Fast and efficient
    Claude35Haiku,
    /// Claude 3 Opus - Previous flagship
    Claude3Opus,
}

impl ClaudeModel {
    /// Get the API model string
    pub fn as_str(&self) -> &'static str {
        match self {
            ClaudeModel::Claude4Opus => "claude-opus-4-20250514",
            ClaudeModel::ClaudeSonnet4 => "claude-sonnet-4-20250514",
            ClaudeModel::Claude35Sonnet => "claude-3-5-sonnet-20241022",
            ClaudeModel::Claude35Haiku => "claude-3-5-haiku-20241022",
            ClaudeModel::Claude3Opus => "claude-3-opus-20240229",
        }
    }

    /// Get the default model (claude-sonnet-4-20250514 as specified)
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        ClaudeModel::ClaudeSonnet4
    }

    /// Get the context window size in tokens
    pub fn context_window(&self) -> usize {
        match self {
            ClaudeModel::Claude4Opus => 200_000,
            ClaudeModel::ClaudeSonnet4 => 200_000,
            ClaudeModel::Claude35Sonnet => 200_000,
            ClaudeModel::Claude35Haiku => 200_000,
            ClaudeModel::Claude3Opus => 200_000,
        }
    }

    /// Get maximum output tokens
    pub fn max_output_tokens(&self) -> usize {
        match self {
            ClaudeModel::Claude4Opus => 32_768,
            ClaudeModel::ClaudeSonnet4 => 16_384,
            ClaudeModel::Claude35Sonnet => 8_192,
            ClaudeModel::Claude35Haiku => 8_192,
            ClaudeModel::Claude3Opus => 4_096,
        }
    }

    /// Whether the model supports vision
    pub fn supports_vision(&self) -> bool {
        true // All Claude 3+ models support vision
    }

    /// Whether the model supports tool use
    pub fn supports_tool_use(&self) -> bool {
        true // All Claude 3+ models support tool use
    }

    /// Get input price per 1M tokens (USD)
    pub fn input_price_per_million(&self) -> f64 {
        match self {
            ClaudeModel::Claude4Opus => 15.00,
            ClaudeModel::ClaudeSonnet4 => 3.00,
            ClaudeModel::Claude35Sonnet => 3.00,
            ClaudeModel::Claude35Haiku => 0.80,
            ClaudeModel::Claude3Opus => 15.00,
        }
    }

    /// Get output price per 1M tokens (USD)
    pub fn output_price_per_million(&self) -> f64 {
        match self {
            ClaudeModel::Claude4Opus => 75.00,
            ClaudeModel::ClaudeSonnet4 => 15.00,
            ClaudeModel::Claude35Sonnet => 15.00,
            ClaudeModel::Claude35Haiku => 4.00,
            ClaudeModel::Claude3Opus => 75.00,
        }
    }
}

impl Default for ClaudeModel {
    fn default() -> Self {
        Self::default()
    }
}

impl std::fmt::Display for ClaudeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_strings() {
        assert_eq!(ClaudeModel::Claude4Opus.as_str(), "claude-opus-4-20250514");
        assert_eq!(
            ClaudeModel::ClaudeSonnet4.as_str(),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(
            ClaudeModel::Claude35Sonnet.as_str(),
            "claude-3-5-sonnet-20241022"
        );
        assert_eq!(
            ClaudeModel::Claude35Haiku.as_str(),
            "claude-3-5-haiku-20241022"
        );
        assert_eq!(ClaudeModel::Claude3Opus.as_str(), "claude-3-opus-20240229");
    }

    #[test]
    fn test_default_model() {
        assert_eq!(ClaudeModel::default(), ClaudeModel::ClaudeSonnet4);
        assert_eq!(ClaudeModel::ClaudeSonnet4.context_window(), 200_000);
    }

    #[test]
    fn test_context_windows() {
        assert_eq!(ClaudeModel::Claude4Opus.context_window(), 200_000);
        assert_eq!(ClaudeModel::ClaudeSonnet4.context_window(), 200_000);
    }

    #[test]
    fn test_max_output_tokens() {
        assert_eq!(ClaudeModel::Claude4Opus.max_output_tokens(), 32_768);
        assert_eq!(ClaudeModel::ClaudeSonnet4.max_output_tokens(), 16_384);
        assert_eq!(ClaudeModel::Claude35Haiku.max_output_tokens(), 8_192);
    }

    #[test]
    fn test_capabilities() {
        for model in [
            ClaudeModel::Claude4Opus,
            ClaudeModel::ClaudeSonnet4,
            ClaudeModel::Claude35Sonnet,
            ClaudeModel::Claude35Haiku,
            ClaudeModel::Claude3Opus,
        ] {
            assert!(model.supports_vision());
            assert!(model.supports_tool_use());
        }
    }

    #[test]
    fn test_pricing() {
        assert_eq!(ClaudeModel::ClaudeSonnet4.input_price_per_million(), 3.00);
        assert_eq!(ClaudeModel::ClaudeSonnet4.output_price_per_million(), 15.00);
        assert_eq!(ClaudeModel::Claude35Haiku.input_price_per_million(), 0.80);
    }
}
