//! Redaction engine for privacy mode
//!
//! Provides in-memory redaction mapping that replaces sensitive identifiers
//! with opaque placeholders. The redaction map is session-scoped and
//! NEVER serialized to disk (critical security requirement).

use std::collections::HashMap;
use uuid::Uuid;

/// Opaque identifier types for redaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedactionKind {
    /// File path (e.g., FILE_001)
    File,
    /// Symbol/variable name (e.g., SYM_001)
    Symbol,
    /// Function name (e.g., FUNC_001)
    Function,
    /// Class/struct name (e.g., TYPE_001)
    Type,
    /// Module/package name (e.g., MOD_001)
    Module,
    /// Generic identifier (e.g., ID_001)
    Generic,
}

impl RedactionKind {
    /// Get the prefix for this redaction kind
    pub fn prefix(&self) -> &'static str {
        match self {
            RedactionKind::File => "FILE",
            RedactionKind::Symbol => "SYM",
            RedactionKind::Function => "FUNC",
            RedactionKind::Type => "TYPE",
            RedactionKind::Module => "MOD",
            RedactionKind::Generic => "ID",
        }
    }
}

/// A single redaction entry
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for debugging and future extension
struct RedactionEntry {
    /// Original value (sensitive)
    original: String,
    /// Opaque replacement
    opaque: String,
    /// Kind of identifier
    kind: RedactionKind,
}

/// In-memory redaction map
///
/// # Security
///
/// This type intentionally does NOT implement `Serialize` or `Deserialize`.
/// The mapping between original identifiers and opaque IDs must never be
/// persisted to disk or transmitted over the network.
///
/// The redaction map is session-scoped:
/// - Created fresh each session
/// - Destroyed when the session ends
/// - Never saved to state files
#[derive(Debug)]
pub struct RedactionMap {
    /// Session ID (random UUID v4)
    session_id: Uuid,
    /// Forward mapping: original -> opaque
    forward: HashMap<String, RedactionEntry>,
    /// Reverse mapping: opaque -> original
    reverse: HashMap<String, String>,
    /// Counters for each redaction kind
    counters: HashMap<RedactionKind, u32>,
}

// SECURITY: Explicitly do NOT derive or implement Serialize/Deserialize
// This is a critical security requirement - the mapping must never be persisted

impl Default for RedactionMap {
    fn default() -> Self {
        Self::new()
    }
}

impl RedactionMap {
    /// Create a new redaction map with a fresh session ID
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            forward: HashMap::new(),
            reverse: HashMap::new(),
            counters: HashMap::new(),
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get the number of redacted items
    pub fn len(&self) -> usize {
        self.forward.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }

    /// Redact an identifier, returning the opaque replacement
    ///
    /// If the identifier has already been redacted in this session,
    /// returns the same opaque ID for consistency.
    pub fn redact(&mut self, original: &str, kind: RedactionKind) -> String {
        // Check if already redacted
        if let Some(entry) = self.forward.get(original) {
            return entry.opaque.clone();
        }

        // Generate new opaque ID
        let counter = self.counters.entry(kind).or_insert(0);
        *counter += 1;
        let opaque = format!("{}_{:03}", kind.prefix(), counter);

        // Store mapping
        let entry = RedactionEntry {
            original: original.to_string(),
            opaque: opaque.clone(),
            kind,
        };
        self.forward.insert(original.to_string(), entry);
        self.reverse.insert(opaque.clone(), original.to_string());

        opaque
    }

    /// Redact a file path
    pub fn redact_file(&mut self, path: &str) -> String {
        self.redact(path, RedactionKind::File)
    }

    /// Redact a symbol/variable name
    pub fn redact_symbol(&mut self, name: &str) -> String {
        self.redact(name, RedactionKind::Symbol)
    }

    /// Redact a function name
    pub fn redact_function(&mut self, name: &str) -> String {
        self.redact(name, RedactionKind::Function)
    }

    /// Redact a type/class name
    pub fn redact_type(&mut self, name: &str) -> String {
        self.redact(name, RedactionKind::Type)
    }

    /// Redact a module name
    pub fn redact_module(&mut self, name: &str) -> String {
        self.redact(name, RedactionKind::Module)
    }

    /// Unredact an opaque ID back to original (session-scoped only)
    ///
    /// Returns `None` if the opaque ID is not in this session's mapping.
    pub fn unredact(&self, opaque: &str) -> Option<&str> {
        self.reverse.get(opaque).map(|s| s.as_str())
    }

    /// Check if an opaque ID exists in this session
    pub fn contains_opaque(&self, opaque: &str) -> bool {
        self.reverse.contains_key(opaque)
    }

    /// Check if an original identifier has been redacted
    pub fn contains_original(&self, original: &str) -> bool {
        self.forward.contains_key(original)
    }

    /// Get statistics about redactions (safe to log)
    pub fn stats(&self) -> RedactionStats {
        let mut by_kind = HashMap::new();
        for (kind, count) in &self.counters {
            by_kind.insert(*kind, *count);
        }
        RedactionStats {
            total: self.forward.len(),
            by_kind,
        }
    }

    /// Clear all mappings (for security/cleanup)
    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
        self.counters.clear();
        // Generate new session ID after clear
        self.session_id = Uuid::new_v4();
    }
}

/// Redaction statistics (safe to log/serialize)
#[derive(Debug, Clone)]
pub struct RedactionStats {
    /// Total number of redacted items
    pub total: usize,
    /// Count by kind
    pub by_kind: HashMap<RedactionKind, u32>,
}

impl RedactionStats {
    /// Format as a summary string (safe for logging)
    pub fn summary(&self) -> String {
        if self.total == 0 {
            return "No redactions".to_string();
        }

        let parts: Vec<String> = self
            .by_kind
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(kind, count)| format!("{} {}s", count, kind.prefix().to_lowercase()))
            .collect();

        format!("Redacted {} identifiers: {}", self.total, parts.join(", "))
    }
}

/// Redactor service for processing content
///
/// Wraps a RedactionMap and provides high-level content processing.
#[derive(Debug)]
pub struct Redactor {
    map: RedactionMap,
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    /// Create a new redactor
    pub fn new() -> Self {
        Self {
            map: RedactionMap::new(),
        }
    }

    /// Get a reference to the redaction map
    pub fn map(&self) -> &RedactionMap {
        &self.map
    }

    /// Get a mutable reference to the redaction map
    pub fn map_mut(&mut self) -> &mut RedactionMap {
        &mut self.map
    }

    /// Redact file paths in content using provided path pattern
    ///
    /// Takes a list of detected file paths and replaces them with opaque IDs.
    pub fn redact_paths(&mut self, content: &str, paths: &[&str]) -> String {
        let mut result = content.to_string();
        for path in paths {
            let opaque = self.map.redact_file(path);
            result = result.replace(path, &opaque);
        }
        result
    }

    /// Redact function names in content
    pub fn redact_functions(&mut self, content: &str, functions: &[&str]) -> String {
        let mut result = content.to_string();
        for func in functions {
            let opaque = self.map.redact_function(func);
            result = result.replace(func, &opaque);
        }
        result
    }

    /// Redact symbols in content
    pub fn redact_symbols(&mut self, content: &str, symbols: &[&str]) -> String {
        let mut result = content.to_string();
        for sym in symbols {
            let opaque = self.map.redact_symbol(sym);
            result = result.replace(sym, &opaque);
        }
        result
    }

    /// Unredact content back to original identifiers
    ///
    /// Only works within the same session.
    pub fn unredact_content(&self, content: &str) -> String {
        let mut result = content.to_string();
        for (opaque, original) in &self.map.reverse {
            result = result.replace(opaque, original);
        }
        result
    }

    /// Get statistics (safe to log)
    pub fn stats(&self) -> RedactionStats {
        self.map.stats()
    }

    /// Clear all redactions
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redaction_map_new() {
        let map = RedactionMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_redact_file() {
        let mut map = RedactionMap::new();
        let opaque = map.redact_file("/home/user/src/main.rs");
        assert_eq!(opaque, "FILE_001");

        let opaque2 = map.redact_file("/home/user/src/lib.rs");
        assert_eq!(opaque2, "FILE_002");

        // Same path should return same ID
        let opaque3 = map.redact_file("/home/user/src/main.rs");
        assert_eq!(opaque3, "FILE_001");
    }

    #[test]
    fn test_redact_symbol() {
        let mut map = RedactionMap::new();
        assert_eq!(map.redact_symbol("user_data"), "SYM_001");
        assert_eq!(map.redact_symbol("auth_token"), "SYM_002");
    }

    #[test]
    fn test_redact_function() {
        let mut map = RedactionMap::new();
        assert_eq!(map.redact_function("authenticate_user"), "FUNC_001");
        assert_eq!(map.redact_function("validate_token"), "FUNC_002");
    }

    #[test]
    fn test_redact_type() {
        let mut map = RedactionMap::new();
        assert_eq!(map.redact_type("UserCredentials"), "TYPE_001");
    }

    #[test]
    fn test_redact_module() {
        let mut map = RedactionMap::new();
        assert_eq!(map.redact_module("authentication"), "MOD_001");
    }

    #[test]
    fn test_unredact() {
        let mut map = RedactionMap::new();
        let opaque = map.redact_file("/home/user/src/main.rs");

        let original = map.unredact(&opaque);
        assert_eq!(original, Some("/home/user/src/main.rs"));
    }

    #[test]
    fn test_unredact_unknown() {
        let map = RedactionMap::new();
        assert_eq!(map.unredact("FILE_999"), None);
    }

    #[test]
    fn test_contains_methods() {
        let mut map = RedactionMap::new();
        let opaque = map.redact_file("/home/user/src/main.rs");

        assert!(map.contains_opaque(&opaque));
        assert!(map.contains_original("/home/user/src/main.rs"));
        assert!(!map.contains_opaque("FILE_999"));
        assert!(!map.contains_original("/unknown/path"));
    }

    #[test]
    fn test_stats() {
        let mut map = RedactionMap::new();
        map.redact_file("/path/one");
        map.redact_file("/path/two");
        map.redact_function("func_a");
        map.redact_symbol("sym_x");

        let stats = map.stats();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.by_kind.get(&RedactionKind::File), Some(&2));
        assert_eq!(stats.by_kind.get(&RedactionKind::Function), Some(&1));
        assert_eq!(stats.by_kind.get(&RedactionKind::Symbol), Some(&1));
    }

    #[test]
    fn test_stats_summary() {
        let mut map = RedactionMap::new();
        map.redact_file("/path/one");
        map.redact_file("/path/two");
        map.redact_function("func_a");

        let stats = map.stats();
        let summary = stats.summary();
        assert!(summary.contains("3 identifiers"));
        assert!(summary.contains("2 files"));
        assert!(summary.contains("1 funcs"));
    }

    #[test]
    fn test_clear() {
        let mut map = RedactionMap::new();
        let original_session = map.session_id();

        map.redact_file("/path/one");
        assert_eq!(map.len(), 1);

        map.clear();
        assert!(map.is_empty());
        // Session ID should change after clear
        assert_ne!(map.session_id(), original_session);
    }

    #[test]
    fn test_session_id_unique() {
        let map1 = RedactionMap::new();
        let map2 = RedactionMap::new();
        assert_ne!(map1.session_id(), map2.session_id());
    }

    #[test]
    fn test_redactor_paths() {
        let mut redactor = Redactor::new();
        let content = "Error in /home/user/src/main.rs at line 42";
        let paths = ["/home/user/src/main.rs"];

        let redacted = redactor.redact_paths(content, &paths);
        assert_eq!(redacted, "Error in FILE_001 at line 42");
    }

    #[test]
    fn test_redactor_functions() {
        let mut redactor = Redactor::new();
        let content = "Function authenticate_user failed with error";
        let functions = ["authenticate_user"];

        let redacted = redactor.redact_functions(content, &functions);
        assert_eq!(redacted, "Function FUNC_001 failed with error");
    }

    #[test]
    fn test_redactor_unredact() {
        let mut redactor = Redactor::new();
        let content = "Error in /home/user/src/main.rs";
        let paths = ["/home/user/src/main.rs"];

        let redacted = redactor.redact_paths(content, &paths);
        assert_eq!(redacted, "Error in FILE_001");

        let restored = redactor.unredact_content(&redacted);
        assert_eq!(restored, content);
    }

    #[test]
    fn test_redactor_clear() {
        let mut redactor = Redactor::new();
        redactor.map_mut().redact_file("/path/one");
        assert!(!redactor.map().is_empty());

        redactor.clear();
        assert!(redactor.map().is_empty());
    }
}
