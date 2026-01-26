//! Privacy policy definitions and violation detection
//!
//! This module defines patterns that indicate potential privacy violations
//! when content would be sent to cloud services.

use once_cell::sync::Lazy;
use regex::Regex;

/// Pattern categories for privacy violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViolationCategory {
    /// File path detected (Unix, Windows, or relative)
    FilePath,
    /// Source code detected
    SourceCode,
    /// Code diff detected
    CodeDiff,
    /// Real identifier (variable, function, class name)
    RealIdentifier,
    /// Secret or credential detected
    Secret,
    /// Configuration value detected
    ConfigValue,
}

/// A detected privacy violation
#[derive(Debug, Clone)]
pub struct Violation {
    /// Category of violation
    pub category: ViolationCategory,
    /// Pattern that matched
    pub pattern_name: &'static str,
    /// Brief description
    pub description: String,
    /// Position in content (byte offset)
    pub position: Option<usize>,
}

/// Privacy policy with detection patterns
pub struct PrivacyPolicy {
    file_path_patterns: Vec<(&'static str, Regex)>,
    code_patterns: Vec<(&'static str, Regex)>,
    diff_patterns: Vec<(&'static str, Regex)>,
    #[allow(dead_code)] // Reserved for future use
    identifier_patterns: Vec<(&'static str, Regex)>,
    secret_patterns: Vec<(&'static str, Regex)>,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyPolicy {
    /// Create a new privacy policy with default patterns
    pub fn new() -> Self {
        Self {
            file_path_patterns: FILE_PATH_PATTERNS.clone(),
            code_patterns: CODE_PATTERNS.clone(),
            diff_patterns: DIFF_PATTERNS.clone(),
            identifier_patterns: IDENTIFIER_PATTERNS.clone(),
            secret_patterns: SECRET_PATTERNS.clone(),
        }
    }

    /// Check content for privacy violations
    ///
    /// Returns a list of all detected violations. An empty list means
    /// the content is safe to send to cloud services.
    pub fn check(&self, content: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Check file paths
        for (name, pattern) in &self.file_path_patterns {
            for m in pattern.find_iter(content) {
                violations.push(Violation {
                    category: ViolationCategory::FilePath,
                    pattern_name: name,
                    description: format!("File path detected: '{}'", m.as_str()),
                    position: Some(m.start()),
                });
            }
        }

        // Check code patterns
        for (name, pattern) in &self.code_patterns {
            for m in pattern.find_iter(content) {
                violations.push(Violation {
                    category: ViolationCategory::SourceCode,
                    pattern_name: name,
                    description: "Source code pattern detected".to_string(),
                    position: Some(m.start()),
                });
            }
        }

        // Check diff patterns
        for (name, pattern) in &self.diff_patterns {
            for m in pattern.find_iter(content) {
                violations.push(Violation {
                    category: ViolationCategory::CodeDiff,
                    pattern_name: name,
                    description: "Code diff detected".to_string(),
                    position: Some(m.start()),
                });
            }
        }

        // Check secrets
        for (name, pattern) in &self.secret_patterns {
            for m in pattern.find_iter(content) {
                violations.push(Violation {
                    category: ViolationCategory::Secret,
                    pattern_name: name,
                    description: "Potential secret detected".to_string(),
                    position: Some(m.start()),
                });
            }
        }

        violations
    }

    /// Check if content has any violations
    pub fn has_violations(&self, content: &str) -> bool {
        !self.check(content).is_empty()
    }

    /// Check if content is safe (no violations)
    pub fn is_safe(&self, content: &str) -> bool {
        self.check(content).is_empty()
    }
}

// =============================================================================
// Pattern Definitions (25+ patterns)
// =============================================================================

/// File path patterns
static FILE_PATH_PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        // Unix absolute paths
        (
            "unix_absolute",
            Regex::new(r"(?:^|[^a-zA-Z0-9])(/(?:home|usr|var|etc|opt|tmp|src|app)/[a-zA-Z0-9_./\-]+)").unwrap(),
        ),
        // Windows paths
        (
            "windows_path",
            Regex::new(r"[A-Z]:\\[a-zA-Z0-9_\\.\-]+").unwrap(),
        ),
        // Relative paths with common directories
        (
            "relative_src",
            Regex::new(r"(?:^|[^a-zA-Z0-9])((?:src|lib|test|tests|spec|pkg)/[a-zA-Z0-9_./\-]+\.[a-zA-Z]+)").unwrap(),
        ),
        // File extensions that indicate source code
        (
            "source_extension",
            Regex::new(r"[a-zA-Z0-9_\-]+\.(?:rs|py|js|ts|go|java|c|cpp|h|hpp|rb|ex|exs|hs|ml|scala|kt)").unwrap(),
        ),
    ]
});

/// Source code patterns
static CODE_PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        // Rust patterns
        (
            "rust_fn",
            Regex::new(r"(?:pub\s+)?(?:async\s+)?fn\s+[a-z_][a-z0-9_]*\s*(?:<[^>]+>)?\s*\(").unwrap(),
        ),
        (
            "rust_impl",
            Regex::new(r"impl(?:\s*<[^>]+>)?\s+[A-Z][a-zA-Z0-9]*").unwrap(),
        ),
        (
            "rust_struct",
            Regex::new(r"(?:pub\s+)?struct\s+[A-Z][a-zA-Z0-9]*").unwrap(),
        ),
        // Python patterns
        (
            "python_def",
            Regex::new(r"def\s+[a-z_][a-z0-9_]*\s*\(").unwrap(),
        ),
        (
            "python_class",
            Regex::new(r"class\s+[A-Z][a-zA-Z0-9_]*\s*[:\(]").unwrap(),
        ),
        // JavaScript/TypeScript patterns
        (
            "js_function",
            Regex::new(r"(?:async\s+)?function\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\(").unwrap(),
        ),
        (
            "js_const_arrow",
            Regex::new(r"const\s+[a-zA-Z_][a-zA-Z0-9_]*\s*=\s*(?:async\s+)?\([^)]*\)\s*=>").unwrap(),
        ),
        // Import statements (language-agnostic)
        (
            "import_statement",
            Regex::new(r#"(?:^|\n)\s*(?:import|from|require|use|include)\s+['"][^'"]+['"]"#).unwrap(),
        ),
    ]
});

/// Diff patterns
static DIFF_PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        // Unified diff headers
        (
            "diff_header",
            Regex::new(r"^(?:\+\+\+|---)\s+[ab]/").unwrap(),
        ),
        // Diff chunk headers
        (
            "diff_chunk",
            Regex::new(r"^@@\s+\-\d+(?:,\d+)?\s+\+\d+(?:,\d+)?\s+@@").unwrap(),
        ),
        // Git diff prefixes
        (
            "git_diff_line",
            Regex::new(r"^(?:\+|\-)[^\+\-]").unwrap(),
        ),
    ]
});

/// Real identifier patterns (to detect when non-redacted names appear)
static IDENTIFIER_PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        // Function calls with parentheses
        (
            "function_call",
            Regex::new(r"[a-z_][a-z0-9_]{2,}\s*\(").unwrap(),
        ),
        // Method chains
        (
            "method_chain",
            Regex::new(r"\.[a-z_][a-z0-9_]+\s*\(").unwrap(),
        ),
    ]
});

/// Secret/credential patterns
static SECRET_PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        // API keys (various providers)
        (
            "openai_key",
            Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
        ),
        (
            "anthropic_key",
            Regex::new(r"sk-ant-[a-zA-Z0-9\-]+").unwrap(),
        ),
        (
            "aws_key",
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        ),
        (
            "github_token",
            Regex::new(r"gh[ps]_[a-zA-Z0-9]{36}").unwrap(),
        ),
        // Generic secret patterns
        (
            "bearer_token",
            Regex::new(r"Bearer\s+[a-zA-Z0-9\-_\.]+").unwrap(),
        ),
        (
            "jwt_token",
            Regex::new(r"eyJ[a-zA-Z0-9\-_]+\.eyJ[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+").unwrap(),
        ),
        // Password patterns
        (
            "password_assignment",
            Regex::new(r#"(?i)password\s*[=:]\s*['\"][^'\"]{4,}['\"]"#).unwrap(),
        ),
        // Connection strings
        (
            "connection_string",
            Regex::new(r"(?:postgres|mysql|mongodb|redis)://[^@]+@[^\s]+").unwrap(),
        ),
    ]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_unix_path() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check("Located at /home/user/project/src/main.rs");
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.category == ViolationCategory::FilePath));
    }

    #[test]
    fn test_detects_windows_path() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check(r"File at C:\Users\dev\project\main.rs");
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.category == ViolationCategory::FilePath));
    }

    #[test]
    fn test_detects_rust_function() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check("pub fn authenticate_user(credentials: &Credentials) {");
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.category == ViolationCategory::SourceCode));
    }

    #[test]
    fn test_detects_python_class() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check("class UserAuthentication:");
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_detects_diff_header() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check("+++ b/src/main.rs\n@@ -10,3 +10,5 @@");
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.category == ViolationCategory::CodeDiff));
    }

    #[test]
    fn test_detects_openai_key() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check("API_KEY=sk-abcdefghijklmnopqrstuvwxyz123456");
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.category == ViolationCategory::Secret));
    }

    #[test]
    fn test_detects_jwt_token() {
        let policy = PrivacyPolicy::new();
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.abc123";
        let violations = policy.check(token);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_detects_connection_string() {
        let policy = PrivacyPolicy::new();
        let violations = policy.check("postgres://user:pass@localhost:5432/db");
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_safe_content_passes() {
        let policy = PrivacyPolicy::new();
        // Opaque identifiers and metadata should pass
        let safe_content = r#"
            FILE_001 has 3 functions
            SYM_014 is called 5 times
            12 files modified
            type mismatch error
        "#;
        assert!(policy.is_safe(safe_content));
    }

    #[test]
    fn test_counts_and_metadata_safe() {
        let policy = PrivacyPolicy::new();
        let content = "Module with 3 public functions, 47 lines total";
        assert!(policy.is_safe(content));
    }
}
