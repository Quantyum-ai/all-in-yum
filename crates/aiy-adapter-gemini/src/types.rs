//! API request/response types for Gemini
//!
//! These types align with the Google Generative AI API format.
//! Note: Gemini uses a different format than OpenAI-compatible APIs.

use serde::{Deserialize, Serialize};

/// A content part in the conversation (text, image, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    /// Text content
    pub text: String,
}

impl Part {
    /// Create a new text part
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            text: content.into(),
        }
    }
}

/// A content block in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Parts of this content (typically text)
    pub parts: Vec<Part>,
    /// Role of the content (optional for requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

impl Content {
    /// Create a new content block with text
    pub fn new_text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![Part::text(text)],
            role: None,
        }
    }

    /// Create a user content block
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            parts: vec![Part::text(text)],
            role: Some("user".to_string()),
        }
    }

    /// Create a model content block
    pub fn model(text: impl Into<String>) -> Self {
        Self {
            parts: vec![Part::text(text)],
            role: Some("model".to_string()),
        }
    }
}

/// Generation configuration for Gemini requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Temperature for sampling (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p (nucleus) sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Maximum output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: None,
            top_p: None,
            top_k: None,
            max_output_tokens: None,
            stop_sequences: None,
        }
    }
}

impl GenerationConfig {
    /// Create a new generation config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max output tokens
    pub fn with_max_output_tokens(mut self, max_tokens: i32) -> Self {
        self.max_output_tokens = Some(max_tokens);
        self
    }

    /// Set top-p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }
}

/// Request payload for generateContent endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    /// Conversation contents
    pub contents: Vec<Content>,
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

impl GeminiRequest {
    /// Create a new request with a single user prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            contents: vec![Content::user(prompt)],
            generation_config: None,
        }
    }

    /// Create a new request with contents
    pub fn with_contents(contents: Vec<Content>) -> Self {
        Self {
            contents,
            generation_config: None,
        }
    }

    /// Set generation config
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }
}

/// A candidate response from Gemini
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Content of the response
    pub content: Content,
    /// Finish reason
    #[serde(default)]
    pub finish_reason: Option<String>,
    /// Safety ratings
    #[serde(default)]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Safety rating for content
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyRating {
    /// Category of safety concern
    pub category: String,
    /// Probability level
    pub probability: String,
}

/// Token usage metadata
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Tokens in the prompt
    #[serde(default)]
    pub prompt_token_count: i32,
    /// Tokens in the response
    #[serde(default)]
    pub candidates_token_count: i32,
    /// Total tokens
    #[serde(default)]
    pub total_token_count: i32,
}

/// Response payload from generateContent endpoint
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    /// Response candidates (usually one)
    pub candidates: Vec<Candidate>,
    /// Token usage metadata
    #[serde(default)]
    pub usage_metadata: Option<UsageMetadata>,
}

impl GeminiResponse {
    /// Extract the text content from the first candidate
    pub fn text(&self) -> Option<String> {
        self.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_serialization() {
        let part = Part::text("Hello, world!");
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    #[test]
    fn test_content_user() {
        let content = Content::user("Test message");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Test message\""));
    }

    #[test]
    fn test_content_model() {
        let content = Content::model("Response text");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"role\":\"model\""));
    }

    #[test]
    fn test_request_builder() {
        let req = GeminiRequest::new("Test prompt")
            .with_generation_config(
                GenerationConfig::new()
                    .with_temperature(0.7)
                    .with_max_output_tokens(1000)
            );

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"maxOutputTokens\":1000"));
    }

    #[test]
    fn test_response_parsing() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello!"}],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }"#;

        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.text(), Some("Hello!".to_string()));
        assert_eq!(response.usage_metadata.unwrap().total_token_count, 15);
    }

    #[test]
    fn test_generation_config_builder() {
        let config = GenerationConfig::new()
            .with_temperature(0.5)
            .with_top_p(0.9)
            .with_max_output_tokens(2048);

        assert_eq!(config.temperature, Some(0.5));
        assert_eq!(config.top_p, Some(0.9));
        assert_eq!(config.max_output_tokens, Some(2048));
    }

    #[test]
    fn test_camel_case_serialization() {
        let config = GenerationConfig::new()
            .with_max_output_tokens(100);
        let json = serde_json::to_string(&config).unwrap();
        // Should be camelCase, not snake_case
        assert!(json.contains("maxOutputTokens"));
        assert!(!json.contains("max_output_tokens"));
    }
}
