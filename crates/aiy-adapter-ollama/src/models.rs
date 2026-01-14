//! Ollama model definitions and metadata

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported Ollama models for privacy mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OllamaModel {
    /// CodeLlama 7B Instruct - Default for privacy mode
    #[serde(rename = "codellama:7b-instruct")]
    CodeLlama7bInstruct,

    /// CodeLlama 13B Instruct - Better quality, more memory
    #[serde(rename = "codellama:13b-instruct")]
    CodeLlama13bInstruct,

    /// CodeLlama 34B Instruct - Best quality, requires significant memory
    #[serde(rename = "codellama:34b-instruct")]
    CodeLlama34bInstruct,

    /// DeepSeek Coder 6.7B Instruct
    #[serde(rename = "deepseek-coder:6.7b-instruct")]
    DeepSeekCoder6_7bInstruct,

    /// DeepSeek Coder 33B Instruct
    #[serde(rename = "deepseek-coder:33b-instruct")]
    DeepSeekCoder33bInstruct,

    /// Llama 3.2 3B - Lightweight general model
    #[serde(rename = "llama3.2:3b")]
    Llama3_2_3b,

    /// Qwen 2.5 Coder 7B Instruct
    #[serde(rename = "qwen2.5-coder:7b-instruct")]
    Qwen2_5Coder7bInstruct,

    /// Custom model specified by string
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
            OllamaModel::DeepSeekCoder6_7bInstruct => "deepseek-coder:6.7b-instruct",
            OllamaModel::DeepSeekCoder33bInstruct => "deepseek-coder:33b-instruct",
            OllamaModel::Llama3_2_3b => "llama3.2:3b",
            OllamaModel::Qwen2_5Coder7bInstruct => "qwen2.5-coder:7b-instruct",
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
            OllamaModel::DeepSeekCoder6_7bInstruct => 16384,
            OllamaModel::DeepSeekCoder33bInstruct => 16384,
            OllamaModel::Llama3_2_3b => 128000, // Llama 3.2 has large context
            OllamaModel::Qwen2_5Coder7bInstruct => 32768,
            OllamaModel::Custom(_) => 8192, // Conservative default
        }
    }

    /// Get approximate model size in GB
    pub fn size_gb(&self) -> f32 {
        match self {
            OllamaModel::CodeLlama7bInstruct => 3.8,
            OllamaModel::CodeLlama13bInstruct => 7.4,
            OllamaModel::CodeLlama34bInstruct => 19.0,
            OllamaModel::DeepSeekCoder6_7bInstruct => 3.8,
            OllamaModel::DeepSeekCoder33bInstruct => 18.5,
            OllamaModel::Llama3_2_3b => 2.0,
            OllamaModel::Qwen2_5Coder7bInstruct => 4.7,
            OllamaModel::Custom(_) => 0.0, // Unknown
        }
    }

    /// Check if this model is optimized for code generation
    pub fn is_code_optimized(&self) -> bool {
        match self {
            OllamaModel::CodeLlama7bInstruct => true,
            OllamaModel::CodeLlama13bInstruct => true,
            OllamaModel::CodeLlama34bInstruct => true,
            OllamaModel::DeepSeekCoder6_7bInstruct => true,
            OllamaModel::DeepSeekCoder33bInstruct => true,
            OllamaModel::Llama3_2_3b => false,
            OllamaModel::Qwen2_5Coder7bInstruct => true,
            OllamaModel::Custom(_) => false, // Unknown
        }
    }

    /// Parse a model name string into an OllamaModel
    pub fn from_str(name: &str) -> Self {
        match name {
            "codellama:7b-instruct" => OllamaModel::CodeLlama7bInstruct,
            "codellama:13b-instruct" => OllamaModel::CodeLlama13bInstruct,
            "codellama:34b-instruct" => OllamaModel::CodeLlama34bInstruct,
            "deepseek-coder:6.7b-instruct" => OllamaModel::DeepSeekCoder6_7bInstruct,
            "deepseek-coder:33b-instruct" => OllamaModel::DeepSeekCoder33bInstruct,
            "llama3.2:3b" => OllamaModel::Llama3_2_3b,
            "qwen2.5-coder:7b-instruct" => OllamaModel::Qwen2_5Coder7bInstruct,
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
        assert!(OllamaModel::DeepSeekCoder6_7bInstruct.is_code_optimized());
        assert!(!OllamaModel::Llama3_2_3b.is_code_optimized());
    }

    #[test]
    fn test_from_str_known_model() {
        let model = OllamaModel::from_str("codellama:7b-instruct");
        assert_eq!(model, OllamaModel::CodeLlama7bInstruct);
    }

    #[test]
    fn test_from_str_custom_model() {
        let model = OllamaModel::from_str("my-custom-model:latest");
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
