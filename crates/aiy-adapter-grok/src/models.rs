//! Grok model configurations
//!
//! Model data extracted from grok-cli TypeScript (config/models.ts)

use serde::{Deserialize, Serialize};

/// Grok model identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrokModel {
    /// Grok 3 Beta - 128K context
    Grok3Beta,
    /// Grok 3 Fast Beta - 128K context, optimized for speed
    Grok3FastBeta,
    /// Grok 3 Mini Beta - 128K context, lightweight
    Grok3MiniBeta,
    /// Grok 3 Mini Fast Beta - 128K context, fastest lightweight
    Grok3MiniFastBeta,
    /// Grok 4.1 Fast - 2M context, vision support (DEFAULT)
    Grok41Fast,
}

impl GrokModel {
    /// Get the API model string
    pub fn as_str(&self) -> &'static str {
        match self {
            GrokModel::Grok3Beta => "grok-3-beta",
            GrokModel::Grok3FastBeta => "grok-3-fast-beta",
            GrokModel::Grok3MiniBeta => "grok-3-mini-beta",
            GrokModel::Grok3MiniFastBeta => "grok-3-mini-fast-beta",
            GrokModel::Grok41Fast => "grok-4-1-fast",
        }
    }

    /// Get the default model (use Default trait instead)
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        GrokModel::Grok41Fast
    }

    /// Get the context window size in tokens
    pub fn context_window(&self) -> usize {
        match self {
            GrokModel::Grok3Beta => 128_000,
            GrokModel::Grok3FastBeta => 128_000,
            GrokModel::Grok3MiniBeta => 128_000,
            GrokModel::Grok3MiniFastBeta => 128_000,
            GrokModel::Grok41Fast => 2_000_000,
        }
    }

    /// Whether the model supports function calling
    pub fn supports_function_calling(&self) -> bool {
        true // All current Grok models support function calling
    }

    /// Whether the model supports streaming
    pub fn supports_streaming(&self) -> bool {
        true // All current Grok models support streaming
    }

    /// Whether the model supports vision
    pub fn supports_vision(&self) -> bool {
        matches!(self, GrokModel::Grok41Fast)
    }

    /// Get input price per 1M tokens (USD)
    pub fn input_price_per_million(&self) -> f64 {
        match self {
            GrokModel::Grok3Beta => 3.00,
            GrokModel::Grok3FastBeta => 0.60,
            GrokModel::Grok3MiniBeta => 0.30,
            GrokModel::Grok3MiniFastBeta => 0.10,
            GrokModel::Grok41Fast => 0.20,
        }
    }

    /// Get output price per 1M tokens (USD)
    pub fn output_price_per_million(&self) -> f64 {
        match self {
            GrokModel::Grok3Beta => 15.00,
            GrokModel::Grok3FastBeta => 3.00,
            GrokModel::Grok3MiniBeta => 0.50,
            GrokModel::Grok3MiniFastBeta => 0.40,
            GrokModel::Grok41Fast => 0.50,
        }
    }
}

impl Default for GrokModel {
    fn default() -> Self {
        Self::default()
    }
}

impl std::fmt::Display for GrokModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_strings() {
        assert_eq!(GrokModel::Grok3Beta.as_str(), "grok-3-beta");
        assert_eq!(GrokModel::Grok3FastBeta.as_str(), "grok-3-fast-beta");
        assert_eq!(GrokModel::Grok3MiniBeta.as_str(), "grok-3-mini-beta");
        assert_eq!(
            GrokModel::Grok3MiniFastBeta.as_str(),
            "grok-3-mini-fast-beta"
        );
        assert_eq!(GrokModel::Grok41Fast.as_str(), "grok-4-1-fast");
    }

    #[test]
    fn test_default_model() {
        assert_eq!(GrokModel::default(), GrokModel::Grok41Fast);
        assert_eq!(GrokModel::Grok41Fast.context_window(), 2_000_000);
        assert!(GrokModel::Grok41Fast.supports_vision());
    }

    #[test]
    fn test_context_windows() {
        assert_eq!(GrokModel::Grok3Beta.context_window(), 128_000);
        assert_eq!(GrokModel::Grok41Fast.context_window(), 2_000_000);
    }

    #[test]
    fn test_capabilities() {
        for model in [
            GrokModel::Grok3Beta,
            GrokModel::Grok3FastBeta,
            GrokModel::Grok3MiniBeta,
            GrokModel::Grok3MiniFastBeta,
            GrokModel::Grok41Fast,
        ] {
            assert!(model.supports_function_calling());
            assert!(model.supports_streaming());
        }

        assert!(!GrokModel::Grok3Beta.supports_vision());
        assert!(GrokModel::Grok41Fast.supports_vision());
    }

    #[test]
    fn test_pricing() {
        assert_eq!(GrokModel::Grok41Fast.input_price_per_million(), 0.20);
        assert_eq!(GrokModel::Grok41Fast.output_price_per_million(), 0.50);
    }
}
