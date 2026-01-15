//! Clippy linting stage using `cargo clippy --message-format=json`.
//!
//! This stage runs clippy with JSON output to get structured diagnostic
//! information including code locations, severity, and suggested fixes.

use super::{run_cargo_command, VerificationStage};
use crate::verification::error::VerificationError;
use crate::verification::types::{
    CodeLocation, DiagnosticSeverity, FailureType, RelatedInfo, StageFailure, StageResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::time::{Duration, Instant};

/// Clippy linting verification stage.
///
/// Runs `cargo clippy --message-format=json` to detect linting issues.
/// Parses JSON output to extract structured diagnostic information.
pub struct ClippyStage {
    /// Package to check (None for workspace)
    package: Option<String>,
    /// Deny all warnings (treat as errors)
    deny_warnings: bool,
    /// Additional clippy lints to allow
    allow_lints: Vec<String>,
    /// Additional clippy lints to deny
    deny_lints: Vec<String>,
}

impl ClippyStage {
    /// Create a new clippy stage for the entire workspace.
    pub fn new() -> Self {
        Self {
            package: None,
            deny_warnings: false,
            allow_lints: Vec::new(),
            deny_lints: Vec::new(),
        }
    }

    /// Create a clippy stage for a specific package.
    pub fn for_package(package: impl Into<String>) -> Self {
        Self {
            package: Some(package.into()),
            deny_warnings: false,
            allow_lints: Vec::new(),
            deny_lints: Vec::new(),
        }
    }

    /// Treat warnings as errors.
    pub fn deny_warnings(mut self) -> Self {
        self.deny_warnings = true;
        self
    }

    /// Allow specific lints.
    pub fn allow_lints(mut self, lints: Vec<String>) -> Self {
        self.allow_lints = lints;
        self
    }

    /// Deny specific lints.
    pub fn deny_lints(mut self, lints: Vec<String>) -> Self {
        self.deny_lints = lints;
        self
    }

    /// Parse cargo JSON diagnostic output.
    fn parse_cargo_json(&self, output: &str) -> Vec<StageFailure> {
        let mut failures = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Try to parse as cargo message
            if let Ok(msg) = serde_json::from_str::<CargoMessage>(line) {
                if let Some(diagnostic) = msg.message {
                    if let Some(failure) = self.diagnostic_to_failure(&diagnostic) {
                        failures.push(failure);
                    }
                }
            }
        }

        failures
    }

    /// Convert a cargo diagnostic to a StageFailure.
    fn diagnostic_to_failure(&self, diagnostic: &Diagnostic) -> Option<StageFailure> {
        // Skip notes and help messages unless they're the primary message
        let severity = match diagnostic.level.as_str() {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "note" => return None, // Skip notes
            "help" => return None, // Skip help
            _ => DiagnosticSeverity::Warning,
        };

        // Get primary span location
        let location = diagnostic.spans.iter().find(|s| s.is_primary).map(|span| {
            CodeLocation::new(&span.file_name, span.line_start, span.line_end)
                .with_columns(span.column_start, span.column_end)
        });

        // Only report if we have a location (skip macro expansions without location)
        let location = location?;

        // Get the lint code (e.g., "clippy::unwrap_used")
        let code = diagnostic.code.as_ref().map(|c| c.code.clone());

        // Look for suggested replacement
        let suggested_fix = diagnostic.spans.iter().find_map(|span| {
            span.suggested_replacement.as_ref().map(|s| s.clone())
        });

        // Collect related spans
        let related: Vec<RelatedInfo> = diagnostic
            .spans
            .iter()
            .filter(|s| !s.is_primary && s.label.is_some())
            .map(|span| {
                let loc = CodeLocation::new(&span.file_name, span.line_start, span.line_end)
                    .with_columns(span.column_start, span.column_end);
                RelatedInfo::new(loc, span.label.clone().unwrap_or_default())
            })
            .collect();

        let mut failure = StageFailure {
            failure_type: FailureType::Clippy,
            location: Some(location),
            message: diagnostic.message.clone(),
            severity: Some(severity),
            code,
            suggested_fix,
            related,
        };

        // Check for rendered suggestion in children
        for child in &diagnostic.children {
            if child.level == "help" {
                if let Some(span) = child.spans.first() {
                    if let Some(ref replacement) = span.suggested_replacement {
                        failure.suggested_fix = Some(replacement.clone());
                    }
                }
            }
        }

        Some(failure)
    }
}

impl Default for ClippyStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationStage for ClippyStage {
    fn name(&self) -> &str {
        "clippy"
    }

    fn display_name(&self) -> &str {
        "Clippy Lints"
    }

    async fn run(&self, working_dir: &Path) -> Result<StageResult, VerificationError> {
        let start = Instant::now();

        let mut args = vec!["clippy", "--message-format=json"];

        if let Some(ref pkg) = self.package {
            args.push("-p");
            args.push(Box::leak(pkg.clone().into_boxed_str()));
        }

        args.push("--");

        if self.deny_warnings {
            args.push("-D");
            args.push("warnings");
        }

        for lint in &self.allow_lints {
            args.push("-A");
            args.push(Box::leak(lint.clone().into_boxed_str()));
        }

        for lint in &self.deny_lints {
            args.push("-D");
            args.push(Box::leak(lint.clone().into_boxed_str()));
        }

        let (success, stdout, stderr) = run_cargo_command(&args, working_dir, self.default_timeout()).await?;

        let duration = start.elapsed();
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Parse JSON output for failures
        let failures = self.parse_failures(&combined_output);

        // Check for compile errors in stderr
        let has_compile_error = combined_output.contains("error[E") ||
            combined_output.contains("aborting due to");

        if success && failures.is_empty() {
            Ok(StageResult::success(self.name(), duration, combined_output))
        } else if has_compile_error {
            // Compile errors are more severe
            Err(VerificationError::compile_error(
                "Compilation failed - fix errors before running clippy"
            ))
        } else {
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
        self.parse_cargo_json(output)
    }

    fn default_timeout(&self) -> Duration {
        Duration::from_secs(180)
    }

    fn supports_repair(&self) -> bool {
        true
    }

    fn repair_hints(&self, failures: &[StageFailure]) -> Vec<String> {
        let mut hints = Vec::new();

        for failure in failures {
            if let Some(ref fix) = failure.suggested_fix {
                hints.push(format!(
                    "Suggested fix for '{}': {}",
                    failure.code.as_deref().unwrap_or("unknown"),
                    fix
                ));
            }
            if let Some(ref code) = failure.code {
                hints.push(format!(
                    "To suppress this lint: #[allow({})]",
                    code
                ));
            }
        }

        if hints.is_empty() {
            hints.push("Run `cargo clippy --fix` to auto-fix some issues".to_string());
        }

        hints
    }
}

// Cargo JSON message structures

#[derive(Debug, Deserialize)]
struct CargoMessage {
    #[serde(default)]
    message: Option<Diagnostic>,
}

#[derive(Debug, Deserialize)]
struct Diagnostic {
    message: String,
    level: String,
    #[serde(default)]
    code: Option<DiagnosticCode>,
    #[serde(default)]
    spans: Vec<DiagnosticSpan>,
    #[serde(default)]
    children: Vec<Diagnostic>,
}

#[derive(Debug, Deserialize)]
struct DiagnosticCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct DiagnosticSpan {
    file_name: String,
    line_start: u32,
    line_end: u32,
    column_start: u32,
    column_end: u32,
    is_primary: bool,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    suggested_replacement: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clippy_stage_name() {
        let stage = ClippyStage::new();
        assert_eq!(stage.name(), "clippy");
        assert_eq!(stage.display_name(), "Clippy Lints");
    }

    #[test]
    fn test_clippy_stage_for_package() {
        let stage = ClippyStage::for_package("my-crate");
        assert_eq!(stage.package, Some("my-crate".to_string()));
    }

    #[test]
    fn test_clippy_stage_supports_repair() {
        let stage = ClippyStage::new();
        assert!(stage.supports_repair());
    }

    #[test]
    fn test_clippy_deny_warnings() {
        let stage = ClippyStage::new().deny_warnings();
        assert!(stage.deny_warnings);
    }

    #[test]
    fn test_clippy_allow_lints() {
        let stage = ClippyStage::new().allow_lints(vec!["clippy::unwrap_used".to_string()]);
        assert!(stage.allow_lints.contains(&"clippy::unwrap_used".to_string()));
    }

    #[test]
    fn test_parse_cargo_json_empty() {
        let stage = ClippyStage::new();
        let failures = stage.parse_cargo_json("");
        assert!(failures.is_empty());
    }

    #[test]
    fn test_parse_cargo_json_warning() {
        let stage = ClippyStage::new();
        let json = r#"{"reason":"compiler-message","package_id":"test","manifest_path":"Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"test","src_path":"src/lib.rs","edition":"2021","doc":true,"doctest":true,"test":true},"message":{"rendered":"warning: unused variable","message":"unused variable: `x`","level":"warning","code":{"code":"unused_variables"},"spans":[{"file_name":"src/lib.rs","byte_start":100,"byte_end":101,"line_start":10,"line_end":10,"column_start":5,"column_end":6,"is_primary":true,"label":null}],"children":[]}}"#;

        let failures = stage.parse_cargo_json(json);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].failure_type, FailureType::Clippy);
        assert_eq!(failures[0].severity, Some(DiagnosticSeverity::Warning));
        assert!(failures[0].message.contains("unused variable"));
    }

    #[test]
    fn test_parse_cargo_json_error() {
        let stage = ClippyStage::new();
        let json = r#"{"reason":"compiler-message","message":{"rendered":"error[E0308]","message":"mismatched types","level":"error","code":{"code":"E0308"},"spans":[{"file_name":"src/main.rs","byte_start":50,"byte_end":60,"line_start":5,"line_end":5,"column_start":10,"column_end":20,"is_primary":true,"label":"expected `i32`, found `&str`"}],"children":[]}}"#;

        let failures = stage.parse_cargo_json(json);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].severity, Some(DiagnosticSeverity::Error));
    }

    #[test]
    fn test_parse_cargo_json_with_suggestion() {
        let stage = ClippyStage::new();
        let json = r#"{"reason":"compiler-message","message":{"message":"unused variable","level":"warning","code":{"code":"unused_variables"},"spans":[{"file_name":"src/lib.rs","byte_start":100,"byte_end":101,"line_start":10,"line_end":10,"column_start":5,"column_end":6,"is_primary":true,"suggested_replacement":"_x"}],"children":[]}}"#;

        let failures = stage.parse_cargo_json(json);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].suggested_fix, Some("_x".to_string()));
    }

    #[test]
    fn test_repair_hints() {
        let stage = ClippyStage::new();
        let loc = CodeLocation::single_line("src/lib.rs", 10);
        let failures = vec![StageFailure {
            failure_type: FailureType::Clippy,
            location: Some(loc),
            message: "unused variable".to_string(),
            severity: Some(DiagnosticSeverity::Warning),
            code: Some("unused_variables".to_string()),
            suggested_fix: Some("_x".to_string()),
            related: vec![],
        }];

        let hints = stage.repair_hints(&failures);
        assert!(!hints.is_empty());
        assert!(hints.iter().any(|h| h.contains("_x")));
        assert!(hints.iter().any(|h| h.contains("#[allow(")));
    }

    #[test]
    fn test_default_timeout() {
        let stage = ClippyStage::new();
        assert_eq!(stage.default_timeout(), Duration::from_secs(180));
    }

    #[test]
    fn test_diagnostic_to_failure_skips_notes() {
        let stage = ClippyStage::new();
        let diagnostic = Diagnostic {
            message: "note: something".to_string(),
            level: "note".to_string(),
            code: None,
            spans: vec![],
            children: vec![],
        };
        assert!(stage.diagnostic_to_failure(&diagnostic).is_none());
    }

    #[test]
    fn test_diagnostic_to_failure_skips_help() {
        let stage = ClippyStage::new();
        let diagnostic = Diagnostic {
            message: "help: try this".to_string(),
            level: "help".to_string(),
            code: None,
            spans: vec![],
            children: vec![],
        };
        assert!(stage.diagnostic_to_failure(&diagnostic).is_none());
    }
}
