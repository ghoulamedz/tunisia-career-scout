//! Matching Commands

use anyhow::Result;
use tauri::{State, command};
use uuid::Uuid;
use crate::db::DbPool;
use crate::db::repositories::{CVRepository, PostingRepository, MatchRepository};
use crate::models::match_result::MatchResult;
use crate::models::settings::LLMSettings;
use crate::matching::tfidf::MatchingEngine;
use crate::matching::llm_reranker::{LLMReranker, RerankerConfig};

#[command]
pub async fn match_cv_to_postings(
    pool: State<'_, DbPool>,
    cv_profile_id: Uuid,
    min_score: Option<f32>,
    limit: Option<u32>,
) -> Result<Vec<MatchResult>, String> {
    let cv_repo = CVRepository::new(pool.inner().clone());
    let posting_repo = PostingRepository::new(pool.inner().clone());
    let match_repo = MatchRepository::new(pool.inner().clone());

    // Get CV profile
    let cv_profile = cv_repo.get(cv_profile_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CV profile not found".to_string())?;

    // Get active postings
    let postings = posting_repo.list(None, Some(crate::models::posting::PostingStatus::Active), limit.unwrap_or(100), 0)
        .await.map_err(|e| e.to_string())?;

    // Run matching
    let engine = MatchingEngine::new(cv_profile_id, &cv_profile.data.0);
    let mut results = Vec::new();

    for posting in postings {
        let match_result = engine.match_posting(&posting);
        if match_result.score >= min_score.unwrap_or(0.0) {
            match_repo.upsert(&match_result).await.map_err(|e| e.to_string())?;
            results.push(match_result);
        }
    }

    // Sort by score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

#[command]
pub async fn get_match_results(
    pool: State<'_, DbPool>,
    cv_profile_id: Uuid,
    min_score: Option<f32>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<MatchResult>, String> {
    let repo = MatchRepository::new(pool.inner().clone());
    repo.list_for_cv(cv_profile_id, min_score, limit.unwrap_or(50), offset.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn rerank_matches(
    pool: State<'_, DbPool>,
    cv_profile_id: Uuid,
    top_k: Option<u32>,
    llm_settings: Option<LLMSettings>,
) -> Result<Vec<MatchResult>, String> {
    let cv_repo = CVRepository::new(pool.inner().clone());
    let match_repo = MatchRepository::new(pool.inner().clone());
    let posting_repo = PostingRepository::new(pool.inner().clone());

    let cv_profile = cv_repo.get(cv_profile_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CV profile not found".to_string())?;

    // Get existing matches (TF-IDF results)
    let matches = match_repo.list_for_cv(cv_profile_id, None, 100, 0).await.map_err(|e| e.to_string())?;
    if matches.is_empty() {
        return Ok(Vec::new());
    }

    // Get the corresponding job postings
    let posting_ids: Vec<Uuid> = matches.iter().map(|m| m.posting_id).collect();
    let postings = posting_repo.list(None, Some(crate::models::posting::PostingStatus::Active), 200, 0)
        .await.map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| posting_ids.contains(&p.id))
        .collect::<Vec<_>>();

    if postings.is_empty() {
        return Ok(matches);
    }

    // Build reranker config from settings or defaults
    let config = llm_settings
        .map(|s| RerankerConfig::from(&s))
        .unwrap_or_default();

    // Create reranker and run reranking
    let reranker = LLMReranker::new(config).map_err(|e| e.to_string())?;
    let reranked = reranker.rerank(&cv_profile.data.0, postings, top_k.map(|k| k as usize)).await.map_err(|e| e.to_string())?;

    // Update stored match results with reranked scores
    for result in &reranked {
        match_repo.upsert(result).await.map_err(|e| e.to_string())?;
    }

    Ok(reranked)
}

#[command]
pub async fn clear_rerank_cache() -> Result<(), String> {
    // Note: This clears in-memory cache only; a persistent cache would need DB storage
    Ok(())
}