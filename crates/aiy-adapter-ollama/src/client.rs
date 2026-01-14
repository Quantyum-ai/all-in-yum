//! Ollama API client implementation

use crate::error::OllamaError;
use crate::models::OllamaModel;
use crate::transport::HttpTransport;
use crate::types::{ChatRequest, ChatResponse, GenerationOptions, Message, TagsResponse};
use aiy_adapters::AgentReview;
use aiy_core::security::sanitization::sanitize_artifact_content;
use std::sync::Arc;

/// System prompt for code review tasks in privacy mode
const CODE_REVIEW_SYSTEM_PROMPT: &str = r#"You are a code review assistant operating in privacy mode. Your task is to review code changes and provide structured feedback.

IMPORTANT: You are running locally to protect proprietary code. Be thorough but concise due to context limitations.

For each review, respond with a JSON object following this exact schema:
{
    "agent_id": "ollama",
    "verdict": "pass" | "issue" | "block",
    "confidence": 0.0 to 1.0,
    "issues": [
        {
            "severity": "critical" | "major" | "minor" | "nit",
            "category": "string (e.g., security, performance, style)",
            "description": "what is wrong",
            "location": "file:line or null",
            "suggested_fix": "how to fix or null"
        }
    ],
    "suggestions": ["general improvement suggestions"],
    "sign_off": true | false,
    "reasoning": "explanation of your verdict"
}

Verdicts:
- "pass": Code is ready to merge
- "issue": Has problems that should be addressed but may not block
- "block": Has critical problems that must be fixed

Focus on: security vulnerabilities, bugs, performance issues, and code quality."#;

/// Ollama API client
pub struct OllamaClient {
    /// Base URL for Ollama API (default: http://127.0.0.1:11434)
    base_url: String,
    /// Model to use for generation
    model: OllamaModel,
    /// Context window size
    context_size: usize,
    /// Temperature for generation
    temperature: f32,
    /// HTTP transport (mock or real)
    transport: Arc<dyn HttpTransport>,
}

impl OllamaClient {
    /// Create a new client with mock transport (default, for testing)
    pub fn new_with_mock(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            model: OllamaModel::default(),
            context_size: 8192,
            temperature: 0.1,
            transport,
        }
    }

    /// Create a new client with real HTTP transport (requires `http` feature)
    #[cfg(feature = "http")]
    pub fn new_with_http() -> Result<Self, OllamaError> {
        use crate::transport::ReqwestTransport;
        let transport = ReqwestTransport::new()?;
        Ok(Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            model: OllamaModel::default(),
            context_size: 8192,
            temperature: 0.1,
            transport: Arc::new(transport),
        })
    }

    /// Create a new client with real HTTP transport and custom timeout
    #[cfg(feature = "http")]
    pub fn new_with_http_timeout(timeout: std::time::Duration) -> Result<Self, OllamaError> {
        use crate::transport::ReqwestTransport;
        let transport = ReqwestTransport::with_timeout(timeout)?;
        Ok(Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            model: OllamaModel::default(),
            context_size: 8192,
            temperature: 0.1,
            transport: Arc::new(transport),
        })
    }

    /// Set the base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: OllamaModel) -> Self {
        self.model = model;
        self
    }

    /// Set the model by name
    pub fn with_model_name(mut self, name: &str) -> Self {
        self.model = OllamaModel::from_str(name);
        self
    }

    /// Set the context size
    pub fn with_context_size(mut self, context_size: usize) -> Self {
        self.context_size = context_size;
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the model
    pub fn model(&self) -> &OllamaModel {
        &self.model
    }

    /// Perform a health check by listing available models
    pub async fn health_check(&self) -> Result<Vec<String>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        let response_text = self.transport.get(&url).await?;

        let tags: TagsResponse = serde_json::from_str(&response_text)
            .map_err(|e| OllamaError::ResponseParsing(e.to_string()))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    /// Check if a specific model is available
    pub async fn is_model_available(&self, model_name: &str) -> Result<bool, OllamaError> {
        let models = self.health_check().await?;
        Ok(models.iter().any(|m| m == model_name))
    }

    /// Generate text from a prompt
    pub async fn generate_text(&self, prompt: &str) -> Result<String, OllamaError> {
        let request = ChatRequest {
            model: self.model.as_str().to_string(),
            messages: vec![Message::user(prompt)],
            stream: false,
            options: Some(GenerationOptions {
                temperature: Some(self.temperature),
                num_ctx: Some(self.context_size),
                ..Default::default()
            }),
        };

        let url = format!("{}/api/chat", self.base_url);
        let body = serde_json::to_string(&request)?;

        let response_text = self
            .transport
            .post_json(&url, &[("Content-Type", "application/json")], &body)
            .await?;

        let response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| OllamaError::ResponseParsing(e.to_string()))?;

        Ok(response.message.content)
    }

    /// Generate text with a system prompt
    pub async fn generate_with_system(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, OllamaError> {
        let request = ChatRequest {
            model: self.model.as_str().to_string(),
            messages: vec![Message::system(system_prompt), Message::user(user_prompt)],
            stream: false,
            options: Some(GenerationOptions {
                temperature: Some(self.temperature),
                num_ctx: Some(self.context_size),
                ..Default::default()
            }),
        };

        let url = format!("{}/api/chat", self.base_url);
        let body = serde_json::to_string(&request)?;

        let response_text = self
            .transport
            .post_json(&url, &[("Content-Type", "application/json")], &body)
            .await?;

        let response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| OllamaError::ResponseParsing(e.to_string()))?;

        Ok(response.message.content)
    }

    /// Review an artifact (code) and return structured feedback
    ///
    /// This method:
    /// 1. Sanitizes the artifact content
    /// 2. Builds a review prompt with the code review system prompt
    /// 3. Parses and validates the JSON response
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, OllamaError> {
        // Step 1: Sanitize artifact content
        let sanitized = sanitize_artifact_content(artifact);

        // Step 2: Build user prompt
        let user_prompt = format!(
            "Please review the following code and provide your assessment as JSON:\n\n```\n{}\n```",
            sanitized
        );

        // Step 3: Generate response
        let response_text = self
            .generate_with_system(CODE_REVIEW_SYSTEM_PROMPT, &user_prompt)
            .await?;

        // Step 4: Extract JSON from response (it might be wrapped in markdown code blocks)
        let json_text = extract_json_from_response(&response_text)?;

        // Step 5: Parse and validate
        let review: AgentReview = serde_json::from_str(&json_text)
            .map_err(|e| OllamaError::ResponseParsing(format!("Invalid review JSON: {}", e)))?;

        // Step 6: Validate schema
        validate_review_schema(&review)?;

        Ok(review)
    }
}

/// Extract JSON from a response that might be wrapped in markdown code blocks
fn extract_json_from_response(response: &str) -> Result<String, OllamaError> {
    let trimmed = response.trim();

    // If it starts with {, assume it's raw JSON
    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }

    // Try to extract from markdown code block
    if let Some(start) = trimmed.find("```json") {
        let after_marker = &trimmed[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return Ok(after_marker[..end].trim().to_string());
        }
    }

    // Try plain code block
    if let Some(start) = trimmed.find("```") {
        let after_marker = &trimmed[start + 3..];
        if let Some(end) = after_marker.find("```") {
            let content = after_marker[..end].trim();
            // Skip language identifier if present
            if let Some(newline) = content.find('\n') {
                let possible_json = content[newline..].trim();
                if possible_json.starts_with('{') {
                    return Ok(possible_json.to_string());
                }
            }
            if content.starts_with('{') {
                return Ok(content.to_string());
            }
        }
    }

    // Last resort: find first { and last }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    Err(OllamaError::ResponseParsing(
        "Could not extract JSON from response".to_string(),
    ))
}

/// Validate review schema requirements
fn validate_review_schema(review: &AgentReview) -> Result<(), OllamaError> {
    // Confidence must be in range
    if !(0.0..=1.0).contains(&review.confidence) {
        return Err(OllamaError::SchemaValidation(format!(
            "confidence must be 0.0-1.0, got {}",
            review.confidence
        )));
    }

    // Reasoning must not be empty
    if review.reasoning.is_empty() {
        return Err(OllamaError::SchemaValidation(
            "reasoning cannot be empty".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;

    fn mock_tags_response() -> String {
        r#"{
            "models": [
                {"name": "codellama:7b-instruct", "digest": "abc", "size": 4000000000, "modified_at": "2024-01-01"}
            ]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_generate_text() {
        let transport = MockTransport::with_canned_response(
            r#"{"model":"codellama:7b-instruct","message":{"role":"assistant","content":"Hello!"},"done":true}"#.to_string()
        );
        let client = OllamaClient::new_with_mock(Arc::new(transport));

        let result = client.generate_text("Say hello").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello!");
    }

    #[tokio::test]
    async fn test_health_check() {
        let transport = MockTransport::with_responses("".to_string(), mock_tags_response());
        let client = OllamaClient::new_with_mock(Arc::new(transport));

        let models = client.health_check().await;
        assert!(models.is_ok());
        assert!(models.unwrap().contains(&"codellama:7b-instruct".to_string()));
    }

    #[tokio::test]
    async fn test_review_artifact() {
        // The content needs to be properly JSON-escaped
        let review_json = mock_review_json();
        let escaped_content = review_json
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");

        let response = format!(
            r#"{{"model":"test","message":{{"role":"assistant","content":"{}"}},"done":true}}"#,
            escaped_content
        );

        let transport = MockTransport::with_canned_response(response);
        let client = OllamaClient::new_with_mock(Arc::new(transport));

        let result = client.review_artifact("fn main() {}").await;
        assert!(result.is_ok(), "review_artifact failed: {:?}", result.err());
        let review = result.unwrap();
        assert_eq!(review.agent_id, "ollama");
        assert!(review.sign_off);
    }

    fn mock_review_json() -> String {
        r#"{"agent_id":"ollama","verdict":"pass","confidence":0.9,"issues":[],"suggestions":[],"sign_off":true,"reasoning":"Code is good"}"#.to_string()
    }

    #[test]
    fn test_extract_json_raw() {
        let response = r#"{"key": "value"}"#;
        let result = extract_json_from_response(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let response = "Here is the JSON:\n```json\n{\"key\": \"value\"}\n```\nDone.";
        let result = extract_json_from_response(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_validate_review_schema_valid() {
        let review = AgentReview {
            agent_id: "ollama".to_string(),
            verdict: aiy_adapters::Verdict::Pass,
            confidence: 0.85,
            issues: vec![],
            suggestions: vec![],
            sign_off: true,
            reasoning: "Looks good".to_string(),
        };
        assert!(validate_review_schema(&review).is_ok());
    }

    #[test]
    fn test_validate_review_schema_invalid_confidence() {
        let review = AgentReview {
            agent_id: "ollama".to_string(),
            verdict: aiy_adapters::Verdict::Pass,
            confidence: 1.5, // Invalid
            issues: vec![],
            suggestions: vec![],
            sign_off: true,
            reasoning: "Looks good".to_string(),
        };
        assert!(validate_review_schema(&review).is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let transport = Arc::new(MockTransport::new());
        let client = OllamaClient::new_with_mock(transport)
            .with_base_url("http://localhost:11435")
            .with_model(OllamaModel::CodeLlama13bInstruct)
            .with_context_size(16384)
            .with_temperature(0.2);

        assert_eq!(client.base_url(), "http://localhost:11435");
        assert_eq!(*client.model(), OllamaModel::CodeLlama13bInstruct);
    }
}
