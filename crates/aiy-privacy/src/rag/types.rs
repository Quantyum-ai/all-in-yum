//! Core types for the local RAG (Retrieval-Augmented Generation) system.
//!
//! This module defines the fundamental types used throughout the RAG system.
//! All types are designed for in-memory-only operation to maintain privacy.
//!
//! # Security
//!
//! - **No file paths stored**: Only opaque IDs reference content
//! - **In-memory only**: Nothing persists to disk
//! - **Session-scoped**: All data destroyed when session ends

use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Opaque identifier for a code chunk.
///
/// This ID provides no information about the underlying file path or content.
/// It is only meaningful within the current session's in-memory store.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(String);

impl ChunkId {
    /// Create a new opaque chunk ID.
    ///
    /// The ID is generated using UUID v4 to ensure uniqueness without
    /// revealing any information about the source content.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create a chunk ID from an existing string (for testing).
    ///
    /// # Security
    ///
    /// This should only be used in tests. Production code should use `new()`.
    #[cfg(test)]
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ChunkId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chunk:{}", &self.0[..8])
    }
}

/// Type of code chunk, used for chunking strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ChunkType {
    /// A function or method definition
    Function,
    /// A struct, enum, or type definition
    TypeDefinition,
    /// An impl block
    ImplBlock,
    /// A module declaration
    Module,
    /// Import/use statements
    Imports,
    /// Documentation comments
    Documentation,
    /// Generic code block (fallback)
    #[default]
    CodeBlock,
    /// Test code
    Test,
}

impl ChunkType {
    /// Get a human-readable description of this chunk type.
    pub fn description(&self) -> &'static str {
        match self {
            ChunkType::Function => "function",
            ChunkType::TypeDefinition => "type definition",
            ChunkType::ImplBlock => "impl block",
            ChunkType::Module => "module",
            ChunkType::Imports => "imports",
            ChunkType::Documentation => "documentation",
            ChunkType::CodeBlock => "code block",
            ChunkType::Test => "test",
        }
    }
}

/// A chunk of code with its embedding vector.
///
/// # Security
///
/// - Content is zeroized on drop to prevent memory inspection
/// - No file path information is stored
/// - Only the opaque ChunkId can reference this chunk
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct CodeChunk {
    /// Opaque identifier (not zeroized as it contains no sensitive data)
    #[zeroize(skip)]
    pub id: ChunkId,

    /// The actual code content (zeroized on drop)
    pub content: String,

    /// Type of this chunk
    #[zeroize(skip)]
    pub chunk_type: ChunkType,

    /// Embedding vector (zeroized on drop as it encodes content)
    pub embedding: Vec<f32>,

    /// Approximate token count for budget tracking
    #[zeroize(skip)]
    pub token_count: usize,

    /// Start line in original file (relative, not absolute path info)
    #[zeroize(skip)]
    pub start_line: usize,

    /// End line in original file
    #[zeroize(skip)]
    pub end_line: usize,
}

impl CodeChunk {
    /// Create a new code chunk without an embedding.
    ///
    /// The embedding should be computed separately using the LocalEmbedder.
    pub fn new(content: String, chunk_type: ChunkType) -> Self {
        let token_count = estimate_tokens(&content);
        Self {
            id: ChunkId::new(),
            content,
            chunk_type,
            embedding: Vec::new(),
            token_count,
            start_line: 0,
            end_line: 0,
        }
    }

    /// Create a chunk with line information.
    pub fn with_lines(
        content: String,
        chunk_type: ChunkType,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        let token_count = estimate_tokens(&content);
        Self {
            id: ChunkId::new(),
            content,
            chunk_type,
            embedding: Vec::new(),
            token_count,
            start_line,
            end_line,
        }
    }

    /// Set the embedding vector for this chunk.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = embedding;
        self
    }

    /// Check if this chunk has an embedding computed.
    pub fn has_embedding(&self) -> bool {
        !self.embedding.is_empty()
    }
}

impl fmt::Debug for CodeChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Intentionally omit content to prevent accidental logging
        f.debug_struct("CodeChunk")
            .field("id", &self.id)
            .field("chunk_type", &self.chunk_type)
            .field("token_count", &self.token_count)
            .field("has_embedding", &self.has_embedding())
            .field("lines", &format!("{}-{}", self.start_line, self.end_line))
            .finish()
    }
}

/// A ranked chunk returned from similarity search.
///
/// Contains the chunk along with its similarity score.
#[derive(Debug, Clone)]
pub struct RankedChunk {
    /// The chunk ID (content can be retrieved from store)
    pub id: ChunkId,

    /// The actual content (for convenience, already retrieved)
    pub content: String,

    /// The chunk type
    pub chunk_type: ChunkType,

    /// Cosine similarity score (0.0 to 1.0)
    pub similarity: f32,

    /// Token count for budget tracking
    pub token_count: usize,
}

impl RankedChunk {
    /// Create a new ranked chunk from a CodeChunk and similarity score.
    pub fn from_chunk(chunk: &CodeChunk, similarity: f32) -> Self {
        Self {
            id: chunk.id.clone(),
            content: chunk.content.clone(),
            chunk_type: chunk.chunk_type,
            similarity,
            token_count: chunk.token_count,
        }
    }
}

/// Estimate the number of tokens in a string.
///
/// Uses a simple heuristic: ~4 characters per token for code.
/// This is a rough approximation that works well for most code.
pub fn estimate_tokens(content: &str) -> usize {
    // For code, we use a more conservative estimate
    // Average English word is ~5 chars, average token is ~4 chars
    // Code has more symbols which are often individual tokens
    let char_count = content.chars().count();

    // Count whitespace-separated "words" and symbols
    let words: usize = content.split_whitespace().count();

    // Symbols that are typically individual tokens
    let symbols = content
        .chars()
        .filter(|c| matches!(c, '(' | ')' | '{' | '}' | '[' | ']' | ';' | ':' | ',' | '.' | '+' | '-' | '*' | '/' | '=' | '<' | '>' | '&' | '|' | '!' | '@' | '#' | '$' | '%' | '^'))
        .count();

    // Conservative estimate: max of word count + symbols, or chars/4
    std::cmp::max(words + symbols / 2, char_count / 4)
}

/// Embedding dimension for the default model (nomic-embed-text).
///
/// Different models may have different dimensions:
/// - nomic-embed-text: 768
/// - mxbai-embed-large: 1024
/// - all-minilm: 384
pub const DEFAULT_EMBEDDING_DIM: usize = 768;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_id_uniqueness() {
        let id1 = ChunkId::new();
        let id2 = ChunkId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_chunk_id_display() {
        let id = ChunkId::from_string("12345678-1234-1234-1234-123456789012");
        let display = format!("{}", id);
        assert!(display.starts_with("chunk:"));
        assert_eq!(display.len(), 14); // "chunk:" + 8 chars
    }

    #[test]
    fn test_code_chunk_creation() {
        let chunk = CodeChunk::new("fn test() {}".to_string(), ChunkType::Function);
        assert!(!chunk.has_embedding());
        assert_eq!(chunk.chunk_type, ChunkType::Function);
        assert!(chunk.token_count > 0);
    }

    #[test]
    fn test_code_chunk_with_embedding() {
        let embedding = vec![0.1, 0.2, 0.3];
        let chunk = CodeChunk::new("fn test() {}".to_string(), ChunkType::Function)
            .with_embedding(embedding.clone());
        assert!(chunk.has_embedding());
        assert_eq!(chunk.embedding.len(), 3);
    }

    #[test]
    fn test_code_chunk_with_lines() {
        let chunk = CodeChunk::with_lines(
            "fn test() {}".to_string(),
            ChunkType::Function,
            10,
            15,
        );
        assert_eq!(chunk.start_line, 10);
        assert_eq!(chunk.end_line, 15);
    }

    #[test]
    fn test_estimate_tokens() {
        // Simple function
        let simple = "fn main() {}";
        let tokens = estimate_tokens(simple);
        assert!(tokens > 0 && tokens < 20);

        // Longer code
        let longer = r#"
            pub fn calculate_sum(numbers: &[i32]) -> i32 {
                numbers.iter().sum()
            }
        "#;
        let longer_tokens = estimate_tokens(longer);
        assert!(longer_tokens > tokens);
    }

    #[test]
    fn test_ranked_chunk_from_chunk() {
        let chunk = CodeChunk::new("fn test() {}".to_string(), ChunkType::Function);
        let ranked = RankedChunk::from_chunk(&chunk, 0.95);

        assert_eq!(ranked.id, chunk.id);
        assert_eq!(ranked.content, chunk.content);
        assert!((ranked.similarity - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_chunk_type_description() {
        assert_eq!(ChunkType::Function.description(), "function");
        assert_eq!(ChunkType::TypeDefinition.description(), "type definition");
        assert_eq!(ChunkType::Test.description(), "test");
    }

    #[test]
    fn test_code_chunk_debug_omits_content() {
        let chunk = CodeChunk::new("SECRET_CODE".to_string(), ChunkType::Function);
        let debug_output = format!("{:?}", chunk);
        assert!(!debug_output.contains("SECRET_CODE"));
    }
}
