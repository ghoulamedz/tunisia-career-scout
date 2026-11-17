//! Posting Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::posting::{JobPosting, PostingStatus, ApplicationStep};

pub struct PostingRepository {
    pool: SqlitePool,
}

impl PostingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, posting: &JobPosting) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO postings (
                id, source_id, source_posting_id, title, company, description, summary,
                skills, experience_required, education_required, languages, location,
                remote_policy, job_type, salary_range, deadline, application_steps,
                application_email, application_url, contact_person, contact_phone,
                posted_date, scraped_at, content_hash, raw_html, status, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            posting.id,
            posting.source_id,
            posting.source_posting_id,
            posting.title,
            posting.company,
            posting.description,
            posting.summary,
            serde_json::to_value(&posting.skills.0)?,
            posting.experience_required,
            posting.education_required,
            serde_json::to_value(&posting.languages.0)?,
            posting.location,
            posting.remote_policy,
            posting.job_type,
            posting.salary_range,
            posting.deadline,
            serde_json::to_value(&posting.application_steps.0)?,
            posting.application_email,
            posting.application_url,
            posting.contact_person,
            posting.contact_phone,
            posting.posted_date,
            posting.scraped_at,
            posting.content_hash,
            posting.raw_html,
            posting.status as _,
            posting.created_at,
            posting.updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert(&self, posting: &JobPosting) -> Result<bool> {
        // Returns true if inserted, false if updated
        let existing = self.get_by_source_id(posting.source_id, &posting.source_posting_id).await?;
        if let Some(mut existing) = existing {
            if existing.content_hash != posting.content_hash {
                existing.description = posting.description.clone();
                existing.summary = posting.summary.clone();
                existing.skills = posting.skills.clone();
                existing.experience_required = posting.experience_required.clone();
                existing.education_required = posting.education_required.clone();
                existing.languages = posting.languages.clone();
                existing.location = posting.location.clone();
                existing.remote_policy = posting.remote_policy.clone();
                existing.job_type = posting.job_type.clone();
                existing.salary_range = posting.salary_range.clone();
                existing.deadline = posting.deadline.clone();
                existing.application_steps = posting.application_steps.clone();
                existing.application_email = posting.application_email.clone();
                existing.application_url = posting.application_url.clone();
                existing.contact_person = posting.contact_person.clone();
                existing.contact_phone = posting.contact_phone.clone();
                existing.content_hash = posting.content_hash.clone();
                existing.raw_html = posting.raw_html.clone();
                existing.updated_at = Utc::now();
                self.update(&existing).await?;
                Ok(false)
            } else {
                Ok(false)
            }
        } else {
            self.create(posting).await?;
            Ok(true)
        }
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<JobPosting>> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM postings WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(self::row_to_posting))
    }

    pub async fn get_by_source_id(&self, source_id: Uuid, source_posting_id: &str) -> Result<Option<JobPosting>> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM postings WHERE source_id = ? AND source_posting_id = ?
            "#,
            source_id, source_posting_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(self::row_to_posting))
    }

    pub async fn list(
        &self,
        source_id: Option<Uuid>,
        status: Option<PostingStatus>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<JobPosting>> {
        let mut query = "SELECT * FROM postings WHERE 1=1".to_string();
        if source_id.is_some() { query.push_str(" AND source_id = ?"); }
        if status.is_some() { query.push_str(" AND status = ?"); }
        query.push_str(" ORDER BY scraped_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query(&query);
        if let Some(sid) = source_id { q = q.bind(sid); }
        if let Some(s) = status { q = q.bind(s as _); }
        q = q.bind(limit as i64).bind(offset as i64);

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(self::row_to_posting).collect())
    }

    pub async fn count(&self, source_id: Option<Uuid>, status: Option<PostingStatus>) -> Result<i64> {
        let mut query = "SELECT COUNT(*) as count FROM postings WHERE 1=1".to_string();
        if source_id.is_some() { query.push_str(" AND source_id = ?"); }
        if status.is_some() { query.push_str(" AND status = ?"); }

        let mut q = sqlx::query(&query);
        if let Some(sid) = source_id { q = q.bind(sid); }
        if let Some(s) = status { q = q.bind(s as _); }

        let row = q.fetch_one(&self.pool).await?;
        Ok(row.get("count"))
    }

    pub async fn update(&self, posting: &JobPosting) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE postings SET
                title = ?, company = ?, description = ?, summary = ?,
                skills = ?, experience_required = ?, education_required = ?,
                languages = ?, location = ?, remote_policy = ?, job_type = ?,
                salary_range = ?, deadline = ?, application_steps = ?,
                application_email = ?, application_url = ?, contact_person = ?,
                contact_phone = ?, posted_date = ?, content_hash = ?,
                raw_html = ?, status = ?, updated_at = ?
            WHERE id = ?
            "#,
            posting.title,
            posting.company,
            posting.description,
            posting.summary,
            serde_json::to_value(&posting.skills.0)?,
            posting.experience_required,
            posting.education_required,
            serde_json::to_value(&posting.languages.0)?,
            posting.location,
            posting.remote_policy,
            posting.job_type,
            posting.salary_range,
            posting.deadline,
            serde_json::to_value(&posting.application_steps.0)?,
            posting.application_email,
            posting.application_url,
            posting.contact_person,
            posting.contact_phone,
            posting.posted_date,
            posting.content_hash,
            posting.raw_html,
            posting.status as _,
            Utc::now(),
            posting.id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: Uuid, status: PostingStatus) -> Result<()> {
        sqlx::query!(
            "UPDATE postings SET status = ?, updated_at = ? WHERE id = ?",
            status as _,
            Utc::now(),
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM postings WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_old(&self, older_than: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM postings WHERE scraped_at < ? AND status NOT IN ('saved', 'applied')",
            older_than
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<JobPosting>> {
        let rows = sqlx::query!(
            r#"
            SELECT * FROM postings
            WHERE title LIKE ? OR company LIKE ? OR description LIKE ?
            ORDER BY scraped_at DESC LIMIT ?
            "#,
            format!("%{}%", query),
            format!("%{}%", query),
            format!("%{}%", query),
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(self::row_to_posting).collect())
    }
}

fn row_to_posting(row: sqlx::sqlite::SqliteRow) -> JobPosting {
    JobPosting {
        id: row.get("id"),
        source_id: row.get("source_id"),
        source_posting_id: row.get("source_posting_id"),
        title: row.get("title"),
        company: row.get("company"),
        description: row.get("description"),
        summary: row.get("summary"),
        skills: serde_json::from_value(row.get("skills")).unwrap_or_default(),
        experience_required: row.get("experience_required"),
        education_required: row.get("education_required"),
        languages: serde_json::from_value(row.get("languages")).unwrap_or_default(),
        location: row.get("location"),
        remote_policy: row.get("remote_policy"),
        job_type: row.get("job_type"),
        salary_range: row.get("salary_range"),
        deadline: row.get("deadline"),
        application_steps: serde_json::from_value(row.get("application_steps")).unwrap_or_default(),
        application_email: row.get("application_email"),
        application_url: row.get("application_url"),
        contact_person: row.get("contact_person"),
        contact_phone: row.get("contact_phone"),
        posted_date: row.get("posted_date"),
        scraped_at: row.get("scraped_at"),
        content_hash: row.get("content_hash"),
        raw_html: row.get("raw_html"),
        status: serde_json::from_value(row.get("status")).unwrap_or(PostingStatus::Active),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}