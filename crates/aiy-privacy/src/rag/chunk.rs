//! Code chunking strategies for the local RAG system.
//!
//! This module provides different strategies for splitting source code into
//! chunks suitable for embedding and retrieval.
//!
//! # Strategies
//!
//! - **SlidingWindow**: Simple overlapping windows, good for any text
//! - **AstAware**: Attempts to split on semantic boundaries (functions, types)
//! - **Hybrid**: Combines AST-aware with sliding window fallback

use crate::rag::error::RagError;
use crate::rag::types::{ChunkType, CodeChunk};
use regex::Regex;
use std::sync::OnceLock;

/// Configuration for chunking strategies.
#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
    /// Maximum chunk size (hard limit)
    pub max_chunk_size: usize,
    /// Minimum chunk size (won't create smaller chunks)
    pub min_chunk_size: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1500,    // ~375 tokens
            overlap: 200,       // ~50 tokens overlap
            max_chunk_size: 3000, // ~750 tokens max
            min_chunk_size: 100,  // Don't create tiny chunks
        }
    }
}

/// Strategy for chunking code into segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingStrategy {
    /// Simple sliding window with overlap
    SlidingWindow,
    /// AST-aware chunking (splits on semantic boundaries)
    AstAware,
    /// Hybrid: AST-aware with sliding window fallback
    Hybrid,
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        ChunkingStrategy::Hybrid
    }
}

/// Chunk source code using the specified strategy.
///
/// # Arguments
///
/// * `content` - The source code to chunk
/// * `strategy` - The chunking strategy to use
/// * `config` - Chunking configuration
///
/// # Returns
///
/// A vector of CodeChunk instances without embeddings.
pub fn chunk_code(
    content: &str,
    strategy: ChunkingStrategy,
    config: &ChunkingConfig,
) -> Result<Vec<CodeChunk>, RagError> {
    if content.is_empty() {
        return Ok(Vec::new());
    }

    match strategy {
        ChunkingStrategy::SlidingWindow => sliding_window_chunk(content, config),
        ChunkingStrategy::AstAware => ast_aware_chunk(content, config),
        ChunkingStrategy::Hybrid => hybrid_chunk(content, config),
    }
}

/// Simple sliding window chunking with overlap.
fn sliding_window_chunk(content: &str, config: &ChunkingConfig) -> Result<Vec<CodeChunk>, RagError> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let total_len = chars.len();

    if total_len == 0 {
        return Ok(chunks);
    }

    // If content is smaller than chunk size, return as single chunk
    if total_len <= config.chunk_size {
        let lines = count_lines(content);
        chunks.push(CodeChunk::with_lines(
            content.to_string(),
            ChunkType::CodeBlock,
            1,
            lines,
        ));
        return Ok(chunks);
    }

    let step = config.chunk_size.saturating_sub(config.overlap);
    let step = std::cmp::max(step, config.min_chunk_size);

    let mut start = 0;
    let mut line_offset = 0;

    while start < total_len {
        let end = std::cmp::min(start + config.chunk_size, total_len);
        let chunk_content: String = chars[start..end].iter().collect();

        // Adjust to line boundary if possible
        let (adjusted_content, adjusted_end) = adjust_to_line_boundary(&chunk_content, end - start);

        if adjusted_content.len() >= config.min_chunk_size {
            let chunk_lines = count_lines(&adjusted_content);
            chunks.push(CodeChunk::with_lines(
                adjusted_content,
                ChunkType::CodeBlock,
                line_offset + 1,
                line_offset + chunk_lines,
            ));
        }

        // Update line offset based on content before overlap
        let non_overlap_content: String = chars[start..std::cmp::min(start + step, total_len)]
            .iter()
            .collect();
        line_offset += count_lines(&non_overlap_content) - 1;

        // Always advance by step to guarantee progress
        start += step;
    }

    Ok(chunks)
}

/// AST-aware chunking that tries to split on semantic boundaries.
fn ast_aware_chunk(content: &str, config: &ChunkingConfig) -> Result<Vec<CodeChunk>, RagError> {
    let mut chunks = Vec::new();

    // Parse content into semantic blocks
    let blocks = parse_semantic_blocks(content);

    for block in blocks {
        // If block is small enough, use it directly
        if block.content.len() <= config.max_chunk_size {
            if block.content.len() >= config.min_chunk_size {
                chunks.push(CodeChunk::with_lines(
                    block.content,
                    block.chunk_type,
                    block.start_line,
                    block.end_line,
                ));
            }
        } else {
            // Block is too large, split with sliding window
            let sub_chunks = sliding_window_chunk(&block.content, config)?;
            for mut sub_chunk in sub_chunks {
                sub_chunk.chunk_type = block.chunk_type;
                // Adjust line numbers relative to block start
                sub_chunk.start_line += block.start_line.saturating_sub(1);
                sub_chunk.end_line += block.start_line.saturating_sub(1);
                chunks.push(sub_chunk);
            }
        }
    }

    Ok(chunks)
}

/// Hybrid chunking: AST-aware with sliding window fallback.
fn hybrid_chunk(content: &str, config: &ChunkingConfig) -> Result<Vec<CodeChunk>, RagError> {
    // Try AST-aware first
    let ast_chunks = ast_aware_chunk(content, config)?;

    // If we got meaningful chunks, use them
    if !ast_chunks.is_empty() {
        return Ok(ast_chunks);
    }

    // Fallback to sliding window
    sliding_window_chunk(content, config)
}

/// A semantic block parsed from source code.
struct SemanticBlock {
    content: String,
    chunk_type: ChunkType,
    start_line: usize,
    end_line: usize,
}

/// Parse source code into semantic blocks.
fn parse_semantic_blocks(content: &str) -> Vec<SemanticBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return blocks;
    }

    // Patterns for Rust code (primary target)
    static FN_PATTERN: OnceLock<Regex> = OnceLock::new();
    static STRUCT_PATTERN: OnceLock<Regex> = OnceLock::new();
    static ENUM_PATTERN: OnceLock<Regex> = OnceLock::new();
    static IMPL_PATTERN: OnceLock<Regex> = OnceLock::new();
    static MOD_PATTERN: OnceLock<Regex> = OnceLock::new();
    static USE_PATTERN: OnceLock<Regex> = OnceLock::new();
    static TEST_PATTERN: OnceLock<Regex> = OnceLock::new();

    let fn_re = FN_PATTERN.get_or_init(|| {
        Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+\w+").unwrap()
    });
    let struct_re = STRUCT_PATTERN.get_or_init(|| {
        Regex::new(r"^\s*(pub\s+)?struct\s+\w+").unwrap()
    });
    let enum_re = ENUM_PATTERN.get_or_init(|| {
        Regex::new(r"^\s*(pub\s+)?enum\s+\w+").unwrap()
    });
    let impl_re = IMPL_PATTERN.get_or_init(|| {
        Regex::new(r"^\s*impl\s+").unwrap()
    });
    let mod_re = MOD_PATTERN.get_or_init(|| {
        Regex::new(r"^\s*(pub\s+)?mod\s+\w+").unwrap()
    });
    let use_re = USE_PATTERN.get_or_init(|| {
        Regex::new(r"^\s*use\s+").unwrap()
    });
    let test_re = TEST_PATTERN.get_or_init(|| {
        Regex::new(r"#\[test\]|#\[tokio::test\]").unwrap()
    });

    let mut current_block_start = 0;
    let mut current_block_type = ChunkType::CodeBlock;
    let mut brace_depth = 0;
    let mut in_block = false;
    let mut is_test = false;

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        // Check for test attribute
        if test_re.is_match(line) {
            is_test = true;
        }

        // Handle imports block specially: end on first non-use, non-blank line
        if in_block && current_block_type == ChunkType::Imports {
            let is_use_line = use_re.is_match(line);
            let is_blank = line.trim().is_empty();

            if !is_use_line && !is_blank {
                // End the imports block before this line
                let imports_end = i.saturating_sub(1);
                // Find the last non-blank line in the imports block
                let mut actual_end = imports_end;
                while actual_end > current_block_start && lines[actual_end].trim().is_empty() {
                    actual_end -= 1;
                }

                if actual_end >= current_block_start {
                    let block_content: String = lines[current_block_start..=actual_end].join("\n");
                    if !block_content.trim().is_empty() {
                        blocks.push(SemanticBlock {
                            content: block_content,
                            chunk_type: ChunkType::Imports,
                            start_line: current_block_start + 1,
                            end_line: actual_end + 1,
                        });
                    }
                }

                current_block_start = i;
                current_block_type = ChunkType::CodeBlock;
                in_block = false;
                brace_depth = 0;
                // Don't continue - fall through to check if this line starts a new block
            }
        }

        // Detect block start
        if !in_block {
            let block_type = if fn_re.is_match(line) {
                Some(if is_test {
                    ChunkType::Test
                } else {
                    ChunkType::Function
                })
            } else if struct_re.is_match(line) || enum_re.is_match(line) {
                Some(ChunkType::TypeDefinition)
            } else if impl_re.is_match(line) {
                Some(ChunkType::ImplBlock)
            } else if mod_re.is_match(line) {
                Some(ChunkType::Module)
            } else if use_re.is_match(line) {
                Some(ChunkType::Imports)
            } else {
                None
            };

            if let Some(bt) = block_type {
                // Save any pending content before this block
                if current_block_start < i && i > 0 {
                    let pending_content: String = lines[current_block_start..i].join("\n");
                    if !pending_content.trim().is_empty() {
                        blocks.push(SemanticBlock {
                            content: pending_content,
                            chunk_type: ChunkType::CodeBlock,
                            start_line: current_block_start + 1,
                            end_line: i,
                        });
                    }
                }

                current_block_start = i;
                current_block_type = bt;
                in_block = true;
                is_test = false;
                // Reset brace depth when starting a new block
                brace_depth = 0;
            }
        }

        // Track brace depth only when inside a block
        if in_block {
            brace_depth += line.chars().filter(|&c| c == '{').count() as i32;
            brace_depth -= line.chars().filter(|&c| c == '}').count() as i32;
        }

        // Detect block end
        if in_block && brace_depth == 0 && line.contains('}') {
            let block_content: String = lines[current_block_start..=i].join("\n");
            blocks.push(SemanticBlock {
                content: block_content,
                chunk_type: current_block_type,
                start_line: current_block_start + 1,
                end_line: line_num,
            });

            current_block_start = i + 1;
            current_block_type = ChunkType::CodeBlock;
            in_block = false;
            brace_depth = 0;
        }
    }

    // Handle remaining content
    if current_block_start < lines.len() {
        let remaining: String = lines[current_block_start..].join("\n");
        if !remaining.trim().is_empty() {
            blocks.push(SemanticBlock {
                content: remaining,
                chunk_type: current_block_type,
                start_line: current_block_start + 1,
                end_line: lines.len(),
            });
        }
    }

    blocks
}

/// Adjust chunk boundary to nearest line boundary.
fn adjust_to_line_boundary(content: &str, max_len: usize) -> (String, usize) {
    if content.len() <= max_len {
        return (content.to_string(), content.len());
    }

    // Find last newline before max_len
    let truncated = &content[..max_len];
    if let Some(last_newline) = truncated.rfind('\n') {
        (content[..=last_newline].to_string(), last_newline + 1)
    } else {
        (content.to_string(), content.len())
    }
}

/// Count the number of lines in a string.
fn count_lines(content: &str) -> usize {
    if content.is_empty() {
        return 0;
    }
    content.lines().count()
}

/// Detect the programming language from content.
///
/// Returns a hint for adjusting chunking behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageHint {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    Unknown,
}

impl LanguageHint {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => LanguageHint::Rust,
            "py" | "pyw" => LanguageHint::Python,
            "js" | "mjs" | "cjs" => LanguageHint::JavaScript,
            "ts" | "tsx" => LanguageHint::TypeScript,
            "go" => LanguageHint::Go,
            "java" => LanguageHint::Java,
            "c" | "h" => LanguageHint::C,
            "cpp" | "cxx" | "cc" | "hpp" => LanguageHint::Cpp,
            _ => LanguageHint::Unknown,
        }
    }

    /// Detect language from content heuristics.
    pub fn from_content(content: &str) -> Self {
        // Quick heuristics based on common patterns
        if content.contains("fn ") && (content.contains("-> ") || content.contains("pub ")) {
            return LanguageHint::Rust;
        }
        if content.contains("def ") && content.contains(":") && !content.contains(";") {
            return LanguageHint::Python;
        }
        if content.contains("func ") && content.contains("package ") {
            return LanguageHint::Go;
        }
        if content.contains("interface ") && content.contains("implements ") {
            return LanguageHint::Java;
        }
        if content.contains("const ") || content.contains("let ") {
            if content.contains(": ") && content.contains("=>") {
                return LanguageHint::TypeScript;
            }
            return LanguageHint::JavaScript;
        }

        LanguageHint::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_small_content() {
        let content = "fn main() { println!(\"Hello\"); }";
        let config = ChunkingConfig::default();
        let chunks = sliding_window_chunk(content, &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, content);
    }

    #[test]
    fn test_sliding_window_large_content() {
        // Use content with newlines to test realistic chunking behavior
        let lines: Vec<String> = (0..50).map(|i| format!("line {} content here", i)).collect();
        let content = lines.join("\n");
        let config = ChunkingConfig {
            chunk_size: 100,
            overlap: 10,
            max_chunk_size: 200,
            min_chunk_size: 50,
        };
        let chunks = sliding_window_chunk(&content, &config).unwrap();

        assert!(chunks.len() > 1, "Expected multiple chunks for multi-line input with chunk_size=100");
        // Each chunk should be roughly chunk_size
        for chunk in &chunks {
            assert!(chunk.content.len() <= config.max_chunk_size);
        }
    }

    #[test]
    fn test_ast_aware_function() {
        let content = r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}
"#;
        // Use small min_chunk_size to test AST parsing of small functions
        let config = ChunkingConfig {
            min_chunk_size: 10,
            ..ChunkingConfig::default()
        };
        let chunks = ast_aware_chunk(content, &config).unwrap();

        // Should detect at least 2 functions
        let fn_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.chunk_type == ChunkType::Function)
            .collect();
        assert!(fn_chunks.len() >= 2, "Expected at least 2 function chunks, got {}", fn_chunks.len());
    }

    #[test]
    fn test_ast_aware_struct() {
        let content = r#"
pub struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
}
"#;
        // Use small min_chunk_size to test AST parsing
        let config = ChunkingConfig {
            min_chunk_size: 10,
            ..ChunkingConfig::default()
        };
        let chunks = ast_aware_chunk(content, &config).unwrap();

        let has_struct = chunks.iter().any(|c| c.chunk_type == ChunkType::TypeDefinition);
        let has_impl = chunks.iter().any(|c| c.chunk_type == ChunkType::ImplBlock);

        assert!(has_struct, "Should detect struct");
        assert!(has_impl, "Should detect impl block");
    }

    #[test]
    fn test_ast_aware_test_function() {
        let content = r#"
#[test]
fn test_something() {
    assert!(true);
}
"#;
        // Use small min_chunk_size to test AST parsing
        let config = ChunkingConfig {
            min_chunk_size: 10,
            ..ChunkingConfig::default()
        };
        let chunks = ast_aware_chunk(content, &config).unwrap();

        let test_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.chunk_type == ChunkType::Test)
            .collect();
        assert!(!test_chunks.is_empty(), "Should detect test function");
    }

    #[test]
    fn test_hybrid_with_rust_code() {
        let content = r#"
use std::collections::HashMap;

pub struct Cache {
    data: HashMap<String, String>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

#[test]
fn test_cache() {
    let cache = Cache::new();
    assert!(cache.data.is_empty());
}
"#;
        // Use small min_chunk_size to test AST parsing
        let config = ChunkingConfig {
            min_chunk_size: 10,
            ..ChunkingConfig::default()
        };
        let chunks = hybrid_chunk(content, &config).unwrap();

        assert!(!chunks.is_empty());

        // Check we have diverse chunk types
        let types: Vec<_> = chunks.iter().map(|c| c.chunk_type).collect();
        assert!(types.contains(&ChunkType::TypeDefinition));
        assert!(types.contains(&ChunkType::ImplBlock));
    }

    #[test]
    fn test_empty_content() {
        let config = ChunkingConfig::default();

        let chunks = chunk_code("", ChunkingStrategy::SlidingWindow, &config).unwrap();
        assert!(chunks.is_empty());

        let chunks = chunk_code("", ChunkingStrategy::AstAware, &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines(""), 0);
        assert_eq!(count_lines("one line"), 1);
        assert_eq!(count_lines("line1\nline2\nline3"), 3);
    }

    #[test]
    fn test_language_hint_from_extension() {
        assert_eq!(LanguageHint::from_extension("rs"), LanguageHint::Rust);
        assert_eq!(LanguageHint::from_extension("py"), LanguageHint::Python);
        assert_eq!(LanguageHint::from_extension("js"), LanguageHint::JavaScript);
        assert_eq!(LanguageHint::from_extension("ts"), LanguageHint::TypeScript);
        assert_eq!(LanguageHint::from_extension("go"), LanguageHint::Go);
        assert_eq!(LanguageHint::from_extension("unknown"), LanguageHint::Unknown);
    }

    #[test]
    fn test_language_hint_from_content() {
        let rust = "pub fn main() -> Result<(), Error> { Ok(()) }";
        assert_eq!(LanguageHint::from_content(rust), LanguageHint::Rust);

        let python = "def hello():\n    print('hello')";
        assert_eq!(LanguageHint::from_content(python), LanguageHint::Python);
    }

    #[test]
    fn test_chunk_has_line_info() {
        let content = "fn test() {\n    // line 2\n    // line 3\n}";
        let config = ChunkingConfig::default();
        let chunks = ast_aware_chunk(content, &config).unwrap();

        for chunk in chunks {
            assert!(chunk.start_line > 0);
            assert!(chunk.end_line >= chunk.start_line);
        }
    }

    #[test]
    fn test_config_defaults() {
        let config = ChunkingConfig::default();
        assert!(config.chunk_size > 0);
        assert!(config.overlap < config.chunk_size);
        assert!(config.max_chunk_size >= config.chunk_size);
        assert!(config.min_chunk_size < config.chunk_size);
    }
}
