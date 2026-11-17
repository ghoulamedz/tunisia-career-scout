//! CV Commands

use anyhow::Result;
use sqlx::types::Json;
use tauri::{State, command};
use uuid::Uuid;
use std::env;
use crate::db::DbPool;
use crate::db::repositories::CVRepository;
use crate::models::cv::{CVData, CVProfile, CVTemplate, CVTheme};
use crate::cv::CVCompiler;
pub use crate::cv::compile_cv_preview;

#[command]
pub async fn get_cv(
    pool: State<'_, DbPool>,
    id: Option<Uuid>,
) -> Result<Option<CVProfile>, String> {
    let repo = CVRepository::new(pool.inner().clone());
    if let Some(id) = id {
        repo.get(id).await.map_err(|e| e.to_string())
    } else {
        repo.get_default().await.map_err(|e| e.to_string())
    }
}

#[command]
pub async fn list_cvs(
    pool: State<'_, DbPool>,
) -> Result<Vec<CVProfile>, String> {
    let repo = CVRepository::new(pool.inner().clone());
    repo.list().await.map_err(|e| e.to_string())
}

#[command]
pub async fn save_cv(
    pool: State<'_, DbPool>,
    name: String,
    data: CVData,
    is_default: Option<bool>,
) -> Result<CVProfile, String> {
    let repo = CVRepository::new(pool.inner().clone());
    let mut profile = CVProfile::new(name, data);
    if is_default.unwrap_or(false) {
        profile.is_default = true;
    }
    repo.create(&profile).await.map_err(|e| e.to_string())?;

    if profile.is_default {
        repo.set_default(profile.id).await.map_err(|e| e.to_string())?;
    }

    Ok(profile)
}

#[command]
pub async fn update_cv(
    pool: State<'_, DbPool>,
    id: Uuid,
    name: Option<String>,
    data: Option<CVData>,
    template: Option<CVTemplate>,
    theme: Option<CVTheme>,
    is_default: Option<bool>,
) -> Result<CVProfile, String> {
    let repo = CVRepository::new(pool.inner().clone());
    let mut profile = repo.get(id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CV profile not found".to_string())?;

    if let Some(n) = name { profile.name = n; }
    if let Some(d) = data { profile.data = Json(d); }
    if let Some(t) = template { profile.template = t; }
    if let Some(t) = theme { profile.theme = t; }
    if let Some(d) = is_default { profile.is_default = d; }
    profile.updated_at = chrono::Utc::now();

    repo.update(&profile).await.map_err(|e| e.to_string())?;

    if profile.is_default {
        repo.set_default(profile.id).await.map_err(|e| e.to_string())?;
    }

    Ok(profile)
}

#[command]
pub async fn delete_cv(
    pool: State<'_, DbPool>,
    id: Uuid,
) -> Result<(), String> {
    let repo = CVRepository::new(pool.inner().clone());
    repo.delete(id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn set_default_cv(
    pool: State<'_, DbPool>,
    id: Uuid,
) -> Result<CVProfile, String> {
    let repo = CVRepository::new(pool.inner().clone());
    repo.set_default(id).await.map_err(|e| e.to_string())?;
    repo.get(id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CV profile not found".to_string())
}

#[command]
pub async fn generate_pdf(
    pool: State<'_, DbPool>,
    cv_id: Uuid,
) -> Result<Vec<u8>, String> {
    let repo = CVRepository::new(pool.inner().clone());
    let profile = repo.get(cv_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CV profile not found".to_string())?;

    let template_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
    let font_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
    let compiler = CVCompiler::from_env(template_fallback, font_fallback);
    compiler.compile(&profile.data.0).map_err(|e| e.to_string())
}

#[command]
pub async fn preview_template(
    template: CVTemplate,
    data: CVData,
) -> Result<Vec<u8>, String> {
    let template_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
    let font_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
    let compiler = CVCompiler::from_env(template_fallback, font_fallback);
    compiler.compile(&data).map_err(|e| e.to_string())
}

#[command]
pub async fn list_templates() -> Result<Vec<CVTemplate>, String> {
    Ok(vec![CVTemplate::Modern, CVTemplate::Classic, CVTemplate::Minimal])
}