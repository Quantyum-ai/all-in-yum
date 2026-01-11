//! API request/response types for Claude (Anthropic Messages API)
//!
//! These types align with the Anthropic Messages API format.

use serde::{Deserialize, Serialize};

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: MessageRole,
    /// Message content (can be a string or array of content blocks)
    pub content: MessageContent,
}

/// Message role in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Message content (simplified - can be string or content blocks)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Content blocks (for structured content)
    Blocks(Vec<ContentBlock>),
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content block
    #[serde(rename = "text")]
    Text { text: String },
}

/// Messages API request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesRequest {
    /// Model to use
    pub model: String,
    /// Maximum tokens in response
    pub max_tokens: usize,
    /// Optional system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl MessagesRequest {
    /// Create a new messages request
    pub fn new(model: String, max_tokens: usize, messages: Vec<Message>) -> Self {
        Self {
            model,
            max_tokens,
            system: None,
            messages,
            temperature: None,
            stream: None,
        }
    }

    /// Set system prompt
    pub fn with_system(mut self, system: String) -> Self {
        self.system = Some(system);
        self
    }

    /// Set streaming flag
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Messages API response payload
#[derive(Debug, Clone, Deserialize)]
pub struct MessagesResponse {
    /// Response ID
    pub id: String,
    /// Object type (should be "message")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Role (should be "assistant")
    pub role: String,
    /// Response content
    pub content: Vec<ContentBlock>,
    /// Model used
    pub model: String,
    /// Stop reason
    pub stop_reason: Option<String>,
    /// Stop sequence (if applicable)
    pub stop_sequence: Option<String>,
    /// Token usage information
    pub usage: Usage,
}

/// Token usage information
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    /// Tokens in the input
    pub input_tokens: usize,
    /// Tokens in the output
    pub output_tokens: usize,
}

impl MessagesResponse {
    /// Extract text content from the response
    pub fn get_text(&self) -> Option<String> {
        self.content.iter().find_map(|block| {
            if let ContentBlock::Text { text } = block {
                Some(text.clone())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: MessageRole::User,
            content: MessageContent::Text("Hello".to_string()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_request_builder() {
        let req = MessagesRequest::new(
            "claude-sonnet-4-20250514".to_string(),
            1024,
            vec![Message {
                role: MessageRole::User,
                content: MessageContent::Text("Test".to_string()),
            }],
        )
        .with_system("You are a helpful assistant.".to_string())
        .with_stream(false)
        .with_temperature(0.7);

        assert_eq!(req.model, "claude-sonnet-4-20250514");
        assert_eq!(req.max_tokens, 1024);
        assert_eq!(req.stream, Some(false));
        assert_eq!(req.temperature, Some(0.7));
        assert!(req.system.is_some());
    }

    #[test]
    fn test_message_role_serde() {
        let roles = vec![
            (MessageRole::User, "\"user\""),
            (MessageRole::Assistant, "\"assistant\""),
        ];

        for (role, expected) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);

            let deserialized: MessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }
    }

    #[test]
    fn test_response_parsing() {
        let response_json = r#"{
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello! How can I help you today?"
                }
            ],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25
            }
        }"#;

        let response: MessagesResponse = serde_json::from_str(response_json).unwrap();
        assert_eq!(response.id, "msg_01XFDUDYJgAACzvnptvVoYEL");
        assert_eq!(response.object_type, "message");
        assert_eq!(response.role, "assistant");
        assert_eq!(
            response.get_text(),
            Some("Hello! How can I help you today?".to_string())
        );
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 25);
    }

    #[test]
    fn test_content_block_serde() {
        let block = ContentBlock::Text {
            text: "Hello world".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello world\""));
    }
}
