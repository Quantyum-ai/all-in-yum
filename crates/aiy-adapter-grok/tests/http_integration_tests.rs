//! HTTP integration tests for aiy-adapter-grok
//!
//! These tests use wiremock to mock HTTP responses on localhost.
//! They are gated behind the `http` feature to ensure offline-safety by default.

#[cfg(feature = "http")]
mod http_tests {
    use aiy_adapter_grok::{GrokClient, ReqwestTransport};
    use aiy_core::security::{CredentialBackend, CredentialManager};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper to create a test credential manager with encrypted file backend
    fn setup_test_credentials() -> (Arc<Mutex<CredentialManager>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_creds.enc");

        let manager =
            CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path }).unwrap();

        (Arc::new(Mutex::new(manager)), temp_dir)
    }

    async fn unlock_and_store_key(manager: &Arc<Mutex<CredentialManager>>) {
        let mut mgr = manager.lock().await;
        mgr.unlock("test-password-123").unwrap();
        mgr.store_key("xai", "test-api-key-for-wiremock").unwrap();
    }

    #[tokio::test]
    async fn test_successful_api_call() {
        let mock_server = MockServer::start().await;

        // Set up mock response
        let response_body = r#"{
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-4-1-fast",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Content-Type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        // Create client with custom base URL pointing to mock server
        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let response = client.generate_text("Hello").await.unwrap();
        assert_eq!(response, "Hello! How can I help you?");
    }

    #[tokio::test]
    async fn test_request_headers_are_correct() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
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

        // Verify the Authorization header format (Bearer token)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer test-api-key-for-wiremock"))
            .and(header("Content-Type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let _ = client.generate_text("Test").await.unwrap();
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let mock_server = MockServer::start().await;

        // Set up mock that delays longer than the timeout
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        // Create client with very short timeout
        let client = GrokClient::new_with_http_timeout(creds, Duration::from_millis(100))
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.generate_text("Test").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("timed out") || error_msg.contains("Transport"),
            "Expected timeout error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_retry_on_5xx_errors() {
        let mock_server = MockServer::start().await;

        // Set up mock to return 500 (which should trigger retries)
        // We verify that retry logic executes without crashing and doesn't leak API keys
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
            .expect(3) // Should retry 3 times total
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        // This test verifies retry logic - should fail after all retries exhausted
        let result = client.generate_text("Test").await;

        // Should fail since all responses are 500
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        // Verify error message doesn't contain API key
        assert!(
            !error_msg.contains("test-api-key-for-wiremock"),
            "API key leaked in error message: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_rate_limit_429_handling() {
        let mock_server = MockServer::start().await;

        // Return 429 rate limit error
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(r#"{"error": {"message": "Rate limit exceeded"}}"#),
            )
            .expect(3) // Should retry 3 times
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.generate_text("Test").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("429") || error_msg.contains("Rate limit"),
            "Expected rate limit error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_unauthorized_401_handling() {
        let mock_server = MockServer::start().await;

        // Return 401 unauthorized error (should NOT retry)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_string(r#"{"error": {"message": "Invalid API key"}}"#),
            )
            .expect(1) // Should NOT retry on 401
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.generate_text("Test").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("401"),
            "Expected 401 error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_error_messages_do_not_leak_api_keys() {
        let mock_server = MockServer::start().await;

        // Return an error with the API key in the response
        let error_with_key = r#"{"error": {"message": "Invalid key: test-api-key-for-wiremock"}}"#;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string(error_with_key))
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.generate_text("Test").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        // Use sanitized error string
        let sanitized_error = error.to_sanitized_string();

        // The sanitized error should not contain the API key
        assert!(
            !sanitized_error.contains("test-api-key-for-wiremock"),
            "API key leaked in sanitized error: {}",
            sanitized_error
        );
    }

    #[tokio::test]
    async fn test_malformed_response_handling() {
        let mock_server = MockServer::start().await;

        // Return malformed JSON
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.generate_text("Test").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("parsing") || error_msg.contains("invalid"),
            "Expected parsing error, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_review_artifact_with_mock_server() {
        let mock_server = MockServer::start().await;

        // Set up mock to return a valid AgentReview JSON
        let review_response = r#"{
            "id": "review-test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-4-1-fast",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "{\"agent_id\":\"grok\",\"verdict\":\"pass\",\"confidence\":0.95,\"issues\":[],\"suggestions\":[\"Consider adding tests\"],\"sign_off\":true,\"reasoning\":\"Code looks good\"}"
                },
                "finish_reason": "stop"
            }]
        }"#;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(review_response))
            .mount(&mock_server)
            .await;

        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http(creds)
            .unwrap()
            .with_base_url(mock_server.uri());

        let artifact = r#"fn main() { println!("Hello, world!"); }"#;
        let review = client.review_artifact(artifact).await.unwrap();

        assert_eq!(review.agent_id, "grok");
        assert_eq!(review.verdict, aiy_adapters::Verdict::Pass);
        assert_eq!(review.confidence, 0.95);
        assert!(review.sign_off);
    }

    #[tokio::test]
    async fn test_reqwest_transport_creation() {
        // Test that ReqwestTransport can be created with default settings
        let transport = ReqwestTransport::new();
        assert!(transport.is_ok());
    }

    #[tokio::test]
    async fn test_reqwest_transport_custom_timeout() {
        // Test that ReqwestTransport can be created with custom timeout
        let transport = ReqwestTransport::with_timeout(Duration::from_secs(60));
        assert!(transport.is_ok());
        // Note: timeout() getter not exposed on ReqwestTransport, just verify creation succeeds
        let _t = transport.unwrap();
    }

    #[tokio::test]
    async fn test_connection_error_handling() {
        // Try to connect to a non-existent server
        let (creds, _temp) = setup_test_credentials();
        unlock_and_store_key(&creds).await;

        let client = GrokClient::new_with_http_timeout(creds, Duration::from_millis(500))
            .unwrap()
            .with_base_url("http://127.0.0.1:59999".to_string()); // Non-existent port

        let result = client.generate_text("Test").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        // Should be a transport/connection error
        assert!(
            error_msg.contains("Transport") || error_msg.contains("connection"),
            "Expected connection error, got: {}",
            error_msg
        );
    }
}

// Tests that run without the http feature (basic transport tests)
#[cfg(not(feature = "http"))]
mod offline_tests {
    use aiy_adapter_grok::{GrokClient, MockTransport};
    use aiy_core::security::{CredentialBackend, CredentialManager};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    fn setup_test_credentials() -> (Arc<Mutex<CredentialManager>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_creds.enc");

        let manager =
            CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path }).unwrap();

        (Arc::new(Mutex::new(manager)), temp_dir)
    }

    #[tokio::test]
    async fn test_mock_transport_works_without_http_feature() {
        let (creds, _temp) = setup_test_credentials();
        {
            let mut mgr = creds.lock().await;
            mgr.unlock("test-password").unwrap();
            mgr.store_key("xai", "test-key").unwrap();
        }

        let mock_response = r#"{
            "id": "test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-4-1-fast",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Mock response"},
                "finish_reason": "stop"
            }]
        }"#;

        let transport = Arc::new(MockTransport::with_canned_response(
            mock_response.to_string(),
        ));
        let client = GrokClient::new_with_mock(creds, transport);

        let response = client.generate_text("Hello").await.unwrap();
        assert_eq!(response, "Mock response");
    }
}
