//! Test support utilities for RAG module.
//!
//! This module provides pure deterministic test infrastructure with NO async/HTTP complexity.

use super::embedder::Embedder;
use super::error::RagError;
use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Pure deterministic embedder for tests - NO async, NO HTTP, NO JSON parsing.
///
/// Maps text to deterministic normalized vectors using hash functions.
/// This eliminates all async mock coordination complexity.
pub struct TestEmbedder {
    dimension: usize,
}

impl TestEmbedder {
    /// Create a new test embedder with specified dimension.
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl Embedder for TestEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError> {
        Ok(deterministic_embedding(text, self.dimension))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, RagError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(deterministic_embedding(text, self.dimension));
        }
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "test-deterministic"
    }
}

/// Generate a deterministic embedding from text using bag-of-words approach.
///
/// Each token is hashed into a bucket, allowing queries with shared tokens
/// (like "add two numbers" and "add" in code) to have positive cosine similarity.
/// The embedding is normalized to unit length.
fn deterministic_embedding(text: &str, dimension: usize) -> Vec<f32> {
    let mut buckets = vec![0.0f32; dimension];

    // Bag-of-words: split on non-alphanumeric boundaries, hash each word into a bucket
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        if word.is_empty() {
            continue;
        }
        let word_lower = word.to_lowercase();
        let mut hasher = DefaultHasher::new();
        word_lower.hash(&mut hasher);
        let bucket_idx = (hasher.finish() as usize) % dimension;
        buckets[bucket_idx] += 1.0;
    }

    // Normalize to unit length
    normalize_vector(&mut buckets);
    buckets
}

/// Normalize a vector to unit length.
fn normalize_vector(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deterministic_embedding() {
        let embedder = TestEmbedder::new(8);

        let emb1 = embedder.embed("test").await.unwrap();
        let emb2 = embedder.embed("test").await.unwrap();

        assert_eq!(emb1, emb2, "Same input must produce same embedding");
        assert_eq!(emb1.len(), 8);
    }

    #[tokio::test]
    async fn test_different_text_different_embedding() {
        let embedder = TestEmbedder::new(8);

        let emb1 = embedder.embed("hello").await.unwrap();
        let emb2 = embedder.embed("world").await.unwrap();

        assert_ne!(emb1, emb2);
    }

    #[tokio::test]
    async fn test_normalized_embeddings() {
        let embedder = TestEmbedder::new(16);
        let embedding = embedder.embed("test text").await.unwrap();

        // Check unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.0001, "Embedding must be normalized");
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let embedder = TestEmbedder::new(8);

        let texts = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let embeddings = embedder.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 8);
        }
    }
}
