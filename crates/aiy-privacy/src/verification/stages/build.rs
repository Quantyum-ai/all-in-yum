//! Build stage using `cargo build`.

use super::{run_cargo_command, VerificationStage};
use crate::verification::error::VerificationError;
use crate::verification::types::{CodeLocation, StageFailure, StageResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::time::{Duration, Instant};

/// Build stage for running `cargo build`
pub struct BuildStage {
    package: Option<String>,
    release: bool,
}

impl BuildStage {
    /// Create a new build stage
    pub fn new() -> Self {
        Self {
            package: None,
            release: false,
        }
    }

    /// Build a specific package
    pub fn for_package(package: impl Into<String>) -> Self {
        Self {
            package: Some(package.into()),
            release: false,
        }
    }

    /// Build in release mode
    pub fn release(mut self) -> Self {
        self.release = true;
        self
    }

    fn parse_cargo_json(&self, output: &str) -> Vec<StageFailure> {
        let mut failures = Vec::new();

        for line in output.lines() {
            if let Ok(msg) = serde_json::from_str::<CompilerMessage>(line) {
                if msg.message.level == "error" {
                    let location = msg.message.spans.first().map(|span| {
                        let mut loc = CodeLocation::new(
                            span.file_name.clone(),
                            span.line_start as u32,
                            span.line_start as u32,
                        );
                        if span.column_start > 0 {
                            loc = loc.with_columns(span.column_start as u32, span.column_start as u32);
                        }
                        loc
                    });

                    let mut failure = StageFailure::compile_error(
                        location.unwrap_or_else(|| CodeLocation::new("unknown", 1, 1)),
                        msg.message.message,
                        None,
                    );

                    if let Some(rendered) = msg.message.rendered {
                        failure = failure.with_suggested_fix(rendered);
                    }

                    failures.push(failure);
                }
            }
        }

        failures
    }
}

#[derive(Debug, Deserialize)]
struct CompilerMessage {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    message: String,
    level: String,
    spans: Vec<Span>,
    rendered: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Span {
    file_name: String,
    line_start: usize,
    column_start: usize,
}

impl Default for BuildStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationStage for BuildStage {
    fn name(&self) -> &str {
        "build"
    }

    fn display_name(&self) -> &str {
        "Build (cargo build)"
    }

    async fn run(&self, working_dir: &Path) -> Result<StageResult, VerificationError> {
        let start = Instant::now();

        let mut args = vec!["build", "--message-format=json"];
        if self.release {
            args.push("--release");
        }
        if let Some(ref pkg) = self.package {
            args.push("-p");
            args.push(pkg);
        }

        let (success, stdout, stderr) =
            run_cargo_command(&args, working_dir, self.default_timeout()).await?;
        let duration = start.elapsed();

        let combined_output = format!("{}\n{}", stdout, stderr);
        let failures = self.parse_failures(&combined_output);

        if success && failures.is_empty() {
            Ok(StageResult::success(
                self.name(),
                duration,
                combined_output,
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
        Duration::from_secs(300)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stage_name() {
        let stage = BuildStage::new();
        assert_eq!(stage.name(), "build");
    }

    #[test]
    fn test_build_stage_for_package() {
        let stage = BuildStage::for_package("aiy-core");
        assert_eq!(stage.package, Some("aiy-core".to_string()));
    }

    #[test]
    fn test_build_stage_release() {
        let stage = BuildStage::new().release();
        assert!(stage.release);
    }
}
