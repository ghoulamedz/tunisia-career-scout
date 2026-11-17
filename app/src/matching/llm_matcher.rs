//! LLM Matcher (Legacy Stub)
//!
//! This is a legacy stub implementation. Use LLMReranker instead.
//! Kept for backward compatibility during transition.

use anyhow::Result;
use crate::models::cv::CVData;
use crate::models::posting::JobPosting;
use crate::models::match_result::MatchResult;
use crate::models::settings::LLMSettings;

pub struct LLMMatcher {
    settings: LLMSettings,
}

impl LLMMatcher {
    pub fn new(settings: LLMSettings) -> Result<Self> {
        Ok(Self { settings })
    }

    pub fn update_settings(&mut self, settings: LLMSettings) -> Result<()> {
        self.settings = settings;
        Ok(())
    }

    pub async fn rerank(
        &self,
        _cv: &CVData,
        matches: &[MatchResult],
        _postings: &[JobPosting],
    ) -> Result<Vec<MatchResult>> {
        // Stub: return matches unchanged
        Ok(matches.to_vec())
    }
}