//! Cognate RAG — Retrieval-Augmented Generation pipeline.
//!
//! # Overview
//!
//! 1. Choose an [`EmbeddingProvider`] (e.g. `OpenAiProvider` from `cognate-providers`).
//! 2. Choose a [`VectorStore`] backend (e.g. [`MemoryVectorStore`] for prototyping).
//! 3. Wrap them in [`RagPipeline`] to get [`ingest`](RagPipeline::ingest) and
//!    [`retrieve`](RagPipeline::retrieve).
//!
//! # Example
//!
//! ```rust,no_run
//! use cognate_rag::{RagPipeline, MemoryVectorStore};
//! use cognate_core::EmbeddingProvider;
//!
//! async fn run(embedder: impl EmbeddingProvider) {
//!     let store = MemoryVectorStore::new();
//!     let pipeline = RagPipeline::new(embedder, store);
//!
//!     pipeline
//!         .ingest(
//!             vec!["Rust is fast".to_string(), "Rust is safe".to_string()],
//!             vec![serde_json::json!({"source": "doc1"}), serde_json::json!({"source": "doc2"})],
//!         )
//!         .await
//!         .unwrap();
//!
//!     let results = pipeline.retrieve("fast systems language", 1).await.unwrap();
//!     println!("{}", results[0].content);
//! }
//! ```
#![warn(missing_docs)]

pub mod memory;

pub use memory::MemoryVectorStore;

use async_trait::async_trait;
use cognate_core::{EmbeddingProvider, Error};
use serde::{Deserialize, Serialize};

// ─── Core types ────────────────────────────────────────────────────────────

/// A dense embedding vector.
pub type Vector = Vec<f32>;

/// A document stored in a vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for this document.
    pub id: String,
    /// The text content of the document.
    pub content: String,
    /// Arbitrary key-value metadata associated with this document.
    pub metadata: serde_json::Value,
    /// The embedding vector, if one has been computed.
    pub embedding: Option<Vector>,
}

// ─── VectorStore trait ─────────────────────────────────────────────────────

/// A persistent or in-memory store of embedded documents.
///
/// Implement this trait to add support for a new vector database backend
/// (pgvector, Qdrant, Pinecone, etc.).
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Persist a batch of documents (with their embeddings) to the store.
    async fn add_documents(
        &self,
        docs: Vec<Document>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Return the `limit` documents whose embeddings are most similar to
    /// `query_vector` (cosine similarity, descending).
    async fn search(
        &self,
        query_vector: Vector,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>;
}

// ─── RagPipeline ───────────────────────────────────────────────────────────

/// A high-level RAG pipeline combining an embedding provider with a vector store.
///
/// # Type parameters
///
/// * `E` — any [`EmbeddingProvider`] (e.g. `OpenAiProvider`).
/// * `V` — any [`VectorStore`] (e.g. [`MemoryVectorStore`]).
pub struct RagPipeline<E, V> {
    embedder: E,
    store: V,
}

impl<E: EmbeddingProvider, V: VectorStore> RagPipeline<E, V> {
    /// Create a new pipeline from an embedder and a vector store.
    pub fn new(embedder: E, store: V) -> Self {
        Self { embedder, store }
    }

    /// Embed `texts` and store them alongside `metadata` in the vector store.
    ///
    /// `texts` and `metadata` must have the same length.
    ///
    /// # Errors
    ///
    /// Returns [`Error::VectorStore`] if embedding or storage fails.
    pub async fn ingest(
        &self,
        texts: Vec<String>,
        metadata: Vec<serde_json::Value>,
    ) -> cognate_core::Result<()> {
        if texts.len() != metadata.len() {
            return Err(Error::InvalidRequest(
                "texts and metadata must have the same length".to_string(),
            ));
        }

        let embeddings = self
            .embedder
            .embed(texts.clone())
            .await
            .map_err(|e| Error::VectorStore(e.to_string()))?;

        let docs: Vec<Document> = texts
            .into_iter()
            .zip(embeddings)
            .zip(metadata)
            .enumerate()
            .map(|(i, ((content, emb), meta))| Document {
                id: i.to_string(),
                content,
                metadata: meta,
                embedding: Some(emb),
            })
            .collect();

        self.store
            .add_documents(docs)
            .await
            .map_err(|e| Error::VectorStore(e.to_string()))
    }

    /// Embed `query` and return the `limit` most similar documents.
    ///
    /// # Errors
    ///
    /// Returns [`Error::VectorStore`] if embedding or search fails.
    pub async fn retrieve(&self, query: &str, limit: usize) -> cognate_core::Result<Vec<Document>> {
        let mut embeddings = self
            .embedder
            .embed(vec![query.to_string()])
            .await
            .map_err(|e| Error::VectorStore(e.to_string()))?;

        let query_vec = if embeddings.is_empty() {
            return Err(Error::VectorStore(
                "embedding provider returned no vectors".to_string(),
            ));
        } else {
            embeddings.remove(0)
        };

        self.store
            .search(query_vec, limit)
            .await
            .map_err(|e| Error::VectorStore(e.to_string()))
    }
}
