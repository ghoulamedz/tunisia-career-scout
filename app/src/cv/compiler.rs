//! CV Compiler
//!
//! Compiles CVData to PDF using Typst 0.15.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::env;
use indexmap::IndexMap;
use typst::diag::{FileError, FileResult, SourceResult};
use typst::foundations::{Array, Bytes, Datetime, Dict, Value, Str};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use typst::World;
use typst_pdf::PdfOptions;
use crate::models::cv::{CVData, CVTemplate, CVTheme, SkillLevel, LanguageProficiency};

/// Theme color configuration
#[derive(Debug, Clone)]
struct ThemeColors {
    accent_color: &'static str,
    text_color: &'static str,
    muted_color: &'static str,
    bg_color: &'static str,
    dark_text: &'static str,
    rule_color: &'static str,
    card_bg: &'static str,
    card_border: &'static str,
    primary_color: &'static str,
    secondary_color: &'static str,
    line_color: &'static str,
}

impl ThemeColors {
    fn for_theme(template: CVTemplate, theme: CVTheme) -> Self {
        match (template, theme) {
            // Modern themes
            (CVTemplate::Modern, CVTheme::Light) => Self {
                accent_color: "#0ea5e9", text_color: "#1e293b", muted_color: "#64748b",
                bg_color: "#f8fafc", dark_text: "#0f172a", rule_color: "#0ea5e9",
                card_bg: "#ffffff", card_border: "#e2e8f0",
                primary_color: "#1e3a5f", secondary_color: "#2c5282", line_color: "#e2e8f0",
            },
            (CVTemplate::Modern, CVTheme::Dark) => Self {
                accent_color: "#38bdf8", text_color: "#f1f5f9", muted_color: "#94a3b8",
                bg_color: "#0f172a", dark_text: "#f8fafc", rule_color: "#38bdf8",
                card_bg: "#1e293b", card_border: "#334155",
                primary_color: "#38bdf8", secondary_color: "#60a5fa", line_color: "#334155",
            },
            (CVTemplate::Modern, CVTheme::Auto) => Self::for_theme(CVTemplate::Modern, CVTheme::Light),
            
            // Classic themes
            (CVTemplate::Classic, CVTheme::Light) => Self {
                accent_color: "#2563eb", text_color: "#1a202c", muted_color: "#718096",
                bg_color: "#ffffff", dark_text: "#1a202c", rule_color: "#e2e8f0",
                card_bg: "#f7fafc", card_border: "#e2e8f0",
                primary_color: "#1e3a5f", secondary_color: "#2c5282", line_color: "#e2e8f0",
            },
            (CVTemplate::Classic, CVTheme::Dark) => Self {
                accent_color: "#60a5fa", text_color: "#e2e8f0", muted_color: "#a0aec0",
                bg_color: "#1a202c", dark_text: "#f7fafc", rule_color: "#2d3748",
                card_bg: "#2d3748", card_border: "#4a5568",
                primary_color: "#60a5fa", secondary_color: "#90cdf4", line_color: "#2d3748",
            },
            (CVTemplate::Classic, CVTheme::Auto) => Self::for_theme(CVTemplate::Classic, CVTheme::Light),
            
            // Minimal themes
            (CVTemplate::Minimal, CVTheme::Light) => Self {
                accent_color: "#374151", text_color: "#111827", muted_color: "#9ca3af",
                bg_color: "#ffffff", dark_text: "#111827", rule_color: "#e5e7eb",
                card_bg: "#f9fafb", card_border: "#e5e7eb",
                primary_color: "#374151", secondary_color: "#4b5563", line_color: "#e5e7eb",
            },
            (CVTemplate::Minimal, CVTheme::Dark) => Self {
                accent_color: "#d1d5db", text_color: "#f9fafb", muted_color: "#9ca3af",
                bg_color: "#111827", dark_text: "#f9fafb", rule_color: "#374151",
                card_bg: "#1f2937", card_border: "#374151",
                primary_color: "#d1d5db", secondary_color: "#9ca3af", line_color: "#374151",
            },
            (CVTemplate::Minimal, CVTheme::Auto) => Self::for_theme(CVTemplate::Minimal, CVTheme::Light),
        }
    }
}

/// Font bundle for CV compilation
#[derive(Clone)]
struct FontBundle {
    fonts: Vec<Font>,
    book: FontBook,
}

impl FontBundle {
    fn load(font_dir: &Path) -> Result<Self> {
        let load_font = |name: &str| -> Result<Bytes> {
            let path = font_dir.join(name);
            let bytes = std::fs::read(&path)
                .with_context(|| format!("Failed to load font: {}", path.display()))?;
            Ok(Bytes::new(bytes))
        };

        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        
        let font_files = [
            "Inter-Regular.ttf", "Inter-Medium.ttf", "Inter-SemiBold.ttf", "Inter-Bold.ttf",
            "Inter-Italic.ttf", "Inter-MediumItalic.ttf", "Inter-SemiBoldItalic.ttf", "Inter-BoldItalic.ttf",
            "JetBrainsMono-Regular.ttf", "JetBrainsMono-Medium.ttf", "JetBrainsMono-SemiBold.ttf", "JetBrainsMono-Bold.ttf",
            "JetBrainsMono-Italic.ttf", "JetBrainsMono-MediumItalic.ttf", "JetBrainsMono-SemiBoldItalic.ttf", "JetBrainsMono-BoldItalic.ttf",
        ];

        for name in font_files {
            let bytes = load_font(name)?;
            match Font::new(bytes, 0) {
                Ok(font) => {
                    book.push(font.clone());
                    fonts.push(font);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load font {}: {}", name, e);
                }
            }
        }

        Ok(Self { fonts, book })
    }
}

/// Typst World implementation for CV compilation
struct CVWorld {
    main_source: Source,
    fonts: Arc<FontBundle>,
    library: LazyHash<Library>,
    main_file: FileId,
}

impl CVWorld {
    fn new(template_source: &str, cv: &CVData, theme: CVTheme, template_dir: &Path, font_dir: &Path) -> Result<Self> {
        let fonts = Arc::new(FontBundle::load(font_dir)?);
        let library = LazyHash::new(Library::default());
        let main_file = FileId::new(None, VirtualPath::new("main.typ").into_owned());
        
        let main_source = Self::build_source(template_source, cv, theme, main_file)?;
        
        Ok(Self {
            main_source,
            fonts,
            library,
            main_file,
        })
    }

    fn build_source(template_source: &str, cv: &CVData, theme: CVTheme, main_file: FileId) -> Result<Source> {
        let colors = ThemeColors::for_theme(cv.template.clone(), theme);
        
        let data_dict = Self::cv_to_typst_dict(cv, &colors)?;
        
        let rendered = format!(
            r#"{template}

// Theme color overrides
#let accent-color = rgb("{accent}")
#let text-color = rgb("{text}")
#let muted-color = rgb("{muted}")
#let bg-color = rgb("{bg}")
#let dark-text = rgb("{dark_text}")
#let rule-color = rgb("{rule}")
#let card-bg = rgb("{card_bg}")
#let card-border = rgb("{card_border}")
#let primary-color = rgb("{primary}")
#let secondary-color = rgb("{secondary}")
#let line-color = rgb("{line}")

// Injected CV data
#let cv-data = {data_dict}

// Render the CV
#render-cv(cv-data)
"#,
            template = template_source,
            accent = colors.accent_color,
            text = colors.text_color,
            muted = colors.muted_color,
            bg = colors.bg_color,
            dark_text = colors.dark_text,
            rule = colors.rule_color,
            card_bg = colors.card_bg,
            card_border = colors.card_border,
            primary = colors.primary_color,
            secondary = colors.secondary_color,
            line = colors.line_color,
            data_dict = data_dict,
        );
        
        Ok(Source::new(main_file, rendered))
    }

    fn cv_to_typst_dict(cv: &CVData, colors: &ThemeColors) -> Result<String> {
        let mut dict = IndexMap::new();
        
        // Personal info
        let mut personal = IndexMap::new();
        personal.insert("full_name".into(), Value::Str(cv.full_name.clone().into()));
        personal.insert("email".into(), Value::Str(cv.email.clone().into()));
        personal.insert("phone".into(), Value::Str(cv.phone.clone().unwrap_or_default().into()));
        personal.insert("location".into(), Value::Str(cv.location.clone().unwrap_or_default().into()));
        personal.insert("linkedin".into(), Value::Str(cv.linkedin.clone().unwrap_or_default().into()));
        personal.insert("github".into(), Value::Str(cv.github.clone().unwrap_or_default().into()));
        personal.insert("portfolio".into(), Value::Str(cv.portfolio.clone().unwrap_or_default().into()));
        personal.insert("summary".into(), Value::Str(cv.summary.clone().unwrap_or_default().into()));
        dict.insert("personal".into(), Value::Dict(Dict::from(personal)));
        
        // Experience
        let exp_array: Vec<Value> = cv.experience.iter().map(|e| {
            let mut m = IndexMap::new();
            m.insert("title".into(), Value::Str(e.title.clone().into()));
            m.insert("company".into(), Value::Str(e.company.clone().into()));
            m.insert("location".into(), Value::Str(e.location.clone().unwrap_or_default().into()));
            let date_str = format!("{} - {}", 
                e.start_date.format("%b %Y"),
                if e.current { "Present".to_string() } else { e.end_date.map(|d| d.format("%b %Y").to_string()).unwrap_or_default() }
            );
            m.insert("date".into(), Value::Str(date_str.into()));
            m.insert("description".into(), Value::Str(e.description.clone().into()));
            m.insert("skills".into(), Value::Array(Array::from(e.skills.iter().map(|s| Value::Str(s.clone().into())).collect::<Vec<_>>())));
            Value::Dict(Dict::from(m))
        }).collect();
        dict.insert("experience".into(), Value::Array(Array::from(exp_array)));
        
        // Education
        let edu_array: Vec<Value> = cv.education.iter().map(|e| {
            let mut m = IndexMap::new();
            m.insert("degree".into(), Value::Str(e.degree.clone().into()));
            m.insert("institution".into(), Value::Str(e.institution.clone().into()));
            m.insert("location".into(), Value::Str(e.location.clone().unwrap_or_default().into()));
            let date_str = format!("{} - {}", 
                e.start_date.format("%Y"),
                if e.current { "Present".to_string() } else { e.end_date.map(|d| d.format("%Y").to_string()).unwrap_or_default() }
            );
            m.insert("date".into(), Value::Str(date_str.into()));
            m.insert("field".into(), Value::Str(e.field_of_study.clone().unwrap_or_default().into()));
            m.insert("grade".into(), Value::Str(e.grade.clone().unwrap_or_default().into()));
            m.insert("description".into(), Value::Str(e.description.clone().unwrap_or_default().into()));
            Value::Dict(Dict::from(m))
        }).collect();
        dict.insert("education".into(), Value::Array(Array::from(edu_array)));
        
        // Skills
        let skills_array: Vec<Value> = cv.skills.iter().map(|s| {
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::Str(s.name.clone().into()));
            m.insert("category".into(), Value::Str(s.category.clone().unwrap_or_else(|| "General".to_string()).into()));
            m.insert("level".into(), Value::Str(s.level.as_ref().map(|l| Self::skill_level_to_str(l)).unwrap_or_default().into()));
            Value::Dict(Dict::from(m))
        }).collect();
        dict.insert("skills".into(), Value::Array(Array::from(skills_array)));
        
        // Languages
        let lang_array: Vec<Value> = cv.languages.iter().map(|l| {
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::Str(l.name.clone().into()));
            m.insert("proficiency".into(), Value::Str(Self::lang_proficiency_to_str(&l.proficiency).into()));
            Value::Dict(Dict::from(m))
        }).collect();
        dict.insert("languages".into(), Value::Array(Array::from(lang_array)));
        
        // Projects
        let proj_array: Vec<Value> = cv.projects.iter().map(|p| {
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::Str(p.name.clone().into()));
            m.insert("description".into(), Value::Str(p.description.clone().into()));
            m.insert("technologies".into(), Value::Array(Array::from(p.technologies.iter().map(|t| Value::Str(t.clone().into())).collect::<Vec<_>>())));
            let mut links = Vec::new();
            if let Some(url) = &p.url {
                links.push(format!("#link(\"{}\")[Live]", Self::escape_typst(url)));
            }
            if let Some(gh) = &p.github {
                links.push(format!("#link(\"{}\")[Code]", Self::escape_typst(gh)));
            }
            m.insert("links".into(), Value::Array(Array::from(links.iter().map(|l| Value::Str(l.clone().into())).collect::<Vec<_>>())));
            Value::Dict(Dict::from(m))
        }).collect();
        dict.insert("projects".into(), Value::Array(Array::from(proj_array)));
        
        // Certifications
        let cert_array: Vec<Value> = cv.certifications.iter().map(|c| {
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::Str(c.name.clone().into()));
            m.insert("issuer".into(), Value::Str(c.issuer.clone().into()));
            m.insert("date".into(), Value::Str(c.date.format("%b %Y").to_string().into()));
            m.insert("expiry".into(), Value::Str(c.expiry.map(|d| d.format("%b %Y").to_string()).unwrap_or_default().into()));
            m.insert("url".into(), Value::Str(c.url.clone().unwrap_or_default().into()));
            m.insert("credential_id".into(), Value::Str(c.credential_id.clone().unwrap_or_default().into()));
            Value::Dict(Dict::from(m))
        }).collect();
        dict.insert("certifications".into(), Value::Array(Array::from(cert_array)));
        
        Ok(Self::dict_to_typst_literal(&Value::Dict(Dict::from(dict))))
    }

    fn skill_level_to_str(level: &SkillLevel) -> &'static str {
        match level {
            SkillLevel::Beginner => "Beginner",
            SkillLevel::Intermediate => "Intermediate",
            SkillLevel::Advanced => "Advanced",
            SkillLevel::Expert => "Expert",
        }
    }

    fn lang_proficiency_to_str(prof: &LanguageProficiency) -> &'static str {
        match prof {
            LanguageProficiency::Native => "Native",
            LanguageProficiency::Fluent => "Fluent",
            LanguageProficiency::Professional => "Professional",
            LanguageProficiency::Conversational => "Conversational",
            LanguageProficiency::Basic => "Basic",
        }
    }

    fn escape_typst(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn dict_to_typst_literal(value: &Value) -> String {
        match value {
            Value::Str(s) => {
                let s_str: &str = s.as_str();
                format!("\"{}\"", s_str.replace('\\', "\\\\").replace('"', "\\\""))
            }
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::dict_to_typst_literal).collect();
                format!("({})", items.join(", "))
            }
            Value::Dict(dict) => {
                let items: Vec<String> = dict.iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::dict_to_typst_literal(v)))
                    .collect();
                format!("({})", items.join(", "))
            }
            _ => "none".to_string(),
        }
    }
}

impl World for CVWorld {
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_source.id() {
            Ok(self.main_source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        let name = path.as_str();
        let bytes = match name {
            "Inter-Regular.ttf" => Some(&self.fonts.fonts[0]),
            "Inter-Medium.ttf" => self.fonts.fonts.get(1),
            "Inter-SemiBold.ttf" => self.fonts.fonts.get(2),
            "Inter-Bold.ttf" => self.fonts.fonts.get(3),
            "Inter-Italic.ttf" => self.fonts.fonts.get(4),
            "Inter-MediumItalic.ttf" => self.fonts.fonts.get(5),
            "Inter-SemiBoldItalic.ttf" => self.fonts.fonts.get(6),
            "Inter-BoldItalic.ttf" => self.fonts.fonts.get(7),
            "JetBrainsMono-Regular.ttf" => self.fonts.fonts.get(8),
            "JetBrainsMono-Medium.ttf" => self.fonts.fonts.get(9),
            "JetBrainsMono-SemiBold.ttf" => self.fonts.fonts.get(10),
            "JetBrainsMono-Bold.ttf" => self.fonts.fonts.get(11),
            "JetBrainsMono-Italic.ttf" => self.fonts.fonts.get(12),
            "JetBrainsMono-MediumItalic.ttf" => self.fonts.fonts.get(13),
            "JetBrainsMono-SemiBoldItalic.ttf" => self.fonts.fonts.get(14),
            "JetBrainsMono-BoldItalic.ttf" => self.fonts.fonts.get(15),
            _ => None,
        };
        
        if let Some(font) = bytes {
            // Font doesn't implement a direct way to get bytes back, so we need a different approach
            // For now, return an error for font files - they're handled by the book() method
            Err(FileError::NotFound(path.to_path_buf()))
        } else {
            Err(FileError::NotFound(path.to_path_buf()))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.book.fonts().get(index).cloned()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        // This is a workaround - FontBook doesn't wrap in LazyHash
        // We'll create a static reference
        static mut BOOK_CACHE: Option<LazyHash<FontBook>> = None;
        unsafe {
            if BOOK_CACHE.is_none() {
                BOOK_CACHE = Some(LazyHash::new(self.fonts.book.clone()));
            }
            BOOK_CACHE.as_ref().unwrap()
        }
    }

    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn main(&self) -> FileId {
        self.main_file
    }

    fn today(&self, offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        let offset_secs = offset.map(|d| d.as_millis() / 1000).unwrap_or(0);
        let now = chrono::Utc::now() + chrono::Duration::seconds(offset_secs as i64);
        Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8).ok()
    }
}

pub struct CVCompiler {
    template_dir: PathBuf,
    font_dir: PathBuf,
}

impl CVCompiler {
    /// Create a new CVCompiler with the given template directory.
    /// Fonts are expected in a "fonts" subdirectory or can be set via with_fonts().
    /// If CV_TEMPLATES_DIR and CV_FONTS_DIR environment variables are set (by build.rs),
    /// they will be used as defaults.
    pub fn new(template_dir: PathBuf) -> Self {
        let font_dir = template_dir.parent()
            .map(|p| p.join("resources/fonts"))
            .unwrap_or_else(|| PathBuf::from("resources/fonts"));
        Self {
            template_dir,
            font_dir,
        }
    }

    /// Create a CVCompiler using the environment variables set by build.rs
    /// Falls back to the provided paths if env vars are not set.
    pub fn from_env(template_fallback: PathBuf, font_fallback: PathBuf) -> Self {
        let template_dir = env::var("CV_TEMPLATES_DIR")
            .map(PathBuf::from)
            .unwrap_or(template_fallback);
        let font_dir = env::var("CV_FONTS_DIR")
            .map(PathBuf::from)
            .unwrap_or(font_fallback);
        Self {
            template_dir,
            font_dir,
        }
    }

    /// Override the font directory
    pub fn with_fonts(mut self, font_dir: PathBuf) -> Self {
        self.font_dir = font_dir;
        self
    }

    /// Compile CVData to PDF bytes
    pub fn compile(&self, cv: &CVData) -> Result<Vec<u8>> {
        // Load template
        let template = self.load_template(cv.template.clone())?;
        
        // Create world and compile
        let world = CVWorld::new(&template, cv, cv.theme.clone(), &self.template_dir, &self.font_dir)?;
        
        // Compile to document
        let compile_result = typst::compile(&world);
        let doc = compile_result.output
            .map_err(|e| anyhow::anyhow!("Typst compilation failed: {}", e.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join("; ")))?;
        
        // Convert to PDF
        let pdf_options = PdfOptions::default();
        let pdf = typst_pdf::pdf(&doc, &pdf_options)
            .map_err(|e| anyhow::anyhow!("PDF generation failed: {}", e.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join("; ")))?;
        
        Ok(pdf)
    }

    fn load_template(&self, template: CVTemplate) -> Result<String> {
        let filename = match template {
            CVTemplate::Modern => "modern.typ",
            CVTemplate::Classic => "classic.typ",
            CVTemplate::Minimal => "minimal.typ",
        };
        let path = self.template_dir.join(filename);
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to load template: {}", path.display()))
    }
}

/// Standalone function to compile CV to PDF
pub fn compile_cv_to_pdf(template_dir: &Path, font_dir: &Path, cv: &CVData) -> Result<Vec<u8>> {
    let compiler = CVCompiler::new(template_dir.to_path_buf())
        .with_fonts(font_dir.to_path_buf());
    compiler.compile(cv)
}

/// Standalone function to compile CV to PDF using environment variables
/// set by build.rs (CV_TEMPLATES_DIR, CV_FONTS_DIR)
pub fn compile_cv_to_pdf_from_env(cv: &CVData) -> Result<Vec<u8>> {
    let template_fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
    let font_fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
    let compiler = CVCompiler::from_env(template_fallback, font_fallback);
    compiler.compile(cv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cv::{CVData, CVTemplate, CVTheme, Experience, Education, Skill, Language, Project, Certification, SkillLevel, LanguageProficiency};
    use uuid::Uuid;
    use chrono::{Utc, TimeZone};

    fn sample_cv() -> CVData {
        CVData {
            full_name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            phone: Some("+1-555-0123".to_string()),
            location: Some("San Francisco, CA".to_string()),
            linkedin: Some("linkedin.com/in/johndoe".to_string()),
            github: Some("github.com/johndoe".to_string()),
            portfolio: Some("johndoe.dev".to_string()),
            summary: Some("Experienced software engineer with a passion for building scalable systems.".to_string()),
            experience: vec![
                Experience {
                    id: Uuid::new_v4(),
                    title: "Senior Software Engineer".to_string(),
                    company: "TechCorp Inc.".to_string(),
                    location: Some("San Francisco, CA".to_string()),
                    start_date: Utc.with_ymd_and_hms(2021, 1, 15, 0, 0, 0).unwrap(),
                    end_date: None,
                    current: true,
                    description: "Lead backend development for high-traffic APIs. Designed and implemented microservices architecture.".to_string(),
                    skills: vec!["Rust".to_string(), "PostgreSQL".to_string(), "Kubernetes".to_string()],
                },
                Experience {
                    id: Uuid::new_v4(),
                    title: "Software Engineer".to_string(),
                    company: "StartupXYZ".to_string(),
                    location: Some("Remote".to_string()),
                    start_date: Utc.with_ymd_and_hms(2018, 6, 1, 0, 0, 0).unwrap(),
                    end_date: Some(Utc.with_ymd_and_hms(2020, 12, 31, 0, 0, 0).unwrap()),
                    current: false,
                    description: "Full-stack development for SaaS platform. Built CI/CD pipelines and monitoring systems.".to_string(),
                    skills: vec!["Python".to_string(), "React".to_string(), "AWS".to_string()],
                },
            ],
            education: vec![
                Education {
                    id: Uuid::new_v4(),
                    institution: "Stanford University".to_string(),
                    degree: "M.S. Computer Science".to_string(),
                    field_of_study: Some("Distributed Systems".to_string()),
                    location: Some("Stanford, CA".to_string()),
                    start_date: Utc.with_ymd_and_hms(2016, 9, 1, 0, 0, 0).unwrap(),
                    end_date: Some(Utc.with_ymd_and_hms(2018, 6, 15, 0, 0, 0).unwrap()),
                    current: false,
                    description: Some("Thesis: Optimizing Consensus in Distributed Databases".to_string()),
                    grade: Some("3.9/4.0".to_string()),
                },
            ],
            skills: vec![
                Skill { id: Uuid::new_v4(), name: "Rust".to_string(), category: Some("Technical".to_string()), level: Some(SkillLevel::Expert), years: Some(5) },
                Skill { id: Uuid::new_v4(), name: "Python".to_string(), category: Some("Technical".to_string()), level: Some(SkillLevel::Advanced), years: Some(7) },
                Skill { id: Uuid::new_v4(), name: "PostgreSQL".to_string(), category: Some("Technical".to_string()), level: Some(SkillLevel::Advanced), years: Some(5) },
                Skill { id: Uuid::new_v4(), name: "Kubernetes".to_string(), category: Some("Technical".to_string()), level: Some(SkillLevel::Intermediate), years: Some(3) },
                Skill { id: Uuid::new_v4(), name: "Leadership".to_string(), category: Some("Soft".to_string()), level: Some(SkillLevel::Advanced), years: Some(4) },
            ],
            languages: vec![
                Language { id: Uuid::new_v4(), name: "English".to_string(), proficiency: LanguageProficiency::Native },
                Language { id: Uuid::new_v4(), name: "Spanish".to_string(), proficiency: LanguageProficiency::Professional },
            ],
            projects: vec![
                Project {
                    id: Uuid::new_v4(),
                    name: "Distributed Task Queue".to_string(),
                    description: "High-performance task queue built in Rust with Redis backend.".to_string(),
                    url: Some("https://github.com/johndoe/taskq".to_string()),
                    github: Some("https://github.com/johndoe/taskq".to_string()),
                    technologies: vec!["Rust".to_string(), "Redis".to_string(), "Tokio".to_string()],
                    start_date: Some(Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap()),
                    end_date: Some(Utc.with_ymd_and_hms(2022, 6, 1, 0, 0, 0).unwrap()),
                },
            ],
            certifications: vec![
                Certification {
                    id: Uuid::new_v4(),
                    name: "AWS Certified Solutions Architect".to_string(),
                    issuer: "Amazon Web Services".to_string(),
                    date: Utc.with_ymd_and_hms(2021, 3, 15, 0, 0, 0).unwrap(),
                    expiry: Some(Utc.with_ymd_and_hms(2024, 3, 15, 0, 0, 0).unwrap()),
                    url: Some("https://aws.amazon.com/certification/".to_string()),
                    credential_id: Some("ABC123XYZ".to_string()),
                },
            ],
            template: CVTemplate::Modern,
            theme: CVTheme::Light,
        }
    }

    #[test]
    fn test_compiler_creation() {
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
        let font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
        let compiler = CVCompiler::new(template_dir).with_fonts(font_dir);
        assert!(compiler.template_dir.exists());
    }

    #[test]
    #[ignore = "requires fonts and templates to be present"]
    fn test_compile_sample_cv_to_pdf() {
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
        let font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
        
        let cv = sample_cv();
        let compiler = CVCompiler::new(template_dir).with_fonts(font_dir);
        
        let pdf = compiler.compile(&cv).expect("Failed to compile CV to PDF");
        
        assert!(pdf.len() > 100, "PDF should have substantial content");
        assert_eq!(&pdf[0..4], b"%PDF", "PDF should start with %PDF magic bytes");
        
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"), "PDF should end with %%EOF marker");
    }

    #[test]
    #[ignore = "requires fonts and templates to be present"]
    fn test_all_templates_compile() {
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
        let font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
        
        for template in [CVTemplate::Modern, CVTemplate::Classic, CVTemplate::Minimal] {
            for theme in [CVTheme::Light, CVTheme::Dark] {
                let mut cv = sample_cv();
                cv.template = template;
                cv.theme = theme;
                
                let compiler = CVCompiler::new(template_dir.clone()).with_fonts(font_dir.clone());
                let pdf = compiler.compile(&cv).expect(&format!("Failed to compile {} {:?}", template, theme));
                
                assert_eq!(&pdf[0..4], b"%PDF", "PDF should start with %PDF magic bytes for {} {:?}", template, theme);
            }
        }
    }
}