//! Match Result Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::match_result::{MatchResult, MatchReason};

#[derive(Clone)]
pub struct MatchRepository {
    pool: SqlitePool,
}

impl MatchRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, match_result: &MatchResult) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO match_results (
                id, cv_profile_id, posting_id, score, matched_skills, missing_skills,
                reasons, tfidf_score, llm_score, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            match_result.id,
            match_result.cv_profile_id,
            match_result.posting_id,
            match_result.score,
            serde_json::to_value(&match_result.matched_skills.0)?,
            serde_json::to_value(&match_result.missing_skills.0)?,
            serde_json::to_value(&match_result.reasons.0)?,
            match_result.tfidf_score,
            match_result.llm_score,
            match_result.created_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert(&self, match_result: &MatchResult) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO match_results (
                id, cv_profile_id, posting_id, score, matched_skills, missing_skills,
                reasons, tfidf_score, llm_score, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(cv_profile_id, posting_id) DO UPDATE SET
                score = excluded.score,
                matched_skills = excluded.matched_skills,
                missing_skills = excluded.missing_skills,
                reasons = excluded.reasons,
                tfidf_score = excluded.tfidf_score,
                llm_score = excluded.llm_score
            "#,
            match_result.id,
            match_result.cv_profile_id,
            match_result.posting_id,
            match_result.score,
            serde_json::to_value(&match_result.matched_skills.0)?,
            serde_json::to_value(&match_result.missing_skills.0)?,
            serde_json::to_value(&match_result.reasons.0)?,
            match_result.tfidf_score,
            match_result.llm_score,
            match_result.created_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, cv_profile_id: Uuid, posting_id: Uuid) -> Result<Option<MatchResult>> {
        let row = sqlx::query!(
            "SELECT * FROM match_results WHERE cv_profile_id = ? AND posting_id = ?",
            cv_profile_id, posting_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| MatchResult {
            id: r.id,
            cv_profile_id: Uuid::parse_str(&r.cv_profile_id).unwrap(),
            posting_id: Uuid::parse_str(&r.posting_id).unwrap(),
            score: r.score,
            matched_skills: serde_json::from_value(r.matched_skills).unwrap_or_default(),
            missing_skills: serde_json::from_value(r.missing_skills).unwrap_or_default(),
            reasons: serde_json::from_value(r.reasons).unwrap_or_default(),
            tfidf_score: r.tfidf_score,
            llm_score: r.llm_score,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
        }))
    }

    pub async fn list_for_cv(
        &self,
        cv_profile_id: Uuid,
        min_score: Option<f32>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MatchResult>> {
        let mut query = "SELECT * FROM match_results WHERE cv_profile_id = ?".to_string();
        if min_score.is_some() { query.push_str(" AND score >= ?"); }
        query.push_str(" ORDER BY score DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query(&query).bind(cv_profile_id);
        if let Some(ms) = min_score { q = q.bind(ms); }
        q = q.bind(limit as i64).bind(offset as i64);

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| MatchResult {
            id: r.id,
            cv_profile_id: Uuid::parse_str(&r.cv_profile_id).unwrap(),
            posting_id: Uuid::parse_str(&r.posting_id).unwrap(),
            score: r.score,
            matched_skills: serde_json::from_value(r.matched_skills).unwrap_or_default(),
            missing_skills: serde_json::from_value(r.missing_skills).unwrap_or_default(),
            reasons: serde_json::from_value(r.reasons).unwrap_or_default(),
            tfidf_score: r.tfidf_score,
            llm_score: r.llm_score,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
        }).collect())
    }

    pub async fn list_for_posting(&self, posting_id: Uuid) -> Result<Vec<MatchResult>> {
        let rows = sqlx::query!(
            "SELECT * FROM match_results WHERE posting_id = ? ORDER BY score DESC",
            posting_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| MatchResult {
            id: r.id,
            cv_profile_id: Uuid::parse_str(&r.cv_profile_id).unwrap(),
            posting_id: Uuid::parse_str(&r.posting_id).unwrap(),
            score: r.score,
            matched_skills: serde_json::from_value(r.matched_skills).unwrap_or_default(),
            missing_skills: serde_json::from_value(r.missing_skills).unwrap_or_default(),
            reasons: serde_json::from_value(r.reasons).unwrap_or_default(),
            tfidf_score: r.tfidf_score,
            llm_score: r.llm_score,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
        }).collect())
    }

    pub async fn delete_for_cv(&self, cv_profile_id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM match_results WHERE cv_profile_id = ?", cv_profile_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_old(&self, older_than: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM match_results WHERE created_at < ?",
            older_than
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}