//! Repair generation for the verification engine.
//!
//! This module provides the `RepairGenerator` that uses RAG context and
//! Ollama to generate code repairs for verification failures.

use super::error::VerificationError;
use super::types::{CodeLocation, FailureType, StageFailure};
use crate::enforcement::{PrivacyGuard, GuardMode};
use aiy_adapter_ollama::{OllamaAdapter, OllamaClient};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Configuration for repair generation.
#[derive(Debug, Clone)]
pub struct RepairConfig {
    /// Maximum tokens for RAG context
    pub rag_token_budget: usize,
    /// Top-k RAG results
    pub rag_top_k: usize,
    /// Temperature for generation
    pub temperature: f32,
    /// Whether to include related code context
    pub include_context: bool,
    /// Maximum lines of context around the failure
    pub context_lines: usize,
}

impl Default for RepairConfig {
    fn default() -> Self {
        Self {
            rag_token_budget: 2048,
            rag_top_k: 5,
            temperature: 0.1,
            include_context: true,
            context_lines: 10,
        }
    }
}

/// Generated repair for a failure.
#[derive(Debug, Clone)]
pub struct Repair {
    /// The failure being repaired
    pub failure: StageFailure,
    /// File to modify
    pub file: PathBuf,
    /// Original content of the region
    pub original: String,
    /// Repaired content
    pub replacement: String,
    /// Line range (start, end) - 1-indexed
    pub line_range: (u32, u32),
    /// Explanation of the repair
    pub explanation: String,
    /// Confidence in the repair (0.0 - 1.0)
    pub confidence: f64,
}

impl Repair {
    /// Create a new repair.
    pub fn new(
        failure: StageFailure,
        file: PathBuf,
        original: String,
        replacement: String,
        line_range: (u32, u32),
        explanation: String,
        confidence: f64,
    ) -> Self {
        Self {
            failure,
            file,
            original,
            replacement,
            line_range,
            explanation,
            confidence,
        }
    }

    /// Check if the repair actually changes the code.
    pub fn has_changes(&self) -> bool {
        self.original != self.replacement
    }
}

/// Repair generator using Ollama for local LLM inference.
///
/// Uses RAG context and structured prompts to generate repairs for
/// verification failures while keeping all code local.
pub struct RepairGenerator {
    /// Ollama adapter for generation
    adapter: OllamaAdapter,
    /// Privacy guard for scanning outputs
    guard: PrivacyGuard,
    /// Configuration
    config: RepairConfig,
    /// RAG context cache (file path -> content)
    context_cache: HashMap<PathBuf, String>,
}

impl RepairGenerator {
    /// Create a new repair generator with the given Ollama adapter.
    pub fn new(adapter: OllamaAdapter) -> Self {
        Self {
            adapter,
            guard: PrivacyGuard::with_mode(GuardMode::Warn),
            config: RepairConfig::default(),
            context_cache: HashMap::new(),
        }
    }

    /// Create a repair generator with custom configuration.
    pub fn with_config(adapter: OllamaAdapter, config: RepairConfig) -> Self {
        Self {
            adapter,
            guard: PrivacyGuard::with_mode(GuardMode::Warn),
            config,
            context_cache: HashMap::new(),
        }
    }

    /// Create a repair generator with a mock transport (for testing).
    #[cfg(test)]
    pub fn with_mock(transport: Arc<dyn aiy_adapter_ollama::transport::HttpTransport>) -> Self {
        let client = OllamaClient::new_with_mock(transport);
        let adapter = OllamaAdapter::new(client);
        Self::new(adapter)
    }

    /// Get the configuration.
    pub fn config(&self) -> &RepairConfig {
        &self.config
    }

    /// Generate a repair for a failure.
    ///
    /// Returns `Ok(Some(Repair))` if a repair was generated successfully,
    /// `Ok(None)` if the failure cannot be repaired automatically,
    /// or `Err` if repair generation failed.
    pub async fn generate_repair(
        &mut self,
        failure: &StageFailure,
        working_dir: &Path,
    ) -> Result<Option<Repair>, VerificationError> {
        // Get the file location
        let location = match &failure.location {
            Some(loc) => loc,
            None => {
                tracing::debug!("No location for failure, cannot repair");
                return Ok(None);
            }
        };

        // Read the file content
        let file_path = working_dir.join(&location.file);
        let content = self.read_file_content(&file_path).await?;

        // Extract the region to repair
        let (original, line_range) = self.extract_region(&content, location)?;

        // Build the repair prompt
        let prompt = self.build_repair_prompt(failure, &original, &content, location)?;

        // Generate repair using Ollama
        let response = self
            .adapter
            .generate_text(&prompt)
            .await
            .map_err(|e| VerificationError::ollama(e.to_string()))?;

        // Parse the repair response
        let repair = self.parse_repair_response(
            failure.clone(),
            file_path,
            original,
            line_range,
            &response,
        )?;

        // Validate the repair with privacy guard
        if let Some(ref r) = repair {
            self.validate_repair(r)?;
        }

        Ok(repair)
    }

    /// Generate repairs for multiple failures.
    pub async fn generate_repairs(
        &mut self,
        failures: &[StageFailure],
        working_dir: &Path,
    ) -> Vec<Result<Option<Repair>, VerificationError>> {
        let mut results = Vec::with_capacity(failures.len());

        for failure in failures {
            let result = self.generate_repair(failure, working_dir).await;
            results.push(result);
        }

        results
    }

    /// Read file content, using cache if available.
    async fn read_file_content(&mut self, path: &Path) -> Result<String, VerificationError> {
        if let Some(content) = self.context_cache.get(path) {
            return Ok(content.clone());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| VerificationError::repair_application(path, e.to_string()))?;

        self.context_cache.insert(path.to_path_buf(), content.clone());
        Ok(content)
    }

    /// Extract the region to repair from file content.
    fn extract_region(
        &self,
        content: &str,
        location: &CodeLocation,
    ) -> Result<(String, (u32, u32)), VerificationError> {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len() as u32;

        // Calculate range with context
        let context = self.config.context_lines as u32;
        let start = location.line_start.saturating_sub(context).max(1);
        let end = (location.line_end + context).min(total_lines);

        // Extract lines (1-indexed to 0-indexed)
        let region: String = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let line_num = (*i as u32) + 1;
                line_num >= start && line_num <= end
            })
            .map(|(_, line)| *line)
            .collect::<Vec<_>>()
            .join("\n");

        Ok((region, (start, end)))
    }

    /// Build the repair prompt for Ollama.
    fn build_repair_prompt(
        &self,
        failure: &StageFailure,
        region: &str,
        _full_content: &str,
        location: &CodeLocation,
    ) -> Result<String, VerificationError> {
        let system_context = match failure.failure_type {
            FailureType::Fmt => SYSTEM_PROMPT_FMT,
            FailureType::Clippy => SYSTEM_PROMPT_CLIPPY,
            FailureType::Test => SYSTEM_PROMPT_TEST,
            FailureType::CompileError => SYSTEM_PROMPT_COMPILE,
        };

        let mut prompt = format!(
            "{}\n\n## Problem\n\nFile: {}\nLines: {}-{}\n\nError: {}\n",
            system_context,
            location.file.display(),
            location.line_start,
            location.line_end,
            failure.message
        );

        if let Some(ref code) = failure.code {
            prompt.push_str(&format!("Error Code: {}\n", code));
        }

        if let Some(ref fix) = failure.suggested_fix {
            prompt.push_str(&format!("Suggested Fix: {}\n", fix));
        }

        prompt.push_str(&format!(
            "\n## Code Region to Fix\n\n```rust\n{}\n```\n\n## Instructions\n\n",
            region
        ));

        prompt.push_str(
            "Output ONLY the corrected code region, no explanation. \
             The code should be a drop-in replacement for the region shown above.\n\
             Start your response with ```rust and end with ```."
        );

        Ok(prompt)
    }

    /// Parse the repair response from Ollama.
    fn parse_repair_response(
        &self,
        failure: StageFailure,
        file: PathBuf,
        original: String,
        line_range: (u32, u32),
        response: &str,
    ) -> Result<Option<Repair>, VerificationError> {
        // Extract code from markdown code block
        let replacement = self.extract_code_from_response(response)?;

        if replacement.is_empty() {
            return Ok(None);
        }

        // Calculate confidence based on response quality
        let confidence = self.calculate_confidence(&original, &replacement, response);

        let repair = Repair::new(
            failure,
            file,
            original,
            replacement,
            line_range,
            "Generated by Ollama repair engine".to_string(),
            confidence,
        );

        if repair.has_changes() {
            Ok(Some(repair))
        } else {
            Ok(None)
        }
    }

    /// Extract code from a markdown code block response.
    fn extract_code_from_response(&self, response: &str) -> Result<String, VerificationError> {
        let trimmed = response.trim();

        // Try to find ```rust block
        if let Some(start) = trimmed.find("```rust") {
            let after_start = &trimmed[start + 7..];
            if let Some(end) = after_start.find("```") {
                return Ok(after_start[..end].trim().to_string());
            }
        }

        // Try plain ``` block
        if let Some(start) = trimmed.find("```") {
            let after_start = &trimmed[start + 3..];
            // Skip language identifier if on same line
            let code_start = if let Some(newline) = after_start.find('\n') {
                &after_start[newline + 1..]
            } else {
                after_start
            };
            if let Some(end) = code_start.find("```") {
                return Ok(code_start[..end].trim().to_string());
            }
        }

        // If no code block, return empty (cannot use unstructured response)
        Ok(String::new())
    }

    /// Calculate confidence in the repair.
    fn calculate_confidence(&self, original: &str, replacement: &str, _response: &str) -> f64 {
        // Simple heuristics for confidence
        let mut confidence: f64 = 0.5;

        // Prefer minimal changes
        let orig_lines = original.lines().count();
        let repl_lines = replacement.lines().count();
        let line_diff = (orig_lines as i32 - repl_lines as i32).unsigned_abs() as usize;

        if line_diff == 0 {
            confidence += 0.2;
        } else if line_diff <= 2 {
            confidence += 0.1;
        }

        // Prefer syntactically valid-looking code
        if replacement.contains("fn ") || replacement.contains("let ") || replacement.contains("use ") {
            confidence += 0.1;
        }

        // Penalize if replacement is empty or very short
        if replacement.len() < 10 {
            confidence -= 0.3;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Validate a repair with the privacy guard.
    fn validate_repair(&self, repair: &Repair) -> Result<(), VerificationError> {
        // Check that the replacement doesn't contain privacy violations
        // (shouldn't happen since it's locally generated, but defense in depth)
        if !self.guard.is_safe(&repair.replacement) {
            let violations = self.guard.inspect(&repair.replacement);
            return Err(VerificationError::privacy_violation(format!(
                "Repair contains {} privacy violation(s)",
                violations.len()
            )));
        }

        Ok(())
    }

    /// Apply a repair to a file.
    pub async fn apply_repair(&self, repair: &Repair) -> Result<(), VerificationError> {
        let content = tokio::fs::read_to_string(&repair.file)
            .await
            .map_err(|e| VerificationError::repair_application(&repair.file, e.to_string()))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::with_capacity(lines.len());

        // Lines before the repair region
        for (i, line) in lines.iter().enumerate() {
            let line_num = (i + 1) as u32;
            if line_num < repair.line_range.0 {
                new_lines.push(line.to_string());
            }
        }

        // The replacement
        for line in repair.replacement.lines() {
            new_lines.push(line.to_string());
        }

        // Lines after the repair region
        for (i, line) in lines.iter().enumerate() {
            let line_num = (i + 1) as u32;
            if line_num > repair.line_range.1 {
                new_lines.push(line.to_string());
            }
        }

        let new_content = new_lines.join("\n");

        tokio::fs::write(&repair.file, new_content)
            .await
            .map_err(|e| VerificationError::repair_application(&repair.file, e.to_string()))?;

        Ok(())
    }

    /// Clear the context cache.
    pub fn clear_cache(&mut self) {
        self.context_cache.clear();
    }
}

// System prompts for different failure types

const SYSTEM_PROMPT_FMT: &str = r#"You are a Rust code formatter assistant. Your task is to fix formatting issues in Rust code to match rustfmt standards.

Rules:
- Use 4-space indentation
- Keep lines under 100 characters
- Use consistent spacing around operators
- Format imports and use statements properly
- Preserve code semantics exactly

Output only the corrected code, no explanations."#;

const SYSTEM_PROMPT_CLIPPY: &str = r#"You are a Rust code quality assistant. Your task is to fix clippy linting issues in Rust code.

Rules:
- Fix the specific lint mentioned in the error
- Preserve code semantics
- Use idiomatic Rust patterns
- Prefer the suggested fix if provided
- Don't change unrelated code

Output only the corrected code, no explanations."#;

const SYSTEM_PROMPT_TEST: &str = r#"You are a Rust test debugging assistant. Your task is to fix failing test assertions.

Rules:
- Analyze the assertion failure (left vs right values)
- Fix the test to match correct expected behavior
- If the test expectation is wrong, fix it
- If the implementation is wrong, note it but focus on the test
- Preserve test structure

Output only the corrected code, no explanations."#;

const SYSTEM_PROMPT_COMPILE: &str = r#"You are a Rust compiler error assistant. Your task is to fix compilation errors.

Rules:
- Fix the specific error mentioned
- Handle type mismatches by adjusting types or adding conversions
- Fix borrow checker issues properly
- Add missing imports if needed
- Preserve code intent

Output only the corrected code, no explanations."#;

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_adapter_ollama::transport::MockTransport;

    fn mock_repair_response(code: &str) -> String {
        let escaped = code.replace('"', "\\\"").replace('\n', "\\n");
        format!(
            r#"{{"model":"test","message":{{"role":"assistant","content":"```rust\\n{}\\n```"}},"done":true}}"#,
            escaped
        )
    }

    #[test]
    fn test_repair_config_default() {
        let config = RepairConfig::default();
        assert_eq!(config.rag_token_budget, 2048);
        assert_eq!(config.rag_top_k, 5);
        assert!((config.temperature - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_repair_has_changes() {
        let failure = StageFailure::fmt(
            CodeLocation::single_line("src/main.rs", 1),
            "needs formatting".to_string(),
        );
        let repair = Repair::new(
            failure,
            PathBuf::from("src/main.rs"),
            "let x=1;".to_string(),
            "let x = 1;".to_string(),
            (1, 1),
            "Fixed spacing".to_string(),
            0.9,
        );
        assert!(repair.has_changes());
    }

    #[test]
    fn test_repair_no_changes() {
        let failure = StageFailure::fmt(
            CodeLocation::single_line("src/main.rs", 1),
            "needs formatting".to_string(),
        );
        let repair = Repair::new(
            failure,
            PathBuf::from("src/main.rs"),
            "let x = 1;".to_string(),
            "let x = 1;".to_string(),
            (1, 1),
            "No changes needed".to_string(),
            0.5,
        );
        assert!(!repair.has_changes());
    }

    #[tokio::test]
    async fn test_repair_generator_creation() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);
        assert_eq!(generator.config().rag_token_budget, 2048);
    }

    #[test]
    fn test_extract_code_from_response_rust_block() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let response = "Here is the fix:\n```rust\nlet x = 1;\n```\nDone.";
        let code = generator.extract_code_from_response(response).unwrap();
        assert_eq!(code, "let x = 1;");
    }

    #[test]
    fn test_extract_code_from_response_plain_block() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let response = "```\nlet x = 1;\n```";
        let code = generator.extract_code_from_response(response).unwrap();
        assert_eq!(code, "let x = 1;");
    }

    #[test]
    fn test_extract_code_from_response_no_block() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let response = "let x = 1;";
        let code = generator.extract_code_from_response(response).unwrap();
        assert!(code.is_empty()); // No code block = empty
    }

    #[test]
    fn test_calculate_confidence() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        // Same line count, contains Rust keywords
        let confidence = generator.calculate_confidence(
            "fn foo() {}",
            "fn foo() { }",
            "",
        );
        assert!(confidence > 0.5);
    }

    #[test]
    fn test_calculate_confidence_empty_replacement() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let confidence = generator.calculate_confidence("fn foo() {}", "", "");
        assert!(confidence < 0.5);
    }

    #[test]
    fn test_extract_region() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let location = CodeLocation::single_line("test.rs", 3);

        let (region, range) = generator.extract_region(content, &location).unwrap();
        assert!(region.contains("line 2"));
        assert!(region.contains("line 3"));
        assert!(region.contains("line 4"));
        assert!(range.0 >= 1);
        assert!(range.1 <= 5);
    }

    #[test]
    fn test_build_repair_prompt_fmt() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let failure = StageFailure::fmt(
            CodeLocation::single_line("src/main.rs", 10),
            "needs formatting".to_string(),
        );
        let location = failure.location.as_ref().unwrap();

        let prompt = generator
            .build_repair_prompt(&failure, "let x=1;", "let x=1;", location)
            .unwrap();

        assert!(prompt.contains("rustfmt"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("let x=1;"));
    }

    #[test]
    fn test_build_repair_prompt_clippy() {
        let transport = Arc::new(MockTransport::new());
        let generator = RepairGenerator::with_mock(transport);

        let failure = StageFailure::clippy(
            CodeLocation::single_line("src/lib.rs", 5),
            "unused variable".to_string(),
            crate::verification::types::DiagnosticSeverity::Warning,
            Some("unused_variables".to_string()),
            Some("_x".to_string()),
        );
        let location = failure.location.as_ref().unwrap();

        let prompt = generator
            .build_repair_prompt(&failure, "let x = 1;", "let x = 1;", location)
            .unwrap();

        assert!(prompt.contains("clippy"));
        assert!(prompt.contains("unused_variables"));
        assert!(prompt.contains("_x")); // Suggested fix
    }

    #[test]
    fn test_clear_cache() {
        let transport = Arc::new(MockTransport::new());
        let mut generator = RepairGenerator::with_mock(transport);

        generator.context_cache.insert(
            PathBuf::from("test.rs"),
            "content".to_string(),
        );
        assert!(!generator.context_cache.is_empty());

        generator.clear_cache();
        assert!(generator.context_cache.is_empty());
    }

    #[tokio::test]
    async fn test_generate_repair_no_location() {
        let transport = Arc::new(MockTransport::new());
        let mut generator = RepairGenerator::with_mock(transport);

        let failure = StageFailure::test("test_foo".to_string(), "failed".to_string());
        // No location set

        let result = generator
            .generate_repair(&failure, Path::new("/tmp"))
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
