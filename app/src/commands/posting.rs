//! Posting Commands

use anyhow::Result;
use tauri::{State, command};
use uuid::Uuid;
use crate::db::DbPool;
use crate::db::repositories::PostingRepository;
use crate::models::posting::{JobPosting, PostingStatus};
// use crate::scrapers::pipeline::ExtractionPipeline;

#[command]
pub async fn list_postings(
    pool: State<'_, DbPool>,
    source_id: Option<Uuid>,
    status: Option<PostingStatus>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<JobPosting>, String> {
    let repo = PostingRepository::new(pool.inner().clone());
    repo.list(source_id, status, limit.unwrap_or(50), offset.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_posting(
    pool: State<'_, DbPool>,
    id: Uuid,
) -> Result<Option<JobPosting>, String> {
    let repo = PostingRepository::new(pool.inner().clone());
    repo.get(id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn delete_posting(
    pool: State<'_, DbPool>,
    id: Uuid,
) -> Result<(), String> {
    let repo = PostingRepository::new(pool.inner().clone());
    repo.delete(id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn re_extract_posting(
    // pool: State<'_, DbPool>,
    // id: Uuid,
    // llm_settings: crate::models::settings::LLMSettings,
) -> Result<JobPosting, String> {
    // Placeholder - requires scraper integration
    Err("Not implemented: requires scraper integration".to_string())
}

#[command]
pub async fn search_postings(
    pool: State<'_, DbPool>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<JobPosting>, String> {
    let repo = PostingRepository::new(pool.inner().clone());
    repo.search(&query, limit.unwrap_or(20))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn update_posting_status(
    pool: State<'_, DbPool>,
    id: Uuid,
    status: PostingStatus,
) -> Result<(), String> {
    let repo = PostingRepository::new(pool.inner().clone());
    repo.update_status(id, status).await.map_err(|e| e.to_string())
}