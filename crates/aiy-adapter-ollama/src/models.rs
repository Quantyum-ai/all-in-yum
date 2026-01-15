//! Ollama model definitions and metadata

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported Ollama models for privacy mode.
///
/// **National Security Policy**: Privacy mode defaults to US-based models only.
/// Non-US models (DeepSeek, Qwen) have been removed from this enum to comply
/// with national security requirements for on-premises deployments.
///
/// If you need to use non-US models, use the `Custom` variant with explicit acknowledgment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OllamaModel {
    /// CodeLlama 7B Instruct - Default for privacy mode (US-based, Meta)
    #[serde(rename = "codellama:7b-instruct")]
    CodeLlama7bInstruct,

    /// CodeLlama 13B Instruct - Better quality, more memory (US-based, Meta)
    #[serde(rename = "codellama:13b-instruct")]
    CodeLlama13bInstruct,

    /// CodeLlama 34B Instruct - Best quality, requires significant memory (US-based, Meta)
    #[serde(rename = "codellama:34b-instruct")]
    CodeLlama34bInstruct,

    /// Llama 3.2 3B - Lightweight general model (US-based, Meta)
    #[serde(rename = "llama3.2:3b")]
    Llama3_2_3b,

    /// Custom model specified by string
    ///
    /// **Warning**: If using non-US models (DeepSeek, Qwen, etc.), ensure compliance
    /// with your organization's security and export control policies.
    #[serde(untagged)]
    Custom(String),
}

impl OllamaModel {
    /// Get the model name string as used in Ollama API
    pub fn as_str(&self) -> &str {
        match self {
            OllamaModel::CodeLlama7bInstruct => "codellama:7b-instruct",
            OllamaModel::CodeLlama13bInstruct => "codellama:13b-instruct",
            OllamaModel::CodeLlama34bInstruct => "codellama:34b-instruct",
            OllamaModel::Llama3_2_3b => "llama3.2:3b",
            OllamaModel::Custom(name) => name.as_str(),
        }
    }

    /// Get the default model for privacy mode
    pub fn default_privacy_model() -> Self {
        OllamaModel::CodeLlama7bInstruct
    }

    /// Get the context window size for this model
    pub fn context_window(&self) -> usize {
        match self {
            OllamaModel::CodeLlama7bInstruct => 8192,
            OllamaModel::CodeLlama13bInstruct => 8192,
            OllamaModel::CodeLlama34bInstruct => 16384,
            OllamaModel::Llama3_2_3b => 128000,
            OllamaModel::Custom(_) => 8192, // Conservative default
        }
    }

    /// Get approximate model size in GB
    pub fn size_gb(&self) -> f32 {
        match self {
            OllamaModel::CodeLlama7bInstruct => 3.8,
            OllamaModel::CodeLlama13bInstruct => 7.4,
            OllamaModel::CodeLlama34bInstruct => 19.0,
            OllamaModel::Llama3_2_3b => 2.0,
            OllamaModel::Custom(_) => 0.0, // Unknown
        }
    }

    /// Check if this model is optimized for code generation
    pub fn is_code_optimized(&self) -> bool {
        match self {
            OllamaModel::CodeLlama7bInstruct => true,
            OllamaModel::CodeLlama13bInstruct => true,
            OllamaModel::CodeLlama34bInstruct => true,
            OllamaModel::Llama3_2_3b => false,
            OllamaModel::Custom(_) => false, // Unknown
        }
    }

    /// Parse a model name string into an OllamaModel
    pub fn parse_model(name: &str) -> Self {
        match name {
            "codellama:7b-instruct" => OllamaModel::CodeLlama7bInstruct,
            "codellama:13b-instruct" => OllamaModel::CodeLlama13bInstruct,
            "codellama:34b-instruct" => OllamaModel::CodeLlama34bInstruct,
            "llama3.2:3b" => OllamaModel::Llama3_2_3b,
            // Non-US models (DeepSeek, Qwen) removed for national security compliance
            // Users can still access via Custom variant if needed
            other => OllamaModel::Custom(other.to_string()),
        }
    }
}

impl Default for OllamaModel {
    fn default() -> Self {
        Self::default_privacy_model()
    }
}

impl fmt::Display for OllamaModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        let model = OllamaModel::default();
        assert_eq!(model, OllamaModel::CodeLlama7bInstruct);
        assert_eq!(model.as_str(), "codellama:7b-instruct");
    }

    #[test]
    fn test_model_context_windows() {
        assert_eq!(OllamaModel::CodeLlama7bInstruct.context_window(), 8192);
        assert_eq!(OllamaModel::CodeLlama34bInstruct.context_window(), 16384);
        assert_eq!(OllamaModel::Llama3_2_3b.context_window(), 128000);
    }

    #[test]
    fn test_code_optimized_models() {
        assert!(OllamaModel::CodeLlama7bInstruct.is_code_optimized());
        assert!(OllamaModel::CodeLlama13bInstruct.is_code_optimized());
        assert!(!OllamaModel::Llama3_2_3b.is_code_optimized());
    }

    #[test]
    fn test_parse_model_known() {
        let model = OllamaModel::parse_model("codellama:7b-instruct");
        assert_eq!(model, OllamaModel::CodeLlama7bInstruct);
    }

    #[test]
    fn test_parse_model_custom() {
        let model = OllamaModel::parse_model("my-custom-model:latest");
        match model {
            OllamaModel::Custom(name) => assert_eq!(name, "my-custom-model:latest"),
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(
            format!("{}", OllamaModel::CodeLlama7bInstruct),
            "codellama:7b-instruct"
        );
    }
}
