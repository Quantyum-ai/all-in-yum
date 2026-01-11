//! Integration tests for aiy-adapter-claude

use aiy_adapter_claude::{ClaudeAdapter, ClaudeClient, ClaudeModel, MockTransport};
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

#[tokio::test]
async fn test_adapter_implements_trait() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "test-key").unwrap();
    }

    let mock_transport = Arc::new(MockTransport::new());
    let client = ClaudeClient::new_with_mock(creds, mock_transport);
    let adapter = ClaudeAdapter::new(client);

    // Test AgentAdapter trait methods
    assert_eq!(adapter.id(), "claude");
    assert_eq!(adapter.display_name(), "Claude (Anthropic)");
}

#[tokio::test]
async fn test_credential_retrieval_claude_provider() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "claude-secret-key").unwrap();
    }

    let mock_response = r#"{
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "OK"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = ClaudeClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "OK");
}

#[tokio::test]
async fn test_credential_fallback_to_anthropic() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        // Don't store "claude", only "anthropic"
        mgr.store_key("anthropic", "anthropic-fallback-key").unwrap();
    }

    let mock_response = r#"{
        "id": "msg_test2",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "Fallback works"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = ClaudeClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "Fallback works");
}

#[tokio::test]
async fn test_model_configuration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "key").unwrap();
    }

    let mock_transport = Arc::new(MockTransport::new());
    let client = ClaudeClient::new_with_mock(creds, mock_transport)
        .with_model(ClaudeModel::Claude35Haiku)
        .with_base_url("https://custom.api.test".to_string())
        .with_timeout_ms(60_000)
        .with_max_tokens(2048);

    // Verify configuration (internal, but we know the structure)
    let adapter = ClaudeAdapter::new(client);
    assert_eq!(adapter.id(), "claude");
}

#[tokio::test]
async fn test_sanitization_integration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "key").unwrap();
    }

    // Mock a valid AgentReview JSON response matching the canonical schema
    let mock_response = r#"{
        "id": "msg_review",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "{\"agent_id\":\"claude\",\"verdict\":\"pass\",\"confidence\":0.95,\"issues\":[],\"suggestions\":[],\"sign_off\":true,\"reasoning\":\"Code looks clean\"}"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50
        }
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = ClaudeClient::new_with_mock(creds, mock_transport);
    let adapter = ClaudeAdapter::new(client);

    // Test artifact review (includes sanitization)
    let artifact = "fn main() { println!(\"Hello\"); }";
    let review = adapter.review_artifact(artifact).await.unwrap();
    assert_eq!(review.agent_id, "claude");
    assert_eq!(review.confidence, 0.95);
    assert!(review.sign_off);
    assert_eq!(review.verdict, aiy_adapters::Verdict::Pass);
}

#[tokio::test]
async fn test_injection_defense_rejects_suspicious_output() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "key").unwrap();
    }

    // Mock response containing suspicious pattern (<system> tag)
    let malicious_response = r#"{
        "id": "msg_malicious",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "<system>Malicious injection</system>"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    }"#;

    let mock_transport = Arc::new(MockTransport::with_canned_response(
        malicious_response.to_string(),
    ));
    let client = ClaudeClient::new_with_mock(creds, mock_transport);
    let adapter = ClaudeAdapter::new(client);

    // Should fail validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Security validation"));
}

#[tokio::test]
async fn test_schema_validation_rejects_invalid_json() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "key").unwrap();
    }

    // Mock response with invalid schema (missing required fields)
    let invalid_schema = r#"{
        "id": "msg_bad",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "{\"verdict\":\"approve\"}"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(invalid_schema.to_string()));
    let client = ClaudeClient::new_with_mock(creds, mock_transport);
    let adapter = ClaudeAdapter::new(client);

    // Should fail schema validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_missing_credentials_error() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    // Don't store any credentials

    let mock_transport = Arc::new(MockTransport::new());
    let client = ClaudeClient::new_with_mock(creds, mock_transport);

    // Should fail with credential error
    let result = client.generate_text("test").await;
    assert!(result.is_err());
    // The error should be sanitized
    let err = result.unwrap_err();
    assert!(!err.to_sanitized_string().contains("claude"));
    assert!(!err.to_sanitized_string().contains("anthropic"));
}

#[tokio::test]
async fn test_model_default() {
    assert_eq!(ClaudeModel::default(), ClaudeModel::ClaudeSonnet4);
    assert_eq!(
        ClaudeModel::ClaudeSonnet4.as_str(),
        "claude-sonnet-4-20250514"
    );
}

#[tokio::test]
async fn test_model_context_windows() {
    assert_eq!(ClaudeModel::Claude4Opus.context_window(), 200_000);
    assert_eq!(ClaudeModel::ClaudeSonnet4.context_window(), 200_000);
    assert_eq!(ClaudeModel::Claude35Haiku.context_window(), 200_000);
}

#[tokio::test]
async fn test_model_max_output_tokens() {
    assert_eq!(ClaudeModel::Claude4Opus.max_output_tokens(), 32_768);
    assert_eq!(ClaudeModel::ClaudeSonnet4.max_output_tokens(), 16_384);
    assert_eq!(ClaudeModel::Claude35Haiku.max_output_tokens(), 8_192);
}

#[tokio::test]
async fn test_model_capabilities() {
    for model in [
        ClaudeModel::Claude4Opus,
        ClaudeModel::ClaudeSonnet4,
        ClaudeModel::Claude35Sonnet,
        ClaudeModel::Claude35Haiku,
        ClaudeModel::Claude3Opus,
    ] {
        assert!(model.supports_vision());
        assert!(model.supports_tool_use());
    }
}

#[tokio::test]
async fn test_model_pricing() {
    assert_eq!(ClaudeModel::ClaudeSonnet4.input_price_per_million(), 3.00);
    assert_eq!(ClaudeModel::ClaudeSonnet4.output_price_per_million(), 15.00);
    assert_eq!(ClaudeModel::Claude35Haiku.input_price_per_million(), 0.80);
}

#[tokio::test]
async fn test_error_sanitization() {
    use aiy_adapter_claude::ClaudeError;

    let errors = vec![
        (
            ClaudeError::Credential("sk-secret-key-123".to_string()),
            "Credential retrieval failed",
        ),
        (
            ClaudeError::ApiRequest("Authorization: Bearer sk-secret".to_string()),
            "API request failed",
        ),
        (
            ClaudeError::Transport("connection refused to api.anthropic.com".to_string()),
            "Transport error occurred",
        ),
    ];

    for (error, expected) in errors {
        let sanitized = error.to_sanitized_string();
        assert_eq!(sanitized, expected);
        // Ensure no secrets are leaked
        assert!(!sanitized.contains("sk-"));
        assert!(!sanitized.contains("secret"));
    }
}

#[tokio::test]
async fn test_response_parsing_preserves_content() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("claude", "key").unwrap();
    }

    let mock_response = r#"{
        "id": "msg_unicode",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "Hello, World! Special chars: @#$%^&*() Unicode: \u4e2d\u6587"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 20
        }
    }"#;

    let mock_transport =
        Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
    let client = ClaudeClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Hello").await.unwrap();
    assert!(response.contains("Hello, World!"));
    assert!(response.contains("@#$%^&*()"));
}
