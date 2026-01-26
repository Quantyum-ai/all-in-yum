//! Live integration tests for Claude (Anthropic) API
//!
//! These tests make real API calls and are **skipped by default**.
//! They only run when explicitly enabled via environment variables.
//!
//! # Requirements
//! - `AIY_LIVE_TESTS=1` environment variable must be set
//! - `ANTHROPIC_API_KEY` must contain a valid Anthropic API key
//!
//! # Running
//! ```bash
//! AIY_LIVE_TESTS=1 ANTHROPIC_API_KEY=sk-ant-xxx cargo test -p aiy-adapter-claude --test live_tests -- --ignored
//! ```
//!
//! # Notes
//! - These tests use minimal token counts to reduce API costs
//! - Only happy-path scenarios are tested
//! - Tests are marked with `#[ignore]` so they don't run in normal CI

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Anthropic API messages endpoint
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Required Anthropic API version header
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Request body for Anthropic messages API
#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response from Anthropic messages API
#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)] // Required for serde deserialization
    content_type: String,
    text: Option<String>,
}

/// Check if live tests should run
fn should_run_live_tests() -> bool {
    std::env::var("AIY_LIVE_TESTS").is_ok()
}

/// Get the Anthropic API key from environment
fn get_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY").ok()
}

/// Live integration test for Claude API
///
/// This test sends a minimal request to verify:
/// 1. The API key is valid
/// 2. The API endpoint is reachable
/// 3. A valid response is returned
///
/// # Running
/// ```bash
/// AIY_LIVE_TESTS=1 ANTHROPIC_API_KEY=sk-ant-xxx cargo test -p aiy-adapter-claude --test live_tests -- --ignored
/// ```
#[tokio::test]
#[ignore] // Only run when explicitly enabled
async fn test_live_claude_happy_path() {
    // Check env vars
    if !should_run_live_tests() {
        eprintln!("Skipping live test: AIY_LIVE_TESTS not set");
        return;
    }

    let api_key = match get_api_key() {
        Some(key) => key,
        None => {
            eprintln!("Skipping live test: ANTHROPIC_API_KEY not set");
            return;
        }
    };

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // Build minimal request (few tokens)
    let request = MessagesRequest {
        model: "claude-3-5-haiku-latest".to_string(),
        max_tokens: 10,
        messages: vec![Message {
            role: "user".to_string(),
            content: "Reply with only the word OK".to_string(),
        }],
    };

    // Send request with required headers
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", &api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(&request)
        .send()
        .await
        .expect("Failed to send request to Anthropic API");

    // Check status
    let status = response.status();
    assert!(
        status.is_success(),
        "Anthropic API returned error status: {}",
        status
    );

    // Parse response
    let body: MessagesResponse = response
        .json()
        .await
        .expect("Failed to parse Anthropic API response");

    // Verify we got a response
    assert!(
        !body.content.is_empty(),
        "Anthropic API returned no content blocks"
    );

    let text = body.content[0]
        .text
        .as_ref()
        .expect("Anthropic API returned no text");

    assert!(!text.is_empty(), "Anthropic API returned empty text");

    eprintln!("Live test passed! Response: {}", text);
}
