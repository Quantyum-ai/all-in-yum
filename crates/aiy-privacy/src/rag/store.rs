//! In-memory vector store for the local RAG system.
//!
//! This module provides a secure, in-memory-only vector store for code chunks.
//! The store implements the Drop trait to zeroize all sensitive data when dropped.
//!
//! # Security
//!
//! - **In-memory only**: No persistence to disk
//! - **Zeroize on drop**: All content and embeddings are zeroed when store is dropped
//! - **No path information**: Only opaque IDs reference chunks

use crate::rag::error::RagError;
use crate::rag::types::{ChunkId, ChunkType, CodeChunk, RankedChunk};
use std::collections::HashMap;
use zeroize::Zeroize;

/// In-memory vector store for code chunks.
///
/// This store holds all indexed code chunks and their embeddings in memory.
/// When dropped, all sensitive data is zeroized to prevent memory inspection.
///
/// # Example
///
/// ```rust,ignore
/// use aiy_privacy::rag::store::InMemoryVectorStore;
/// use aiy_privacy::rag::types::CodeChunk;
///
/// let mut store = InMemoryVectorStore::new();
/// store.insert(chunk);
///
/// // When store goes out of scope, all data is zeroized
/// ```
pub struct InMemoryVectorStore {
    /// Chunks indexed by their opaque ID
    chunks: HashMap<ChunkId, CodeChunk>,

    /// Embedding dimension (validated on insert)
    embedding_dim: Option<usize>,

    /// Total token count across all chunks
    total_tokens: usize,
}

impl InMemoryVectorStore {
    /// Create a new empty vector store.
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            embedding_dim: None,
            total_tokens: 0,
        }
    }

    /// Create a new store with a specified embedding dimension.
    ///
    /// All inserted chunks must have embeddings of this dimension.
    pub fn with_dimension(dim: usize) -> Self {
        Self {
            chunks: HashMap::new(),
            embedding_dim: Some(dim),
            total_tokens: 0,
        }
    }

    /// Insert a chunk into the store.
    ///
    /// Returns the chunk ID that can be used to retrieve it later.
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk's embedding dimension doesn't match
    /// the store's expected dimension.
    pub fn insert(&mut self, chunk: CodeChunk) -> Result<ChunkId, RagError> {
        // Validate embedding dimension
        if chunk.has_embedding() {
            let chunk_dim = chunk.embedding.len();
            if let Some(expected_dim) = self.embedding_dim {
                if chunk_dim != expected_dim {
                    return Err(RagError::DimensionMismatch {
                        expected: expected_dim,
                        actual: chunk_dim,
                    });
                }
            } else {
                // Set the dimension on first insert
                self.embedding_dim = Some(chunk_dim);
            }
        }

        let id = chunk.id.clone();
        self.total_tokens += chunk.token_count;
        self.chunks.insert(id.clone(), chunk);

        Ok(id)
    }

    /// Insert multiple chunks into the store.
    ///
    /// Returns a vector of chunk IDs.
    pub fn insert_many(&mut self, chunks: Vec<CodeChunk>) -> Result<Vec<ChunkId>, RagError> {
        let mut ids = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            ids.push(self.insert(chunk)?);
        }
        Ok(ids)
    }

    /// Get a chunk by its ID.
    ///
    /// Returns None if the chunk doesn't exist.
    pub fn get(&self, id: &ChunkId) -> Option<&CodeChunk> {
        self.chunks.get(id)
    }

    /// Remove a chunk by its ID.
    ///
    /// Returns the removed chunk if it existed.
    pub fn remove(&mut self, id: &ChunkId) -> Option<CodeChunk> {
        if let Some(chunk) = self.chunks.remove(id) {
            self.total_tokens = self.total_tokens.saturating_sub(chunk.token_count);
            Some(chunk)
        } else {
            None
        }
    }

    /// Get the number of chunks in the store.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get the total token count across all chunks.
    pub fn total_tokens(&self) -> usize {
        self.total_tokens
    }

    /// Get the embedding dimension (if set).
    pub fn embedding_dim(&self) -> Option<usize> {
        self.embedding_dim
    }

    /// Get all chunk IDs.
    pub fn chunk_ids(&self) -> Vec<ChunkId> {
        self.chunks.keys().cloned().collect()
    }

    /// Get chunks by type.
    pub fn chunks_by_type(&self, chunk_type: ChunkType) -> Vec<&CodeChunk> {
        self.chunks
            .values()
            .filter(|c| c.chunk_type == chunk_type)
            .collect()
    }

    /// Find similar chunks using cosine similarity.
    ///
    /// Returns chunks ranked by similarity score, highest first.
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - The query vector to compare against
    /// * `top_k` - Maximum number of results to return
    /// * `min_similarity` - Minimum similarity threshold (0.0 to 1.0)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The store is empty
    /// - The query embedding dimension doesn't match
    pub fn find_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        min_similarity: f32,
    ) -> Result<Vec<RankedChunk>, RagError> {
        if self.chunks.is_empty() {
            return Err(RagError::NoChunksAvailable);
        }

        // Validate query dimension
        if let Some(expected_dim) = self.embedding_dim {
            if query_embedding.len() != expected_dim {
                return Err(RagError::DimensionMismatch {
                    expected: expected_dim,
                    actual: query_embedding.len(),
                });
            }
        }

        // Calculate similarities for all chunks with embeddings
        let mut scored: Vec<(&CodeChunk, f32)> = self
            .chunks
            .values()
            .filter(|c| c.has_embedding())
            .map(|chunk| {
                let similarity = cosine_similarity(&chunk.embedding, query_embedding);
                (chunk, similarity)
            })
            .filter(|(_, sim)| *sim >= min_similarity)
            .collect();

        // Sort by similarity (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top_k results
        let results: Vec<RankedChunk> = scored
            .into_iter()
            .take(top_k)
            .map(|(chunk, similarity)| RankedChunk::from_chunk(chunk, similarity))
            .collect();

        Ok(results)
    }

    /// Clear all chunks from the store.
    ///
    /// All data is zeroized before removal.
    pub fn clear(&mut self) {
        // Zeroize each chunk
        for (_, mut chunk) in self.chunks.drain() {
            chunk.zeroize();
        }
        self.total_tokens = 0;
    }

    /// Get statistics about the store.
    pub fn stats(&self) -> StoreStats {
        let mut type_counts: HashMap<ChunkType, usize> = HashMap::new();
        let mut embedded_count = 0;

        for chunk in self.chunks.values() {
            *type_counts.entry(chunk.chunk_type).or_insert(0) += 1;
            if chunk.has_embedding() {
                embedded_count += 1;
            }
        }

        StoreStats {
            total_chunks: self.chunks.len(),
            embedded_chunks: embedded_count,
            total_tokens: self.total_tokens,
            embedding_dim: self.embedding_dim,
            chunks_by_type: type_counts,
        }
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InMemoryVectorStore {
    fn drop(&mut self) {
        // Zeroize all sensitive data
        for (_, mut chunk) in self.chunks.drain() {
            chunk.zeroize();
        }
        self.total_tokens = 0;
        self.embedding_dim = None;
    }
}

/// Statistics about the vector store.
#[derive(Debug, Clone)]
pub struct StoreStats {
    /// Total number of chunks
    pub total_chunks: usize,
    /// Number of chunks with embeddings
    pub embedded_chunks: usize,
    /// Total token count
    pub total_tokens: usize,
    /// Embedding dimension
    pub embedding_dim: Option<usize>,
    /// Chunk counts by type
    pub chunks_by_type: HashMap<ChunkType, usize>,
}

/// Calculate cosine similarity between two vectors.
///
/// Returns a value between -1.0 and 1.0, where 1.0 means identical direction.
///
/// # Pure Rust Implementation
///
/// This is a pure Rust implementation without any C++ dependencies.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let norm_product = (norm_a * norm_b).sqrt();

    if norm_product == 0.0 {
        return 0.0;
    }

    dot_product / norm_product
}

/// Normalize a vector to unit length.
///
/// Returns the normalized vector (L2 normalization).
pub fn normalize_vector(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm == 0.0 {
        return v.to_vec();
    }

    v.iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chunk(content: &str, embedding: Vec<f32>) -> CodeChunk {
        CodeChunk::new(content.to_string(), ChunkType::Function).with_embedding(embedding)
    }

    #[test]
    fn test_store_insert_and_get() {
        let mut store = InMemoryVectorStore::new();
        let chunk = create_test_chunk("fn test() {}", vec![0.1, 0.2, 0.3]);
        let id = chunk.id.clone();

        store.insert(chunk).unwrap();

        assert_eq!(store.len(), 1);
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn test_store_dimension_validation() {
        let mut store = InMemoryVectorStore::with_dimension(3);

        // Should succeed with matching dimension
        let chunk1 = create_test_chunk("fn a() {}", vec![0.1, 0.2, 0.3]);
        assert!(store.insert(chunk1).is_ok());

        // Should fail with mismatched dimension
        let chunk2 = create_test_chunk("fn b() {}", vec![0.1, 0.2, 0.3, 0.4]);
        let result = store.insert(chunk2);
        assert!(matches!(result, Err(RagError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_store_remove() {
        let mut store = InMemoryVectorStore::new();
        let chunk = create_test_chunk("fn test() {}", vec![0.1, 0.2, 0.3]);
        let id = chunk.id.clone();

        store.insert(chunk).unwrap();
        assert_eq!(store.len(), 1);

        let removed = store.remove(&id);
        assert!(removed.is_some());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_store_clear() {
        let mut store = InMemoryVectorStore::new();
        store
            .insert(create_test_chunk("fn a() {}", vec![0.1, 0.2, 0.3]))
            .unwrap();
        store
            .insert(create_test_chunk("fn b() {}", vec![0.4, 0.5, 0.6]))
            .unwrap();

        assert_eq!(store.len(), 2);
        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&v1, &v2);
        assert!(sim.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&v1, &v2);
        assert!((sim - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_normalize_vector() {
        let v = vec![3.0, 4.0];
        let normalized = normalize_vector(&v);
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_find_similar() {
        let mut store = InMemoryVectorStore::new();

        // Insert some chunks with known embeddings
        store
            .insert(create_test_chunk("fn similar() {}", vec![0.9, 0.1, 0.0]))
            .unwrap();
        store
            .insert(create_test_chunk("fn different() {}", vec![0.0, 0.9, 0.1]))
            .unwrap();
        store
            .insert(create_test_chunk("fn very_similar() {}", vec![0.95, 0.05, 0.0]))
            .unwrap();

        // Query with embedding close to "similar" and "very_similar"
        let query = vec![1.0, 0.0, 0.0];
        let results = store.find_similar(&query, 2, 0.5).unwrap();

        assert_eq!(results.len(), 2);
        // Results should be sorted by similarity
        assert!(results[0].similarity >= results[1].similarity);
    }

    #[test]
    fn test_find_similar_empty_store() {
        let store = InMemoryVectorStore::new();
        let query = vec![1.0, 0.0, 0.0];
        let result = store.find_similar(&query, 5, 0.0);
        assert!(matches!(result, Err(RagError::NoChunksAvailable)));
    }

    #[test]
    fn test_find_similar_dimension_mismatch() {
        let mut store = InMemoryVectorStore::new();
        store
            .insert(create_test_chunk("fn test() {}", vec![0.1, 0.2, 0.3]))
            .unwrap();

        let query = vec![1.0, 0.0]; // Wrong dimension
        let result = store.find_similar(&query, 5, 0.0);
        assert!(matches!(result, Err(RagError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_store_stats() {
        let mut store = InMemoryVectorStore::new();
        store
            .insert(
                CodeChunk::new("fn a() {}".to_string(), ChunkType::Function)
                    .with_embedding(vec![0.1, 0.2, 0.3]),
            )
            .unwrap();
        store
            .insert(
                CodeChunk::new("struct B {}".to_string(), ChunkType::TypeDefinition)
                    .with_embedding(vec![0.4, 0.5, 0.6]),
            )
            .unwrap();
        store
            .insert(CodeChunk::new(
                "impl C {}".to_string(),
                ChunkType::ImplBlock,
            ))
            .unwrap();

        let stats = store.stats();
        assert_eq!(stats.total_chunks, 3);
        assert_eq!(stats.embedded_chunks, 2);
        assert_eq!(stats.embedding_dim, Some(3));
        assert_eq!(stats.chunks_by_type.get(&ChunkType::Function), Some(&1));
        assert_eq!(
            stats.chunks_by_type.get(&ChunkType::TypeDefinition),
            Some(&1)
        );
    }

    #[test]
    fn test_chunks_by_type() {
        let mut store = InMemoryVectorStore::new();
        store
            .insert(CodeChunk::new("fn a() {}".to_string(), ChunkType::Function))
            .unwrap();
        store
            .insert(CodeChunk::new("fn b() {}".to_string(), ChunkType::Function))
            .unwrap();
        store
            .insert(CodeChunk::new(
                "struct C {}".to_string(),
                ChunkType::TypeDefinition,
            ))
            .unwrap();

        let functions = store.chunks_by_type(ChunkType::Function);
        assert_eq!(functions.len(), 2);
    }

    #[test]
    fn test_total_tokens() {
        let mut store = InMemoryVectorStore::new();
        let chunk1 = CodeChunk::new("fn test() {}".to_string(), ChunkType::Function);
        let chunk2 = CodeChunk::new("fn another() {}".to_string(), ChunkType::Function);

        let tokens1 = chunk1.token_count;
        let tokens2 = chunk2.token_count;

        store.insert(chunk1).unwrap();
        store.insert(chunk2).unwrap();

        assert_eq!(store.total_tokens(), tokens1 + tokens2);
    }

    #[test]
    fn test_insert_many() {
        let mut store = InMemoryVectorStore::new();
        let chunks = vec![
            create_test_chunk("fn a() {}", vec![0.1, 0.2, 0.3]),
            create_test_chunk("fn b() {}", vec![0.4, 0.5, 0.6]),
            create_test_chunk("fn c() {}", vec![0.7, 0.8, 0.9]),
        ];

        let ids = store.insert_many(chunks).unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(store.len(), 3);
    }
}
