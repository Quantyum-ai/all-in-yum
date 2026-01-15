//! Comprehensive tests for the RAG (Retrieval-Augmented Generation) system.
//!
//! These tests are designed to be offline-safe and do not require a running
//! Ollama instance.

use aiy_privacy::rag::{
    chunk::{chunk_code, ChunkingConfig, ChunkingStrategy, LanguageHint},
    embedder::{Embedder, LocalEmbedder, MockEmbeddingTransport},
    error::RagError,
    indexer::{CodeIndexer, IndexerConfig},
    query::{QueryBuilder, QueryProcessor, QueryResult},
    store::{cosine_similarity, normalize_vector, InMemoryVectorStore},
    types::{estimate_tokens, ChunkId, ChunkType, CodeChunk, RankedChunk, DEFAULT_EMBEDDING_DIM},
    RagSystem, RagSystemConfig,
};
use aiy_core::config::RagConfig;
use std::sync::Arc;
use tempfile::TempDir;

// =============================================================================
// Types Tests
// =============================================================================

#[test]
fn test_chunk_id_is_opaque() {
    let id1 = ChunkId::new();
    let id2 = ChunkId::new();

    // IDs should be unique
    assert_ne!(id1, id2);

    // ID should not contain any path-like information
    assert!(!id1.as_str().contains('/'));
    assert!(!id1.as_str().contains('\\'));
    assert!(!id1.as_str().contains(".rs"));
}

#[test]
fn test_code_chunk_zeroize_content_not_leaked() {
    let chunk = CodeChunk::new("SECRET_CONTENT".to_string(), ChunkType::Function);

    // Debug output should not contain content
    let debug = format!("{:?}", chunk);
    assert!(
        !debug.contains("SECRET_CONTENT"),
        "Debug should not leak content"
    );
}

#[test]
fn test_estimate_tokens_reasonable() {
    // Empty string
    assert_eq!(estimate_tokens(""), 0);

    // Simple function
    let simple = "fn main() {}";
    let tokens = estimate_tokens(simple);
    assert!(tokens > 0 && tokens < 20, "Expected 1-20 tokens, got {}", tokens);

    // Longer code should have more tokens
    let longer = "fn calculate_sum(numbers: &[i32]) -> i32 { numbers.iter().sum() }";
    assert!(estimate_tokens(longer) > estimate_tokens(simple));
}

#[test]
fn test_ranked_chunk_preserves_similarity() {
    let chunk = CodeChunk::new("fn test() {}".to_string(), ChunkType::Function);
    let ranked = RankedChunk::from_chunk(&chunk, 0.95);

    assert!((ranked.similarity - 0.95).abs() < 0.001);
    assert_eq!(ranked.chunk_type, ChunkType::Function);
}

// =============================================================================
// Chunking Tests
// =============================================================================

#[test]
fn test_sliding_window_creates_overlapping_chunks() {
    let content = "x".repeat(3000);
    let config = ChunkingConfig {
        chunk_size: 1000,
        overlap: 200,
        max_chunk_size: 1500,
        min_chunk_size: 100,
    };

    let chunks = chunk_code(&content, ChunkingStrategy::SlidingWindow, &config).unwrap();

    // Should have multiple chunks
    assert!(chunks.len() > 1, "Expected multiple chunks");

    // No chunk should exceed max size
    for chunk in &chunks {
        assert!(
            chunk.content.len() <= config.max_chunk_size,
            "Chunk exceeds max size"
        );
    }
}

#[test]
fn test_ast_aware_detects_functions() {
    let content = r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}
"#;

    let config = ChunkingConfig::default();
    let chunks = chunk_code(&content, ChunkingStrategy::AstAware, &config).unwrap();

    let function_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.chunk_type == ChunkType::Function)
        .collect();

    assert!(
        function_chunks.len() >= 2,
        "Expected at least 2 function chunks, got {}",
        function_chunks.len()
    );
}

#[test]
fn test_ast_aware_detects_structs() {
    let content = r#"
pub struct Person {
    name: String,
    age: u32,
}

pub enum Status {
    Active,
    Inactive,
}
"#;

    let config = ChunkingConfig::default();
    let chunks = chunk_code(&content, ChunkingStrategy::AstAware, &config).unwrap();

    let type_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.chunk_type == ChunkType::TypeDefinition)
        .collect();

    assert!(
        !type_chunks.is_empty(),
        "Expected type definition chunks"
    );
}

#[test]
fn test_hybrid_strategy_falls_back() {
    // Non-Rust code should still be chunked
    let content = "random text that doesn't look like code but is still content";

    let config = ChunkingConfig::default();
    let chunks = chunk_code(&content, ChunkingStrategy::Hybrid, &config).unwrap();

    assert!(!chunks.is_empty(), "Hybrid should chunk any content");
}

#[test]
fn test_language_hint_detection() {
    assert_eq!(LanguageHint::from_extension("rs"), LanguageHint::Rust);
    assert_eq!(LanguageHint::from_extension("py"), LanguageHint::Python);
    assert_eq!(LanguageHint::from_extension("js"), LanguageHint::JavaScript);
    assert_eq!(LanguageHint::from_extension("ts"), LanguageHint::TypeScript);
    assert_eq!(LanguageHint::from_extension("go"), LanguageHint::Go);
    assert_eq!(LanguageHint::from_extension("unknown"), LanguageHint::Unknown);
}

// =============================================================================
// Vector Store Tests
// =============================================================================

#[test]
fn test_store_insert_and_retrieve() {
    let mut store = InMemoryVectorStore::new();
    let chunk = CodeChunk::new("fn test() {}".to_string(), ChunkType::Function)
        .with_embedding(vec![0.1, 0.2, 0.3]);

    let id = chunk.id.clone();
    store.insert(chunk).unwrap();

    let retrieved = store.get(&id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content, "fn test() {}");
}

#[test]
fn test_store_dimension_validation() {
    let mut store = InMemoryVectorStore::with_dimension(3);

    // Should succeed with matching dimension
    let chunk1 = CodeChunk::new("fn a() {}".to_string(), ChunkType::Function)
        .with_embedding(vec![0.1, 0.2, 0.3]);
    assert!(store.insert(chunk1).is_ok());

    // Should fail with mismatched dimension
    let chunk2 = CodeChunk::new("fn b() {}".to_string(), ChunkType::Function)
        .with_embedding(vec![0.1, 0.2, 0.3, 0.4]);

    let result = store.insert(chunk2);
    assert!(matches!(result, Err(RagError::DimensionMismatch { .. })));
}

#[test]
fn test_cosine_similarity_identical_vectors() {
    let v = vec![1.0, 0.0, 0.0];
    let sim = cosine_similarity(&v, &v);
    assert!((sim - 1.0).abs() < 0.0001, "Identical vectors should have similarity 1.0");
}

#[test]
fn test_cosine_similarity_orthogonal_vectors() {
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![0.0, 1.0, 0.0];
    let sim = cosine_similarity(&v1, &v2);
    assert!(sim.abs() < 0.0001, "Orthogonal vectors should have similarity 0.0");
}

#[test]
fn test_cosine_similarity_opposite_vectors() {
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![-1.0, 0.0, 0.0];
    let sim = cosine_similarity(&v1, &v2);
    assert!((sim - (-1.0)).abs() < 0.0001, "Opposite vectors should have similarity -1.0");
}

#[test]
fn test_normalize_vector_unit_length() {
    let v = vec![3.0, 4.0];
    let normalized = normalize_vector(&v);
    let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.0001, "Normalized vector should have unit length");
}

#[test]
fn test_store_find_similar() {
    let mut store = InMemoryVectorStore::new();

    // Insert chunks with known embeddings
    let chunk1 = CodeChunk::new("fn similar() {}".to_string(), ChunkType::Function)
        .with_embedding(vec![0.9, 0.1, 0.0]);
    let chunk2 = CodeChunk::new("fn different() {}".to_string(), ChunkType::Function)
        .with_embedding(vec![0.0, 0.9, 0.1]);

    store.insert(chunk1).unwrap();
    store.insert(chunk2).unwrap();

    // Query with vector similar to chunk1
    let query = vec![1.0, 0.0, 0.0];
    let results = store.find_similar(&query, 2, 0.0).unwrap();

    assert_eq!(results.len(), 2);
    // First result should be more similar
    assert!(results[0].similarity > results[1].similarity);
    assert!(results[0].content.contains("similar"));
}

#[test]
fn test_store_clear_zeroizes() {
    let mut store = InMemoryVectorStore::new();

    store
        .insert(
            CodeChunk::new("sensitive code".to_string(), ChunkType::Function)
                .with_embedding(vec![0.1, 0.2, 0.3]),
        )
        .unwrap();

    assert_eq!(store.len(), 1);

    store.clear();

    assert_eq!(store.len(), 0);
    assert!(store.is_empty());
}

// =============================================================================
// Embedder Tests
// =============================================================================

#[tokio::test]
async fn test_mock_embedder_consistent() {
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder = LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

    // Same input should produce same output
    let emb1 = embedder.embed("test text").await.unwrap();
    let emb2 = embedder.embed("test text").await.unwrap();
    assert_eq!(emb1, emb2);
}

#[tokio::test]
async fn test_mock_embedder_different_inputs() {
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder = LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

    let emb1 = embedder.embed("input one").await.unwrap();
    let emb2 = embedder.embed("input two").await.unwrap();
    assert_ne!(emb1, emb2);
}

#[tokio::test]
async fn test_embedder_batch() {
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder = LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

    let texts = vec!["one".to_string(), "two".to_string(), "three".to_string()];
    let embeddings = embedder.embed_batch(&texts).await.unwrap();

    assert_eq!(embeddings.len(), 3);
    for emb in &embeddings {
        assert_eq!(emb.len(), 768);
    }
}

#[test]
fn test_embedder_dimension() {
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder = LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

    assert_eq!(embedder.dimension(), 768);
    assert_eq!(embedder.model_name(), "nomic-embed-text");
}

// =============================================================================
// Indexer Tests
// =============================================================================

#[test]
fn test_indexer_config_defaults() {
    let config = IndexerConfig::default();

    assert!(config.extensions.contains("rs"));
    assert!(config.extensions.contains("py"));
    assert!(config.extensions.contains("js"));
    assert!(!config.exclude_patterns.is_empty());
}

#[test]
fn test_indexer_excludes_target() {
    let config = IndexerConfig::default();
    assert!(config.is_excluded(std::path::Path::new("target/debug/main")));
}

#[test]
fn test_indexer_excludes_git() {
    let config = IndexerConfig::default();
    assert!(config.is_excluded(std::path::Path::new(".git/config")));
}

#[test]
fn test_indexer_includes_source() {
    let config = IndexerConfig::default();
    assert!(!config.is_excluded(std::path::Path::new("src/main.rs")));
}

#[test]
fn test_indexer_index_content() {
    let indexer = CodeIndexer::default_config();
    let content = "fn test() { assert!(true); }";

    let chunks = indexer.index_content(content).unwrap();
    assert!(!chunks.is_empty());
}

#[test]
fn test_indexer_with_tempdir() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("lib.rs"), "pub fn lib() {}").unwrap();

    // Create excluded directory
    let target = temp_dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("excluded.rs"), "fn excluded() {}").unwrap();

    let indexer = CodeIndexer::default_config();
    let chunks = indexer.index_directory(temp_dir.path()).unwrap();

    // Should have chunks from main.rs and lib.rs, but not target/excluded.rs
    assert!(chunks.len() >= 2);

    // Verify no chunk contains "excluded"
    for chunk in &chunks {
        assert!(!chunk.content.contains("excluded"));
    }
}

// =============================================================================
// Query Processor Tests
// =============================================================================

fn create_test_processor() -> (QueryProcessor, InMemoryVectorStore) {
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder: Arc<dyn Embedder> = Arc::new(LocalEmbedder::with_mock_transport(
        "http://test",
        "nomic-embed-text",
        transport,
    ));

    let processor = QueryProcessor::new(
        embedder.clone(),
        RagConfig {
            token_budget: 1000,
            top_k: 5,
            min_similarity: 0.3,
        },
    );

    let mut store = InMemoryVectorStore::new();

    // Add test chunks with deterministic embeddings
    let chunks = vec![
        ("fn add(a: i32, b: i32) -> i32 { a + b }", ChunkType::Function),
        ("fn subtract(a: i32, b: i32) -> i32 { a - b }", ChunkType::Function),
        ("struct Calculator { value: i32 }", ChunkType::TypeDefinition),
    ];

    for (content, chunk_type) in chunks {
        // Generate deterministic embedding
        let embedding = generate_deterministic_embedding(content, 768);
        let chunk = CodeChunk::new(content.to_string(), chunk_type).with_embedding(embedding);
        store.insert(chunk).unwrap();
    }

    (processor, store)
}

fn generate_deterministic_embedding(text: &str, dim: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut embedding = Vec::with_capacity(dim);
    let mut hasher = DefaultHasher::new();

    for i in 0..dim {
        hasher.write(text.as_bytes());
        hasher.write_usize(i);
        let hash = hasher.finish();
        let value = ((hash % 2000) as f32 - 1000.0) / 1000.0;
        embedding.push(value);
        hasher = DefaultHasher::new();
    }

    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }

    embedding
}

#[tokio::test]
async fn test_query_returns_results() {
    let (processor, store) = create_test_processor();

    let result = processor.query("add numbers", &store).await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn test_query_respects_token_budget() {
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder: Arc<dyn Embedder> = Arc::new(LocalEmbedder::with_mock_transport(
        "http://test",
        "nomic-embed-text",
        transport,
    ));

    let processor = QueryProcessor::new(
        embedder,
        RagConfig {
            token_budget: 10, // Very small budget
            top_k: 10,
            min_similarity: 0.0,
        },
    );

    let (_, store) = create_test_processor();
    let result = processor.query("test", &store).await.unwrap();

    assert!(
        result.total_tokens <= 10,
        "Token budget should be respected"
    );
}

#[tokio::test]
async fn test_query_empty_store() {
    let (processor, _) = create_test_processor();
    let empty_store = InMemoryVectorStore::new();

    let result = processor.query("test", &empty_store).await;
    assert!(matches!(result, Err(RagError::NoChunksAvailable)));
}

#[test]
fn test_query_result_format_context() {
    let result = QueryResult {
        chunks: vec![RankedChunk {
            id: ChunkId::new(),
            content: "fn example() {}".to_string(),
            chunk_type: ChunkType::Function,
            similarity: 0.85,
            token_count: 10,
        }],
        total_tokens: 10,
        budget_remaining: 990,
        query_text: "test query".to_string(),
    };

    let context = result.format_as_context();
    assert!(context.contains("fn example()"));
    assert!(context.contains("85%"));
    assert!(context.contains("function"));
}

#[test]
fn test_query_result_format_json() {
    let result = QueryResult {
        chunks: vec![RankedChunk {
            id: ChunkId::new(),
            content: "fn example() {}".to_string(),
            chunk_type: ChunkType::Function,
            similarity: 0.85,
            token_count: 10,
        }],
        total_tokens: 10,
        budget_remaining: 990,
        query_text: "test".to_string(),
    };

    let json = result.format_as_json().unwrap();
    assert!(json.contains("\"chunks\""));
    assert!(json.contains("\"similarity\""));
}

// =============================================================================
// RAG System Integration Tests
// =============================================================================

fn create_test_rag() -> RagSystem {
    let config = RagSystemConfig::default();
    let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
    let embedder: Arc<dyn Embedder> = Arc::new(LocalEmbedder::with_mock_transport(
        "http://test",
        "nomic-embed-text",
        transport,
    ));

    RagSystem::with_mock_embedder(config, embedder)
}

#[tokio::test]
async fn test_rag_system_index_and_query() {
    let mut rag = create_test_rag();

    let content = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;

    // Index content
    let result = rag.index_content(content).await.unwrap();
    assert!(result.chunks_indexed > 0);
    assert!(rag.is_indexed());

    // Query
    let query_result = rag.query("add two numbers").await.unwrap();
    assert!(!query_result.is_empty());
}

#[tokio::test]
async fn test_rag_system_query_before_index() {
    let rag = create_test_rag();

    let result = rag.query("test").await;
    assert!(matches!(result, Err(RagError::NoChunksAvailable)));
}

#[tokio::test]
async fn test_rag_system_clear() {
    let mut rag = create_test_rag();

    rag.index_content("fn test() {}").await.unwrap();
    assert!(rag.is_indexed());

    rag.clear();
    assert!(!rag.is_indexed());
}

#[tokio::test]
async fn test_rag_system_stats() {
    let mut rag = create_test_rag();

    rag.index_content("fn hello() {} fn world() {}").await.unwrap();

    let stats = rag.stats();
    assert!(stats.total_chunks > 0);
    assert!(stats.total_tokens > 0);
}

#[tokio::test]
async fn test_rag_system_multi_query() {
    let mut rag = create_test_rag();

    let content = r#"
fn add(a: i32, b: i32) -> i32 { a + b }
fn subtract(a: i32, b: i32) -> i32 { a - b }
"#;

    rag.index_content(content).await.unwrap();

    let result = rag.multi_query(&["add", "subtract"]).await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn test_rag_system_query_builder() {
    let mut rag = create_test_rag();

    rag.index_content("fn test() { let x = 1; }").await.unwrap();

    let builder = rag.query_builder().query("test").top_k(3);
    let result = rag.execute_query(builder).await.unwrap();

    assert!(!result.is_empty());
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_error_sanitization() {
    let err = RagError::indexing_sanitized("Error in /home/user/secret/file.rs");
    let msg = err.to_user_message();

    assert!(!msg.contains("/home"));
    assert!(!msg.contains("secret"));
    assert!(!msg.contains("file.rs"));
}

#[test]
fn test_error_is_retryable() {
    assert!(RagError::ConnectionFailed("test".to_string()).is_retryable());
    assert!(RagError::Timeout(1000).is_retryable());
    assert!(RagError::RateLimited(60).is_retryable());

    assert!(!RagError::ChunkNotFound("test".to_string()).is_retryable());
    assert!(!RagError::InvalidConfig("test".to_string()).is_retryable());
}

#[test]
fn test_error_user_messages() {
    let err = RagError::ModelNotAvailable("nomic-embed-text".to_string());
    let msg = err.to_user_message();
    assert!(msg.contains("nomic-embed-text"));
    assert!(msg.contains("ollama pull"));

    let err = RagError::TokenBudgetExceeded {
        used: 5000,
        budget: 2048,
    };
    let msg = err.to_user_message();
    assert!(msg.contains("5000"));
    assert!(msg.contains("2048"));
}

// =============================================================================
// Security Tests
// =============================================================================

#[test]
fn test_no_paths_in_chunk_ids() {
    let chunk = CodeChunk::new(
        "fn secret_function_in_secret_path() {}".to_string(),
        ChunkType::Function,
    );

    let id_str = chunk.id.as_str();
    assert!(!id_str.contains("secret"));
    assert!(!id_str.contains("path"));
    assert!(!id_str.contains(".rs"));
}

#[test]
fn test_debug_output_safe() {
    let chunk = CodeChunk::new(
        "// API_KEY=sk-secret-key\nfn auth() {}".to_string(),
        ChunkType::Function,
    );

    let debug = format!("{:?}", chunk);
    assert!(!debug.contains("sk-secret"));
    assert!(!debug.contains("API_KEY"));
}

#[tokio::test]
async fn test_indexed_chunks_have_no_path_info() {
    let temp_dir = TempDir::new().unwrap();
    let secret_file = temp_dir.path().join("super_secret_file.rs");
    std::fs::write(&secret_file, "fn secret() {}").unwrap();

    let indexer = CodeIndexer::default_config();
    let chunks = indexer.index_file(&secret_file).unwrap();

    for chunk in chunks {
        assert!(!chunk.id.as_str().contains("secret"));
        assert!(!format!("{:?}", chunk).contains("super_secret_file"));
    }
}

// =============================================================================
// Config Tests
// =============================================================================

#[test]
fn test_rag_config_defaults() {
    let config = RagConfig::default();

    assert_eq!(config.token_budget, 2048);
    assert_eq!(config.top_k, 5);
    assert!((config.min_similarity - 0.3).abs() < 0.01);
}

#[test]
fn test_rag_system_config_from_privacy_config() {
    use aiy_core::config::PrivacyModeConfig;

    let privacy_config = PrivacyModeConfig {
        enabled: true,
        exclude_patterns: vec!["custom/**".to_string()],
        ..Default::default()
    };

    let config = RagSystemConfig::from_privacy_config(&privacy_config).unwrap();
    assert_eq!(config.ollama_url, privacy_config.local_executor.ollama_url);
    assert_eq!(config.rag_config.token_budget, privacy_config.rag.token_budget);
}

#[test]
fn test_embedding_dimension_constant() {
    assert_eq!(DEFAULT_EMBEDDING_DIM, 768);
}
