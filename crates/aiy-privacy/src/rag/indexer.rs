//! Code indexer for the local RAG system.
//!
//! This module provides directory walking and file indexing functionality.
//! It respects exclude patterns and only indexes source code files.
//!
//! # Security
//!
//! - NO file paths are stored in the indexed data
//! - Only opaque IDs reference chunks
//! - Exclude patterns protect sensitive files from indexing

use crate::rag::chunk::{chunk_code, ChunkingConfig, ChunkingStrategy, LanguageHint};
use crate::rag::error::RagError;
use crate::rag::types::CodeChunk;
use glob::Pattern;
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, trace, warn};

/// Default file extensions to index.
const DEFAULT_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "ts", "tsx", "go", "java", "c", "cpp", "h", "hpp", "rb", "php", "swift",
    "kt", "scala", "cs", "lua", "sh", "bash", "zsh", "yaml", "yml", "json", "toml", "md",
];

/// Maximum file size to index (in bytes).
const MAX_FILE_SIZE: u64 = 1_000_000; // 1MB

/// Configuration for the code indexer.
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// File extensions to include
    pub extensions: HashSet<String>,

    /// Glob patterns to exclude (e.g., "target/**", ".git/**")
    pub exclude_patterns: Vec<Pattern>,

    /// Maximum file size to index
    pub max_file_size: u64,

    /// Chunking configuration
    pub chunking_config: ChunkingConfig,

    /// Chunking strategy to use
    pub chunking_strategy: ChunkingStrategy,

    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            extensions: DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            exclude_patterns: Self::default_exclude_patterns(),
            max_file_size: MAX_FILE_SIZE,
            chunking_config: ChunkingConfig::default(),
            chunking_strategy: ChunkingStrategy::Hybrid,
            follow_symlinks: false,
        }
    }
}

impl IndexerConfig {
    /// Create default exclude patterns.
    fn default_exclude_patterns() -> Vec<Pattern> {
        [
            "target/**",
            ".git/**",
            "node_modules/**",
            "*.lock",
            ".env*",
            "*.secret*",
            "*.key",
            "*.pem",
            "*.crt",
            ".cache/**",
            "__pycache__/**",
            "*.pyc",
            "dist/**",
            "build/**",
            ".next/**",
            "coverage/**",
            "*.min.js",
            "*.min.css",
            "vendor/**",
        ]
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect()
    }

    /// Add an exclude pattern.
    pub fn with_exclude_pattern(mut self, pattern: &str) -> Result<Self, RagError> {
        let pat = Pattern::new(pattern)
            .map_err(|e| RagError::InvalidConfig(format!("Invalid pattern '{}': {}", pattern, e)))?;
        self.exclude_patterns.push(pat);
        Ok(self)
    }

    /// Set exclude patterns from string list.
    pub fn with_exclude_patterns(mut self, patterns: &[String]) -> Result<Self, RagError> {
        self.exclude_patterns = patterns
            .iter()
            .map(|p| {
                Pattern::new(p).map_err(|e| {
                    RagError::InvalidConfig(format!("Invalid pattern '{}': {}", p, e))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self)
    }

    /// Check if a path should be excluded.
    pub fn is_excluded(&self, path: &Path) -> bool {
        // Normalize path to forward slashes for consistent matching
        let path_str = path.to_string_lossy().replace('\\', "/");

        // Collect path components for suffix matching
        let components: Vec<&str> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        for pattern in &self.exclude_patterns {
            // Check full path (preserves current behavior for absolute patterns)
            if pattern.matches(&path_str) {
                return true;
            }

            // Check individual components (e.g., ".git", "target")
            for component in &components {
                if pattern.matches(component) {
                    return true;
                }
            }

            // Suffix matching: test patterns against all path suffixes
            // e.g., for /tmp/project/target/debug.rs, test:
            //   "target/debug.rs", "debug.rs"
            for start in 0..components.len() {
                let suffix = components[start..].join("/");
                if pattern.matches(&suffix) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a file extension should be indexed.
    pub fn should_index_extension(&self, ext: &str) -> bool {
        self.extensions.contains(&ext.to_lowercase())
    }
}

/// Code indexer that walks directories and creates chunks.
///
/// # Example
///
/// ```rust,ignore
/// use aiy_privacy::rag::indexer::{CodeIndexer, IndexerConfig};
///
/// let config = IndexerConfig::default();
/// let indexer = CodeIndexer::new(config);
///
/// let chunks = indexer.index_directory("/path/to/project").await?;
/// ```
pub struct CodeIndexer {
    config: IndexerConfig,
}

impl CodeIndexer {
    /// Create a new code indexer with the given configuration.
    pub fn new(config: IndexerConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(IndexerConfig::default())
    }

    /// Get the indexer configuration.
    pub fn config(&self) -> &IndexerConfig {
        &self.config
    }

    /// Index a directory and return all code chunks.
    ///
    /// This method walks the directory tree, reads source files,
    /// and creates chunks for each file.
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory to index
    ///
    /// # Returns
    ///
    /// A vector of CodeChunk instances (without embeddings).
    pub fn index_directory(&self, root: &Path) -> Result<Vec<CodeChunk>, RagError> {
        let mut all_chunks = Vec::new();

        // Walk the directory tree
        let walker = self.create_walker(root);

        for entry in walker {
            let entry = entry.map_err(|e| RagError::indexing_sanitized(e.to_string()))?;
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Check if file should be indexed
            if !self.should_index_file(path) {
                trace!("Skipping file (not indexable)");
                continue;
            }

            // Index the file
            match self.index_file(path) {
                Ok(chunks) => {
                    debug!("Indexed file: {} chunks created", chunks.len());
                    all_chunks.extend(chunks);
                }
                Err(e) => {
                    warn!("Failed to index file: {}", e.to_user_message());
                }
            }
        }

        debug!(
            "Indexing complete: {} total chunks from directory",
            all_chunks.len()
        );

        Ok(all_chunks)
    }

    /// Index a single file and return its chunks.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to index
    ///
    /// # Returns
    ///
    /// A vector of CodeChunk instances for this file.
    pub fn index_file(&self, path: &Path) -> Result<Vec<CodeChunk>, RagError> {
        // Check file size
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > self.config.max_file_size {
            return Err(RagError::IndexingError(
                "File too large to index".to_string(),
            ));
        }

        // Read file content
        let content = std::fs::read_to_string(path)?;

        if content.is_empty() {
            return Ok(Vec::new());
        }

        // Detect language for better chunking
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let _lang_hint = LanguageHint::from_extension(ext);

        // Chunk the content
        let chunks = chunk_code(
            &content,
            self.config.chunking_strategy,
            &self.config.chunking_config,
        )?;

        Ok(chunks)
    }

    /// Index content directly (for testing or when path is not available).
    pub fn index_content(&self, content: &str) -> Result<Vec<CodeChunk>, RagError> {
        if content.is_empty() {
            return Ok(Vec::new());
        }

        chunk_code(
            content,
            self.config.chunking_strategy,
            &self.config.chunking_config,
        )
    }

    /// Check if a file should be indexed.
    fn should_index_file(&self, path: &Path) -> bool {
        // Check exclude patterns
        if self.config.is_excluded(path) {
            return false;
        }

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !self.config.should_index_extension(ext) {
                return false;
            }
        } else {
            // No extension - skip unless it's a known script file
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Common script files without extensions
                if !matches!(
                    name,
                    "Makefile" | "Dockerfile" | "Jenkinsfile" | "Vagrantfile"
                ) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Create a directory walker with appropriate settings.
    fn create_walker(&self, root: &Path) -> impl Iterator<Item = Result<walkdir::DirEntry, walkdir::Error>> {
        walkdir::WalkDir::new(root)
            .follow_links(self.config.follow_symlinks)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories (but not the root directory)
                if e.file_type().is_dir() && e.depth() > 0 {
                    if let Some(name) = e.file_name().to_str() {
                        if name.starts_with('.') {
                            return false;
                        }
                    }
                }
                true
            })
    }

    /// Get statistics about what would be indexed.
    pub fn preview_index(&self, root: &Path) -> IndexPreview {
        let mut preview = IndexPreview::default();

        let walker = self.create_walker(root);

        for entry in walker.flatten() {
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            if self.should_index_file(path) {
                preview.file_count += 1;
                if let Ok(metadata) = std::fs::metadata(path) {
                    preview.total_bytes += metadata.len();
                }
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    *preview.by_extension.entry(ext.to_string()).or_insert(0) += 1;
                }
            } else {
                preview.excluded_count += 1;
            }
        }

        preview
    }
}

impl Default for CodeIndexer {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Preview of what would be indexed.
#[derive(Debug, Default)]
pub struct IndexPreview {
    /// Number of files that would be indexed
    pub file_count: usize,
    /// Number of files excluded
    pub excluded_count: usize,
    /// Total bytes that would be indexed
    pub total_bytes: u64,
    /// File counts by extension
    pub by_extension: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project(dir: &TempDir) -> std::io::Result<()> {
        // Create source files
        fs::write(
            dir.path().join("main.rs"),
            "fn main() { println!(\"Hello\"); }",
        )?;
        fs::write(
            dir.path().join("lib.rs"),
            "pub fn hello() -> &'static str { \"hello\" }",
        )?;

        // Create subdirectory with more files
        let src = dir.path().join("src");
        fs::create_dir(&src)?;
        fs::write(src.join("utils.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }")?;

        // Create files that should be excluded
        let target = dir.path().join("target");
        fs::create_dir(&target)?;
        fs::write(target.join("debug.rs"), "// should be excluded")?;

        let git = dir.path().join(".git");
        fs::create_dir(&git)?;
        fs::write(git.join("config"), "# git config")?;

        Ok(())
    }

    #[test]
    fn test_default_config() {
        let config = IndexerConfig::default();
        assert!(config.extensions.contains("rs"));
        assert!(config.extensions.contains("py"));
        assert!(!config.exclude_patterns.is_empty());
    }

    #[test]
    fn test_is_excluded() {
        let config = IndexerConfig::default();

        assert!(config.is_excluded(Path::new("target/debug/main")));
        assert!(config.is_excluded(Path::new(".git/config")));
        assert!(config.is_excluded(Path::new("node_modules/package/index.js")));
        assert!(!config.is_excluded(Path::new("src/main.rs")));

        // Test absolute paths that should be excluded
        assert!(config.is_excluded(Path::new("/tmp/xyz/target/debug.rs")));
        assert!(config.is_excluded(Path::new("/tmp/xyz/.git/config")));

        // Test absolute paths that should NOT be excluded
        assert!(!config.is_excluded(Path::new("/tmp/xyz/main.rs")));
        assert!(!config.is_excluded(Path::new("/tmp/xyz/src/utils.rs")));
    }

    #[test]
    fn test_should_index_extension() {
        let config = IndexerConfig::default();

        assert!(config.should_index_extension("rs"));
        assert!(config.should_index_extension("py"));
        assert!(config.should_index_extension("js"));
        assert!(!config.should_index_extension("exe"));
        assert!(!config.should_index_extension("dll"));
    }

    #[test]
    fn test_index_content() {
        let indexer = CodeIndexer::default_config();
        let content = r#"
fn main() {
    println!("Hello, world!");
}

fn other() {
    // other function
}
"#;

        let chunks = indexer.index_content(content).unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_index_empty_content() {
        let indexer = CodeIndexer::default_config();
        let chunks = indexer.index_content("").unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_index_directory() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(&temp_dir).unwrap();

        let indexer = CodeIndexer::default_config();
        let chunks = indexer.index_directory(temp_dir.path()).unwrap();

        // Should index main.rs, lib.rs, src/utils.rs
        // Should NOT index target/debug.rs or .git/config
        assert!(chunks.len() >= 3);
    }

    #[test]
    fn test_index_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn test() { assert!(true); }").unwrap();

        let indexer = CodeIndexer::default_config();
        let chunks = indexer.index_file(&file_path).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_preview_index() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(&temp_dir).unwrap();

        let indexer = CodeIndexer::default_config();
        let preview = indexer.preview_index(temp_dir.path());

        assert!(preview.file_count >= 3);
        assert!(preview.excluded_count > 0);
        assert!(preview.total_bytes > 0);
        assert!(preview.by_extension.contains_key("rs"));
    }

    #[test]
    fn test_with_exclude_pattern() {
        let config = IndexerConfig::default()
            .with_exclude_pattern("*.test.rs")
            .unwrap();

        assert!(config.is_excluded(Path::new("foo.test.rs")));
    }

    #[test]
    fn test_with_exclude_patterns() {
        let patterns = vec!["*.test.rs".to_string(), "*_test.go".to_string()];
        let config = IndexerConfig::default()
            .with_exclude_patterns(&patterns)
            .unwrap();

        assert!(config.is_excluded(Path::new("foo.test.rs")));
        assert!(config.is_excluded(Path::new("bar_test.go")));
    }

    #[test]
    fn test_file_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.rs");

        // Create a file larger than the default max size
        let large_content = "x".repeat(MAX_FILE_SIZE as usize + 1);
        fs::write(&file_path, large_content).unwrap();

        let indexer = CodeIndexer::default_config();
        let result = indexer.index_file(&file_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_chunks_have_no_path_info() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("secret_file.rs");
        fs::write(&file_path, "fn secret() {}").unwrap();

        let indexer = CodeIndexer::default_config();
        let chunks = indexer.index_file(&file_path).unwrap();

        // Ensure chunks don't contain path information
        for chunk in chunks {
            assert!(!chunk.id.as_str().contains("secret"));
            assert!(!format!("{:?}", chunk).contains("secret_file"));
        }
    }

    #[test]
    fn test_hidden_directories_skipped() {
        let temp_dir = TempDir::new().unwrap();

        // Create hidden directory with files
        let hidden = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("secret.rs"), "fn secret() {}").unwrap();

        // Create visible directory
        fs::write(temp_dir.path().join("visible.rs"), "fn visible() {}").unwrap();

        let indexer = CodeIndexer::default_config();
        let chunks = indexer.index_directory(temp_dir.path()).unwrap();

        // Should only index visible.rs, not .hidden/secret.rs
        let contents: Vec<_> = chunks.iter().map(|c| c.content.as_str()).collect();
        assert!(contents.iter().any(|c| c.contains("visible")));
        assert!(!contents.iter().any(|c| c.contains("secret")));
    }
}
