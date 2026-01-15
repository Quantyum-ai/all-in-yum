//! Verification stages for the privacy mode verification engine.
//!
//! This module defines the `VerificationStage` trait and provides concrete
//! implementations for fmt, clippy, and test stages.

pub mod build;
pub mod clippy;
pub mod fmt;
pub mod test;

use super::error::VerificationError;
use super::types::{StageFailure, StageResult};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

pub use build::BuildStage;
pub use clippy::ClippyStage;
pub use fmt::FmtStage;
pub use test::TestStage;

/// Default stage order for verification.
///
/// Stages are run in order: fmt -> clippy -> test
/// This ensures formatting is fixed first (fastest), then lints, then tests.
pub fn default_stage_order() -> Vec<Box<dyn VerificationStage>> {
    vec![
        Box::new(FmtStage::new()),
        Box::new(ClippyStage::new()),
        Box::new(TestStage::new()),
    ]
}

/// Create a stage by name.
///
/// Returns `None` for unknown stage names.
pub fn stage_by_name(name: &str) -> Option<Box<dyn VerificationStage>> {
    match name.to_lowercase().as_str() {
        "fmt" | "format" | "rustfmt" => Some(Box::new(FmtStage::new())),
        "clippy" | "lint" => Some(Box::new(ClippyStage::new())),
        "build" | "compile" => Some(Box::new(BuildStage::new())),
        "test" | "tests" => Some(Box::new(TestStage::new())),
        _ => None,
    }
}

/// Trait for verification stages.
///
/// Each stage represents a cargo command (fmt, clippy, test) that can be
/// run as part of the verification pipeline. Stages can parse cargo output
/// to extract structured failure information.
#[async_trait]
pub trait VerificationStage: Send + Sync {
    /// Get the stage name (e.g., "fmt", "clippy", "test").
    fn name(&self) -> &str;

    /// Get the display name for user-facing output.
    fn display_name(&self) -> &str;

    /// Run the verification stage.
    ///
    /// Returns `Ok(StageResult)` with success=true if the stage passed,
    /// or success=false with failures if there were issues.
    ///
    /// Returns `Err(VerificationError)` only for unexpected errors like
    /// cargo not being available or IO errors.
    async fn run(&self, working_dir: &Path) -> Result<StageResult, VerificationError>;

    /// Parse raw output into structured failures.
    ///
    /// This is called by the engine when repair is needed to extract
    /// specific issues that need to be fixed.
    fn parse_failures(&self, output: &str) -> Vec<StageFailure>;

    /// Get the default timeout for this stage.
    fn default_timeout(&self) -> Duration;

    /// Check if this stage supports automatic repair.
    fn supports_repair(&self) -> bool {
        true
    }

    /// Get repair hints for the given failures.
    ///
    /// Returns suggestions that can help guide the repair process.
    fn repair_hints(&self, failures: &[StageFailure]) -> Vec<String> {
        failures
            .iter()
            .filter_map(|f| f.suggested_fix.clone())
            .collect()
    }
}

/// Configuration for a stage execution.
#[derive(Debug, Clone)]
pub struct StageConfig {
    /// Working directory for the stage
    pub working_dir: std::path::PathBuf,
    /// Optional package to run the stage on
    pub package: Option<String>,
    /// Additional arguments to pass to cargo
    pub extra_args: Vec<String>,
    /// Timeout for the stage
    pub timeout: Duration,
    /// Whether to capture stdout/stderr
    pub capture_output: bool,
}

impl StageConfig {
    /// Create a new stage config for a working directory.
    pub fn new(working_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            package: None,
            extra_args: Vec::new(),
            timeout: Duration::from_secs(120),
            capture_output: true,
        }
    }

    /// Set the package to run the stage on.
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// Add extra arguments.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Helper to run a cargo command and capture output.
pub async fn run_cargo_command(
    args: &[&str],
    working_dir: &Path,
    timeout: Duration,
) -> Result<(bool, String, String), VerificationError> {
    use tokio::process::Command;
    use tokio::time::timeout as tokio_timeout;

    let mut cmd = Command::new("cargo");
    cmd.args(args).current_dir(working_dir);

    // Capture both stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Set environment variables for consistent output
    cmd.env("CARGO_TERM_COLOR", "never");
    cmd.env("RUST_BACKTRACE", "1");

    let child = cmd.spawn().map_err(|e| {
        VerificationError::stage_execution(
            args.first().unwrap_or(&"cargo"),
            format!("Failed to spawn cargo: {}", e),
            None,
        )
    })?;

    let output = tokio_timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| {
            VerificationError::timeout(args.first().unwrap_or(&"cargo"), timeout.as_secs())
        })?
        .map_err(|e| {
            VerificationError::stage_execution(
                args.first().unwrap_or(&"cargo"),
                format!("Failed to wait for cargo: {}", e),
                None,
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    Ok((success, stdout, stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stage_order() {
        let stages = default_stage_order();
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0].name(), "fmt");
        assert_eq!(stages[1].name(), "clippy");
        assert_eq!(stages[2].name(), "test");
    }

    #[test]
    fn test_stage_by_name() {
        assert!(stage_by_name("fmt").is_some());
        assert!(stage_by_name("format").is_some());
        assert!(stage_by_name("rustfmt").is_some());
        assert!(stage_by_name("clippy").is_some());
        assert!(stage_by_name("lint").is_some());
        assert!(stage_by_name("test").is_some());
        assert!(stage_by_name("tests").is_some());
        assert!(stage_by_name("unknown").is_none());
    }

    #[test]
    fn test_stage_by_name_case_insensitive() {
        assert!(stage_by_name("FMT").is_some());
        assert!(stage_by_name("Clippy").is_some());
        assert!(stage_by_name("TEST").is_some());
    }

    #[test]
    fn test_stage_config() {
        let config = StageConfig::new("/path/to/project")
            .with_package("my-crate")
            .with_timeout(Duration::from_secs(300))
            .with_extra_args(vec!["--release".to_string()]);

        assert_eq!(
            config.working_dir,
            std::path::PathBuf::from("/path/to/project")
        );
        assert_eq!(config.package, Some("my-crate".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert!(config.extra_args.contains(&"--release".to_string()));
    }

    #[test]
    fn test_stage_display_names() {
        let fmt = FmtStage::new();
        let clippy = ClippyStage::new();
        let test = TestStage::new();

        assert_eq!(fmt.display_name(), "Format (rustfmt)");
        assert_eq!(clippy.display_name(), "Clippy Lints");
        assert_eq!(test.display_name(), "Tests");
    }

    #[test]
    fn test_stage_default_timeouts() {
        let fmt = FmtStage::new();
        let clippy = ClippyStage::new();
        let test = TestStage::new();

        // Fmt should be fastest
        assert!(fmt.default_timeout() < clippy.default_timeout());
        // Test can take longer
        assert!(clippy.default_timeout() <= test.default_timeout());
    }

    #[test]
    fn test_stages_support_repair() {
        let fmt = FmtStage::new();
        let clippy = ClippyStage::new();
        let test = TestStage::new();

        assert!(fmt.supports_repair());
        assert!(clippy.supports_repair());
        assert!(test.supports_repair());
    }
}
