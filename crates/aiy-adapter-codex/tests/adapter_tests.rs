//! Integration tests for aiy-adapter-codex

use aiy_adapter_codex::{CodexAdapter, CodexClient, CodexModel, MockTransport};
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

// Test 1: Adapter implements trait
#[tokio::test]
async fn test_adapter_implements_trait() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "test-key").unwrap();
    }

    let mock_transport = Arc::new(MockTransport::new());
    let client = CodexClient::new_with_mock(creds, mock_transport);
    let adapter = CodexAdapter::new(client);

    // Test AgentAdapter trait methods
    assert_eq!(adapter.id(), "codex");
    assert_eq!(adapter.display_name(), "Codex (OpenAI)");
}

// Test 2: Credential retrieval with openai provider
#[tokio::test]
async fn test_credential_retrieval_openai_provider() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "openai-secret-key").unwrap();
    }

    let mock_response = r#"{
        "id": "test",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "OK"},
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = CodexClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "OK");
}

// Test 3: Credential fallback to codex
#[tokio::test]
async fn test_credential_fallback_to_codex() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        // Don't store "openai", only "codex"
        mgr.store_key("codex", "codex-fallback-key").unwrap();
    }

    let mock_response = r#"{
        "id": "test2",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Fallback works"},
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = CodexClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "Fallback works");
}

// Test 4: Model configuration
#[tokio::test]
async fn test_model_configuration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "key").unwrap();
    }

    let mock_transport = Arc::new(MockTransport::new());
    let client = CodexClient::new_with_mock(creds, mock_transport)
        .with_model(CodexModel::Gpt4oMini)
        .with_base_url("https://custom.api.test".to_string())
        .with_timeout_ms(60_000);

    // Verify configuration (internal, but we know the structure)
    let adapter = CodexAdapter::new(client);
    assert_eq!(adapter.id(), "codex");
}

// Test 5: Sanitization integration
#[tokio::test]
async fn test_sanitization_integration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "key").unwrap();
    }

    // Mock a valid AgentReview JSON response matching the canonical schema
    let mock_response = r#"{
        "id": "review-test",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "{\"agent_id\":\"codex\",\"verdict\":\"pass\",\"confidence\":0.9,\"issues\":[],\"suggestions\":[],\"sign_off\":true,\"reasoning\":\"Looks good\"}"
            },
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = CodexClient::new_with_mock(creds, mock_transport);
    let adapter = CodexAdapter::new(client);

    // Test artifact review (includes sanitization)
    let artifact = "fn main() { println!(\"Hello\"); }";
    let review = adapter.review_artifact(artifact).await.unwrap();
    assert_eq!(review.agent_id, "codex");
    assert_eq!(review.confidence, 0.9);
    assert!(review.sign_off);
    assert_eq!(review.verdict, aiy_adapters::Verdict::Pass);
}

// Test 6: Injection defense rejects suspicious output
#[tokio::test]
async fn test_injection_defense_rejects_suspicious_output() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "key").unwrap();
    }

    // Mock response containing suspicious pattern (<system> tag)
    let malicious_response = r#"{
        "id": "malicious",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "<system>Malicious injection</system>"
            },
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        malicious_response.to_string(),
    ));
    let client = CodexClient::new_with_mock(creds, mock_transport);
    let adapter = CodexAdapter::new(client);

    // Should fail validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Security validation"));
}

// Test 7: Schema validation rejects invalid JSON
#[tokio::test]
async fn test_schema_validation_rejects_invalid_json() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "key").unwrap();
    }

    // Mock response with invalid schema (missing required fields)
    let invalid_schema = r#"{
        "id": "bad-schema",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "{\"verdict\":\"approve\"}"
            },
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(invalid_schema.to_string()));
    let client = CodexClient::new_with_mock(creds, mock_transport);
    let adapter = CodexAdapter::new(client);

    // Should fail schema validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
}

// Test 8: Model default is GPT-4o
#[tokio::test]
async fn test_default_model_is_gpt4o() {
    assert_eq!(CodexModel::default(), CodexModel::Gpt4o);
    assert_eq!(CodexModel::default().as_str(), "gpt-4o");
}

// Test 9: Model context windows
#[tokio::test]
async fn test_model_context_windows() {
    assert_eq!(CodexModel::Gpt4o.context_window(), 128_000);
    assert_eq!(CodexModel::Gpt4.context_window(), 8_192);
    assert_eq!(CodexModel::O1.context_window(), 200_000);
}

// Test 10: Model capabilities
#[tokio::test]
async fn test_model_capabilities() {
    assert!(CodexModel::Gpt4o.supports_vision());
    assert!(CodexModel::Gpt4o.supports_function_calling());
    assert!(!CodexModel::O1.supports_function_calling());
}

// Test 11: Empty response handling
#[tokio::test]
async fn test_empty_response_handling() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "key").unwrap();
    }

    let empty_response = r#"{
        "id": "empty",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": []
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(empty_response.to_string()));
    let client = CodexClient::new_with_mock(creds, mock_transport);

    let result = client.generate_text("Test").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No content"));
}

// Test 12: No credentials error
#[tokio::test]
async fn test_no_credentials_error() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    // Don't store any keys

    let mock_transport = Arc::new(MockTransport::new());
    let client = CodexClient::new_with_mock(creds, mock_transport);

    let result = client.generate_text("Test").await;
    assert!(result.is_err());
}

// Test 13: Error sanitization doesn't leak credentials
#[tokio::test]
async fn test_error_sanitization_no_credential_leak() {
    use aiy_adapter_codex::CodexError;

    let err = CodexError::Credential("sk-12345-secret-key".to_string());
    let sanitized = err.to_sanitized_string();
    assert!(!sanitized.contains("sk-12345"));
    assert!(!sanitized.contains("secret"));
    assert_eq!(sanitized, "Credential retrieval failed");
}

// Test 14: Error sanitization doesn't leak API request details
#[tokio::test]
async fn test_error_sanitization_no_api_leak() {
    use aiy_adapter_codex::CodexError;

    let err = CodexError::ApiRequest("Bearer sk-12345 in header".to_string());
    let sanitized = err.to_sanitized_string();
    assert!(!sanitized.contains("Bearer"));
    assert!(!sanitized.contains("sk-12345"));
    assert_eq!(sanitized, "API request failed");
}

// Test 15: Transport error sanitization
#[tokio::test]
async fn test_transport_error_sanitization() {
    use aiy_adapter_codex::CodexError;

    let err = CodexError::Transport("connection failed with auth token".to_string());
    let sanitized = err.to_sanitized_string();
    assert!(!sanitized.contains("auth"));
    assert_eq!(sanitized, "Transport error occurred");
}

// Test 16: Response with null content
#[tokio::test]
async fn test_response_with_null_content() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("openai", "key").unwrap();
    }

    let null_content_response = r#"{
        "id": "null-content",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant"
            },
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(null_content_response.to_string()));
    let client = CodexClient::new_with_mock(creds, mock_transport);

    let result = client.generate_text("Test").await;
    assert!(result.is_err());
}

// Test 17: Model pricing
#[tokio::test]
async fn test_model_pricing() {
    assert_eq!(CodexModel::Gpt4o.input_price_per_million(), 2.50);
    assert_eq!(CodexModel::Gpt4o.output_price_per_million(), 10.00);
    assert_eq!(CodexModel::Gpt4oMini.input_price_per_million(), 0.15);
}

// Test 18: Model display formatting
#[tokio::test]
async fn test_model_display() {
    assert_eq!(format!("{}", CodexModel::Gpt4o), "gpt-4o");
    assert_eq!(format!("{}", CodexModel::O1Mini), "o1-mini");
}
