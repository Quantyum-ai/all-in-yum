//! Formatting stage using `cargo fmt --check`.
//!
//! This stage runs rustfmt to check code formatting and parses the diff output
//! to identify specific formatting issues.

use super::{run_cargo_command, VerificationStage};
use crate::verification::error::VerificationError;
use crate::verification::types::{CodeLocation, StageFailure, StageResult};
use async_trait::async_trait;
use std::path::Path;
use std::time::{Duration, Instant};

/// Formatting verification stage.
///
/// Runs `cargo fmt --check` to detect formatting issues without modifying files.
/// When issues are found, parses the diff output to identify specific files and
/// lines that need formatting.
pub struct FmtStage {
    /// Package to check (None for workspace)
    package: Option<String>,
    /// Additional rustfmt arguments
    extra_args: Vec<String>,
}

impl FmtStage {
    /// Create a new fmt stage for the entire workspace.
    pub fn new() -> Self {
        Self {
            package: None,
            extra_args: Vec::new(),
        }
    }

    /// Create a fmt stage for a specific package.
    pub fn for_package(package: impl Into<String>) -> Self {
        Self {
            package: Some(package.into()),
            extra_args: Vec::new(),
        }
    }

    /// Add extra arguments to rustfmt.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Build the cargo fmt command arguments.
    fn build_args(&self) -> Vec<&str> {
        let mut args = vec!["fmt", "--check"];

        if let Some(ref pkg) = self.package {
            args.push("-p");
            // We need to leak this string to get a static lifetime
            // In practice, this is fine as the stage lives for the duration
            args.push(Box::leak(pkg.clone().into_boxed_str()));
        }

        args.push("--");

        for arg in &self.extra_args {
            args.push(Box::leak(arg.clone().into_boxed_str()));
        }

        args
    }

    /// Parse a unified diff output to extract file and line information.
    fn parse_diff(&self, diff: &str) -> Vec<StageFailure> {
        let mut failures = Vec::new();
        let mut current_file: Option<String> = None;
        let mut diff_lines: Vec<String> = Vec::new();
        let mut start_line: u32 = 1;

        for line in diff.lines() {
            // New file in diff: "Diff in /path/to/file.rs at line X:"
            if line.starts_with("Diff in ") {
                // Save previous file's failure if any
                if let Some(ref file) = current_file {
                    if !diff_lines.is_empty() {
                        let loc = CodeLocation::single_line(file.clone(), start_line);
                        let message = format!(
                            "Formatting differs from rustfmt:\n{}",
                            diff_lines.join("\n")
                        );
                        failures.push(StageFailure::fmt(loc, message));
                        diff_lines.clear();
                    }
                }

                // Parse new file path and line
                if let Some(path_start) = line.find("Diff in ") {
                    let rest = &line[path_start + 8..];
                    if let Some(at_pos) = rest.find(" at line ") {
                        let file_path = &rest[..at_pos];
                        current_file = Some(file_path.to_string());

                        // Try to parse line number
                        let line_str = &rest[at_pos + 9..];
                        if let Some(colon_pos) = line_str.find(':') {
                            if let Ok(line_num) = line_str[..colon_pos].parse::<u32>() {
                                start_line = line_num;
                            }
                        }
                    } else {
                        current_file = Some(rest.trim_end_matches(':').to_string());
                        start_line = 1;
                    }
                }
            } else if line.starts_with('+') || line.starts_with('-') || line.starts_with(' ') {
                diff_lines.push(line.to_string());
            }
        }

        // Handle last file
        if let Some(ref file) = current_file {
            if !diff_lines.is_empty() {
                let loc = CodeLocation::single_line(file.clone(), start_line);
                let message = format!(
                    "Formatting differs from rustfmt:\n{}",
                    diff_lines.join("\n")
                );
                failures.push(StageFailure::fmt(loc, message));
            }
        }

        // If no structured diff found, create a generic failure
        if failures.is_empty() && !diff.trim().is_empty() {
            // Try to extract file paths from the output
            for line in diff.lines() {
                if line.contains(".rs") {
                    // Extract file path
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.ends_with(".rs") || part.contains(".rs:") {
                            let file = part.trim_end_matches(':').trim_end_matches(',');
                            let loc = CodeLocation::single_line(file, 1);
                            failures.push(StageFailure::fmt(
                                loc,
                                "File needs formatting".to_string(),
                            ));
                            break;
                        }
                    }
                }
            }
        }

        failures
    }
}

impl Default for FmtStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationStage for FmtStage {
    fn name(&self) -> &str {
        "fmt"
    }

    fn display_name(&self) -> &str {
        "Format (rustfmt)"
    }

    async fn run(&self, working_dir: &Path) -> Result<StageResult, VerificationError> {
        let start = Instant::now();
        let args: Vec<&str> = vec!["fmt", "--check"];

        let (success, stdout, stderr) = run_cargo_command(&args, working_dir, self.default_timeout()).await?;

        let duration = start.elapsed();
        let combined_output = format!("{}\n{}", stdout, stderr);

        if success {
            Ok(StageResult::success(self.name(), duration, combined_output))
        } else {
            let failures = self.parse_failures(&combined_output);
            Ok(StageResult::failure(
                self.name(),
                failures,
                0, // No repairs attempted yet
                false,
                duration,
                combined_output,
            ))
        }
    }

    fn parse_failures(&self, output: &str) -> Vec<StageFailure> {
        self.parse_diff(output)
    }

    fn default_timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn supports_repair(&self) -> bool {
        true
    }

    fn repair_hints(&self, failures: &[StageFailure]) -> Vec<String> {
        let mut hints = Vec::new();

        for failure in failures {
            if let Some(ref loc) = failure.location {
                hints.push(format!(
                    "Run `cargo fmt` to fix formatting in {}",
                    loc.file.display()
                ));
            }
        }

        if hints.is_empty() {
            hints.push("Run `cargo fmt` to fix all formatting issues".to_string());
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_stage_name() {
        let stage = FmtStage::new();
        assert_eq!(stage.name(), "fmt");
        assert_eq!(stage.display_name(), "Format (rustfmt)");
    }

    #[test]
    fn test_fmt_stage_for_package() {
        let stage = FmtStage::for_package("my-crate");
        assert_eq!(stage.package, Some("my-crate".to_string()));
    }

    #[test]
    fn test_fmt_stage_supports_repair() {
        let stage = FmtStage::new();
        assert!(stage.supports_repair());
    }

    #[test]
    fn test_parse_diff_empty() {
        let stage = FmtStage::new();
        let failures = stage.parse_diff("");
        assert!(failures.is_empty());
    }

    #[test]
    fn test_parse_diff_with_file() {
        let stage = FmtStage::new();
        let diff = r#"Diff in src/main.rs at line 10:
-    let x=1;
+    let x = 1;
"#;
        let failures = stage.parse_diff(diff);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].location.is_some());
        let loc = failures[0].location.as_ref().unwrap();
        assert_eq!(loc.file.to_string_lossy(), "src/main.rs");
        assert_eq!(loc.line_start, 10);
    }

    #[test]
    fn test_parse_diff_multiple_files() {
        let stage = FmtStage::new();
        let diff = r#"Diff in src/main.rs at line 5:
-fn main() {
+fn main(){
Diff in src/lib.rs at line 1:
-use std::io;
+use std :: io;
"#;
        let failures = stage.parse_diff(diff);
        assert_eq!(failures.len(), 2);
    }

    #[test]
    fn test_repair_hints() {
        let stage = FmtStage::new();
        let loc = CodeLocation::single_line("src/main.rs", 1);
        let failures = vec![StageFailure::fmt(loc, "needs formatting".to_string())];

        let hints = stage.repair_hints(&failures);
        assert!(!hints.is_empty());
        assert!(hints[0].contains("cargo fmt"));
    }

    #[test]
    fn test_repair_hints_empty() {
        let stage = FmtStage::new();
        let hints = stage.repair_hints(&[]);
        assert!(!hints.is_empty());
        assert!(hints[0].contains("cargo fmt"));
    }

    #[test]
    fn test_default_timeout() {
        let stage = FmtStage::new();
        assert_eq!(stage.default_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_with_extra_args() {
        let stage = FmtStage::new().with_extra_args(vec!["--edition".to_string(), "2021".to_string()]);
        assert_eq!(stage.extra_args.len(), 2);
    }
}
