//! Privacy guard for runtime enforcement
//!
//! The PrivacyGuard wraps cloud adapter calls and scans outgoing payloads
//! for forbidden patterns, blocking or warning as configured.

use super::policy::{PrivacyPolicy, Violation, ViolationCategory};
use thiserror::Error;
use tracing::{error, warn};

/// Guard action modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardMode {
    /// Log violations but allow request (development)
    Warn,
    /// Block request if violations detected (production)
    Block,
    /// Panic on violations (security audit)
    Panic,
}

impl Default for GuardMode {
    fn default() -> Self {
        GuardMode::Block
    }
}

/// Errors from privacy guard enforcement
#[derive(Debug, Error)]
pub enum GuardError {
    /// Content contains privacy violations
    #[error("Privacy violation: {summary}")]
    ViolationDetected {
        /// Summary of violations
        summary: String,
        /// Number of violations
        count: usize,
        /// Categories of violations
        categories: Vec<ViolationCategory>,
    },
}

/// Runtime privacy guard
///
/// Scans outgoing payloads before they reach cloud services
/// and enforces the privacy policy.
pub struct PrivacyGuard {
    policy: PrivacyPolicy,
    mode: GuardMode,
}

impl Default for PrivacyGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyGuard {
    /// Create a new privacy guard with default policy and Block mode
    pub fn new() -> Self {
        Self {
            policy: PrivacyPolicy::new(),
            mode: GuardMode::Block,
        }
    }

    /// Create a guard with specific mode
    pub fn with_mode(mode: GuardMode) -> Self {
        Self {
            policy: PrivacyPolicy::new(),
            mode,
        }
    }

    /// Set the guard mode
    pub fn set_mode(&mut self, mode: GuardMode) {
        self.mode = mode;
    }

    /// Get the current mode
    pub fn mode(&self) -> GuardMode {
        self.mode
    }

    /// Check content before sending to cloud
    ///
    /// Returns `Ok(())` if content is safe or mode is Warn.
    /// Returns `Err(GuardError)` if violations detected and mode is Block.
    /// Panics if violations detected and mode is Panic.
    pub fn check(&self, content: &str) -> Result<(), GuardError> {
        let violations = self.policy.check(content);

        if violations.is_empty() {
            return Ok(());
        }

        let count = violations.len();
        let categories: Vec<ViolationCategory> =
            violations.iter().map(|v| v.category).collect::<std::collections::HashSet<_>>().into_iter().collect();

        let summary = self.format_summary(&violations);

        match self.mode {
            GuardMode::Warn => {
                warn!(
                    "Privacy guard: {} violation(s) detected but allowed in Warn mode: {}",
                    count, summary
                );
                Ok(())
            }
            GuardMode::Block => {
                error!(
                    "Privacy guard: Blocking request with {} violation(s): {}",
                    count, summary
                );
                Err(GuardError::ViolationDetected {
                    summary,
                    count,
                    categories,
                })
            }
            GuardMode::Panic => {
                panic!(
                    "Privacy guard: CRITICAL - {} violation(s) detected in Panic mode: {}",
                    count, summary
                );
            }
        }
    }

    /// Check if content is safe (for conditional logic)
    pub fn is_safe(&self, content: &str) -> bool {
        self.policy.is_safe(content)
    }

    /// Get violations without enforcing (for inspection)
    pub fn inspect(&self, content: &str) -> Vec<Violation> {
        self.policy.check(content)
    }

    /// Format a summary of violations for logging
    fn format_summary(&self, violations: &[Violation]) -> String {
        let mut summary = String::new();

        let mut by_category = std::collections::HashMap::new();
        for v in violations {
            *by_category.entry(v.category).or_insert(0) += 1;
        }

        let parts: Vec<String> = by_category
            .into_iter()
            .map(|(cat, count)| format!("{:?}:{}", cat, count))
            .collect();

        summary.push_str(&parts.join(", "));
        summary
    }
}

/// Guard wrapper for async operations
///
/// Provides convenience methods for guarding cloud adapter calls.
pub struct AsyncGuard {
    guard: PrivacyGuard,
}

impl Default for AsyncGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncGuard {
    /// Create a new async guard
    pub fn new() -> Self {
        Self {
            guard: PrivacyGuard::new(),
        }
    }

    /// Create with specific mode
    pub fn with_mode(mode: GuardMode) -> Self {
        Self {
            guard: PrivacyGuard::with_mode(mode),
        }
    }

    /// Guard a payload before sending
    ///
    /// Returns the content unchanged if safe, or error if violations.
    pub fn guard_payload<'a>(&self, content: &'a str) -> Result<&'a str, GuardError> {
        self.guard.check(content)?;
        Ok(content)
    }

    /// Check if payload is safe
    pub fn is_safe(&self, content: &str) -> bool {
        self.guard.is_safe(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_allows_safe_content() {
        let guard = PrivacyGuard::new();
        let content = "FILE_001 has 3 functions and 47 lines";
        assert!(guard.check(content).is_ok());
    }

    #[test]
    fn test_guard_blocks_file_path() {
        let guard = PrivacyGuard::new();
        let content = "Error in /home/user/project/src/auth.rs";
        let result = guard.check(content);
        assert!(result.is_err());

        if let Err(GuardError::ViolationDetected { count, .. }) = result {
            assert!(count > 0);
        }
    }

    #[test]
    fn test_guard_blocks_source_code() {
        let guard = PrivacyGuard::new();
        let content = "pub fn authenticate_user(creds: &Credentials) { }";
        assert!(guard.check(content).is_err());
    }

    #[test]
    fn test_guard_warn_mode_allows_violations() {
        let guard = PrivacyGuard::with_mode(GuardMode::Warn);
        let content = "Error in /home/user/src/main.rs";
        // Should return Ok even with violations in Warn mode
        assert!(guard.check(content).is_ok());
    }

    #[test]
    #[should_panic(expected = "Privacy guard: CRITICAL")]
    fn test_guard_panic_mode() {
        let guard = PrivacyGuard::with_mode(GuardMode::Panic);
        let content = "pub fn secret_function() {}";
        let _ = guard.check(content);
    }

    #[test]
    fn test_guard_inspect() {
        let guard = PrivacyGuard::new();
        let content = "Located at /home/user/src/main.rs with function authenticate()";
        let violations = guard.inspect(content);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_async_guard_payload() {
        let guard = AsyncGuard::new();

        // Safe content passes through
        let safe = "FILE_001 modified";
        assert!(guard.guard_payload(safe).is_ok());

        // Unsafe content blocked
        let unsafe_content = "pub fn main() {}";
        assert!(guard.guard_payload(unsafe_content).is_err());
    }

    #[test]
    fn test_mode_default_is_block() {
        let guard = PrivacyGuard::default();
        assert_eq!(guard.mode(), GuardMode::Block);
    }

    #[test]
    fn test_set_mode() {
        let mut guard = PrivacyGuard::new();
        assert_eq!(guard.mode(), GuardMode::Block);

        guard.set_mode(GuardMode::Warn);
        assert_eq!(guard.mode(), GuardMode::Warn);
    }
}
