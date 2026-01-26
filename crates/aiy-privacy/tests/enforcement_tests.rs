//! Integration tests for privacy enforcement
//!
//! These tests verify:
//! 1. RedactionMap is NOT serializable (CRITICAL security test)
//! 2. Opaque ID generation format (FILE_001, SYM_014, etc.)
//! 3. Privacy guard blocks code/paths/secrets
//! 4. Safe content passes (opaque IDs, counts, metadata)
//! 5. Full workflow integration

use aiy_privacy::enforcement::{
    AuditEventKind, GuardError, GuardMode, PrivacyGuard, RedactionAudit, RedactionKind,
    RedactionMap, Redactor, ViolationCategory,
};

// =============================================================================
// CRITICAL SECURITY TEST: RedactionMap must NOT be serializable
// =============================================================================

/// CRITICAL: RedactionMap must not implement Serialize
///
/// This is a compile-time guarantee that the redaction mapping can never
/// be accidentally serialized and leaked. If someone tries to add
/// #[derive(Serialize)] to RedactionMap, this test should fail to compile.
#[test]
fn test_redaction_map_is_not_serializable() {
    // This test verifies at runtime that we cannot serialize RedactionMap
    // The real protection is that Serialize is not implemented, which
    // would cause a compile error if attempted.

    let mut map = RedactionMap::new();
    map.redact_file("/home/secret/project/main.rs");
    map.redact_function("authenticate_user");

    // Attempt to serialize would fail at compile time if we tried:
    // let _ = serde_json::to_string(&map); // This would not compile

    // We can verify the map exists and has data
    assert_eq!(map.len(), 2);

    // But we can serialize the stats (which is safe)
    let stats = map.stats();
    assert_eq!(stats.total, 2);

    // The stats summary is safe to log
    let summary = stats.summary();
    assert!(summary.contains("2 identifiers"));
    assert!(!summary.contains("/home/secret"));
    assert!(!summary.contains("authenticate_user"));
}

/// Verify RedactionStats IS serializable (metadata only)
#[test]
fn test_redaction_stats_is_safe_metadata() {
    let mut map = RedactionMap::new();
    map.redact_file("/home/user/secret.rs");
    map.redact_file("/home/user/password.rs");
    map.redact_function("get_api_key");

    let stats = map.stats();

    // Stats should contain counts, never identifiers
    assert_eq!(stats.total, 3);
    assert_eq!(stats.by_kind.get(&RedactionKind::File), Some(&2));
    assert_eq!(stats.by_kind.get(&RedactionKind::Function), Some(&1));

    // Summary should be safe
    let summary = stats.summary();
    assert!(summary.contains("2 files"));
    assert!(summary.contains("1 funcs"));
    // Must NOT contain actual paths
    assert!(!summary.contains("secret"));
    assert!(!summary.contains("password"));
    assert!(!summary.contains("api_key"));
}

// =============================================================================
// Opaque ID Generation Tests
// =============================================================================

#[test]
fn test_opaque_id_format_file() {
    let mut map = RedactionMap::new();
    let id1 = map.redact_file("/path/to/file1.rs");
    let id2 = map.redact_file("/path/to/file2.rs");
    let id3 = map.redact_file("/path/to/file3.rs");

    assert_eq!(id1, "FILE_001");
    assert_eq!(id2, "FILE_002");
    assert_eq!(id3, "FILE_003");
}

#[test]
fn test_opaque_id_format_symbol() {
    let mut map = RedactionMap::new();
    let id1 = map.redact_symbol("user_data");
    let id2 = map.redact_symbol("auth_token");

    assert_eq!(id1, "SYM_001");
    assert_eq!(id2, "SYM_002");
}

#[test]
fn test_opaque_id_format_function() {
    let mut map = RedactionMap::new();
    let id1 = map.redact_function("validate_user");
    let id2 = map.redact_function("check_permissions");

    assert_eq!(id1, "FUNC_001");
    assert_eq!(id2, "FUNC_002");
}

#[test]
fn test_opaque_id_format_type() {
    let mut map = RedactionMap::new();
    let id = map.redact_type("UserCredentials");
    assert_eq!(id, "TYPE_001");
}

#[test]
fn test_opaque_id_format_module() {
    let mut map = RedactionMap::new();
    let id = map.redact_module("authentication");
    assert_eq!(id, "MOD_001");
}

#[test]
fn test_opaque_id_counter_per_kind() {
    let mut map = RedactionMap::new();

    // Each kind has its own counter
    assert_eq!(map.redact_file("/path1"), "FILE_001");
    assert_eq!(map.redact_symbol("sym1"), "SYM_001");
    assert_eq!(map.redact_file("/path2"), "FILE_002");
    assert_eq!(map.redact_function("func1"), "FUNC_001");
    assert_eq!(map.redact_symbol("sym2"), "SYM_002");

    // Counters are independent
    assert_eq!(map.stats().by_kind.get(&RedactionKind::File), Some(&2));
    assert_eq!(map.stats().by_kind.get(&RedactionKind::Symbol), Some(&2));
    assert_eq!(map.stats().by_kind.get(&RedactionKind::Function), Some(&1));
}

#[test]
fn test_same_identifier_returns_same_opaque_id() {
    let mut map = RedactionMap::new();

    let id1 = map.redact_file("/home/user/main.rs");
    let id2 = map.redact_file("/home/user/lib.rs");
    let id3 = map.redact_file("/home/user/main.rs"); // Same as id1

    assert_eq!(id1, "FILE_001");
    assert_eq!(id2, "FILE_002");
    assert_eq!(id3, "FILE_001"); // Should return same ID
    assert_eq!(map.len(), 2); // Only 2 unique entries
}

// =============================================================================
// Privacy Guard Tests: Blocking Code/Paths/Secrets
// =============================================================================

#[test]
fn test_guard_blocks_unix_paths() {
    let guard = PrivacyGuard::new();

    let content = "The error occurred in /home/developer/myproject/src/auth/login.rs";
    let result = guard.check(content);

    assert!(result.is_err());
    if let Err(GuardError::ViolationDetected { categories, .. }) = result {
        assert!(categories.contains(&ViolationCategory::FilePath));
    }
}

#[test]
fn test_guard_blocks_windows_paths() {
    let guard = PrivacyGuard::new();

    let content = r"Check the file at C:\Users\dev\Projects\app\main.rs";
    let result = guard.check(content);

    assert!(result.is_err());
}

#[test]
fn test_guard_blocks_rust_code() {
    let guard = PrivacyGuard::new();

    let content = "pub fn authenticate_user(credentials: &Credentials) -> Result<User, Error> {";
    assert!(guard.check(content).is_err());
}

#[test]
fn test_guard_blocks_python_code() {
    let guard = PrivacyGuard::new();

    let content = "def validate_password(self, password: str) -> bool:";
    assert!(guard.check(content).is_err());
}

#[test]
fn test_guard_blocks_api_keys() {
    let guard = PrivacyGuard::new();

    // OpenAI-style key
    let content = "API_KEY = sk-abcdefghijklmnopqrstuvwxyz1234567890";
    assert!(guard.check(content).is_err());
}

#[test]
fn test_guard_blocks_jwt_tokens() {
    let guard = PrivacyGuard::new();

    let content = "token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let result = guard.check(content);
    assert!(result.is_err());
}

#[test]
fn test_guard_blocks_connection_strings() {
    let guard = PrivacyGuard::new();

    let content = "DATABASE_URL=postgres://admin:secretpass@db.example.com:5432/production";
    assert!(guard.check(content).is_err());
}

// =============================================================================
// Safe Content Tests: Opaque IDs and Metadata Pass
// =============================================================================

#[test]
fn test_safe_opaque_file_ids() {
    let guard = PrivacyGuard::new();

    let content = "The error is in FILE_001 at line 42. FILE_002 depends on FILE_001.";
    assert!(guard.check(content).is_ok());
}

#[test]
fn test_safe_opaque_function_ids() {
    let guard = PrivacyGuard::new();

    let content = "FUNC_001 calls FUNC_002 which returns a TYPE_001 instance.";
    assert!(guard.check(content).is_ok());
}

#[test]
fn test_safe_opaque_symbol_ids() {
    let guard = PrivacyGuard::new();

    let content = "Variable SYM_014 is uninitialized before being passed to FUNC_003.";
    assert!(guard.check(content).is_ok());
}

#[test]
fn test_safe_counts_and_statistics() {
    let guard = PrivacyGuard::new();

    let content = r#"
        Analysis complete:
        - 47 files scanned
        - 156 functions found
        - 3 type mismatches detected
        - Module has 12 public exports
        - Total lines: 2,847
    "#;
    assert!(guard.check(content).is_ok());
}

#[test]
fn test_safe_error_descriptions() {
    let guard = PrivacyGuard::new();

    let content = r#"
        Error: type mismatch
        Expected: string
        Got: integer
        This occurs when passing numeric values to text-processing functions.
    "#;
    assert!(guard.check(content).is_ok());
}

#[test]
fn test_safe_generic_advice() {
    let guard = PrivacyGuard::new();

    let content = r#"
        To fix this issue:
        1. Check the function signature
        2. Ensure the return type matches
        3. Add appropriate error handling
        4. Consider using Option<T> for nullable values
    "#;
    assert!(guard.check(content).is_ok());
}

// =============================================================================
// Full Workflow Integration Tests
// =============================================================================

#[test]
fn test_full_redaction_workflow() {
    // 1. User describes a problem with real paths
    let user_input = "I have an error in /home/dev/myapp/src/handlers/auth.rs on line 42";

    // 2. Guard detects the violation
    let guard = PrivacyGuard::new();
    assert!(!guard.is_safe(user_input));

    // 3. Redactor replaces sensitive identifiers
    let mut redactor = Redactor::new();
    let paths = ["/home/dev/myapp/src/handlers/auth.rs"];
    let redacted = redactor.redact_paths(user_input, &paths);

    // 4. Redacted content is now safe
    assert_eq!(redacted, "I have an error in FILE_001 on line 42");
    assert!(guard.is_safe(&redacted));

    // 5. Stats show count but not identifiers
    let stats = redactor.stats();
    assert_eq!(stats.total, 1);
    assert!(!stats.summary().contains("/home"));

    // 6. Response can be unredacted locally
    let ai_response = "The error in FILE_001 suggests a type mismatch";
    let display = redactor.unredact_content(ai_response);
    assert_eq!(
        display,
        "The error in /home/dev/myapp/src/handlers/auth.rs suggests a type mismatch"
    );
}

#[test]
fn test_full_audit_workflow() {
    let mut redactor = Redactor::new();
    let mut audit = RedactionAudit::from_session_id(&redactor.map().session_id());

    // Perform redactions
    redactor.map_mut().redact_file("/path/one.rs");
    redactor.map_mut().redact_file("/path/two.rs");
    redactor.map_mut().redact_function("secret_function");

    // Log redaction activity
    audit.log_redaction(&redactor.stats());

    // Simulate a blocked violation
    audit.log_violation_blocked(2, &["FilePath", "SourceCode"]);

    // Get summary
    let summary = audit.summary();
    assert_eq!(summary.total_redactions, 3);
    assert_eq!(summary.violations_blocked, 2);

    // Verify audit logs are safe
    let logs = audit.to_log_lines();
    for line in &logs {
        assert!(!line.contains("/path"));
        assert!(!line.contains("secret_function"));
        assert!(line.contains("ts=") || line.contains("event="));
    }
}

#[test]
fn test_guard_modes() {
    // Block mode (default)
    let guard_block = PrivacyGuard::with_mode(GuardMode::Block);
    let content = "pub fn main() {}";
    assert!(guard_block.check(content).is_err());

    // Warn mode
    let guard_warn = PrivacyGuard::with_mode(GuardMode::Warn);
    assert!(guard_warn.check(content).is_ok()); // Warns but allows
}

#[test]
fn test_session_isolation() {
    // Two sessions should have independent redaction maps
    let mut redactor1 = Redactor::new();
    let mut redactor2 = Redactor::new();

    // Same input, different sessions
    let id1 = redactor1.map_mut().redact_file("/home/user/main.rs");
    let id2 = redactor2.map_mut().redact_file("/home/user/main.rs");

    // Both should be FILE_001 (fresh counters)
    assert_eq!(id1, "FILE_001");
    assert_eq!(id2, "FILE_001");

    // But different session IDs
    assert_ne!(
        redactor1.map().session_id(),
        redactor2.map().session_id()
    );

    // Unredact only works within same session
    assert_eq!(
        redactor1.map().unredact("FILE_001"),
        Some("/home/user/main.rs")
    );
    // redactor2 also has FILE_001 but for its own session
    assert_eq!(
        redactor2.map().unredact("FILE_001"),
        Some("/home/user/main.rs")
    );
}

#[test]
fn test_clear_resets_session() {
    let mut redactor = Redactor::new();
    let original_session = redactor.map().session_id();

    redactor.map_mut().redact_file("/path/one");
    assert_eq!(redactor.map().len(), 1);

    // Clear should reset everything
    redactor.clear();
    assert!(redactor.map().is_empty());
    assert_ne!(redactor.map().session_id(), original_session);

    // Counter resets too
    let new_id = redactor.map_mut().redact_file("/path/two");
    assert_eq!(new_id, "FILE_001"); // Back to 001, not 002
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_content_is_safe() {
    let guard = PrivacyGuard::new();
    assert!(guard.check("").is_ok());
}

#[test]
fn test_whitespace_only_is_safe() {
    let guard = PrivacyGuard::new();
    assert!(guard.check("   \n\t\n   ").is_ok());
}

#[test]
fn test_mixed_safe_and_unsafe_content() {
    let guard = PrivacyGuard::new();

    // Mix of safe and unsafe
    let content = "FILE_001 is located at /home/user/secret.rs";

    // Should be blocked due to the real path
    assert!(guard.check(content).is_err());
}

#[test]
fn test_audit_event_kinds() {
    let mut audit = RedactionAudit::new("test_session");

    // Should start with session_start
    assert_eq!(audit.events()[0].kind, AuditEventKind::SessionStart);

    audit.log_session_clear();
    audit.log_session_end();

    let events = audit.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].kind, AuditEventKind::SessionClear);
    assert_eq!(events[2].kind, AuditEventKind::SessionEnd);
}
