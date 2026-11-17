//! Matching Engine
//!
//! TF-IDF vectorization + LLM reranking for CV-to-job matching.

pub mod tfidf;
pub mod llm_matcher;
pub mod llm_reranker;

pub use tfidf::{TfIdfVectorizer, MatchingEngine};
pub use llm_matcher::LLMMatcher;
pub use llm_reranker::{LLMReranker, RerankerConfig, Reranker};