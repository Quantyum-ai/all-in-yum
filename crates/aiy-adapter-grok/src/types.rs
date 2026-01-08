//! API request/response types for Grok
//!
//! These types align with the OpenAI-compatible chat completion format
//! used by xAI's Grok API.

use serde::{Deserialize, Serialize};

/// A chat message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role
    pub role: MessageRole,
    /// Message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Message role in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions)
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool result message
    Tool,
}

/// Chat completion request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model to use
    pub model: String,
    /// Conversation messages
    pub messages: Vec<ChatMessage>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Maximum tokens in response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

impl ChatRequest {
    /// Create a new chat request
    pub fn new(model: String, messages: Vec<ChatMessage>) -> Self {
        Self {
            model,
            messages,
            stream: None,
            max_tokens: None,
            temperature: None,
        }
    }

    /// Set streaming flag
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Chat completion response payload
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// Response ID
    pub id: String,
    /// Object type (should be "chat.completion")
    pub object: String,
    /// Creation timestamp
    pub created: u64,
    /// Model used
    pub model: String,
    /// Response choices
    pub choices: Vec<Choice>,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// A response choice
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    /// Choice index
    pub index: usize,
    /// The message
    pub message: ChatMessage,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: usize,
    /// Tokens in the completion
    pub completion_tokens: usize,
    /// Total tokens used
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = ChatMessage {
            role: MessageRole::User,
            content: Some("Hello".to_string()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_request_builder() {
        let req = ChatRequest::new(
            "grok-4-1-fast".to_string(),
            vec![ChatMessage {
                role: MessageRole::User,
                content: Some("Test".to_string()),
            }],
        )
        .with_stream(false)
        .with_max_tokens(100)
        .with_temperature(0.7);

        assert_eq!(req.model, "grok-4-1-fast");
        assert_eq!(req.stream, Some(false));
        assert_eq!(req.max_tokens, Some(100));
        assert_eq!(req.temperature, Some(0.7));
    }

    #[test]
    fn test_message_role_serde() {
        let roles = vec![
            (MessageRole::System, "\"system\""),
            (MessageRole::User, "\"user\""),
            (MessageRole::Assistant, "\"assistant\""),
            (MessageRole::Tool, "\"tool\""),
        ];

        for (role, expected) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);

            let deserialized: MessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }
    }
}
