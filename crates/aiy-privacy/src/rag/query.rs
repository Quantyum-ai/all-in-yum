//! Query processing for the local RAG system.
//!
//! This module provides query processing with similarity search and
//! token budget enforcement.

use crate::rag::embedder::Embedder;
use crate::rag::error::RagError;
use crate::rag::store::InMemoryVectorStore;
use crate::rag::types::RankedChunk;
use aiy_core::config::RagConfig;
use std::sync::Arc;

/// Query processor for searching indexed code.
///
/// The query processor:
/// 1. Embeds the query text
/// 2. Searches the vector store for similar chunks
/// 3. Enforces token budget constraints
/// 4. Returns ranked results
pub struct QueryProcessor {
    /// Embedder for query text
    embedder: Arc<dyn Embedder>,
    /// RAG configuration (token budget, top_k, min_similarity)
    config: RagConfig,
}

impl QueryProcessor {
    /// Create a new query processor.
    pub fn new(embedder: Arc<dyn Embedder>, config: RagConfig) -> Self {
        Self { embedder, config }
    }

    /// Create with default configuration.
    pub fn with_defaults(embedder: Arc<dyn Embedder>) -> Self {
        Self::new(embedder, RagConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &RagConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: RagConfig) {
        self.config = config;
    }

    /// Process a query and return relevant chunks.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query text
    /// * `store` - The vector store to search
    ///
    /// # Returns
    ///
    /// Ranked chunks that match the query, respecting token budget.
    pub async fn query(
        &self,
        query: &str,
        store: &InMemoryVectorStore,
    ) -> Result<QueryResult, RagError> {
        // Embed the query
        let query_embedding = self.embedder.embed(query).await?;

        // Search the store
        let mut results = store.find_similar(
            &query_embedding,
            self.config.top_k,
            self.config.min_similarity,
        )?;

        // Apply token budget
        let (selected, total_tokens) = self.apply_token_budget(&mut results);

        Ok(QueryResult {
            chunks: selected,
            total_tokens,
            budget_remaining: self.config.token_budget.saturating_sub(total_tokens),
            query_text: query.to_string(),
        })
    }

    /// Apply token budget constraints to results.
    ///
    /// Returns the selected chunks and total token count.
    fn apply_token_budget(&self, results: &mut Vec<RankedChunk>) -> (Vec<RankedChunk>, usize) {
        let mut selected = Vec::new();
        let mut total_tokens = 0;

        for chunk in results.drain(..) {
            if total_tokens + chunk.token_count <= self.config.token_budget {
                total_tokens += chunk.token_count;
                selected.push(chunk);
            } else {
                // Budget exhausted - could optionally include partial
                break;
            }
        }

        (selected, total_tokens)
    }

    /// Query with custom parameters (override config).
    pub async fn query_with_params(
        &self,
        query: &str,
        store: &InMemoryVectorStore,
        top_k: usize,
        min_similarity: f32,
        token_budget: usize,
    ) -> Result<QueryResult, RagError> {
        let query_embedding = self.embedder.embed(query).await?;

        let mut results = store.find_similar(&query_embedding, top_k, min_similarity)?;

        // Apply custom token budget
        let mut selected = Vec::new();
        let mut total_tokens = 0;

        for chunk in results.drain(..) {
            if total_tokens + chunk.token_count <= token_budget {
                total_tokens += chunk.token_count;
                selected.push(chunk);
            } else {
                break;
            }
        }

        Ok(QueryResult {
            chunks: selected,
            total_tokens,
            budget_remaining: token_budget.saturating_sub(total_tokens),
            query_text: query.to_string(),
        })
    }

    /// Multi-query search: combine results from multiple queries.
    ///
    /// This is useful for expanding search coverage with related queries.
    pub async fn multi_query(
        &self,
        queries: &[&str],
        store: &InMemoryVectorStore,
    ) -> Result<QueryResult, RagError> {
        if queries.is_empty() {
            return Err(RagError::QueryError("No queries provided".to_string()));
        }

        // Collect results from all queries
        let mut all_results: Vec<RankedChunk> = Vec::new();

        for query in queries {
            let query_embedding = self.embedder.embed(query).await?;

            if let Ok(results) = store.find_similar(
                &query_embedding,
                self.config.top_k,
                self.config.min_similarity,
            ) {
                all_results.extend(results);
            }
        }

        // Deduplicate by chunk ID, keeping highest similarity
        let mut seen = std::collections::HashMap::new();
        for chunk in all_results {
            let entry = seen.entry(chunk.id.clone()).or_insert(chunk.clone());
            if chunk.similarity > entry.similarity {
                *entry = chunk;
            }
        }

        // Sort by similarity
        let mut deduped: Vec<RankedChunk> = seen.into_values().collect();
        deduped.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply token budget
        let (selected, total_tokens) = self.apply_token_budget(&mut deduped);

        Ok(QueryResult {
            chunks: selected,
            total_tokens,
            budget_remaining: self.config.token_budget.saturating_sub(total_tokens),
            query_text: queries.join(" | "),
        })
    }
}

/// Result of a query operation.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Ranked chunks matching the query
    pub chunks: Vec<RankedChunk>,
    /// Total tokens in returned chunks
    pub total_tokens: usize,
    /// Remaining token budget
    pub budget_remaining: usize,
    /// Original query text
    pub query_text: String,
}

impl QueryResult {
    /// Check if any results were found.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Get the highest similarity score.
    pub fn best_similarity(&self) -> Option<f32> {
        self.chunks.first().map(|c| c.similarity)
    }

    /// Format results as context for an LLM prompt.
    ///
    /// Returns a string with code snippets formatted for inclusion
    /// in an LLM context.
    pub fn format_as_context(&self) -> String {
        if self.chunks.is_empty() {
            return String::from("No relevant code found.");
        }

        let mut context = String::new();
        context.push_str("## Relevant Code Context\n\n");

        for (i, chunk) in self.chunks.iter().enumerate() {
            context.push_str(&format!(
                "### Snippet {} ({}%, {})\n",
                i + 1,
                (chunk.similarity * 100.0) as u32,
                chunk.chunk_type.description()
            ));
            context.push_str("```\n");
            context.push_str(&chunk.content);
            if !chunk.content.ends_with('\n') {
                context.push('\n');
            }
            context.push_str("```\n\n");
        }

        context.push_str(&format!(
            "_Found {} snippets, {} tokens used, {} remaining_\n",
            self.chunks.len(),
            self.total_tokens,
            self.budget_remaining
        ));

        context
    }

    /// Format results as JSON for structured output.
    pub fn format_as_json(&self) -> Result<String, RagError> {
        let json = serde_json::json!({
            "chunks": self.chunks.iter().map(|c| {
                serde_json::json!({
                    "id": c.id.as_str(),
                    "similarity": c.similarity,
                    "chunk_type": format!("{:?}", c.chunk_type),
                    "token_count": c.token_count,
                    "content": c.content,
                })
            }).collect::<Vec<_>>(),
            "total_tokens": self.total_tokens,
            "budget_remaining": self.budget_remaining,
        });

        serde_json::to_string_pretty(&json).map_err(|e| RagError::SerializationError(e.to_string()))
    }
}

/// Builder for constructing complex queries.
pub struct QueryBuilder {
    queries: Vec<String>,
    top_k: Option<usize>,
    min_similarity: Option<f32>,
    token_budget: Option<usize>,
}

impl QueryBuilder {
    /// Create a new query builder.
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            top_k: None,
            min_similarity: None,
            token_budget: None,
        }
    }

    /// Add a query.
    pub fn query(mut self, q: impl Into<String>) -> Self {
        self.queries.push(q.into());
        self
    }

    /// Add multiple queries.
    pub fn queries(mut self, qs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.queries.extend(qs.into_iter().map(|q| q.into()));
        self
    }

    /// Set maximum results.
    pub fn top_k(mut self, k: usize) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set minimum similarity threshold.
    pub fn min_similarity(mut self, sim: f32) -> Self {
        self.min_similarity = Some(sim);
        self
    }

    /// Set token budget.
    pub fn token_budget(mut self, budget: usize) -> Self {
        self.token_budget = Some(budget);
        self
    }

    /// Execute the query.
    pub async fn execute(
        self,
        processor: &QueryProcessor,
        store: &InMemoryVectorStore,
    ) -> Result<QueryResult, RagError> {
        if self.queries.is_empty() {
            return Err(RagError::QueryError("No queries specified".to_string()));
        }

        let top_k = self.top_k.unwrap_or(processor.config.top_k);
        let min_similarity = self.min_similarity.unwrap_or(processor.config.min_similarity);
        let token_budget = self.token_budget.unwrap_or(processor.config.token_budget);

        if self.queries.len() == 1 {
            processor
                .query_with_params(&self.queries[0], store, top_k, min_similarity, token_budget)
                .await
        } else {
            let query_refs: Vec<&str> = self.queries.iter().map(|s| s.as_str()).collect();
            processor.multi_query(&query_refs, store).await
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::test_support::TestEmbedder;
    use crate::rag::types::{ChunkType, CodeChunk};

    fn create_test_processor() -> QueryProcessor {
        let embedder = Arc::new(TestEmbedder::new(768));

        QueryProcessor::new(
            embedder,
            RagConfig {
                token_budget: 500,
                top_k: 5,
                min_similarity: 0.0, // Low threshold for bag-of-words sparse embeddings
            },
        )
    }

    fn create_test_store() -> InMemoryVectorStore {
        let mut store = InMemoryVectorStore::new();

        // Add some test chunks with deterministic embeddings
        let chunks = vec![
            ("fn add(a: i32, b: i32) -> i32 { a + b }", ChunkType::Function),
            (
                "fn subtract(a: i32, b: i32) -> i32 { a - b }",
                ChunkType::Function,
            ),
            (
                "struct Calculator { value: i32 }",
                ChunkType::TypeDefinition,
            ),
            (
                "impl Calculator { fn new() -> Self { Self { value: 0 } } }",
                ChunkType::ImplBlock,
            ),
        ];

        for (content, chunk_type) in chunks {
            let mut chunk = CodeChunk::new(content.to_string(), chunk_type);
            // Generate deterministic embedding
            let embedding = generate_test_embedding(content, 768);
            chunk.embedding = embedding;
            store.insert(chunk).unwrap();
        }

        store
    }

    fn generate_test_embedding(text: &str, dim: usize) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut buckets = vec![0.0f32; dim];

        // Bag-of-words: split on non-alphanumeric boundaries, hash each word into a bucket
        for word in text.split(|c: char| !c.is_alphanumeric()) {
            if word.is_empty() {
                continue;
            }
            let word_lower = word.to_lowercase();
            let mut hasher = DefaultHasher::new();
            word_lower.hash(&mut hasher);
            let bucket_idx = (hasher.finish() as usize) % dim;
            buckets[bucket_idx] += 1.0;
        }

        // Normalize to unit length
        let norm: f32 = buckets.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut buckets {
                *x /= norm;
            }
        }

        buckets
    }

    #[tokio::test]
    async fn test_basic_query() {
        let processor = create_test_processor();
        let store = create_test_store();

        let result = processor.query("add two numbers", &store).await.unwrap();

        assert!(!result.is_empty());
        assert!(result.total_tokens <= processor.config.token_budget);
    }

    #[tokio::test]
    async fn test_query_with_params() {
        let processor = create_test_processor();
        let store = create_test_store();

        let result = processor
            .query_with_params("calculator", &store, 2, 0.0, 1000)
            .await
            .unwrap();

        assert!(result.len() <= 2);
    }

    #[tokio::test]
    async fn test_multi_query() {
        let processor = create_test_processor();
        let store = create_test_store();

        let result = processor
            .multi_query(&["add numbers", "subtract values"], &store)
            .await
            .unwrap();

        // Should return deduplicated results
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_token_budget_enforcement() {
        let embedder = Arc::new(TestEmbedder::new(768));

        // Very small budget
        let processor = QueryProcessor::new(
            embedder,
            RagConfig {
                token_budget: 10, // Very small
                top_k: 10,
                min_similarity: 0.0,
            },
        );

        let store = create_test_store();
        let result = processor.query("test", &store).await.unwrap();

        // Budget should be respected
        assert!(result.total_tokens <= 10);
    }

    #[test]
    fn test_query_result_format_context() {
        let result = QueryResult {
            chunks: vec![RankedChunk {
                id: crate::rag::types::ChunkId::default(),
                content: "fn test() {}".to_string(),
                chunk_type: ChunkType::Function,
                similarity: 0.95,
                token_count: 10,
            }],
            total_tokens: 10,
            budget_remaining: 490,
            query_text: "test".to_string(),
        };

        let context = result.format_as_context();
        assert!(context.contains("fn test()"));
        assert!(context.contains("95%"));
        assert!(context.contains("function"));
    }

    #[test]
    fn test_query_result_format_json() {
        let result = QueryResult {
            chunks: vec![RankedChunk {
                id: crate::rag::types::ChunkId::default(),
                content: "fn test() {}".to_string(),
                chunk_type: ChunkType::Function,
                similarity: 0.95,
                token_count: 10,
            }],
            total_tokens: 10,
            budget_remaining: 490,
            query_text: "test".to_string(),
        };

        let json = result.format_as_json().unwrap();
        assert!(json.contains("\"chunks\""));
        assert!(json.contains("\"total_tokens\""));
    }

    #[test]
    fn test_query_result_is_empty() {
        let empty = QueryResult {
            chunks: vec![],
            total_tokens: 0,
            budget_remaining: 500,
            query_text: "test".to_string(),
        };

        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.best_similarity().is_none());
    }

    #[test]
    fn test_query_result_best_similarity() {
        let result = QueryResult {
            chunks: vec![
                RankedChunk {
                    id: crate::rag::types::ChunkId::default(),
                    content: "best".to_string(),
                    chunk_type: ChunkType::Function,
                    similarity: 0.95,
                    token_count: 5,
                },
                RankedChunk {
                    id: crate::rag::types::ChunkId::default(),
                    content: "second".to_string(),
                    chunk_type: ChunkType::Function,
                    similarity: 0.80,
                    token_count: 5,
                },
            ],
            total_tokens: 10,
            budget_remaining: 490,
            query_text: "test".to_string(),
        };

        assert!((result.best_similarity().unwrap() - 0.95).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_query_builder() {
        let processor = create_test_processor();
        let store = create_test_store();

        let result = QueryBuilder::new()
            .query("add numbers")
            .top_k(3)
            .min_similarity(0.0)
            .execute(&processor, &store)
            .await
            .unwrap();

        assert!(result.len() <= 3);
    }

    #[tokio::test]
    async fn test_query_builder_multiple_queries() {
        let processor = create_test_processor();
        let store = create_test_store();

        let result = QueryBuilder::new()
            .queries(vec!["add", "subtract"])
            .execute(&processor, &store)
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_query_empty_store() {
        let processor = create_test_processor();
        let store = InMemoryVectorStore::new();

        let result = processor.query("test", &store).await;
        assert!(matches!(result, Err(RagError::NoChunksAvailable)));
    }

    #[test]
    fn test_config_accessors() {
        let mut processor = create_test_processor();

        assert_eq!(processor.config().top_k, 5);

        processor.set_config(RagConfig {
            token_budget: 1000,
            top_k: 10,
            min_similarity: 0.5,
        });

        assert_eq!(processor.config().top_k, 10);
    }
}
