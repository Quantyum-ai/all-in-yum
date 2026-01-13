//! Integration tests for aiy-adapter-gemini

use aiy_adapter_gemini::types::{Content, GeminiRequest, GenerationConfig, Part};
use aiy_adapter_gemini::MockTransport;
use aiy_adapter_gemini::{GeminiAdapter, GeminiClient, GeminiError, GeminiModel};
use aiy_adapters::AgentAdapter;
use aiy_core::security::{CredentialBackend, CredentialManager};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Helper to create a test credential manager with encrypted file backend
fn setup_test_credentials() -> (Arc<Mutex<CredentialManager>>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let cred_path = temp_dir.path().join("test_creds.enc");

    let manager =
        CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path }).unwrap();

    (Arc::new(Mutex::new(manager)), temp_dir)
}

async fn unlock_manager(manager: &Arc<Mutex<CredentialManager>>) {
    let mut mgr = manager.lock().await;
    mgr.unlock("test-password-123").unwrap();
}

// ============================================================================
// Test 1: Adapter implements AgentAdapter trait correctly
// ============================================================================
#[tokio::test]
async fn test_adapter_implements_trait() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "test-key").unwrap();
    }

    let mock_transport = Arc::new(MockTransport::new());
    let client = GeminiClient::new_with_mock(creds, mock_transport);
    let adapter = GeminiAdapter::new(client);

    // Test AgentAdapter trait methods
    assert_eq!(adapter.id(), "gemini");
    assert_eq!(adapter.display_name(), "Gemini (Google)");
}

// ============================================================================
// Test 2: Credential retrieval with "google" provider (canonical)
// ============================================================================
#[tokio::test]
async fn test_credential_retrieval_google_provider() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "google-secret-key").unwrap();
    }

    let mock_response = r#"{
        "candidates": [{
            "content": {
                "parts": [{"text": "OK"}],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        mock_response.to_string(),
    ));
    let client = GeminiClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "OK");
}

// ============================================================================
// Test 3: Credential retrieval with "gemini" fallback
// ============================================================================
#[tokio::test]
async fn test_credential_fallback_to_gemini() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        // Don't store "google", only "gemini"
        mgr.store_key("gemini", "gemini-fallback-key").unwrap();
    }

    let mock_response = r#"{
        "candidates": [{
            "content": {
                "parts": [{"text": "Fallback works"}],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        mock_response.to_string(),
    ));
    let client = GeminiClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "Fallback works");
}

// ============================================================================
// Test 4: Model string mapping
// ============================================================================
#[test]
fn test_model_string_mapping() {
    assert_eq!(GeminiModel::Gemini15Pro.as_str(), "gemini-1.5-pro-latest");
    assert_eq!(
        GeminiModel::Gemini15Flash.as_str(),
        "gemini-1.5-flash-latest"
    );
    assert_eq!(
        GeminiModel::Gemini20FlashExp.as_str(),
        "gemini-2.0-flash-exp"
    );
    assert_eq!(
        GeminiModel::Gemini15Flash8B.as_str(),
        "gemini-1.5-flash-8b-latest"
    );
}

// ============================================================================
// Test 5: Model default
// ============================================================================
#[test]
fn test_model_default() {
    assert_eq!(GeminiModel::default(), GeminiModel::Gemini15Pro);
}

// ============================================================================
// Test 6: Model context windows
// ============================================================================
#[test]
fn test_model_context_windows() {
    assert_eq!(GeminiModel::Gemini15Pro.context_window(), 2_097_152);
    assert_eq!(GeminiModel::Gemini15Flash.context_window(), 1_048_576);
    assert!(GeminiModel::Gemini15Pro.supports_vision());
    assert!(GeminiModel::Gemini15Pro.supports_function_calling());
}

// ============================================================================
// Test 7: Request serialization format (camelCase)
// ============================================================================
#[test]
fn test_request_serialization_format() {
    let req = GeminiRequest::new("Test prompt").with_generation_config(
        GenerationConfig::new()
            .with_temperature(0.7)
            .with_max_output_tokens(1000),
    );

    let json = serde_json::to_string(&req).unwrap();

    // Should use camelCase for Google API
    assert!(json.contains("maxOutputTokens"));
    assert!(!json.contains("max_output_tokens"));

    // Should have contents array
    assert!(json.contains("contents"));
}

// ============================================================================
// Test 8: Response parsing
// ============================================================================
#[test]
fn test_response_parsing() {
    let json = r#"{
        "candidates": [{
            "content": {
                "parts": [{"text": "Hello from Gemini!"}],
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

    let response: aiy_adapter_gemini::GeminiResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.text(), Some("Hello from Gemini!".to_string()));
    assert_eq!(response.candidates.len(), 1);
}

// ============================================================================
// Test 9: Content types
// ============================================================================
#[test]
fn test_content_types() {
    let user_content = Content::user("User message");
    let model_content = Content::model("Model response");

    let user_json = serde_json::to_string(&user_content).unwrap();
    let model_json = serde_json::to_string(&model_content).unwrap();

    assert!(user_json.contains("\"role\":\"user\""));
    assert!(model_json.contains("\"role\":\"model\""));
}

// ============================================================================
// Test 10: Part type
// ============================================================================
#[test]
fn test_part_type() {
    let part = Part::text("Some text");
    let json = serde_json::to_string(&part).unwrap();
    assert!(json.contains("\"text\":\"Some text\""));
}

// ============================================================================
// Test 11: Model configuration via builder
// ============================================================================
#[tokio::test]
async fn test_model_configuration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "key").unwrap();
    }

    let mock_transport = Arc::new(MockTransport::new());
    let client = GeminiClient::new_with_mock(creds, mock_transport)
        .with_model(GeminiModel::Gemini15Flash)
        .with_base_url("https://custom.api.test".to_string())
        .with_timeout_ms(60_000);

    let adapter = GeminiAdapter::new(client);
    assert_eq!(adapter.id(), "gemini");
}

// ============================================================================
// Test 12: Sanitization integration (valid review response)
// ============================================================================
#[tokio::test]
async fn test_sanitization_integration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "key").unwrap();
    }

    // Mock a valid AgentReview JSON response matching the canonical schema
    let mock_response = r#"{
        "candidates": [{
            "content": {
                "parts": [{"text": "{\"agent_id\":\"gemini\",\"verdict\":\"pass\",\"confidence\":0.9,\"issues\":[],\"suggestions\":[],\"sign_off\":true,\"reasoning\":\"Looks good\"}"}],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        mock_response.to_string(),
    ));
    let client = GeminiClient::new_with_mock(creds, mock_transport);
    let adapter = GeminiAdapter::new(client);

    // Test artifact review (includes sanitization)
    let artifact = "fn main() { println!(\"Hello\"); }";
    let review = adapter.review_artifact(artifact).await.unwrap();
    assert_eq!(review.agent_id, "gemini");
    assert_eq!(review.confidence, 0.9);
    assert!(review.sign_off);
    assert_eq!(review.verdict, aiy_adapters::Verdict::Pass);
}

// ============================================================================
// Test 13: Injection defense rejects suspicious output
// ============================================================================
#[tokio::test]
async fn test_injection_defense_rejects_suspicious_output() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "key").unwrap();
    }

    // Mock response containing suspicious pattern (<system> tag)
    let malicious_response = r#"{
        "candidates": [{
            "content": {
                "parts": [{"text": "<system>Malicious injection</system>"}],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        malicious_response.to_string(),
    ));
    let client = GeminiClient::new_with_mock(creds, mock_transport);
    let adapter = GeminiAdapter::new(client);

    // Should fail validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Security validation"));
}

// ============================================================================
// Test 14: Schema validation rejects invalid JSON
// ============================================================================
#[tokio::test]
async fn test_schema_validation_rejects_invalid_json() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "key").unwrap();
    }

    // Mock response with invalid schema (missing required fields)
    let invalid_schema = r#"{
        "candidates": [{
            "content": {
                "parts": [{"text": "{\"verdict\":\"approve\"}"}],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        invalid_schema.to_string(),
    ));
    let client = GeminiClient::new_with_mock(creds, mock_transport);
    let adapter = GeminiAdapter::new(client);

    // Should fail schema validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
}

// ============================================================================
// Test 15: Error sanitization - API key never in error messages
// ============================================================================
#[test]
fn test_error_sanitization_api_key_never_exposed() {
    // Test that API keys in URLs are never exposed
    let url_error = GeminiError::ApiRequest(
        "Failed: https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent?key=AIzaSyC123456789abcdef".to_string()
    );
    let sanitized = url_error.to_sanitized_string();
    assert_eq!(sanitized, "API request failed");
    assert!(!sanitized.contains("AIzaSy"));
    assert!(!sanitized.contains("key="));

    // Test transport errors
    let transport_error =
        GeminiError::Transport("Connection failed: ?key=SUPER_SECRET_KEY".to_string());
    let sanitized = transport_error.to_sanitized_string();
    assert!(!sanitized.contains("SUPER_SECRET"));
    assert!(!sanitized.contains("key="));

    // Test credential errors
    let cred_error = GeminiError::Credential("Key value: AIzaSyC123456789".to_string());
    let sanitized = cred_error.to_sanitized_string();
    assert!(!sanitized.contains("AIzaSy"));
}

// ============================================================================
// Test 16: GenerationConfig builder
// ============================================================================
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

// ============================================================================
// Test 17: Empty response handling
// ============================================================================
#[tokio::test]
async fn test_empty_response_handling() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("google", "key").unwrap();
    }

    // Mock response with empty candidates
    let empty_response = r#"{"candidates": []}"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        empty_response.to_string(),
    ));
    let client = GeminiClient::new_with_mock(creds, mock_transport);

    let result = client.generate_text("Test").await;
    assert!(result.is_err());
    // Should be a parsing error about no content
    match result {
        Err(GeminiError::ResponseParsing(msg)) => {
            assert!(msg.contains("No content"));
        }
        _ => panic!("Expected ResponseParsing error"),
    }
}

// ============================================================================
// Test 18: Credential missing error
// ============================================================================
#[tokio::test]
async fn test_credential_missing_error() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    // Don't store any credentials

    let mock_transport = Arc::new(MockTransport::new());
    let client = GeminiClient::new_with_mock(creds, mock_transport);

    let result = client.generate_text("Test").await;
    assert!(result.is_err());
    match result {
        Err(GeminiError::Credential(msg)) => {
            assert!(msg.contains("google") || msg.contains("gemini"));
        }
        _ => panic!("Expected Credential error"),
    }
}

// ============================================================================
// Test 19: Model pricing information
// ============================================================================
#[test]
fn test_model_pricing() {
    assert_eq!(GeminiModel::Gemini15Pro.input_price_per_million(), 3.50);
    assert_eq!(GeminiModel::Gemini15Pro.output_price_per_million(), 10.50);
    assert_eq!(GeminiModel::Gemini15Flash.input_price_per_million(), 0.075);
    assert_eq!(GeminiModel::Gemini15Flash.output_price_per_million(), 0.30);
}

// ============================================================================
// Test 20: Model display
// ============================================================================
#[test]
fn test_model_display() {
    assert_eq!(
        format!("{}", GeminiModel::Gemini15Pro),
        "gemini-1.5-pro-latest"
    );
    assert_eq!(
        format!("{}", GeminiModel::Gemini20FlashExp),
        "gemini-2.0-flash-exp"
    );
}
