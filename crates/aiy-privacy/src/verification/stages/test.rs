//! Test stage using `cargo test`.
//!
//! This stage runs the test suite and parses output to identify failing tests.

use super::{run_cargo_command, VerificationStage};
use crate::verification::error::VerificationError;
use crate::verification::types::{
    CodeLocation, DiagnosticSeverity, FailureType, StageFailure, StageResult,
};
use async_trait::async_trait;
use regex::Regex;
use std::path::Path;
use std::time::{Duration, Instant};

/// Test verification stage.
///
/// Runs `cargo test` to execute the test suite.
/// Parses output to identify failing tests and extract error information.
pub struct TestStage {
    /// Package to test (None for workspace)
    package: Option<String>,
    /// Run tests in release mode
    release: bool,
    /// Test filter pattern
    filter: Option<String>,
    /// Additional test arguments
    extra_args: Vec<String>,
    /// Whether to run ignored tests
    include_ignored: bool,
}

impl TestStage {
    /// Create a new test stage for the entire workspace.
    pub fn new() -> Self {
        Self {
            package: None,
            release: false,
            filter: None,
            extra_args: Vec::new(),
            include_ignored: false,
        }
    }

    /// Create a test stage for a specific package.
    pub fn for_package(package: impl Into<String>) -> Self {
        Self {
            package: Some(package.into()),
            release: false,
            filter: None,
            extra_args: Vec::new(),
            include_ignored: false,
        }
    }

    /// Run tests in release mode.
    pub fn release(mut self) -> Self {
        self.release = true;
        self
    }

    /// Set a test filter pattern.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Add extra test arguments.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Include ignored tests.
    pub fn include_ignored(mut self) -> Self {
        self.include_ignored = true;
        self
    }

    /// Parse test output to extract failures.
    fn parse_test_output(&self, output: &str) -> Vec<StageFailure> {
        let mut failures = Vec::new();

        // Regex patterns for test failures
        let failure_pattern =
            Regex::new(r"---- ([\w:]+) stdout ----").expect("Invalid regex");
        let thread_panic =
            Regex::new(r"thread '([^']+)' panicked at (.+):(\d+):(\d+)").expect("Invalid regex");
        let assertion_failed =
            Regex::new(r"assertion (?:failed|`.*` failed)").expect("Invalid regex");
        let left_right =
            Regex::new(r"left:\s*`([^`]*)`[,\s]+right:\s*`([^`]*)`").expect("Invalid regex");

        let mut current_test: Option<String> = None;
        let mut current_failure_text = String::new();

        for line in output.lines() {
            // Check for test failure header
            if let Some(caps) = failure_pattern.captures(line) {
                // Save previous test if any
                if let Some(ref test_name) = current_test {
                    if !current_failure_text.is_empty() {
                        failures.push(self.create_test_failure(
                            test_name,
                            &current_failure_text,
                            output,
                        ));
                    }
                }
                current_test = Some(caps[1].to_string());
                current_failure_text.clear();
            } else if current_test.is_some() {
                // Collect failure text
                current_failure_text.push_str(line);
                current_failure_text.push('\n');
            }

            // Check for thread panic (independent of test header)
            if let Some(caps) = thread_panic.captures(line) {
                let test_name = caps[1].to_string();
                let file = caps[2].to_string();
                let line_num: u32 = caps[3].parse().unwrap_or(1);
                let _col: u32 = caps[4].parse().unwrap_or(1);

                let loc = CodeLocation::single_line(&file, line_num);
                let mut failure = StageFailure {
                    failure_type: FailureType::Test,
                    location: Some(loc),
                    message: format!("Test '{}' panicked", test_name),
                    severity: Some(DiagnosticSeverity::Error),
                    code: None,
                    suggested_fix: None,
                    related: vec![],
                };

                // Try to extract assertion details
                if assertion_failed.is_match(&current_failure_text) {
                    if let Some(lr_caps) = left_right.captures(&current_failure_text) {
                        let left = &lr_caps[1];
                        let right = &lr_caps[2];
                        failure.message = format!(
                            "Test '{}' failed: assertion failed\n  left: `{}`\n right: `{}`",
                            test_name, left, right
                        );
                    }
                }

                failures.push(failure);
            }
        }

        // Handle last test
        if let Some(ref test_name) = current_test {
            if !current_failure_text.is_empty() {
                failures.push(self.create_test_failure(
                    test_name,
                    &current_failure_text,
                    output,
                ));
            }
        }

        // If no structured failures found, check for FAILED summary
        if failures.is_empty() && output.contains("FAILED") {
            // Look for "failures:" section
            if let Some(pos) = output.find("failures:") {
                let failures_section = &output[pos..];
                for line in failures_section.lines().skip(1) {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with("test result:") {
                        break;
                    }
                    if !trimmed.is_empty() && !trimmed.starts_with("failures:") {
                        failures.push(StageFailure::test(
                            trimmed.to_string(),
                            "Test failed".to_string(),
                        ));
                    }
                }
            }
        }

        failures
    }

    /// Create a test failure from collected output.
    fn create_test_failure(&self, test_name: &str, failure_text: &str, full_output: &str) -> StageFailure {
        // Try to find panic location
        let panic_pattern =
            Regex::new(r"panicked at (.+):(\d+):(\d+)").expect("Invalid regex");

        let location = if let Some(caps) = panic_pattern.captures(failure_text) {
            let file = caps[1].to_string();
            let line: u32 = caps[2].parse().unwrap_or(1);
            Some(CodeLocation::single_line(file, line))
        } else {
            None
        };

        // Extract meaningful message
        let message = self.extract_failure_message(test_name, failure_text, full_output);

        StageFailure {
            failure_type: FailureType::Test,
            location,
            message,
            severity: Some(DiagnosticSeverity::Error),
            code: None,
            suggested_fix: None,
            related: vec![],
        }
    }

    /// Extract a meaningful failure message from test output.
    fn extract_failure_message(&self, test_name: &str, failure_text: &str, _full_output: &str) -> String {
        let assertion_pattern = Regex::new(
            r"assertion (?:failed|`.*` failed)(?:: ([^\n]+))?"
        ).expect("Invalid regex");

        if let Some(caps) = assertion_pattern.captures(failure_text) {
            if let Some(msg) = caps.get(1) {
                return format!("Test '{}' failed: {}", test_name, msg.as_str());
            }
            return format!("Test '{}' failed: assertion failed", test_name);
        }

        // Check for panic message
        if let Some(pos) = failure_text.find("panicked at") {
            let msg = &failure_text[pos..];
            if let Some(end) = msg.find('\n') {
                return format!("Test '{}': {}", test_name, &msg[..end]);
            }
        }

        format!("Test '{}' failed", test_name)
    }
}

impl Default for TestStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationStage for TestStage {
    fn name(&self) -> &str {
        "test"
    }

    fn display_name(&self) -> &str {
        "Tests"
    }

    async fn run(&self, working_dir: &Path) -> Result<StageResult, VerificationError> {
        let start = Instant::now();

        let mut args = vec!["test"];

        if let Some(ref pkg) = self.package {
            args.push("-p");
            args.push(Box::leak(pkg.clone().into_boxed_str()));
        }

        if self.release {
            args.push("--release");
        }

        if let Some(ref filter) = self.filter {
            args.push(Box::leak(filter.clone().into_boxed_str()));
        }

        if self.include_ignored {
            args.push("--");
            args.push("--include-ignored");
        }

        for arg in &self.extra_args {
            args.push(Box::leak(arg.clone().into_boxed_str()));
        }

        let (success, stdout, stderr) = run_cargo_command(&args, working_dir, self.default_timeout()).await?;

        let duration = start.elapsed();
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Check for compile errors first
        if combined_output.contains("error[E") || combined_output.contains("could not compile") {
            return Err(VerificationError::compile_error(
                "Compilation failed - fix errors before running tests"
            ));
        }

        if success {
            Ok(StageResult::success(self.name(), duration, combined_output))
        } else {
            let failures = self.parse_failures(&combined_output);
            Ok(StageResult::failure(
                self.name(),
                failures,
                0,
                false,
                duration,
                combined_output,
            ))
        }
    }

    fn parse_failures(&self, output: &str) -> Vec<StageFailure> {
        self.parse_test_output(output)
    }

    fn default_timeout(&self) -> Duration {
        Duration::from_secs(300) // 5 minutes for tests
    }

    fn supports_repair(&self) -> bool {
        true
    }

    fn repair_hints(&self, failures: &[StageFailure]) -> Vec<String> {
        let mut hints = Vec::new();

        for failure in failures {
            if let Some(ref loc) = failure.location {
                hints.push(format!(
                    "Check test failure at {}",
                    loc.display()
                ));
            }

            // Extract test name from message
            if failure.message.contains("assertion failed") {
                hints.push("Assertion failure - check expected vs actual values".to_string());
            }
            if failure.message.contains("panicked") {
                hints.push("Test panicked - check for unwrap() on None/Err".to_string());
            }
        }

        if hints.is_empty() {
            hints.push("Run `cargo test` to see detailed failure output".to_string());
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_stage_name() {
        let stage = TestStage::new();
        assert_eq!(stage.name(), "test");
        assert_eq!(stage.display_name(), "Tests");
    }

    #[test]
    fn test_test_stage_for_package() {
        let stage = TestStage::for_package("my-crate");
        assert_eq!(stage.package, Some("my-crate".to_string()));
    }

    #[test]
    fn test_test_stage_supports_repair() {
        let stage = TestStage::new();
        assert!(stage.supports_repair());
    }

    #[test]
    fn test_test_stage_release() {
        let stage = TestStage::new().release();
        assert!(stage.release);
    }

    #[test]
    fn test_test_stage_filter() {
        let stage = TestStage::new().with_filter("my_test");
        assert_eq!(stage.filter, Some("my_test".to_string()));
    }

    #[test]
    fn test_parse_test_output_empty() {
        let stage = TestStage::new();
        let failures = stage.parse_test_output("");
        assert!(failures.is_empty());
    }

    #[test]
    fn test_parse_test_output_success() {
        let stage = TestStage::new();
        let output = r#"
running 3 tests
test test_one ... ok
test test_two ... ok
test test_three ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
"#;
        let failures = stage.parse_test_output(output);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_parse_test_output_failure() {
        let stage = TestStage::new();
        let output = r#"
running 2 tests
test test_one ... ok
test test_failing ... FAILED

failures:

---- test_failing stdout ----
thread 'test_failing' panicked at src/lib.rs:10:5:
assertion `left == right` failed
  left: `1`
 right: `2`

failures:
    test_failing

test result: FAILED. 1 passed; 1 failed; 0 ignored
"#;
        let failures = stage.parse_test_output(output);
        assert!(!failures.is_empty());
    }

    #[test]
    fn test_parse_test_output_panic_location() {
        let stage = TestStage::new();
        let output = r#"
---- test_foo stdout ----
thread 'test_foo' panicked at src/main.rs:42:9:
explicit panic
"#;
        let failures = stage.parse_test_output(output);
        assert!(!failures.is_empty());
        if let Some(ref loc) = failures[0].location {
            assert_eq!(loc.file.to_string_lossy(), "src/main.rs");
            assert_eq!(loc.line_start, 42);
        }
    }

    #[test]
    fn test_parse_test_output_failures_section() {
        let stage = TestStage::new();
        let output = r#"
test result: FAILED. 0 passed; 2 failed;

failures:
    module::test_one
    module::test_two

"#;
        let failures = stage.parse_test_output(output);
        assert_eq!(failures.len(), 2);
    }

    #[test]
    fn test_repair_hints() {
        let stage = TestStage::new();
        let loc = CodeLocation::single_line("src/lib.rs", 10);
        let failures = vec![StageFailure {
            failure_type: FailureType::Test,
            location: Some(loc),
            message: "assertion failed".to_string(),
            severity: Some(DiagnosticSeverity::Error),
            code: None,
            suggested_fix: None,
            related: vec![],
        }];

        let hints = stage.repair_hints(&failures);
        assert!(!hints.is_empty());
        assert!(hints.iter().any(|h| h.contains("Check test failure")));
        assert!(hints.iter().any(|h| h.contains("expected vs actual")));
    }

    #[test]
    fn test_repair_hints_panic() {
        let stage = TestStage::new();
        let failures = vec![StageFailure::test(
            "test_foo".to_string(),
            "panicked at unwrap".to_string(),
        )];

        let hints = stage.repair_hints(&failures);
        assert!(hints.iter().any(|h| h.contains("panicked")));
    }

    #[test]
    fn test_default_timeout() {
        let stage = TestStage::new();
        assert_eq!(stage.default_timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_include_ignored() {
        let stage = TestStage::new().include_ignored();
        assert!(stage.include_ignored);
    }
}
