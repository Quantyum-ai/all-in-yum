//! Local RAG (Retrieval-Augmented Generation) system for privacy mode.
//!
//! This module provides a complete RAG system that keeps all code local:
//! - Indexes code from directories
//! - Generates embeddings using local Ollama
//! - Stores vectors in-memory only (never persisted)
//! - Retrieves relevant code for LLM context
//!
//! # Security
//!
//! - **NO file paths stored**: Only opaque IDs reference content
//! - **In-memory only**: Nothing persists to disk
//! - **Zeroize on drop**: Sensitive data zeroed when system is dropped
//!
//! # Example
//!
//! ```rust,ignore
//! use aiy_privacy::rag::{RagSystem, RagSystemConfig};
//! use std::path::Path;
//!
//! // Create RAG system
//! let config = RagSystemConfig::default();
//! let mut rag = RagSystem::new(config).await?;
//!
//! // Index a project
//! rag.index_directory(Path::new("/path/to/project")).await?;
//!
//! // Query for relevant code
//! let context = rag.query("How does authentication work?").await?;
//! ```

pub mod chunk;
pub mod embedder;
pub mod error;
pub mod indexer;
pub mod query;
pub mod store;
#[cfg(test)]
pub mod test_support;
pub mod types;

pub use chunk::{chunk_code, ChunkingConfig, ChunkingStrategy};
pub use embedder::{Embedder, EmbeddingTransport, LocalEmbedder, MockEmbeddingTransport};
pub use error::RagError;
pub use indexer::{CodeIndexer, IndexerConfig, IndexPreview};
pub use query::{QueryBuilder, QueryProcessor, QueryResult};
pub use store::{cosine_similarity, normalize_vector, InMemoryVectorStore, StoreStats};
pub use types::{ChunkId, ChunkType, CodeChunk, RankedChunk, DEFAULT_EMBEDDING_DIM};

use aiy_core::config::{PrivacyModeConfig, RagConfig};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Configuration for the RAG system.
#[derive(Debug, Clone)]
pub struct RagSystemConfig {
    /// Ollama base URL
    pub ollama_url: String,
    /// Embedding model name
    pub embedding_model: String,
    /// RAG configuration (token budget, top_k, min_similarity)
    pub rag_config: RagConfig,
    /// Indexer configuration
    pub indexer_config: IndexerConfig,
}

impl Default for RagSystemConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            rag_config: RagConfig::default(),
            indexer_config: IndexerConfig::default(),
        }
    }
}

impl RagSystemConfig {
    /// Create from privacy mode configuration.
    pub fn from_privacy_config(privacy: &PrivacyModeConfig) -> Result<Self, RagError> {
        let indexer_config = IndexerConfig::default()
            .with_exclude_patterns(&privacy.exclude_patterns)?;

        Ok(Self {
            ollama_url: privacy.local_executor.ollama_url.clone(),
            embedding_model: "nomic-embed-text".to_string(),
            rag_config: privacy.rag.clone(),
            indexer_config,
        })
    }

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Set exclude patterns.
    pub fn with_exclude_patterns(mut self, patterns: &[String]) -> Result<Self, RagError> {
        self.indexer_config = self.indexer_config.with_exclude_patterns(patterns)?;
        Ok(self)
    }
}

/// Complete RAG system coordinator.
///
/// This struct coordinates all RAG components:
/// - Indexer: Walks directories and creates chunks
/// - Embedder: Generates embeddings for chunks and queries
/// - Store: Holds vectors in memory
/// - Query Processor: Searches for relevant code
///
/// # Security
///
/// When the RagSystem is dropped, all in-memory data is zeroized.
pub struct RagSystem {
    /// Vector store
    store: InMemoryVectorStore,
    /// Embedder for generating embeddings
    embedder: Arc<dyn Embedder>,
    /// Query processor
    query_processor: QueryProcessor,
    /// Indexer for directory walking
    indexer: CodeIndexer,
    /// Configuration
    config: RagSystemConfig,
    /// Whether the system has been indexed
    indexed: bool,
}

impl RagSystem {
    /// Create a new RAG system with mock transport (for testing).
    pub fn with_mock_embedder(
        config: RagSystemConfig,
        embedder: Arc<dyn Embedder>,
    ) -> Self {
        let query_processor = QueryProcessor::new(embedder.clone(), config.rag_config.clone());
        let indexer = CodeIndexer::new(config.indexer_config.clone());
        let store = InMemoryVectorStore::with_dimension(embedder.dimension());

        Self {
            store,
            embedder,
            query_processor,
            indexer,
            config,
            indexed: false,
        }
    }

    /// Create a new RAG system with real HTTP transport.
    #[cfg(feature = "http")]
    pub fn new(config: RagSystemConfig) -> Result<Self, RagError> {
        let embedder = Arc::new(LocalEmbedder::new(
            &config.ollama_url,
            &config.embedding_model,
        )?);

        let query_processor = QueryProcessor::new(embedder.clone(), config.rag_config.clone());
        let indexer = CodeIndexer::new(config.indexer_config.clone());
        let store = InMemoryVectorStore::with_dimension(embedder.dimension());

        Ok(Self {
            store,
            embedder,
            query_processor,
            indexer,
            config,
            indexed: false,
        })
    }

    /// Get the RAG system configuration.
    pub fn config(&self) -> &RagSystemConfig {
        &self.config
    }

    /// Get store statistics.
    pub fn stats(&self) -> StoreStats {
        self.store.stats()
    }

    /// Check if the system has indexed any content.
    pub fn is_indexed(&self) -> bool {
        self.indexed && !self.store.is_empty()
    }

    /// Preview what would be indexed in a directory.
    pub fn preview_index(&self, root: &Path) -> IndexPreview {
        self.indexer.preview_index(root)
    }

    /// Index a directory and store chunks with embeddings.
    ///
    /// This is the main entry point for indexing code:
    /// 1. Walks the directory and creates chunks
    /// 2. Generates embeddings for each chunk
    /// 3. Stores chunks in the vector store
    pub async fn index_directory(&mut self, root: &Path) -> Result<IndexingResult, RagError> {
        info!("Starting directory indexing");

        // Step 1: Create chunks from files
        let chunks = self.indexer.index_directory(root)?;
        debug!("Created {} chunks from directory", chunks.len());

        if chunks.is_empty() {
            return Ok(IndexingResult {
                chunks_indexed: 0,
                tokens_indexed: 0,
                files_processed: 0,
            });
        }

        // Step 2: Generate embeddings
        let embedded_chunks = self.embed_chunks(chunks).await?;
        debug!("Generated embeddings for {} chunks", embedded_chunks.len());

        // Step 3: Store in vector store
        let mut tokens_indexed = 0;
        for chunk in embedded_chunks {
            tokens_indexed += chunk.token_count;
            self.store.insert(chunk)?;
        }

        self.indexed = true;

        let result = IndexingResult {
            chunks_indexed: self.store.len(),
            tokens_indexed,
            files_processed: self.store.len(), // Approximate
        };

        info!(
            "Indexing complete: {} chunks, {} tokens",
            result.chunks_indexed, result.tokens_indexed
        );

        Ok(result)
    }

    /// Index content directly (useful for testing or single files).
    pub async fn index_content(&mut self, content: &str) -> Result<IndexingResult, RagError> {
        let chunks = self.indexer.index_content(content)?;

        if chunks.is_empty() {
            return Ok(IndexingResult {
                chunks_indexed: 0,
                tokens_indexed: 0,
                files_processed: 0,
            });
        }

        let embedded_chunks = self.embed_chunks(chunks).await?;

        let mut tokens_indexed = 0;
        for chunk in embedded_chunks {
            tokens_indexed += chunk.token_count;
            self.store.insert(chunk)?;
        }

        self.indexed = true;

        Ok(IndexingResult {
            chunks_indexed: self.store.len(),
            tokens_indexed,
            files_processed: 1,
        })
    }

    /// Query for relevant code.
    ///
    /// Returns chunks that are semantically similar to the query.
    pub async fn query(&self, query: &str) -> Result<QueryResult, RagError> {
        if !self.is_indexed() {
            return Err(RagError::NoChunksAvailable);
        }

        self.query_processor.query(query, &self.store).await
    }

    /// Query with multiple search terms.
    pub async fn multi_query(&self, queries: &[&str]) -> Result<QueryResult, RagError> {
        if !self.is_indexed() {
            return Err(RagError::NoChunksAvailable);
        }

        self.query_processor.multi_query(queries, &self.store).await
    }

    /// Get a query builder for complex queries.
    pub fn query_builder(&self) -> QueryBuilder {
        QueryBuilder::new()
    }

    /// Execute a query builder.
    pub async fn execute_query(&self, builder: QueryBuilder) -> Result<QueryResult, RagError> {
        if !self.is_indexed() {
            return Err(RagError::NoChunksAvailable);
        }

        builder.execute(&self.query_processor, &self.store).await
    }

    /// Clear all indexed data.
    ///
    /// All data is zeroized.
    pub fn clear(&mut self) {
        self.store.clear();
        self.indexed = false;
    }

    /// Helper to embed chunks with the configured embedder.
    async fn embed_chunks(&self, chunks: Vec<CodeChunk>) -> Result<Vec<CodeChunk>, RagError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // For now, embed one at a time (Ollama doesn't support batch)
        let mut embedded = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            let embedding = self.embedder.embed(&chunk.content).await?;
            embedded.push(chunk.with_embedding(embedding));
        }

        Ok(embedded)
    }
}

impl Drop for RagSystem {
    fn drop(&mut self) {
        // Store handles its own zeroization via Drop
        self.indexed = false;
    }
}

/// Result of an indexing operation.
#[derive(Debug, Clone)]
pub struct IndexingResult {
    /// Number of chunks indexed
    pub chunks_indexed: usize,
    /// Total tokens indexed
    pub tokens_indexed: usize,
    /// Number of files processed
    pub files_processed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::test_support::TestEmbedder;

    fn create_test_rag() -> RagSystem {
        let mut config = RagSystemConfig::default();
        // Lower threshold for bag-of-words sparse embeddings
        config.rag_config.min_similarity = 0.0;
        let embedder = Arc::new(TestEmbedder::new(768));

        RagSystem::with_mock_embedder(config, embedder)
    }

    #[tokio::test]
    async fn test_rag_system_creation() {
        let rag = create_test_rag();
        assert!(!rag.is_indexed());
        assert_eq!(rag.stats().total_chunks, 0);
    }

    #[tokio::test]
    async fn test_index_content() {
        let mut rag = create_test_rag();

        let content = r#"
fn hello() {
    println!("Hello, world!");
}

fn goodbye() {
    println!("Goodbye!");
}
"#;

        let result = rag.index_content(content).await.unwrap();
        assert!(result.chunks_indexed > 0);
        assert!(rag.is_indexed());
    }

    #[tokio::test]
    async fn test_query_after_indexing() {
        let mut rag = create_test_rag();

        let content = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;

        rag.index_content(content).await.unwrap();

        let result = rag.query("add two numbers").await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_query_before_indexing() {
        let rag = create_test_rag();
        let result = rag.query("test").await;
        assert!(matches!(result, Err(RagError::NoChunksAvailable)));
    }

    #[tokio::test]
    async fn test_clear() {
        let mut rag = create_test_rag();

        rag.index_content("fn test() {}").await.unwrap();
        assert!(rag.is_indexed());

        rag.clear();
        assert!(!rag.is_indexed());
        assert_eq!(rag.stats().total_chunks, 0);
    }

    #[tokio::test]
    async fn test_multi_query() {
        let mut rag = create_test_rag();

        let content = r#"
fn add(a: i32, b: i32) -> i32 { a + b }
fn subtract(a: i32, b: i32) -> i32 { a - b }
fn multiply(a: i32, b: i32) -> i32 { a * b }
"#;

        rag.index_content(content).await.unwrap();

        let result = rag.multi_query(&["add", "subtract"]).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_query_builder() {
        let mut rag = create_test_rag();

        rag.index_content("fn test() { let x = 1; }").await.unwrap();

        let builder = rag.query_builder().query("test").top_k(3);

        let result = rag.execute_query(builder).await.unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_config_from_privacy_config() {
        let privacy_config = PrivacyModeConfig {
            enabled: true,
            exclude_patterns: vec!["custom/**".to_string()],
            ..Default::default()
        };

        let config = RagSystemConfig::from_privacy_config(&privacy_config).unwrap();
        assert_eq!(config.ollama_url, privacy_config.local_executor.ollama_url);
    }

    #[test]
    fn test_config_with_embedding_model() {
        let config = RagSystemConfig::default().with_embedding_model("mxbai-embed-large");

        assert_eq!(config.embedding_model, "mxbai-embed-large");
    }

    #[tokio::test]
    async fn test_stats_after_indexing() {
        let mut rag = create_test_rag();

        rag.index_content(
            "fn hello() { println!(\"Hello\"); } fn world() { println!(\"World\"); }",
        )
        .await
        .unwrap();

        let stats = rag.stats();
        assert!(stats.total_chunks > 0);
        assert!(stats.total_tokens > 0);
    }

    #[test]
    fn test_preview_index() {
        let rag = create_test_rag();

        // Create temp directory would be needed for real test
        // For now, just verify the method exists and returns
        let temp_dir = std::env::temp_dir();
        let preview = rag.preview_index(&temp_dir);

        // Verify file_count is a valid usize (existence check - always true since usize)
        let _ = preview.file_count;
    }
}
