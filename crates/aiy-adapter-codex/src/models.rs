//! OpenAI/Codex model configurations
//!
//! Available GPT models for the Codex adapter

use serde::{Deserialize, Serialize};

/// OpenAI model identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodexModel {
    /// GPT-4o - Latest flagship model (default)
    #[serde(rename = "gpt-4o")]
    Gpt4o,
    /// GPT-4o Mini - Smaller, faster, cheaper
    #[serde(rename = "gpt-4o-mini")]
    Gpt4oMini,
    /// GPT-4 Turbo - Previous generation flagship
    #[serde(rename = "gpt-4-turbo")]
    Gpt4Turbo,
    /// GPT-4 - Original GPT-4
    #[serde(rename = "gpt-4")]
    Gpt4,
    /// o1 - Reasoning model
    #[serde(rename = "o1")]
    O1,
    /// o1 Mini - Smaller reasoning model
    #[serde(rename = "o1-mini")]
    O1Mini,
}

impl CodexModel {
    /// Get the API model string
    pub fn as_str(&self) -> &'static str {
        match self {
            CodexModel::Gpt4o => "gpt-4o",
            CodexModel::Gpt4oMini => "gpt-4o-mini",
            CodexModel::Gpt4Turbo => "gpt-4-turbo",
            CodexModel::Gpt4 => "gpt-4",
            CodexModel::O1 => "o1",
            CodexModel::O1Mini => "o1-mini",
        }
    }

    /// Get the context window size in tokens
    pub fn context_window(&self) -> usize {
        match self {
            CodexModel::Gpt4o => 128_000,
            CodexModel::Gpt4oMini => 128_000,
            CodexModel::Gpt4Turbo => 128_000,
            CodexModel::Gpt4 => 8_192,
            CodexModel::O1 => 200_000,
            CodexModel::O1Mini => 128_000,
        }
    }

    /// Whether the model supports function calling
    pub fn supports_function_calling(&self) -> bool {
        match self {
            CodexModel::O1 | CodexModel::O1Mini => false,
            _ => true,
        }
    }

    /// Whether the model supports streaming
    pub fn supports_streaming(&self) -> bool {
        true
    }

    /// Whether the model supports vision
    pub fn supports_vision(&self) -> bool {
        matches!(
            self,
            CodexModel::Gpt4o | CodexModel::Gpt4oMini | CodexModel::Gpt4Turbo
        )
    }

    /// Get input price per 1M tokens (USD)
    pub fn input_price_per_million(&self) -> f64 {
        match self {
            CodexModel::Gpt4o => 2.50,
            CodexModel::Gpt4oMini => 0.15,
            CodexModel::Gpt4Turbo => 10.00,
            CodexModel::Gpt4 => 30.00,
            CodexModel::O1 => 15.00,
            CodexModel::O1Mini => 3.00,
        }
    }

    /// Get output price per 1M tokens (USD)
    pub fn output_price_per_million(&self) -> f64 {
        match self {
            CodexModel::Gpt4o => 10.00,
            CodexModel::Gpt4oMini => 0.60,
            CodexModel::Gpt4Turbo => 30.00,
            CodexModel::Gpt4 => 60.00,
            CodexModel::O1 => 60.00,
            CodexModel::O1Mini => 12.00,
        }
    }
}

impl Default for CodexModel {
    fn default() -> Self {
        CodexModel::Gpt4o
    }
}

impl std::fmt::Display for CodexModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_strings() {
        assert_eq!(CodexModel::Gpt4o.as_str(), "gpt-4o");
        assert_eq!(CodexModel::Gpt4oMini.as_str(), "gpt-4o-mini");
        assert_eq!(CodexModel::Gpt4Turbo.as_str(), "gpt-4-turbo");
        assert_eq!(CodexModel::Gpt4.as_str(), "gpt-4");
        assert_eq!(CodexModel::O1.as_str(), "o1");
        assert_eq!(CodexModel::O1Mini.as_str(), "o1-mini");
    }

    #[test]
    fn test_default_model() {
        assert_eq!(CodexModel::default(), CodexModel::Gpt4o);
        assert_eq!(CodexModel::Gpt4o.context_window(), 128_000);
        assert!(CodexModel::Gpt4o.supports_vision());
    }

    #[test]
    fn test_context_windows() {
        assert_eq!(CodexModel::Gpt4o.context_window(), 128_000);
        assert_eq!(CodexModel::Gpt4.context_window(), 8_192);
        assert_eq!(CodexModel::O1.context_window(), 200_000);
    }

    #[test]
    fn test_capabilities() {
        // All models support streaming
        for model in [
            CodexModel::Gpt4o,
            CodexModel::Gpt4oMini,
            CodexModel::Gpt4Turbo,
            CodexModel::Gpt4,
            CodexModel::O1,
            CodexModel::O1Mini,
        ] {
            assert!(model.supports_streaming());
        }

        // O1 models don't support function calling
        assert!(!CodexModel::O1.supports_function_calling());
        assert!(!CodexModel::O1Mini.supports_function_calling());
        assert!(CodexModel::Gpt4o.supports_function_calling());

        // Vision support
        assert!(CodexModel::Gpt4o.supports_vision());
        assert!(CodexModel::Gpt4oMini.supports_vision());
        assert!(!CodexModel::Gpt4.supports_vision());
    }

    #[test]
    fn test_pricing() {
        assert_eq!(CodexModel::Gpt4o.input_price_per_million(), 2.50);
        assert_eq!(CodexModel::Gpt4o.output_price_per_million(), 10.00);
        assert_eq!(CodexModel::Gpt4oMini.input_price_per_million(), 0.15);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", CodexModel::Gpt4o), "gpt-4o");
        assert_eq!(format!("{}", CodexModel::O1), "o1");
    }

    #[test]
    fn test_serde() {
        let model = CodexModel::Gpt4o;
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(json, "\"gpt-4o\"");

        let deserialized: CodexModel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CodexModel::Gpt4o);
    }
}
