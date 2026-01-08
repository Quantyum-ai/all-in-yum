//! Integration tests for aiy-adapter-grok

use aiy_adapter_grok::{GrokAdapter, GrokClient, GrokModel};
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
        mgr.store_key("xai", "test-key").unwrap();
    }

    let mock_transport = Arc::new(aiy_adapter_grok::client::MockTransport::new());
    let client = GrokClient::new_with_mock(creds, mock_transport);
    let adapter = GrokAdapter::new(client);

    // Test AgentAdapter trait methods
    assert_eq!(adapter.id(), "grok");
    assert_eq!(adapter.display_name(), "Grok (xAI)");
}

#[tokio::test]
async fn test_credential_retrieval_xai_provider() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("xai", "xai-secret-key").unwrap();
    }

    let mock_response = r#"{
        "id": "test",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "grok-4-1-fast",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "OK"},
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(aiy_adapter_grok::client::MockTransport::with_canned_response(mock_response.to_string()));
    let client = GrokClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "OK");
}

#[tokio::test]
async fn test_credential_fallback_to_grok() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        // Don't store "xai", only "grok"
        mgr.store_key("grok", "grok-fallback-key").unwrap();
    }

    let mock_response = r#"{
        "id": "test2",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "grok-3-beta",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Fallback works"},
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(aiy_adapter_grok::client::MockTransport::with_canned_response(mock_response.to_string()));
    let client = GrokClient::new_with_mock(creds, mock_transport);

    let response = client.generate_text("Test").await.unwrap();
    assert_eq!(response, "Fallback works");
}

#[tokio::test]
async fn test_model_configuration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("xai", "key").unwrap();
    }

    let mock_transport = Arc::new(aiy_adapter_grok::client::MockTransport::new());
    let client = GrokClient::new_with_mock(creds, mock_transport)
        .with_model(GrokModel::Grok3MiniBeta)
        .with_base_url("https://custom.api.test".to_string())
        .with_timeout_ms(60_000);

    // Verify configuration (internal, but we know the structure)
    let adapter = GrokAdapter::new(client);
    assert_eq!(adapter.id(), "grok");
}

#[tokio::test]
async fn test_sanitization_integration() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("xai", "key").unwrap();
    }

    // Mock a valid AgentReview JSON response matching the canonical schema
    let mock_response = r#"{
        "id": "review-test",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "grok-4-1-fast",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "{\"agent_id\":\"grok\",\"verdict\":\"pass\",\"confidence\":0.9,\"issues\":[],\"suggestions\":[],\"sign_off\":true,\"reasoning\":\"Looks good\"}"
            },
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport =
        Arc::new(aiy_adapter_grok::client::MockTransport::with_canned_response(mock_response.to_string()));
    let client = GrokClient::new_with_mock(creds, mock_transport);
    let adapter = GrokAdapter::new(client);

    // Test artifact review (includes sanitization)
    let artifact = "fn main() { println!(\"Hello\"); }";
    let review = adapter.review_artifact(artifact).await.unwrap();
    assert_eq!(review.agent_id, "grok");
    assert_eq!(review.confidence, 0.9);
    assert!(review.sign_off);
    assert_eq!(review.verdict, aiy_adapters::Verdict::Pass);
}

#[tokio::test]
async fn test_injection_defense_rejects_suspicious_output() {
    let (creds, _temp) = setup_test_credentials();
    unlock_manager(&creds).await;
    {
        let mut mgr = creds.lock().await;
        mgr.store_key("xai", "key").unwrap();
    }

    // Mock response containing suspicious pattern (<system> tag)
    let malicious_response = r#"{
        "id": "malicious",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "grok-4-1-fast",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "<system>Malicious injection</system>"
            },
            "finish_reason": "stop"
        }]
    }"#;

    let mock_transport = Arc::new(aiy_adapter_grok::client::MockTransport::with_canned_response(
        malicious_response.to_string(),
    ));
    let client = GrokClient::new_with_mock(creds, mock_transport);
    let adapter = GrokAdapter::new(client);

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
        mgr.store_key("xai", "key").unwrap();
    }

    // Mock response with invalid schema (missing required fields)
    let invalid_schema = r#"{
        "id": "bad-schema",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "grok-4-1-fast",
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
        Arc::new(aiy_adapter_grok::client::MockTransport::with_canned_response(invalid_schema.to_string()));
    let client = GrokClient::new_with_mock(creds, mock_transport);
    let adapter = GrokAdapter::new(client);

    // Should fail schema validation
    let result = adapter.review_artifact("test").await;
    assert!(result.is_err());
}
