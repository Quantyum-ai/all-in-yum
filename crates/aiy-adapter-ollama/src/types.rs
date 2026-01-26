//! Ollama API request and response types
//!
//! Based on Ollama's REST API: https://github.com/ollama/ollama/blob/main/docs/api.md

use serde::{Deserialize, Serialize};

/// Request body for /api/chat endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model name (e.g., "codellama:7b-instruct")
    pub model: String,

    /// Message history
    pub messages: Vec<Message>,

    /// Whether to stream the response (we always use false for simplicity)
    #[serde(default)]
    pub stream: bool,

    /// Optional generation options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerationOptions>,
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "system", "user", or "assistant"
    pub role: String,

    /// Message content
    pub content: String,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Generation options for controlling model behavior
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationOptions {
    /// Temperature (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Number of tokens to keep from context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<usize>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,

    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top-k sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

/// Response from /api/chat endpoint (non-streaming)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Model used for generation
    pub model: String,

    /// Generated message
    pub message: Message,

    /// Whether generation is complete
    pub done: bool,

    /// Total duration in nanoseconds
    #[serde(default)]
    pub total_duration: u64,

    /// Load duration in nanoseconds
    #[serde(default)]
    pub load_duration: u64,

    /// Prompt evaluation count
    #[serde(default)]
    pub prompt_eval_count: u32,

    /// Prompt evaluation duration in nanoseconds
    #[serde(default)]
    pub prompt_eval_duration: u64,

    /// Generation count
    #[serde(default)]
    pub eval_count: u32,

    /// Generation duration in nanoseconds
    #[serde(default)]
    pub eval_duration: u64,
}

impl ChatResponse {
    /// Get tokens per second for prompt evaluation
    pub fn prompt_tokens_per_second(&self) -> f64 {
        if self.prompt_eval_duration > 0 {
            self.prompt_eval_count as f64 / (self.prompt_eval_duration as f64 / 1_000_000_000.0)
        } else {
            0.0
        }
    }

    /// Get tokens per second for generation
    pub fn generation_tokens_per_second(&self) -> f64 {
        if self.eval_duration > 0 {
            self.eval_count as f64 / (self.eval_duration as f64 / 1_000_000_000.0)
        } else {
            0.0
        }
    }
}

/// Response from /api/tags endpoint (list models)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsResponse {
    /// List of available models
    pub models: Vec<ModelInfo>,
}

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name
    pub name: String,

    /// Model digest
    #[serde(default)]
    pub digest: String,

    /// Size in bytes
    #[serde(default)]
    pub size: u64,

    /// Modified timestamp
    #[serde(default)]
    pub modified_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let sys = Message::system("You are a helpful assistant");
        assert_eq!(sys.role, "system");
        assert_eq!(sys.content, "You are a helpful assistant");

        let user = Message::user("Hello");
        assert_eq!(user.role, "user");

        let assistant = Message::assistant("Hi there!");
        assert_eq!(assistant.role, "assistant");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "codellama:7b-instruct".to_string(),
            messages: vec![
                Message::system("You are a coding assistant"),
                Message::user("Write a hello world in Rust"),
            ],
            stream: false,
            options: Some(GenerationOptions {
                temperature: Some(0.1),
                num_ctx: Some(8192),
                ..Default::default()
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("codellama:7b-instruct"));
        assert!(json.contains("system"));
        assert!(json.contains("user"));
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "model": "codellama:7b-instruct",
            "message": {
                "role": "assistant",
                "content": "fn main() { println!(\"Hello, world!\"); }"
            },
            "done": true,
            "total_duration": 1000000000,
            "eval_count": 50,
            "eval_duration": 500000000
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "codellama:7b-instruct");
        assert!(response.done);
        assert_eq!(response.generation_tokens_per_second(), 100.0);
    }

    #[test]
    fn test_tags_response_deserialization() {
        let json = r#"{
            "models": [
                {
                    "name": "codellama:7b-instruct",
                    "digest": "abc123",
                    "size": 4000000000,
                    "modified_at": "2024-01-01T00:00:00Z"
                }
            ]
        }"#;

        let response: TagsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].name, "codellama:7b-instruct");
    }
}
