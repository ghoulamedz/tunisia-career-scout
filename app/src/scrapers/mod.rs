//! Scrapers Module
//!
//! Extraction pipeline and source adapters.

pub mod pipeline;
pub mod rule_extractor;
pub mod llm_extractor;
pub mod adapters;

pub use pipeline::{ExtractionPipeline, ExtractionResult, RawContent, SourceAdapter};
pub use rule_extractor::RuleExtractor;
pub use llm_extractor::LLMExtractor;
pub use adapters::{UrlAdapter, RssAdapter, GmailAdapter, SocialAdapter};