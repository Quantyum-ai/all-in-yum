//! Verification result types for the privacy mode verification engine.
//!
//! This module defines the core data structures for representing verification
//! outcomes, stage results, and failure information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Overall result of the verification pipeline.
///
/// Contains the final status along with individual stage results
/// and aggregate statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the verification passed completely
    pub success: bool,
    /// Individual results for each stage that ran
    pub stage_results: Vec<StageResult>,
    /// Total repairs attempted across all stages
    pub total_repairs: usize,
    /// Total time spent in verification
    pub total_duration: Duration,
    /// Final summary message
    pub summary: String,
}

impl VerificationResult {
    /// Create a successful verification result
    pub fn success(stage_results: Vec<StageResult>, total_repairs: usize, duration: Duration) -> Self {
        let summary = format!(
            "Verification passed: {} stage(s) completed with {} repair(s)",
            stage_results.len(),
            total_repairs
        );
        Self {
            success: true,
            stage_results,
            total_repairs,
            total_duration: duration,
            summary,
        }
    }

    /// Create a failed verification result
    pub fn failure(
        stage_results: Vec<StageResult>,
        total_repairs: usize,
        duration: Duration,
        failure_reason: &str,
    ) -> Self {
        let summary = format!(
            "Verification failed: {} - {} stage(s) completed with {} repair(s)",
            failure_reason,
            stage_results.len(),
            total_repairs
        );
        Self {
            success: false,
            stage_results,
            total_repairs,
            total_duration: duration,
            summary,
        }
    }

    /// Get the failed stage if any
    pub fn failed_stage(&self) -> Option<&StageResult> {
        self.stage_results.iter().find(|r| !r.success)
    }

    /// Check if global repair limit was exceeded
    pub fn repair_limit_exceeded(&self) -> bool {
        self.stage_results.iter().any(|r| r.repair_limit_exceeded)
    }
}

/// Result of a single verification stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Name of the stage (e.g., "fmt", "clippy", "test")
    pub stage_name: String,
    /// Whether this stage passed
    pub success: bool,
    /// Number of repair attempts for this stage
    pub repair_attempts: usize,
    /// Whether the stage-specific repair limit was exceeded
    pub repair_limit_exceeded: bool,
    /// List of failures detected in this stage (empty if success)
    pub failures: Vec<StageFailure>,
    /// Duration of this stage
    pub duration: Duration,
    /// Raw output from the cargo command
    pub raw_output: String,
}

impl StageResult {
    /// Create a successful stage result
    pub fn success(stage_name: &str, duration: Duration, raw_output: String) -> Self {
        Self {
            stage_name: stage_name.to_string(),
            success: true,
            repair_attempts: 0,
            repair_limit_exceeded: false,
            failures: Vec::new(),
            duration,
            raw_output,
        }
    }

    /// Create a failed stage result
    pub fn failure(
        stage_name: &str,
        failures: Vec<StageFailure>,
        repair_attempts: usize,
        repair_limit_exceeded: bool,
        duration: Duration,
        raw_output: String,
    ) -> Self {
        Self {
            stage_name: stage_name.to_string(),
            success: false,
            repair_attempts,
            repair_limit_exceeded,
            failures,
            duration,
            raw_output,
        }
    }

    /// Create a stage result after successful repair
    pub fn repaired(
        stage_name: &str,
        repair_attempts: usize,
        duration: Duration,
        raw_output: String,
    ) -> Self {
        Self {
            stage_name: stage_name.to_string(),
            success: true,
            repair_attempts,
            repair_limit_exceeded: false,
            failures: Vec::new(),
            duration,
            raw_output,
        }
    }
}

/// A single failure detected during verification.
///
/// Represents a fmt diff, clippy warning, or test failure with
/// location and diagnostic information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageFailure {
    /// Type of failure (fmt, clippy, test)
    pub failure_type: FailureType,
    /// Location in source code
    pub location: Option<CodeLocation>,
    /// Diagnostic message
    pub message: String,
    /// Severity level (for clippy)
    pub severity: Option<DiagnosticSeverity>,
    /// Error code (e.g., "E0308", "clippy::unwrap_used")
    pub code: Option<String>,
    /// Suggested fix from cargo/clippy if available
    pub suggested_fix: Option<String>,
    /// Related spans (for complex diagnostics)
    pub related: Vec<RelatedInfo>,
}

impl StageFailure {
    /// Create a new fmt failure
    pub fn fmt(location: CodeLocation, message: String) -> Self {
        Self {
            failure_type: FailureType::Fmt,
            location: Some(location),
            message,
            severity: None,
            code: None,
            suggested_fix: None,
            related: Vec::new(),
        }
    }

    /// Create a new clippy failure
    pub fn clippy(
        location: CodeLocation,
        message: String,
        severity: DiagnosticSeverity,
        code: Option<String>,
        suggested_fix: Option<String>,
    ) -> Self {
        Self {
            failure_type: FailureType::Clippy,
            location: Some(location),
            message,
            severity: Some(severity),
            code,
            suggested_fix,
            related: Vec::new(),
        }
    }

    /// Create a new test failure
    pub fn test(test_name: String, message: String) -> Self {
        Self {
            failure_type: FailureType::Test,
            location: None,
            message: format!("{}: {}", test_name, message),
            severity: Some(DiagnosticSeverity::Error),
            code: None,
            suggested_fix: None,
            related: Vec::new(),
        }
    }

    /// Create a compile error
    pub fn compile_error(
        location: CodeLocation,
        message: String,
        code: Option<String>,
    ) -> Self {
        Self {
            failure_type: FailureType::CompileError,
            location: Some(location),
            message,
            severity: Some(DiagnosticSeverity::Error),
            code,
            suggested_fix: None,
            related: Vec::new(),
        }
    }

    /// Add related information
    pub fn with_related(mut self, related: Vec<RelatedInfo>) -> Self {
        self.related = related;
        self
    }

    /// Add a suggested fix
    pub fn with_suggested_fix(mut self, fix: String) -> Self {
        self.suggested_fix = Some(fix);
        self
    }
}

/// Type of verification failure.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FailureType {
    /// Formatting issue (cargo fmt)
    Fmt,
    /// Linting issue (cargo clippy)
    Clippy,
    /// Test failure (cargo test)
    Test,
    /// Compilation error
    CompileError,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::Fmt => write!(f, "fmt"),
            FailureType::Clippy => write!(f, "clippy"),
            FailureType::Test => write!(f, "test"),
            FailureType::CompileError => write!(f, "compile"),
        }
    }
}

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    /// Informational hint
    Hint,
    /// Note attached to another diagnostic
    Note,
    /// Warning that should be addressed
    Warning,
    /// Error that must be fixed
    Error,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticSeverity::Hint => write!(f, "hint"),
            DiagnosticSeverity::Note => write!(f, "note"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Error => write!(f, "error"),
        }
    }
}

/// Location in source code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CodeLocation {
    /// File path (absolute or relative to project root)
    pub file: PathBuf,
    /// Starting line (1-indexed)
    pub line_start: u32,
    /// Ending line (1-indexed, inclusive)
    pub line_end: u32,
    /// Starting column (1-indexed, optional)
    pub column_start: Option<u32>,
    /// Ending column (1-indexed, optional)
    pub column_end: Option<u32>,
}

impl CodeLocation {
    /// Create a new code location
    pub fn new(file: impl Into<PathBuf>, line_start: u32, line_end: u32) -> Self {
        Self {
            file: file.into(),
            line_start,
            line_end,
            column_start: None,
            column_end: None,
        }
    }

    /// Create a single-line location
    pub fn single_line(file: impl Into<PathBuf>, line: u32) -> Self {
        Self::new(file, line, line)
    }

    /// Add column information
    pub fn with_columns(mut self, start: u32, end: u32) -> Self {
        self.column_start = Some(start);
        self.column_end = Some(end);
        self
    }

    /// Add just the start column
    pub fn with_column_start(mut self, start: u32) -> Self {
        self.column_start = Some(start);
        self
    }

    /// Format as "file:line" or "file:line:column"
    pub fn display(&self) -> String {
        let file_str = self.file.display();
        match (self.column_start, self.column_end) {
            (Some(col_start), Some(col_end)) if self.line_start == self.line_end => {
                format!("{}:{}:{}-{}", file_str, self.line_start, col_start, col_end)
            }
            (Some(col_start), _) => {
                format!("{}:{}:{}", file_str, self.line_start, col_start)
            }
            _ if self.line_start == self.line_end => {
                format!("{}:{}", file_str, self.line_start)
            }
            _ => {
                format!("{}:{}-{}", file_str, self.line_start, self.line_end)
            }
        }
    }
}

impl std::fmt::Display for CodeLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Related information for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelatedInfo {
    /// Location of related information
    pub location: CodeLocation,
    /// Description of the relation
    pub message: String,
}

impl RelatedInfo {
    /// Create new related info
    pub fn new(location: CodeLocation, message: impl Into<String>) -> Self {
        Self {
            location,
            message: message.into(),
        }
    }
}

/// Summary of repairs made during verification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepairSummary {
    /// Repairs by stage name
    pub by_stage: HashMap<String, usize>,
    /// Total repairs across all stages
    pub total: usize,
    /// Files that were modified during repair
    pub modified_files: Vec<PathBuf>,
}

impl RepairSummary {
    /// Record a repair for a stage
    pub fn record_repair(&mut self, stage_name: &str) {
        *self.by_stage.entry(stage_name.to_string()).or_insert(0) += 1;
        self.total += 1;
    }

    /// Record a modified file
    pub fn record_modified_file(&mut self, path: PathBuf) {
        if !self.modified_files.contains(&path) {
            self.modified_files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_success() {
        let stages = vec![StageResult::success(
            "fmt",
            Duration::from_millis(100),
            String::new(),
        )];
        let result = VerificationResult::success(stages, 0, Duration::from_millis(100));
        assert!(result.success);
        assert!(result.summary.contains("passed"));
    }

    #[test]
    fn test_verification_result_failure() {
        let stages = vec![StageResult::failure(
            "clippy",
            vec![],
            2,
            true,
            Duration::from_millis(500),
            String::new(),
        )];
        let result = VerificationResult::failure(
            stages,
            2,
            Duration::from_millis(500),
            "repair limit exceeded",
        );
        assert!(!result.success);
        assert!(result.summary.contains("failed"));
        assert!(result.repair_limit_exceeded());
    }

    #[test]
    fn test_stage_result_repaired() {
        let result = StageResult::repaired("fmt", 1, Duration::from_millis(200), String::new());
        assert!(result.success);
        assert_eq!(result.repair_attempts, 1);
    }

    #[test]
    fn test_code_location_single_line() {
        let loc = CodeLocation::single_line("src/main.rs", 42);
        assert_eq!(loc.display(), "src/main.rs:42");
    }

    #[test]
    fn test_code_location_with_columns() {
        let loc = CodeLocation::single_line("src/lib.rs", 10).with_columns(5, 15);
        assert_eq!(loc.display(), "src/lib.rs:10:5-15");
    }

    #[test]
    fn test_code_location_multi_line() {
        let loc = CodeLocation::new("src/test.rs", 10, 20);
        assert_eq!(loc.display(), "src/test.rs:10-20");
    }

    #[test]
    fn test_stage_failure_fmt() {
        let loc = CodeLocation::single_line("src/main.rs", 1);
        let failure = StageFailure::fmt(loc, "Diff detected".to_string());
        assert_eq!(failure.failure_type, FailureType::Fmt);
    }

    #[test]
    fn test_stage_failure_clippy() {
        let loc = CodeLocation::single_line("src/lib.rs", 42);
        let failure = StageFailure::clippy(
            loc,
            "unused variable".to_string(),
            DiagnosticSeverity::Warning,
            Some("unused_variables".to_string()),
            Some("prefix with underscore".to_string()),
        );
        assert_eq!(failure.failure_type, FailureType::Clippy);
        assert!(failure.suggested_fix.is_some());
    }

    #[test]
    fn test_stage_failure_test() {
        let failure = StageFailure::test("test_foo".to_string(), "assertion failed".to_string());
        assert_eq!(failure.failure_type, FailureType::Test);
        assert!(failure.message.contains("test_foo"));
    }

    #[test]
    fn test_repair_summary() {
        let mut summary = RepairSummary::default();
        summary.record_repair("fmt");
        summary.record_repair("fmt");
        summary.record_repair("clippy");
        summary.record_modified_file(PathBuf::from("src/main.rs"));

        assert_eq!(summary.total, 3);
        assert_eq!(*summary.by_stage.get("fmt").unwrap(), 2);
        assert_eq!(*summary.by_stage.get("clippy").unwrap(), 1);
        assert_eq!(summary.modified_files.len(), 1);
    }

    #[test]
    fn test_diagnostic_severity_ordering() {
        assert!(DiagnosticSeverity::Error > DiagnosticSeverity::Warning);
        assert!(DiagnosticSeverity::Warning > DiagnosticSeverity::Note);
        assert!(DiagnosticSeverity::Note > DiagnosticSeverity::Hint);
    }

    #[test]
    fn test_failure_type_display() {
        assert_eq!(FailureType::Fmt.to_string(), "fmt");
        assert_eq!(FailureType::Clippy.to_string(), "clippy");
        assert_eq!(FailureType::Test.to_string(), "test");
        assert_eq!(FailureType::CompileError.to_string(), "compile");
    }
}
