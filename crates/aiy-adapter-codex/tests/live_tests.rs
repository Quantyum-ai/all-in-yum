//! Live integration tests for Codex (OpenAI) API
//!
//! These tests make real API calls and are **skipped by default**.
//! They only run when explicitly enabled via environment variables.
//!
//! # Requirements
//! - `AIY_LIVE_TESTS=1` environment variable must be set
//! - `OPENAI_API_KEY` must contain a valid OpenAI API key
//!
//! # Running
//! ```bash
//! AIY_LIVE_TESTS=1 OPENAI_API_KEY=sk-xxx cargo test -p aiy-adapter-codex --test live_tests -- --ignored
//! ```
//!
//! # Notes
//! - These tests use minimal token counts to reduce API costs
//! - Only happy-path scenarios are tested
//! - Tests are marked with `#[ignore]` so they don't run in normal CI

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OpenAI API chat completions endpoint
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Request body for OpenAI chat completions
#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response from OpenAI chat completions
#[derive(Deserialize)]
struct ChatCompletionResponse {
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

/// Get the OpenAI API key from environment
fn get_api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY").ok()
}

/// Live integration test for OpenAI API
///
/// This test sends a minimal request to verify:
/// 1. The API key is valid
/// 2. The API endpoint is reachable
/// 3. A valid response is returned
///
/// # Running
/// ```bash
/// AIY_LIVE_TESTS=1 OPENAI_API_KEY=sk-xxx cargo test -p aiy-adapter-codex --test live_tests -- --ignored
/// ```
#[tokio::test]
#[ignore] // Only run when explicitly enabled
async fn test_live_codex_happy_path() {
    // Check env vars
    if !should_run_live_tests() {
        eprintln!("Skipping live test: AIY_LIVE_TESTS not set");
        return;
    }

    let api_key = match get_api_key() {
        Some(key) => key,
        None => {
            eprintln!("Skipping live test: OPENAI_API_KEY not set");
            return;
        }
    };

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // Build minimal request (few tokens)
    let request = ChatCompletionRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Reply with only the word OK".to_string(),
        }],
        max_tokens: 5,
    };

    // Send request
    let response = client
        .post(OPENAI_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .expect("Failed to send request to OpenAI API");

    // Check status
    let status = response.status();
    assert!(
        status.is_success(),
        "OpenAI API returned error status: {}",
        status
    );

    // Parse response
    let body: ChatCompletionResponse = response
        .json()
        .await
        .expect("Failed to parse OpenAI API response");

    // Verify we got a response
    assert!(!body.choices.is_empty(), "OpenAI API returned no choices");

    let content = body.choices[0]
        .message
        .content
        .as_ref()
        .expect("OpenAI API returned no content");

    assert!(!content.is_empty(), "OpenAI API returned empty content");

    eprintln!("Live test passed! Response: {}", content);
}
