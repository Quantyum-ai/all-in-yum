# PRP: Pre-Human CLI Testing for all-in-yum

## Title
Unified Credential Backend + CLI Smoke Test Suite

## Objective
Establish deterministic, automated CLI smoke tests that validate CLI UX, credential flows, ask/review commands (offline/mock mode), error handling, and security invariants before any human runs live API calls.

## Scope

### In Scope
1. **Code Change**: Unify credential backend selection and unlock behavior across `ask`, `review`, and `credentials` commands via a shared helper module
2. **Integration Tests**: Create CLI integration tests in `crates/aiy-cli/tests/` using `std::process::Command` (via `CARGO_BIN_EXE_aiy`) so they remain runnable in `--offline` / restricted-network environments
3. **Safety Assertions**: Verify no API keys leak to stdout/stderr
4. **Security Invariant**: Validate zero-review-exit-non-zero behavior
5. **Human Live Test Runbook**: Document live API testing workflow

### Non-Goals
- Adding HTTP integration tests requiring network access
- Modifying adapter implementations
- Adding new CLI commands
- Changes to `aiy-core` security module

## Background Analysis

### Current State Summary

**Known CLI blocker to address (must be included in scope)**:

Credential backend selection/unlock is inconsistent across commands:

- **`crates/aiy-cli/src/commands/ask.rs`** (lines 99-127): Hard-codes `EncryptedFile` backend and unlocks via `AIY_CREDENTIALS_PASSWORD` or prompt, but ignores `config.credential_backend`
  ```rust
  // Hard-codes EncryptedFile backend, ignores config.credential_backend
  let credential_manager = CredentialManager::new(CredentialBackend::EncryptedFile {
      path: cred_path.clone(),
  })?;
  // ...
  // DOES support AIY_CREDENTIALS_PASSWORD (lines 202-209)
  if let Ok(password) = std::env::var("AIY_CREDENTIALS_PASSWORD") {
      manager.unlock(&password).map_err(|_| { ... })?;
      return Ok(());
  }
  ```

- **`crates/aiy-cli/src/commands/review.rs`** (lines 235-252): Selects backend from `PipelineConfig` but **does NOT unlock** encrypted-file creds
  ```rust
  fn get_credential_manager(config: &PipelineConfig) -> anyhow::Result<Arc<Mutex<CredentialManager>>> {
      // Reads config.credential_backend (good)
      if config.credential_backend == "system" { ... }
      // Falls back to EncryptedFile (good)
      let manager = CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })?;
      // BUG: NEVER calls unlock() - file backend will fail silently
      Ok(Arc::new(Mutex::new(manager)))
  }
  ```

- **`crates/aiy-cli/src/commands/credentials.rs`** (lines 70-78): Uses config-based backend selection, prompts for password but **does not use `AIY_CREDENTIALS_PASSWORD`** for non-interactive automation
  ```rust
  // Reads config but prompts for password interactively
  // Does NOT check AIY_CREDENTIALS_PASSWORD for non-interactive automation
  if needs_password {
      let password = prompt_password("Enter master password: ")?;
      manager.unlock(&password)?;
  }
  ```

### Root Cause
There is no shared helper for:
1. Selecting backend based on config
2. Unlocking with `AIY_CREDENTIALS_PASSWORD` env var (for automation)
3. Falling back to interactive prompt (for human use)
4. Returning clear error when non-interactive and no password available

---

## Implementation Plan

### Task 1: Create Shared Credential Helper Module
**File**: `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/credential_helper.rs`

Create a new module that provides:

```rust
//! Unified credential backend selection and unlock helper
//!
//! This module provides consistent credential management across all CLI commands.

use aiy_core::security::{CredentialBackend, CredentialManager};
use aiy_core::PipelineConfig;
use std::io::IsTerminal;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;

/// Environment variable for non-interactive credential unlock
pub const CREDENTIALS_PASSWORD_ENV: &str = "AIY_CREDENTIALS_PASSWORD";

/// Get credential file path
pub fn get_credential_path() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(config_dir.join("all-in-yum").join("credentials.enc"))
}

/// Create credential manager based on config preference
/// Returns (manager, needs_unlock) tuple
pub fn create_credential_manager(
    config: &PipelineConfig,
) -> anyhow::Result<(CredentialManager, bool)> {
    // Try system keychain first if preferred
    if config.credential_backend == "system" {
        if let Ok(manager) = CredentialManager::new(CredentialBackend::SystemKeychain) {
            return Ok((manager, false)); // System keychain doesn't need unlock
        }
        // Fall through to encrypted file
    }

    // Use encrypted file backend
    let cred_path = get_credential_path()?;
    let manager = CredentialManager::new(CredentialBackend::EncryptedFile {
        path: cred_path,
    })?;
    Ok((manager, true)) // File backend needs unlock
}

/// Unlock credential manager using env var or interactive prompt
///
/// Returns error if:
/// - Non-interactive mode and no AIY_CREDENTIALS_PASSWORD set
/// - Password is incorrect
pub fn unlock_credential_manager(
    manager: &mut CredentialManager,
    cred_path: &std::path::Path,
) -> anyhow::Result<()> {
    // Check if credentials file exists
    if !cred_path.exists() {
        anyhow::bail!(
            "No credentials configured. Set up your API key with:\n  \
             aiy credentials set <provider>\n\n\
             Providers: xai (Grok), anthropic (Claude), google (Gemini), openai (Codex)"
        );
    }

    // First try environment variable
    if let Ok(password) = std::env::var(CREDENTIALS_PASSWORD_ENV) {
        manager.unlock(&password).map_err(|_| {
            anyhow::anyhow!(
                "Failed to unlock credentials with {}.\n\
                 Check that the password is correct.",
                CREDENTIALS_PASSWORD_ENV
            )
        })?;
        return Ok(());
    }

    // For non-interactive use, require the environment variable
    if std::io::stdin().is_terminal() {
        // Interactive: prompt for password
        let password = rpassword::prompt_password("Enter credentials password: ")?;
        manager
            .unlock(&password)
            .map_err(|_| anyhow::anyhow!("Failed to unlock credentials. Incorrect password?"))?;
    } else {
        // Non-interactive: require AIY_CREDENTIALS_PASSWORD
        anyhow::bail!(
            "Credentials are locked. Set {} environment variable\n\
             or run interactively to enter the password.",
            CREDENTIALS_PASSWORD_ENV
        );
    }

    Ok(())
}

/// Create and unlock credential manager (convenience function)
pub async fn get_unlocked_credential_manager(
    config: &PipelineConfig,
) -> anyhow::Result<Arc<Mutex<CredentialManager>>> {
    let (mut manager, needs_unlock) = create_credential_manager(config)?;

    if needs_unlock {
        let cred_path = get_credential_path()?;
        unlock_credential_manager(&mut manager, &cred_path)?;
    }

    Ok(Arc::new(Mutex::new(manager)))
}
```

**Validation**:
```bash
cargo check -p aiy-cli
```

**Important**: Add the `credential_helper` module declaration in `crates/aiy-cli/src/main.rs` (see Task 5) *before* starting Task 2, otherwise `use crate::credential_helper;` will not compile.

---

### Task 2: Update ask.rs to Use Shared Helper
**File**: `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/commands/ask.rs`

Replace lines 99-111 and the `unlock_credentials` function with:

```rust
use aiy_core::PipelineConfig;
use crate::credential_helper;

// In run() function, replace credential setup:
let config = PipelineConfig::load(&PipelineConfig::config_path()?)
    .unwrap_or_else(|_| PipelineConfig::default());

let credential_manager = credential_helper::get_unlocked_credential_manager(&config).await?;
```

Remove the local `unlock_credentials` function (lines 184-228).

**Validation**:
```bash
cargo check -p aiy-cli
cargo test -p aiy-cli --offline
```

---

### Task 3: Update review.rs to Use Shared Helper
**File**: `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/commands/review.rs`

Replace the `get_credential_manager` function (lines 235-252) with:

```rust
use crate::credential_helper;

/// Create adapters based on configuration and requested agents
async fn create_adapters(
    config: &PipelineConfig,
    requested_agents: &[String],
) -> anyhow::Result<Vec<Box<dyn AgentAdapter>>> {
    let mut adapters: Vec<Box<dyn AgentAdapter>> = Vec::new();

    // Get unlocked credential manager using shared helper
    let credential_manager = credential_helper::get_unlocked_credential_manager(config).await?;

    // ... rest of function unchanged
}
```

Remove the local `get_credential_manager` function.

**Validation**:
```bash
cargo check -p aiy-cli
cargo test -p aiy-cli --offline
```

---

### Task 4: Update credentials.rs to Support Non-Interactive Unlock
**File**: `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/commands/credentials.rs`

Replace the `get_credential_manager` function (lines 13-40) with:

```rust
use crate::credential_helper::{self, CREDENTIALS_PASSWORD_ENV};
use std::io::IsTerminal;

/// Get the credential manager with backend selection and unlock
fn get_credential_manager() -> anyhow::Result<(CredentialManager, bool)> {
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    credential_helper::create_credential_manager(&config)
}

/// Unlock manager using env var or prompt
fn unlock_manager(manager: &mut CredentialManager, needs_password: bool) -> anyhow::Result<()> {
    if !needs_password {
        return Ok(()); // System keychain doesn't need unlock
    }

    // Try environment variable first (enables non-interactive automation)
    if let Ok(password) = std::env::var(CREDENTIALS_PASSWORD_ENV) {
        manager.unlock(&password).map_err(|_| {
            anyhow::anyhow!(
                "Failed to unlock credentials with {}.\n\
                 Check that the password is correct.",
                CREDENTIALS_PASSWORD_ENV
            )
        })?;
        return Ok(());
    }

    // Non-interactive: require env var instead of attempting to prompt
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "Credentials are locked. Set {} environment variable\n\
             or run interactively to enter the password.",
            CREDENTIALS_PASSWORD_ENV
        );
    }

    // Fall back to interactive prompt
    let password = prompt_password("Enter master password: ")?;
    manager.unlock(&password)?;
    Ok(())
}
```

Update each command variant to use `unlock_manager`.

**Validation**:
```bash
cargo check -p aiy-cli
cargo test -p aiy-cli --offline
```

---

### Task 5: Update main.rs Module Declaration
**File**: `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/main.rs`

Add module declaration after line 7:

```rust
pub mod adapters;
mod commands;
pub mod credential_helper;  // Add this line
pub mod registry;
```

**Validation**:
```bash
cargo check -p aiy-cli
```

---

### Task 6: Keep Smoke Tests Offline-Safe (No New Dev Deps)

This repo is validated in restricted-network environments using `--offline`. Adding new crates (e.g., `assert_cmd`, `predicates`) would require downloading new dependencies and updating `Cargo.lock`, which will fail offline.

**Decision**: implement CLI smoke tests using only the standard library (`std::process::Command`) and Cargo’s `CARGO_BIN_EXE_aiy` test env var, plus existing workspace deps (e.g., `tempfile`, `aiy-core`).

**File**: no change required to `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/Cargo.toml`

**Validation**:
```bash
cargo check -p aiy-cli --tests --offline
```

---

### Task 7: Create CLI Smoke Test Suite

**CRITICAL NOTE**: The new CLI smoke tests MUST NOT run under `--features http` to ensure offline safety. The execution team MUST gate these tests using `#[cfg(not(feature = "http"))]` at the module level OR create a separate test target configuration in `Cargo.toml` that explicitly excludes them when the http feature is enabled. This ensures that `cargo test -p aiy-cli --features http --offline` remains safe and does not accidentally invoke real HTTP codepaths.

**File**: `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/tests/cli_smoke_tests.rs`

Add at the top of the file:
```rust
// CRITICAL: These tests must only run with mock transport (default feature)
// They MUST NOT run when --features http is enabled
#![cfg(not(feature = "http"))]
```

Full test suite content:

```rust
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

use std::process::{Command, Output, Stdio};
use std::fs;
use tempfile::TempDir;

/// Dummy API keys for testing - MUST NOT appear in any output
const DUMMY_XAI_KEY: &str = "xai-DUMMY-TEST-KEY-12345678901234567890";
const DUMMY_ANTHROPIC_KEY: &str = "sk-ant-DUMMY-TEST-KEY-123456789012345";
const DUMMY_GOOGLE_KEY: &str = "AIzaSyDUMMY-TEST-KEY-1234567890";
const DUMMY_OPENAI_KEY: &str = "sk-DUMMY-TEST-KEY-1234567890123456789012";

const TEST_PASSWORD: &str = "test-smoke-password-123";

fn aiy_bin() -> std::path::PathBuf {
    std::env::var_os("CARGO_BIN_EXE_aiy")
        .map(std::path::PathBuf::from)
        .expect("CARGO_BIN_EXE_aiy not set; run tests via `cargo test -p aiy-cli`")
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

        Self { temp_dir, config_dir }
    }

    /// Get XDG_CONFIG_HOME value for this test env
    fn xdg_config_home(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Seed credentials file with test keys
    fn seed_credentials(&self) {
        use aiy_core::security::{CredentialBackend, CredentialManager};

        let cred_path = self.config_dir.join("credentials.enc");
        let mut manager = CredentialManager::new(CredentialBackend::EncryptedFile {
            path: cred_path,
        }).expect("Failed to create credential manager");

        manager.unlock(TEST_PASSWORD).expect("Failed to unlock");
        manager.store_key("xai", DUMMY_XAI_KEY).expect("Failed to store xai key");
        manager.store_key("anthropic", DUMMY_ANTHROPIC_KEY).expect("Failed to store anthropic key");
        manager.store_key("google", DUMMY_GOOGLE_KEY).expect("Failed to store google key");
        manager.store_key("openai", DUMMY_OPENAI_KEY).expect("Failed to store openai key");
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
    assert!(combined.contains("Review Results") || combined.contains("Pass") || combined.contains("pass"));
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
    assert!(!combined.contains(DUMMY_XAI_KEY), "XAI key leaked to output!");
    assert!(!combined.contains(DUMMY_ANTHROPIC_KEY), "Anthropic key leaked!");
    assert!(!combined.contains(DUMMY_GOOGLE_KEY), "Google key leaked!");
    assert!(!combined.contains(DUMMY_OPENAI_KEY), "OpenAI key leaked!");

    // Also check for partial key patterns
    assert!(!combined.contains("xai-DUMMY"), "Partial XAI key leaked!");
    assert!(!combined.contains("sk-ant-DUMMY"), "Partial Anthropic key leaked!");
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
    assert!(!combined.contains(DUMMY_XAI_KEY), "XAI key leaked in review!");
    assert!(!combined.contains(DUMMY_ANTHROPIC_KEY), "Anthropic key leaked in review!");
    assert!(!combined.contains(DUMMY_GOOGLE_KEY), "Google key leaked in review!");
    assert!(!combined.contains(DUMMY_OPENAI_KEY), "OpenAI key leaked in review!");
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
    assert!(!stdout.contains(DUMMY_XAI_KEY), "Full key should not appear!");
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
```

**Validation**:
```bash
# Should pass (default = mock transport)
cargo test -p aiy-cli --test cli_smoke_tests --offline

# Should skip all smoke tests (http feature enabled)
cargo test -p aiy-cli --features http --test cli_smoke_tests --offline
```

---

### Task 8: Run Full Validation Matrix
Execute all validation commands in sequence:

```bash
# Format check
cargo fmt --all -- --check

# Clippy (no warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings

# All workspace tests offline
cargo test --workspace --offline

# CLI-specific tests
cargo test -p aiy-cli --offline

# CLI with HTTP feature (compile + non-network tests)
# NOTE: Smoke tests are gated and will NOT run with http feature
cargo test -p aiy-cli --features http --offline

# New smoke tests specifically (mock transport only)
cargo test -p aiy-cli --test cli_smoke_tests --offline
```

---

## Acceptance Criteria

1. **All validation commands pass**:
   - `cargo fmt --all -- --check` - no formatting issues
   - `cargo clippy --workspace --all-targets -- -D warnings` - no warnings
   - `cargo test --workspace --offline` - all tests pass
   - `cargo test -p aiy-cli --offline` - CLI unit tests pass
   - `cargo test -p aiy-cli --features http --offline` - compiles and non-network tests pass (smoke tests properly gated)
   - `cargo test -p aiy-cli --test cli_smoke_tests --offline` - all smoke tests pass (mock transport only)

2. **Credential behavior unified**:
   - `aiy ask` respects `credential_backend` from config
   - `aiy review` properly unlocks encrypted file credentials
   - `aiy credentials` supports `AIY_CREDENTIALS_PASSWORD` for automation
   - All commands fail gracefully in non-interactive mode without password

3. **Security invariants preserved**:
   - Zero reviews trigger exit code 1 (not 0)
   - No API keys appear in stdout or stderr
   - `credentials get` shows redacted output

4. **No new `#[allow(...)]` directives** added to bypass security checks

5. **HTTP feature gating verified**:
   - Smoke tests are properly gated with `#![cfg(not(feature = "http"))]`
   - Running `cargo test -p aiy-cli --features http --offline` skips all smoke tests
   - No risk of smoke tests invoking real HTTP codepaths

---

## Risk Notes

### Secret Handling
- **Risk**: Test credentials could leak into CI logs or test output
- **Mitigation**: Tests use obviously fake keys (`DUMMY-TEST-KEY`) and assert they never appear in output
- **Review Point**: Verify no real-looking API keys in test code

### Offline Constraints
- **Risk**: Mock transport behavior may not match real API behavior
- **Mitigation**: Mock responses use actual API response formats; HTTP tests remain separate and gated
- **Note**: Integration tests only validate CLI UX, not API correctness

### HTTP Feature Gating
- **Risk**: Smoke tests could accidentally run with HTTP transport enabled
- **Mitigation**: Tests are gated with `#![cfg(not(feature = "http"))]` at module level
- **Validation**: Execution team must verify smoke tests do not run with `--features http`

### Regression Risks
- **Risk**: Changing credential flow could break existing workflows
- **Mitigation**:
  - Maintain backward compatibility (env var takes precedence over prompt)
  - Run existing unit tests after each change
  - Smoke tests cover common use cases

### Localhost Binding
- **Risk**: Some CI environments prohibit binding to localhost
- **Mitigation**: Current tests do not require localhost binding; they use mock transport only

---

## Suggested Commit Breakdown

### Commit 1: Add credential_helper module
```
feat(cli): Add unified credential_helper module

- Create crates/aiy-cli/src/credential_helper.rs
- Provides create_credential_manager() with config-based backend selection
- Provides unlock_credential_manager() with AIY_CREDENTIALS_PASSWORD support
- Provides get_unlocked_credential_manager() convenience function
- Add module declaration to main.rs
```

### Commit 2: Update ask.rs to use shared helper
```
refactor(cli): Use credential_helper in ask command

- Replace hard-coded EncryptedFile backend with config-based selection
- Remove duplicate unlock_credentials function
- Maintain AIY_CREDENTIALS_PASSWORD support
```

### Commit 3: Fix review.rs credential unlock
```
fix(cli): Properly unlock credentials in review command

- Use get_unlocked_credential_manager() instead of broken get_credential_manager()
- Remove local get_credential_manager() function
- Fixes: file backend credentials not being unlocked
```

### Commit 4: Update credentials.rs for automation
```
feat(cli): Support AIY_CREDENTIALS_PASSWORD in credentials command

- Use shared unlock_manager() with env var support
- Enable non-interactive credential operations
- Maintain interactive prompt as fallback
```

### Commit 5: Add CLI smoke test suite
```
test(cli): Add comprehensive CLI smoke test suite

- Create tests/cli_smoke_tests.rs with 25+ test cases
- Gate tests with #![cfg(not(feature = "http"))] for offline safety
- Test coverage:
  - Basic CLI commands (version, help, config, agents)
  - Credential flow (missing, locked, unlocked)
  - Mock transport (ask, review with seeded creds)
  - Security (no key leaks, zero-review-exit-nonzero)
  - Error handling (invalid agent, empty prompt, wrong password)
```

### Commit 6: Documentation update
```
docs(cli): Add pre-human testing notes to README

- Document AIY_CREDENTIALS_PASSWORD usage
- Add offline testing instructions
- Include credential backend configuration
```

---

## Human Live Test Runbook

After automated tests pass, follow these steps for live API validation:

### Build for Live API Calls
```bash
# Build with HTTP transport enabled
cargo build -p aiy-cli --release --features http

# Verify binary location
ls -la target/release/aiy
```

### Set Up Credentials

```bash
# Option 1: Use system keychain (recommended for interactive use)
./target/release/aiy config set credential_backend system
./target/release/aiy credentials set xai       # Enter xAI API key when prompted
./target/release/aiy credentials set anthropic # Enter Anthropic API key
./target/release/aiy credentials set google    # Enter Google API key
./target/release/aiy credentials set openai    # Enter OpenAI API key

# Option 2: Use encrypted file backend (for automation)
./target/release/aiy config set credential_backend file
export AIY_CREDENTIALS_PASSWORD="your-master-password"
./target/release/aiy credentials set xai       # Will use env var for unlock
./target/release/aiy credentials set anthropic
./target/release/aiy credentials set google
./target/release/aiy credentials set openai

# Verify credentials are stored
./target/release/aiy credentials status
```

### Example Live Commands

```bash
# Simple ask (interactive, system keychain)
./target/release/aiy ask --agent grok --prompt "What is 2+2?"

# Ask with JSON output
./target/release/aiy ask --agent claude --prompt "Explain Rust ownership" -f json

# Review a file
./target/release/aiy review src/main.rs

# Review with specific agents
./target/release/aiy review src/main.rs --agents grok,claude

# Review with JSON output
./target/release/aiy review src/main.rs -f json

# Non-interactive (CI/automation) with file backend
export AIY_CREDENTIALS_PASSWORD="your-master-password"
./target/release/aiy ask --agent gemini --prompt "Hello"
echo "fn main() {}" | ./target/release/aiy review /dev/stdin
```

### Backend Selection Notes

| Backend | Use Case | Password Handling |
|---------|----------|-------------------|
| `system` | Interactive desktop use | OS handles authentication |
| `file` | CI/automation, headless | Requires `AIY_CREDENTIALS_PASSWORD` or interactive prompt |

```bash
# Check current backend
./target/release/aiy config show | grep credential_backend

# Switch backend
./target/release/aiy config set credential_backend file
./target/release/aiy config set credential_backend system
```

### Troubleshooting

1. **"Credentials are locked"**: Set `AIY_CREDENTIALS_PASSWORD` or run interactively
2. **"No credentials configured"**: Run `aiy credentials set <provider>`
3. **"System keychain unavailable"**: Switch to file backend: `aiy config set credential_backend file`
4. **"HTTP transport not enabled"**: Rebuild with `--features http`

---

## Critical Files for Implementation

- `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/credential_helper.rs` - New module to create (core credential logic)
- `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/commands/ask.rs` - Replace credential handling (lines 99-228)
- `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/commands/review.rs` - Fix missing unlock (lines 235-252)
- `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/commands/credentials.rs` - Add AIY_CREDENTIALS_PASSWORD support (lines 13-58)
- `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/src/main.rs` - Add module declaration (after line 7)
- `/home/aip0rt/Desktop/all-in-yum/crates/aiy-cli/tests/cli_smoke_tests.rs` - New test file (all smoke tests)
