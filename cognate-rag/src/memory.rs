//! In-memory vector store implementation.
//!
//! Suitable for prototyping and testing.  All documents are stored in a
//! `Vec` protected by an `Arc<Mutex>` and similarity is computed as
//! cosine similarity over `f32` vectors.

use crate::{Document, Vector, VectorStore};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// A simple in-memory [`VectorStore`].
///
/// Thread-safe via `Arc<Mutex<_>>` — suitable for concurrent Axum handlers.
/// For production use, replace with a dedicated vector database.
#[derive(Debug, Clone, Default)]
pub struct MemoryVectorStore {
    documents: Arc<Mutex<Vec<Document>>>,
}

impl MemoryVectorStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of stored documents.
    pub fn len(&self) -> usize {
        self.documents.lock().unwrap().len()
    }

    /// Return `true` if the store contains no documents.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl VectorStore for MemoryVectorStore {
    async fn add_documents(
        &self,
        docs: Vec<Document>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.documents.lock().unwrap().extend(docs);
        Ok(())
    }

    async fn search(
        &self,
        query_vector: Vector,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let documents = self.documents.lock().unwrap();

        let mut scored: Vec<(f32, &Document)> = documents
            .iter()
            .filter_map(|doc| {
                doc.embedding
                    .as_ref()
                    .map(|emb| (cosine_similarity(&query_vector, emb), doc))
            })
            .collect();

        // Sort descending by score — NaN is treated as -infinity so it sinks to the bottom.
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Less));

        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(_, doc)| doc.clone())
            .collect())
    }
}

/// Cosine similarity between two equal-length vectors.
///
/// Returns `0.0` if either vector has zero norm.
fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let n1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let n2: f32 = v2.iter().map(|b| b * b).sum::<f32>().sqrt();
    if n1 == 0.0 || n2 == 0.0 {
        0.0
    } else {
        dot / (n1 * n2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VectorStore;

    fn doc(id: &str, content: &str, embedding: Vec<f32>) -> Document {
        Document {
            id: id.to_string(),
            content: content.to_string(),
            metadata: serde_json::Value::Null,
            embedding: Some(embedding),
        }
    }

    #[tokio::test]
    async fn test_search_returns_closest() {
        let store = MemoryVectorStore::new();
        store
            .add_documents(vec![
                doc("1", "close",  vec![1.0, 0.0, 0.0]),
                doc("2", "far",    vec![0.0, 1.0, 0.0]),
                doc("3", "medium", vec![0.7, 0.7, 0.0]),
            ])
            .await
            .unwrap();

        let results = store
            .search(vec![1.0, 0.0, 0.0], 1)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "close");
    }

    #[tokio::test]
    async fn test_search_respects_limit() {
        let store = MemoryVectorStore::new();
        store
            .add_documents(vec![
                doc("a", "a", vec![1.0, 0.0]),
                doc("b", "b", vec![0.8, 0.6]),
                doc("c", "c", vec![0.0, 1.0]),
            ])
            .await
            .unwrap();

        let results = store.search(vec![1.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_cosine_zero_vector() {
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
    }
}
