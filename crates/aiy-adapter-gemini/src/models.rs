//! Gemini model configurations
//!
//! Model data for Google's Generative AI API

use serde::{Deserialize, Serialize};

/// Gemini model identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeminiModel {
    /// Gemini 1.5 Pro - Latest version, best for complex tasks
    Gemini15Pro,
    /// Gemini 1.5 Flash - Faster, optimized for speed
    Gemini15Flash,
    /// Gemini 2.0 Flash Experimental - Cutting-edge experimental model
    Gemini20FlashExp,
    /// Gemini 1.5 Flash 8B - Lightweight, efficient model
    Gemini15Flash8B,
}

impl GeminiModel {
    /// Get the API model string
    pub fn as_str(&self) -> &'static str {
        match self {
            GeminiModel::Gemini15Pro => "gemini-1.5-pro-latest",
            GeminiModel::Gemini15Flash => "gemini-1.5-flash-latest",
            GeminiModel::Gemini20FlashExp => "gemini-2.0-flash-exp",
            GeminiModel::Gemini15Flash8B => "gemini-1.5-flash-8b-latest",
        }
    }

    /// Get the context window size in tokens
    pub fn context_window(&self) -> usize {
        match self {
            GeminiModel::Gemini15Pro => 2_097_152, // 2M tokens
            GeminiModel::Gemini15Flash => 1_048_576, // 1M tokens
            GeminiModel::Gemini20FlashExp => 1_048_576, // 1M tokens
            GeminiModel::Gemini15Flash8B => 1_048_576, // 1M tokens
        }
    }

    /// Whether the model supports function calling
    pub fn supports_function_calling(&self) -> bool {
        true // All current Gemini models support function calling
    }

    /// Whether the model supports streaming
    pub fn supports_streaming(&self) -> bool {
        true // All current Gemini models support streaming
    }

    /// Whether the model supports vision/multimodal
    pub fn supports_vision(&self) -> bool {
        true // All Gemini 1.5+ models support vision
    }

    /// Get approximate input price per 1M tokens (USD)
    /// Note: Google pricing varies by prompt length; these are estimates
    pub fn input_price_per_million(&self) -> f64 {
        match self {
            GeminiModel::Gemini15Pro => 3.50,
            GeminiModel::Gemini15Flash => 0.075,
            GeminiModel::Gemini20FlashExp => 0.10,
            GeminiModel::Gemini15Flash8B => 0.0375,
        }
    }

    /// Get approximate output price per 1M tokens (USD)
    pub fn output_price_per_million(&self) -> f64 {
        match self {
            GeminiModel::Gemini15Pro => 10.50,
            GeminiModel::Gemini15Flash => 0.30,
            GeminiModel::Gemini20FlashExp => 0.40,
            GeminiModel::Gemini15Flash8B => 0.15,
        }
    }
}

impl Default for GeminiModel {
    fn default() -> Self {
        GeminiModel::Gemini15Pro
    }
}

impl std::fmt::Display for GeminiModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_strings() {
        assert_eq!(GeminiModel::Gemini15Pro.as_str(), "gemini-1.5-pro-latest");
        assert_eq!(GeminiModel::Gemini15Flash.as_str(), "gemini-1.5-flash-latest");
        assert_eq!(GeminiModel::Gemini20FlashExp.as_str(), "gemini-2.0-flash-exp");
        assert_eq!(GeminiModel::Gemini15Flash8B.as_str(), "gemini-1.5-flash-8b-latest");
    }

    #[test]
    fn test_default_model() {
        assert_eq!(GeminiModel::default(), GeminiModel::Gemini15Pro);
        assert_eq!(GeminiModel::Gemini15Pro.context_window(), 2_097_152);
        assert!(GeminiModel::Gemini15Pro.supports_vision());
    }

    #[test]
    fn test_context_windows() {
        assert_eq!(GeminiModel::Gemini15Pro.context_window(), 2_097_152);
        assert_eq!(GeminiModel::Gemini15Flash.context_window(), 1_048_576);
    }

    #[test]
    fn test_capabilities() {
        for model in [
            GeminiModel::Gemini15Pro,
            GeminiModel::Gemini15Flash,
            GeminiModel::Gemini20FlashExp,
            GeminiModel::Gemini15Flash8B,
        ] {
            assert!(model.supports_function_calling());
            assert!(model.supports_streaming());
            assert!(model.supports_vision());
        }
    }

    #[test]
    fn test_pricing() {
        assert_eq!(GeminiModel::Gemini15Pro.input_price_per_million(), 3.50);
        assert_eq!(GeminiModel::Gemini15Pro.output_price_per_million(), 10.50);
        assert_eq!(GeminiModel::Gemini15Flash.input_price_per_million(), 0.075);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", GeminiModel::Gemini15Pro), "gemini-1.5-pro-latest");
        assert_eq!(format!("{}", GeminiModel::Gemini20FlashExp), "gemini-2.0-flash-exp");
    }
}
