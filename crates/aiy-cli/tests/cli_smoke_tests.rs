//! CLI Smoke Tests for aiy binary
//!
//! These tests validate CLI behavior without requiring network access.
//! They use mock transport (default feature) and seeded credentials.
//!
//! ## Security Invariants Tested
//! - API keys never appear in stdout/stderr
//! - Zero reviews must exit non-zero
//! - Missing credentials produce helpful errors

// CRITICAL: These tests must only run with mock transport (default feature)
// They MUST NOT run when --features http is enabled
#![cfg(not(feature = "http"))]

use std::fs;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

/// Dummy API keys for testing - MUST NOT appear in any output
const DUMMY_XAI_KEY: &str = "xai-DUMMY-TEST-KEY-12345678901234567890";
const DUMMY_ANTHROPIC_KEY: &str = "sk-ant-DUMMY-TEST-KEY-123456789012345";
const DUMMY_GOOGLE_KEY: &str = "AIzaSyDUMMY-TEST-KEY-1234567890";
const DUMMY_OPENAI_KEY: &str = "sk-DUMMY-TEST-KEY-1234567890123456789012";

const TEST_PASSWORD: &str = "test-smoke-password-123";

fn aiy_bin() -> std::path::PathBuf {
    // First try CARGO_BIN_EXE_aiy (set by cargo when running as part of package tests)
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_aiy") {
        return std::path::PathBuf::from(bin);
    }

    // Fallback: find binary in target directory relative to manifest
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    // Go up to workspace root (crates/aiy-cli -> workspace root)
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent() // crates
        .and_then(|p| p.parent()) // workspace root
        .expect("Failed to find workspace root");

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    let bin_path = workspace_root.join("target").join(profile).join("aiy");
    if bin_path.exists() {
        return bin_path;
    }

    panic!(
        "aiy binary not found. Run `cargo build -p aiy-cli` first.\n\
         Expected at: {}",
        bin_path.display()
    );
}

fn stdout_str(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_str(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn combined_str(output: &Output) -> String {
    format!("{}{}", stdout_str(output), stderr_str(output))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout_str(output),
        stderr_str(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout_str(output),
        stderr_str(output)
    );
}

/// Create a test environment with isolated config and credentials
struct TestEnv {
    temp_dir: TempDir,
    config_dir: std::path::PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_dir = temp_dir.path().join("all-in-yum");
        fs::create_dir_all(&config_dir).expect("Failed to create config dir");

        Self {
            temp_dir,
            config_dir,
        }
    }

    /// Get XDG_CONFIG_HOME value for this test env
    fn xdg_config_home(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Seed credentials file with test keys
    fn seed_credentials(&self) {
        use aiy_core::security::{CredentialBackend, CredentialManager};

        let cred_path = self.config_dir.join("credentials.enc");
        let mut manager =
            CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })
                .expect("Failed to create credential manager");

        manager.unlock(TEST_PASSWORD).expect("Failed to unlock");
        manager
            .store_key("xai", DUMMY_XAI_KEY)
            .expect("Failed to store xai key");
        manager
            .store_key("anthropic", DUMMY_ANTHROPIC_KEY)
            .expect("Failed to store anthropic key");
        manager
            .store_key("google", DUMMY_GOOGLE_KEY)
            .expect("Failed to store google key");
        manager
            .store_key("openai", DUMMY_OPENAI_KEY)
            .expect("Failed to store openai key");
    }

    /// Create config file with file backend
    fn create_config_file_backend(&self) {
        let config_content = r#"
enabled_agents = ["grok", "claude", "gemini", "codex"]
credential_backend = "file"

[default_models]
claude = "claude-sonnet-4-20250514"
grok = "grok-4-1-fast"
gemini = "gemini-1.5-pro-latest"
codex = "gpt-4o"

[base_urls]
claude = "https://api.anthropic.com"
grok = "https://api.x.ai"
gemini = "https://generativelanguage.googleapis.com"

[timeouts]
claude = 120000
grok = 120000
gemini = 120000
"#;
        let config_path = self.config_dir.join("config.toml");
        fs::write(&config_path, config_content).expect("Failed to write config");
    }

    /// Create a Command with test environment
    fn cmd(&self) -> Command {
        let mut cmd = Command::new(aiy_bin());
        cmd.env("XDG_CONFIG_HOME", self.xdg_config_home());
        cmd.env("HOME", self.temp_dir.path()); // Linux fallback
                                               // Always force non-interactive so tests never hang on prompts.
        cmd.stdin(Stdio::null());
        cmd
    }

    /// Create a Command with credentials password set
    fn cmd_with_password(&self) -> Command {
        let mut cmd = self.cmd();
        cmd.env("AIY_CREDENTIALS_PASSWORD", TEST_PASSWORD);
        cmd
    }
}

// =============================================================================
// Basic CLI Tests (no credentials needed)
// =============================================================================

#[test]
fn test_version_command() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.arg("version");
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    assert!(stdout_str(&output).contains("aiy"));
}

#[test]
fn test_help_command() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.arg("--help");
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let stdout = stdout_str(&output);
    assert!(stdout.contains("Usage"));
    assert!(stdout.contains("ask"));
    assert!(stdout.contains("review"));
    assert!(stdout.contains("agents"));
    assert!(stdout.contains("config"));
    assert!(stdout.contains("credentials"));
}

#[test]
fn test_config_reset() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.args(["config", "reset"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    assert!(stdout_str(&output).contains("reset to defaults"));
}

#[test]
fn test_config_show_default() {
    let env = TestEnv::new();
    // First reset to ensure config exists
    let mut reset = env.cmd();
    reset.args(["config", "reset"]);
    assert_success(&reset.output().expect("Failed to execute aiy"));

    let mut cmd = env.cmd();
    cmd.args(["config", "show"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let stdout = stdout_str(&output);
    assert!(stdout.contains("enabled_agents"));
    assert!(stdout.contains("credential_backend"));
}

#[test]
fn test_config_set_credential_backend_file() {
    let env = TestEnv::new();
    let mut reset = env.cmd();
    reset.args(["config", "reset"]);
    assert_success(&reset.output().expect("Failed to execute aiy"));

    let mut cmd = env.cmd();
    cmd.args(["config", "set", "credential_backend", "file"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let stdout = stdout_str(&output);
    assert!(stdout.contains("credential_backend"));
    assert!(stdout.contains("file"));
}

#[test]
fn test_config_set_invalid_backend() {
    let env = TestEnv::new();
    let mut reset = env.cmd();
    reset.args(["config", "reset"]);
    assert_success(&reset.output().expect("Failed to execute aiy"));

    let mut cmd = env.cmd();
    cmd.args(["config", "set", "credential_backend", "invalid"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    assert!(stderr_str(&output).contains("Invalid credential_backend"));
}

#[test]
fn test_agents_list_no_crash() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.args(["agents", "list"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let stdout = stdout_str(&output);
    assert!(stdout.contains("grok"));
    assert!(stdout.contains("claude"));
    assert!(stdout.contains("gemini"));
    assert!(stdout.contains("codex"));
}

#[test]
fn test_agents_status_no_crash() {
    let env = TestEnv::new();
    // Note: May show "missing credentials" but should not crash
    let mut cmd = env.cmd();
    cmd.args(["agents", "status"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
}

#[test]
fn test_agents_enable_disable() {
    let env = TestEnv::new();
    let mut reset = env.cmd();
    reset.args(["config", "reset"]);
    assert_success(&reset.output().expect("Failed to execute aiy"));

    // Enable grok
    let mut enable = env.cmd();
    enable.args(["agents", "enable", "grok"]);
    assert_success(&enable.output().expect("Failed to execute aiy"));

    // Disable grok
    let mut disable = env.cmd();
    disable.args(["agents", "disable", "grok"]);
    assert_success(&disable.output().expect("Failed to execute aiy"));
}

#[test]
fn test_agents_enable_invalid_agent() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.args(["agents", "enable", "invalid-agent"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    assert!(stderr_str(&output).contains("Unknown agent"));
}

// =============================================================================
// Credential Flow Tests
// =============================================================================

#[test]
fn test_ask_missing_credentials_helpful_error() {
    let env = TestEnv::new();
    env.create_config_file_backend();

    // No credentials file exists, should get helpful error
    let mut cmd = env.cmd();
    cmd.args(["ask", "--agent", "grok", "--prompt", "test"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    let stderr = stderr_str(&output);
    assert!(stderr.contains("credentials") || stderr.contains("No credentials"));
}

#[test]
fn test_ask_locked_credentials_non_interactive() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    // Credentials exist but no password provided in non-interactive mode
    let mut cmd = env.cmd();
    cmd.args(["ask", "--agent", "grok", "--prompt", "test"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    let stderr = stderr_str(&output);
    assert!(stderr.contains("AIY_CREDENTIALS_PASSWORD") || stderr.contains("locked"));
}

#[test]
fn test_review_missing_file() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.args(["review", "/nonexistent/file.rs"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    let stderr = stderr_str(&output);
    assert!(stderr.contains("not found") || stderr.contains("No such file"));
}

// =============================================================================
// Mock Transport Tests (offline, seeded credentials)
// =============================================================================

#[test]
fn test_ask_mock_transport_success() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd_with_password();
    cmd.args(["ask", "--agent", "grok", "--prompt", "ping"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    assert!(!stdout_str(&output).trim().is_empty());
}

#[test]
fn test_ask_json_format() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd_with_password();
    cmd.args(["ask", "--agent", "grok", "--prompt", "test", "-f", "json"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let stdout = stdout_str(&output);
    assert!(stdout.contains("\"agent_id\""));
    assert!(stdout.contains("\"text\""));
}

#[test]
fn test_review_mock_transport_success() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    // Create a test file to review
    let test_file = env.temp_dir.path().join("test_code.rs");
    fs::write(&test_file, "fn main() { println!(\"Hello\"); }").expect("Failed to write test file");

    let mut cmd = env.cmd_with_password();
    cmd.args(["review", test_file.to_str().unwrap()]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let combined = combined_str(&output);
    assert!(
        combined.contains("Review Results")
            || combined.contains("Pass")
            || combined.contains("pass")
    );
}

#[test]
fn test_review_json_format() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let test_file = env.temp_dir.path().join("test_code.rs");
    fs::write(&test_file, "fn main() {}").expect("Failed to write test file");

    let mut cmd = env.cmd_with_password();
    cmd.args(["review", test_file.to_str().unwrap(), "-f", "json"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
    let stdout = stdout_str(&output);
    assert!(stdout.contains("\"agent_id\""));
    assert!(stdout.contains("\"verdict\""));
}

#[test]
fn test_review_specific_agent() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let test_file = env.temp_dir.path().join("test_code.rs");
    fs::write(&test_file, "fn main() {}").expect("Failed to write test file");

    let mut cmd = env.cmd_with_password();
    cmd.args(["review", test_file.to_str().unwrap(), "--agents", "grok"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_success(&output);
}

// =============================================================================
// Security Tests - API Key Leak Prevention
// =============================================================================

#[test]
fn test_ask_output_does_not_contain_api_keys() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd_with_password();
    cmd.args(["ask", "--agent", "grok", "--prompt", "test"]);
    let output = cmd.output().expect("Failed to execute aiy");
    let combined = combined_str(&output);

    // CRITICAL: Dummy keys must NEVER appear in output
    assert!(
        !combined.contains(DUMMY_XAI_KEY),
        "XAI key leaked to output!"
    );
    assert!(
        !combined.contains(DUMMY_ANTHROPIC_KEY),
        "Anthropic key leaked!"
    );
    assert!(!combined.contains(DUMMY_GOOGLE_KEY), "Google key leaked!");
    assert!(!combined.contains(DUMMY_OPENAI_KEY), "OpenAI key leaked!");

    // Also check for partial key patterns
    assert!(!combined.contains("xai-DUMMY"), "Partial XAI key leaked!");
    assert!(
        !combined.contains("sk-ant-DUMMY"),
        "Partial Anthropic key leaked!"
    );
}

#[test]
fn test_review_output_does_not_contain_api_keys() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let test_file = env.temp_dir.path().join("test_code.rs");
    fs::write(&test_file, "fn main() {}").expect("Failed to write test file");

    let mut cmd = env.cmd_with_password();
    cmd.args(["review", test_file.to_str().unwrap()]);
    let output = cmd.output().expect("Failed to execute aiy");
    let combined = combined_str(&output);

    // CRITICAL: Dummy keys must NEVER appear in output
    assert!(
        !combined.contains(DUMMY_XAI_KEY),
        "XAI key leaked in review!"
    );
    assert!(
        !combined.contains(DUMMY_ANTHROPIC_KEY),
        "Anthropic key leaked in review!"
    );
    assert!(
        !combined.contains(DUMMY_GOOGLE_KEY),
        "Google key leaked in review!"
    );
    assert!(
        !combined.contains(DUMMY_OPENAI_KEY),
        "OpenAI key leaked in review!"
    );
}

#[test]
fn test_credentials_get_redacts_key() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd_with_password();
    cmd.args(["credentials", "get", "xai"]);
    let output = cmd.output().expect("Failed to execute aiy");

    assert_success(&output);
    let stdout = stdout_str(&output);

    // Should show redacted key, not full key
    assert!(
        !stdout.contains(DUMMY_XAI_KEY),
        "Full key should not appear!"
    );
    // Redaction indicator (first 4 + last 4) uses "..."
    assert!(stdout.contains("...") || stdout.contains("[REDACTED]"));
}

// =============================================================================
// Security Invariant: Zero Reviews Must Exit Non-Zero
// =============================================================================

#[test]
fn test_review_no_agents_enabled_exits_nonzero() {
    let env = TestEnv::new();

    // Create config with no enabled agents
    let config_content = r#"
enabled_agents = []
credential_backend = "file"
[default_models]
[base_urls]
[timeouts]
"#;
    let config_path = env.config_dir.join("config.toml");
    fs::write(&config_path, config_content).expect("Failed to write config");

    env.seed_credentials();

    let test_file = env.temp_dir.path().join("test_code.rs");
    fs::write(&test_file, "fn main() {}").expect("Failed to write test file");

    // Should fail because no agents are enabled
    let mut cmd = env.cmd_with_password();
    cmd.args(["review", test_file.to_str().unwrap()]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_ask_empty_prompt() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd_with_password();
    cmd.args(["ask", "--agent", "grok", "--prompt", ""]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    let stderr = stderr_str(&output);
    assert!(stderr.contains("empty") || stderr.contains("Prompt"));
}

#[test]
fn test_ask_unknown_agent() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd_with_password();
    cmd.args(["ask", "--agent", "unknown-agent", "--prompt", "test"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    assert!(stderr_str(&output).contains("Unknown agent"));
}

#[test]
fn test_ask_missing_agent_flag() {
    let env = TestEnv::new();
    let mut cmd = env.cmd();
    cmd.args(["ask", "--prompt", "test"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
}

#[test]
fn test_wrong_password_fails() {
    let env = TestEnv::new();
    env.create_config_file_backend();
    env.seed_credentials();

    let mut cmd = env.cmd();
    cmd.env("AIY_CREDENTIALS_PASSWORD", "wrong-password");
    cmd.args(["ask", "--agent", "grok", "--prompt", "test"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert_failure(&output);
    let stderr = stderr_str(&output);
    assert!(stderr.contains("unlock") || stderr.contains("password"));
}
