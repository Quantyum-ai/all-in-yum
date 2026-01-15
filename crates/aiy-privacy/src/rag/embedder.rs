//! Local embedding generation for the RAG system.
//!
//! This module provides embedding generation using Ollama's /api/embeddings endpoint.
//! All embedding operations are performed locally to maintain privacy.
//!
//! # Supported Models
//!
//! - `nomic-embed-text` (default, 768 dimensions)
//! - `mxbai-embed-large` (1024 dimensions)
//! - `all-minilm` (384 dimensions)

use crate::rag::error::RagError;
use crate::rag::types::{CodeChunk, DEFAULT_EMBEDDING_DIM};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Trait for embedding generation.
///
/// This trait allows for different embedding implementations (local, mock, etc.)
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate an embedding for a single text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError>;

    /// Generate embeddings for multiple texts.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, RagError>;

    /// Get the embedding dimension for this embedder.
    fn dimension(&self) -> usize;

    /// Get the model name being used.
    fn model_name(&self) -> &str;
}

/// Local embedder using Ollama's /api/embeddings endpoint.
///
/// # Example
///
/// ```rust,ignore
/// use aiy_privacy::rag::embedder::LocalEmbedder;
///
/// let embedder = LocalEmbedder::new("http://127.0.0.1:11434", "nomic-embed-text")?;
/// let embedding = embedder.embed("fn main() {}").await?;
/// ```
pub struct LocalEmbedder {
    /// Base URL for Ollama API
    base_url: String,
    /// Model to use for embeddings
    model: String,
    /// Embedding dimension for this model
    dimension: usize,
    /// HTTP transport (allows mocking)
    transport: Arc<dyn EmbeddingTransport>,
}

impl LocalEmbedder {
    /// Create a new local embedder with real HTTP transport.
    #[cfg(feature = "http")]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Result<Self, RagError> {
        let model = model.into();
        let dimension = model_dimension(&model);
        let transport = Arc::new(ReqwestEmbeddingTransport::new()?);

        Ok(Self {
            base_url: base_url.into(),
            model,
            dimension,
            transport,
        })
    }

    /// Create a new local embedder with a mock transport (for testing).
    pub fn with_mock_transport(
        base_url: impl Into<String>,
        model: impl Into<String>,
        transport: Arc<dyn EmbeddingTransport>,
    ) -> Self {
        let model = model.into();
        let dimension = model_dimension(&model);

        Self {
            base_url: base_url.into(),
            model,
            dimension,
            transport,
        }
    }

    /// Create a local embedder with default settings.
    #[cfg(feature = "http")]
    pub fn default_local() -> Result<Self, RagError> {
        Self::new("http://127.0.0.1:11434", "nomic-embed-text")
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Embed a code chunk and return the chunk with embedding set.
    pub async fn embed_chunk(&self, mut chunk: CodeChunk) -> Result<CodeChunk, RagError> {
        let embedding = self.embed(&chunk.content).await?;
        chunk.embedding = embedding;
        Ok(chunk)
    }

    /// Embed multiple code chunks.
    pub async fn embed_chunks(&self, chunks: Vec<CodeChunk>) -> Result<Vec<CodeChunk>, RagError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Extract contents for batch embedding
        let contents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

        // Get embeddings in batch
        let embeddings = self.embed_batch(&contents).await?;

        // Combine chunks with embeddings
        let result: Vec<CodeChunk> = chunks
            .into_iter()
            .zip(embeddings.into_iter())
            .map(|(mut chunk, embedding)| {
                chunk.embedding = embedding;
                chunk
            })
            .collect();

        Ok(result)
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let url = format!("{}/api/embeddings", self.base_url);
        let body = serde_json::to_string(&request)?;

        let response_text = self.transport.post(&url, &body).await?;

        let response: EmbeddingResponse = serde_json::from_str(&response_text)
            .map_err(|e| RagError::EmbeddingFailed(format!("Failed to parse response: {}", e)))?;

        Ok(response.embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, RagError> {
        // Ollama doesn't support batch embeddings natively, so we process sequentially
        // For better performance, consider using tokio::join! or similar
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let embedding = self.embed(text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Get the embedding dimension for a model.
fn model_dimension(model: &str) -> usize {
    match model {
        "nomic-embed-text" => 768,
        "mxbai-embed-large" => 1024,
        "all-minilm" | "all-minilm-l6-v2" => 384,
        "snowflake-arctic-embed" => 1024,
        _ => DEFAULT_EMBEDDING_DIM, // Default fallback
    }
}

/// Request body for Ollama's /api/embeddings endpoint.
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

/// Response from Ollama's /api/embeddings endpoint.
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

/// Transport trait for embedding requests.
///
/// This allows mocking HTTP requests in tests.
#[async_trait]
pub trait EmbeddingTransport: Send + Sync {
    /// POST a request and return the response body.
    async fn post(&self, url: &str, body: &str) -> Result<String, RagError>;
}

/// Real HTTP transport using reqwest.
#[cfg(feature = "http")]
pub struct ReqwestEmbeddingTransport {
    client: reqwest::Client,
}

#[cfg(feature = "http")]
impl ReqwestEmbeddingTransport {
    /// Create a new reqwest transport.
    pub fn new() -> Result<Self, RagError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| RagError::ConnectionFailed(e.to_string()))?;

        Ok(Self { client })
    }

    /// Create with custom timeout.
    pub fn with_timeout(timeout: std::time::Duration) -> Result<Self, RagError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| RagError::ConnectionFailed(e.to_string()))?;

        Ok(Self { client })
    }
}

#[cfg(feature = "http")]
#[async_trait]
impl EmbeddingTransport for ReqwestEmbeddingTransport {
    async fn post(&self, url: &str, body: &str) -> Result<String, RagError> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| RagError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();

            if error_body.contains("model") && error_body.contains("not found") {
                return Err(RagError::ModelNotAvailable(
                    "Embedding model not found. Run: ollama pull nomic-embed-text".to_string(),
                ));
            }

            return Err(RagError::EmbeddingFailed(format!(
                "HTTP {}: {}",
                status, error_body
            )));
        }

        response
            .text()
            .await
            .map_err(|e| RagError::TransportError(e.to_string()))
    }
}

/// Mock transport for testing.
pub struct MockEmbeddingTransport {
    /// Function to generate mock embeddings
    response_fn: Box<dyn Fn(&str) -> Result<String, RagError> + Send + Sync>,
}

impl MockEmbeddingTransport {
    /// Create a mock transport with a fixed embedding.
    pub fn with_fixed_embedding(embedding: Vec<f32>) -> Self {
        let response = serde_json::json!({
            "embedding": embedding
        })
        .to_string();

        Self {
            response_fn: Box::new(move |_| Ok(response.clone())),
        }
    }

    /// Create a mock transport with a custom response function.
    pub fn with_response<F>(f: F) -> Self
    where
        F: Fn(&str) -> Result<String, RagError> + Send + Sync + 'static,
    {
        Self {
            response_fn: Box::new(f),
        }
    }

    /// Create a mock that generates deterministic embeddings based on input.
    pub fn deterministic(dimension: usize) -> Self {
        Self {
            response_fn: Box::new(move |body| {
                // Parse the request to get the prompt
                let request: serde_json::Value =
                    serde_json::from_str(body).unwrap_or(serde_json::Value::Null);

                let prompt = request
                    .get("prompt")
                    .and_then(|p| p.as_str())
                    .unwrap_or("");

                // Generate a deterministic embedding based on prompt hash
                let embedding = generate_deterministic_embedding(prompt, dimension);

                let response = serde_json::json!({
                    "embedding": embedding
                });

                Ok(response.to_string())
            }),
        }
    }
}

#[async_trait]
impl EmbeddingTransport for MockEmbeddingTransport {
    async fn post(&self, _url: &str, body: &str) -> Result<String, RagError> {
        (self.response_fn)(body)
    }
}

/// Generate a deterministic embedding based on input text.
///
/// This is useful for testing as it produces consistent results.
fn generate_deterministic_embedding(text: &str, dimension: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut embedding = Vec::with_capacity(dimension);
    let mut hasher = DefaultHasher::new();

    for i in 0..dimension {
        // Hash the text combined with the dimension index
        hasher.write(text.as_bytes());
        hasher.write_usize(i);
        let hash = hasher.finish();

        // Convert hash to a float in [-1, 1]
        let value = ((hash % 2000) as f32 - 1000.0) / 1000.0;
        embedding.push(value);

        // Reset hasher for next iteration
        hasher = DefaultHasher::new();
    }

    // Normalize the embedding
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }

    embedding
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_dimension() {
        assert_eq!(model_dimension("nomic-embed-text"), 768);
        assert_eq!(model_dimension("mxbai-embed-large"), 1024);
        assert_eq!(model_dimension("all-minilm"), 384);
        assert_eq!(model_dimension("unknown-model"), DEFAULT_EMBEDDING_DIM);
    }

    #[tokio::test]
    async fn test_mock_embedder_fixed() {
        let embedding = vec![0.1, 0.2, 0.3];
        let transport = Arc::new(MockEmbeddingTransport::with_fixed_embedding(
            embedding.clone(),
        ));

        let embedder =
            LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

        let result = embedder.embed("test text").await.unwrap();
        assert_eq!(result, embedding);
    }

    #[tokio::test]
    async fn test_mock_embedder_deterministic() {
        let transport = Arc::new(MockEmbeddingTransport::deterministic(768));

        let embedder =
            LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

        // Same input should produce same output
        let result1 = embedder.embed("fn main() {}").await.unwrap();
        let result2 = embedder.embed("fn main() {}").await.unwrap();
        assert_eq!(result1, result2);

        // Different input should produce different output
        let result3 = embedder.embed("fn other() {}").await.unwrap();
        assert_ne!(result1, result3);
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let transport = Arc::new(MockEmbeddingTransport::deterministic(768));

        let embedder =
            LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

        let texts = vec!["text1".to_string(), "text2".to_string(), "text3".to_string()];

        let results = embedder.embed_batch(&texts).await.unwrap();
        assert_eq!(results.len(), 3);

        // Each embedding should have correct dimension
        for embedding in &results {
            assert_eq!(embedding.len(), 768);
        }
    }

    #[tokio::test]
    async fn test_embed_chunk() {
        let transport = Arc::new(MockEmbeddingTransport::deterministic(768));

        let embedder =
            LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

        let chunk =
            CodeChunk::new("fn test() { println!(\"hello\"); }".to_string(), crate::rag::types::ChunkType::Function);

        assert!(!chunk.has_embedding());

        let embedded = embedder.embed_chunk(chunk).await.unwrap();
        assert!(embedded.has_embedding());
        assert_eq!(embedded.embedding.len(), 768);
    }

    #[tokio::test]
    async fn test_embed_chunks() {
        let transport = Arc::new(MockEmbeddingTransport::deterministic(768));

        let embedder =
            LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

        let chunks = vec![
            CodeChunk::new("fn a() {}".to_string(), crate::rag::types::ChunkType::Function),
            CodeChunk::new("fn b() {}".to_string(), crate::rag::types::ChunkType::Function),
        ];

        let embedded = embedder.embed_chunks(chunks).await.unwrap();
        assert_eq!(embedded.len(), 2);
        assert!(embedded[0].has_embedding());
        assert!(embedded[1].has_embedding());
    }

    #[test]
    fn test_deterministic_embedding_consistency() {
        let emb1 = generate_deterministic_embedding("test", 10);
        let emb2 = generate_deterministic_embedding("test", 10);
        assert_eq!(emb1, emb2);
    }

    #[test]
    fn test_deterministic_embedding_normalized() {
        let embedding = generate_deterministic_embedding("test input", 100);
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_embedder_properties() {
        let transport = Arc::new(MockEmbeddingTransport::deterministic(768));
        let embedder =
            LocalEmbedder::with_mock_transport("http://test:11434", "nomic-embed-text", transport);

        assert_eq!(embedder.base_url(), "http://test:11434");
        assert_eq!(embedder.model(), "nomic-embed-text");
        assert_eq!(embedder.dimension(), 768);
        assert_eq!(embedder.model_name(), "nomic-embed-text");
    }

    #[tokio::test]
    async fn test_mock_error_response() {
        let transport = Arc::new(MockEmbeddingTransport::with_response(|_| {
            Err(RagError::ModelNotAvailable("test model".to_string()))
        }));

        let embedder =
            LocalEmbedder::with_mock_transport("http://test", "nomic-embed-text", transport);

        let result = embedder.embed("test").await;
        assert!(matches!(result, Err(RagError::ModelNotAvailable(_))));
    }
}
