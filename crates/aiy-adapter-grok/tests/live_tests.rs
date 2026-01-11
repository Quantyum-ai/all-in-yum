//! Live integration tests for Grok (xAI) API
//!
//! These tests make real API calls and are **skipped by default**.
//! They only run when explicitly enabled via environment variables.
//!
//! # Requirements
//! - `AIY_LIVE_TESTS=1` environment variable must be set
//! - `XAI_API_KEY` must contain a valid xAI API key
//!
//! # Running
//! ```bash
//! AIY_LIVE_TESTS=1 XAI_API_KEY=xai-xxx cargo test -p aiy-adapter-grok --test live_tests -- --ignored
//! ```
//!
//! # Notes
//! - These tests use minimal token counts to reduce API costs
//! - Only happy-path scenarios are tested
//! - Tests are marked with `#[ignore]` so they don't run in normal CI

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// xAI API base URL
const XAI_API_URL: &str = "https://api.x.ai/v1/chat/completions";

/// Request body for xAI chat completions
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response from xAI chat completions
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

/// Check if live tests should run
fn should_run_live_tests() -> bool {
    std::env::var("AIY_LIVE_TESTS").is_ok()
}

/// Get the xAI API key from environment
fn get_api_key() -> Option<String> {
    std::env::var("XAI_API_KEY").ok()
}

/// Live integration test for Grok API
///
/// This test sends a minimal request to verify:
/// 1. The API key is valid
/// 2. The API endpoint is reachable
/// 3. A valid response is returned
///
/// # Running
/// ```bash
/// AIY_LIVE_TESTS=1 XAI_API_KEY=xai-xxx cargo test -p aiy-adapter-grok --test live_tests -- --ignored
/// ```
#[tokio::test]
#[ignore] // Only run when explicitly enabled
async fn test_live_grok_happy_path() {
    // Check env vars
    if !should_run_live_tests() {
        eprintln!("Skipping live test: AIY_LIVE_TESTS not set");
        return;
    }

    let api_key = match get_api_key() {
        Some(key) => key,
        None => {
            eprintln!("Skipping live test: XAI_API_KEY not set");
            return;
        }
    };

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // Build minimal request (few tokens)
    let request = ChatRequest {
        model: "grok-3-mini-fast".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Reply with only the word OK".to_string(),
        }],
        max_tokens: 5,
    };

    // Send request
    let response = client
        .post(XAI_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .expect("Failed to send request to xAI API");

    // Check status
    let status = response.status();
    assert!(
        status.is_success(),
        "xAI API returned error status: {}",
        status
    );

    // Parse response
    let body: ChatResponse = response
        .json()
        .await
        .expect("Failed to parse xAI API response");

    // Verify we got a response
    assert!(!body.choices.is_empty(), "xAI API returned no choices");

    let content = body.choices[0]
        .message
        .content
        .as_ref()
        .expect("xAI API returned no content");

    assert!(!content.is_empty(), "xAI API returned empty content");

    eprintln!("Live test passed! Response: {}", content);
}
