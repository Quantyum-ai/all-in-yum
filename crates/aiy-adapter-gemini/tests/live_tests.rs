//! Live integration tests for Gemini (Google) API
//!
//! These tests make real API calls and are **skipped by default**.
//! They only run when explicitly enabled via environment variables.
//!
//! # Requirements
//! - `AIY_LIVE_TESTS=1` environment variable must be set
//! - `GOOGLE_API_KEY` must contain a valid Google AI API key
//!
//! # Running
//! ```bash
//! AIY_LIVE_TESTS=1 GOOGLE_API_KEY=xxx cargo test -p aiy-adapter-gemini --features http --test live_tests -- --ignored
//! ```
//!
//! # Notes
//! - These tests use minimal token counts to reduce API costs
//! - Only happy-path scenarios are tested
//! - Tests are marked with `#[ignore]` so they don't run in normal CI
//! - SECURITY: The API key is passed as a query parameter (Google's design),
//!   so URLs should never be logged

#![cfg(feature = "http")]

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Google Generative AI API base URL (key is appended as query param)
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Default model for testing
const DEFAULT_MODEL: &str = "gemini-2.0-flash-lite";

/// Request body for Gemini generateContent API
#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

/// Response from Gemini generateContent API
#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

/// Check if live tests should run
fn should_run_live_tests() -> bool {
    std::env::var("AIY_LIVE_TESTS").is_ok()
}

/// Get the Google API key from environment
fn get_api_key() -> Option<String> {
    std::env::var("GOOGLE_API_KEY").ok()
}

/// Build the API endpoint URL with the API key as a query parameter
/// SECURITY: This URL should never be logged!
fn build_endpoint_url(api_key: &str) -> String {
    format!(
        "{}/{}:generateContent?key={}",
        GEMINI_API_BASE, DEFAULT_MODEL, api_key
    )
}

/// Live integration test for Gemini API
///
/// This test sends a minimal request to verify:
/// 1. The API key is valid
/// 2. The API endpoint is reachable
/// 3. A valid response is returned
///
/// # Running
/// ```bash
/// AIY_LIVE_TESTS=1 GOOGLE_API_KEY=xxx cargo test -p aiy-adapter-gemini --test live_tests -- --ignored
/// ```
#[tokio::test]
#[ignore] // Only run when explicitly enabled
async fn test_live_gemini_happy_path() {
    // Check env vars
    if !should_run_live_tests() {
        eprintln!("Skipping live test: AIY_LIVE_TESTS not set");
        return;
    }

    let api_key = match get_api_key() {
        Some(key) => key,
        None => {
            eprintln!("Skipping live test: GOOGLE_API_KEY not set");
            return;
        }
    };

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // Build minimal request (few tokens)
    let request = GenerateContentRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: "Reply with only the word OK".to_string(),
            }],
        }],
        generation_config: GenerationConfig {
            max_output_tokens: 10,
        },
    };

    // Build URL with API key (SECURITY: don't log this!)
    let url = build_endpoint_url(&api_key);

    // Send request
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request to Gemini API");

    // Check status
    let status = response.status();
    assert!(
        status.is_success(),
        "Gemini API returned error status: {}",
        status
    );

    // Parse response
    let body: GenerateContentResponse = response
        .json()
        .await
        .expect("Failed to parse Gemini API response");

    // Verify we got a response
    let candidates = body.candidates.expect("Gemini API returned no candidates");

    assert!(
        !candidates.is_empty(),
        "Gemini API returned empty candidates"
    );

    let text = candidates[0].content.parts[0]
        .text
        .as_ref()
        .expect("Gemini API returned no text");

    assert!(!text.is_empty(), "Gemini API returned empty text");

    eprintln!("Live test passed! Response: {}", text);
}
