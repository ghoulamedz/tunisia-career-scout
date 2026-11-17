//! Posting Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::posting::{JobPosting, PostingStatus, ApplicationStep};

#[derive(Clone)]
pub struct PostingRepository {
    pool: SqlitePool,
}

impl PostingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, posting: &JobPosting) -> Result<()> {
        let skills = serde_json::to_value(&posting.skills.0)?;
        let languages = serde_json::to_value(&posting.languages.0)?;
        let application_steps = serde_json::to_value(&posting.application_steps.0)?;
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
            skills,
            posting.experience_required,
            posting.education_required,
            languages,
            posting.location,
            posting.remote_policy,
            posting.job_type,
            posting.salary_range,
            posting.deadline,
            application_steps,
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

        Ok(row.map(|r| JobPosting {
            id: Uuid::parse_str(&r.id).unwrap(),
            source_id: Uuid::parse_str(&r.source_id).unwrap(),
            source_posting_id: r.source_posting_id,
            title: r.title,
            company: r.company,
            description: r.description,
            summary: r.summary,
            skills: serde_json::from_str(&r.skills).unwrap_or_default(),
            experience_required: r.experience_required,
            education_required: r.education_required,
            languages: serde_json::from_str(&r.languages).unwrap_or_default(),
            location: r.location,
            remote_policy: r.remote_policy,
            job_type: r.job_type,
            salary_range: r.salary_range,
            deadline: r.deadline.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            application_steps: serde_json::from_str(&r.application_steps).unwrap_or_default(),
            application_email: r.application_email,
            application_url: r.application_url,
            contact_person: r.contact_person,
            contact_phone: r.contact_phone,
            posted_date: r.posted_date.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            scraped_at: chrono::DateTime::parse_from_rfc3339(&r.scraped_at).unwrap().with_timezone(&chrono::Utc),
            content_hash: r.content_hash,
            raw_html: r.raw_html,
            status: serde_json::from_str(&format!("\"{}\"", r.status)).unwrap_or(PostingStatus::Active),
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap().with_timezone(&chrono::Utc),
        }))
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

        Ok(row.map(|r| JobPosting {
            id: Uuid::parse_str(&r.id).unwrap(),
            source_id: Uuid::parse_str(&r.source_id).unwrap(),
            source_posting_id: r.source_posting_id,
            title: r.title,
            company: r.company,
            description: r.description,
            summary: r.summary,
            skills: serde_json::from_str(&r.skills).unwrap_or_default(),
            experience_required: r.experience_required,
            education_required: r.education_required,
            languages: serde_json::from_str(&r.languages).unwrap_or_default(),
            location: r.location,
            remote_policy: r.remote_policy,
            job_type: r.job_type,
            salary_range: r.salary_range,
            deadline: r.deadline.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            application_steps: serde_json::from_str(&r.application_steps).unwrap_or_default(),
            application_email: r.application_email,
            application_url: r.application_url,
            contact_person: r.contact_person,
            contact_phone: r.contact_phone,
            posted_date: r.posted_date.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            scraped_at: chrono::DateTime::parse_from_rfc3339(&r.scraped_at).unwrap().with_timezone(&chrono::Utc),
            content_hash: r.content_hash,
            raw_html: r.raw_html,
            status: serde_json::from_str(&format!("\"{}\"", r.status)).unwrap_or(PostingStatus::Active),
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap().with_timezone(&chrono::Utc),
        }))
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
        if let Some(s) = status { q = q.bind(s.as_str()); }
        q = q.bind(limit as i64).bind(offset as i64);

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| JobPosting {
            id: Uuid::parse_str(&r.get::<String, _>("id")).unwrap(),
            source_id: Uuid::parse_str(&r.get::<String, _>("source_id")).unwrap(),
            source_posting_id: r.get("source_posting_id"),
            title: r.get("title"),
            company: r.get("company"),
            description: r.get("description"),
            summary: r.get("summary"),
            skills: serde_json::from_str(&r.get::<String, _>("skills")).unwrap_or_default(),
            experience_required: r.get("experience_required"),
            education_required: r.get("education_required"),
            languages: serde_json::from_str(&r.get::<String, _>("languages")).unwrap_or_default(),
            location: r.get("location"),
            remote_policy: r.get("remote_policy"),
            job_type: r.get("job_type"),
            salary_range: r.get("salary_range"),
            deadline: r.get::<Option<String>, _>("deadline").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            application_steps: serde_json::from_str(&r.get::<String, _>("application_steps")).unwrap_or_default(),
            application_email: r.get("application_email"),
            application_url: r.get("application_url"),
            contact_person: r.get("contact_person"),
            contact_phone: r.get("contact_phone"),
            posted_date: r.get::<Option<String>, _>("posted_date").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            scraped_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("scraped_at")).unwrap().with_timezone(&chrono::Utc),
            content_hash: r.get("content_hash"),
            raw_html: r.get("raw_html"),
            status: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("status"))).unwrap_or(PostingStatus::Active),
            created_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        }).collect())
    }

    pub async fn count(&self, source_id: Option<Uuid>, status: Option<PostingStatus>) -> Result<i64> {
        let mut query = "SELECT COUNT(*) as count FROM postings WHERE 1=1".to_string();
        if source_id.is_some() { query.push_str(" AND source_id = ?"); }
        if status.is_some() { query.push_str(" AND status = ?"); }

        let mut q = sqlx::query(&query);
        if let Some(sid) = source_id { q = q.bind(sid); }
        if let Some(s) = status { q = q.bind(s.as_str()); }

        let row = q.fetch_one(&self.pool).await?;
        Ok(row.get("count"))
    }

    pub async fn update(&self, posting: &JobPosting) -> Result<()> {
        let skills = serde_json::to_value(&posting.skills.0)?;
        let languages = serde_json::to_value(&posting.languages.0)?;
        let application_steps = serde_json::to_value(&posting.application_steps.0)?;
        let now = Utc::now();
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
            skills,
            posting.experience_required,
            posting.education_required,
            languages,
            posting.location,
            posting.remote_policy,
            posting.job_type,
            posting.salary_range,
            posting.deadline,
            application_steps,
            posting.application_email,
            posting.application_url,
            posting.contact_person,
            posting.contact_phone,
            posting.posted_date,
            posting.content_hash,
            posting.raw_html,
            posting.status as _,
            now,
            posting.id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: Uuid, status: PostingStatus) -> Result<()> {
        let now = Utc::now();
        let status_str = status.as_str();
        sqlx::query!(
            "UPDATE postings SET status = ?, updated_at = ? WHERE id = ?",
            status_str,
            now,
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
        let rows = sqlx::query(
            r#"
            SELECT * FROM postings
            WHERE title LIKE ? OR company LIKE ? OR description LIKE ?
            ORDER BY scraped_at DESC LIMIT ?
            "#
        )
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| JobPosting {
            id: Uuid::parse_str(&r.get::<String, _>("id")).unwrap(),
            source_id: Uuid::parse_str(&r.get::<String, _>("source_id")).unwrap(),
            source_posting_id: r.get("source_posting_id"),
            title: r.get("title"),
            company: r.get("company"),
            description: r.get("description"),
            summary: r.get("summary"),
            skills: serde_json::from_str(&r.get::<String, _>("skills")).unwrap_or_default(),
            experience_required: r.get("experience_required"),
            education_required: r.get("education_required"),
            languages: serde_json::from_str(&r.get::<String, _>("languages")).unwrap_or_default(),
            location: r.get("location"),
            remote_policy: r.get("remote_policy"),
            job_type: r.get("job_type"),
            salary_range: r.get("salary_range"),
            deadline: r.get::<Option<String>, _>("deadline").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            application_steps: serde_json::from_str(&r.get::<String, _>("application_steps")).unwrap_or_default(),
            application_email: r.get("application_email"),
            application_url: r.get("application_url"),
            contact_person: r.get("contact_person"),
            contact_phone: r.get("contact_phone"),
            posted_date: r.get::<Option<String>, _>("posted_date").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            scraped_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("scraped_at")).unwrap().with_timezone(&chrono::Utc),
            content_hash: r.get("content_hash"),
            raw_html: r.get("raw_html"),
            status: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("status"))).unwrap_or(PostingStatus::Active),
            created_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        }).collect())
    }
}