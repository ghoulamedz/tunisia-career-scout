//! Rule-Based Extractor
//!
//! Deterministic extraction using schema.org JSON-LD, microdata, ATS heuristics, and meta tags.
//! Returns partial JobPosting with per-field confidence scores.

use anyhow::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::scrapers::pipeline::{RawContent, ExtractionResult, ExtractionMethod, ContentType, ExtractionMetrics};
use crate::models::posting::{JobPosting, ApplicationStep, ActionType};

/// Per-field confidence score (0.0 - 1.0)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfidenceMap {
    pub fields: HashMap<String, f32>,
}

impl ConfidenceMap {
    pub fn new() -> Self {
        Self { fields: HashMap::new() }
    }

    pub fn set(&mut self, field: &str, score: f32) {
        self.fields.insert(field.to_string(), score.clamp(0.0, 1.0));
    }

    pub fn get(&self, field: &str) -> f32 {
        self.fields.get(field).copied().unwrap_or(0.0)
    }

    pub fn average(&self) -> f32 {
        if self.fields.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.fields.values().sum();
        sum / self.fields.len() as f32
    }
}

pub struct RuleExtractor;

impl RuleExtractor {
    pub fn new() -> Self {
        Self
    }

    pub async fn extract(&self, raw: &RawContent, source_id: Uuid) -> Result<ExtractionResult> {
        let result = match raw.content_type {
            ContentType::Html => self.extract_from_html(raw, source_id).await,
            ContentType::Rss => self.extract_from_rss(raw, source_id).await,
            ContentType::Email => self.extract_from_email(raw, source_id).await,
            ContentType::Json => self.extract_from_json(raw, source_id).await,
        }?;

        Ok(result)
    }

    async fn extract_from_html(&self, raw: &RawContent, source_id: Uuid) -> Result<ExtractionResult> {
        let document = Html::parse_document(&raw.content);
        let mut posting = JobPosting::new(
            source_id,
            raw.content_hash.clone(),
            String::new(),
            String::new(),
            raw.content.clone(),
            raw.fetched_at,
            raw.content_hash.clone(),
        );
        posting.raw_html = Some(raw.content.clone());
        posting.application_url = Some(raw.source_url.clone());

        let mut confidence = ConfidenceMap::new();

        // 1. Try schema.org JSON-LD (highest confidence)
        if let Some(ld) = self.extract_json_ld(&document) {
            self.apply_json_ld(&mut posting, &ld, &mut confidence);
        }

        // 2. Try microdata (high confidence)
        self.extract_microdata(&document, &mut posting, &mut confidence);

        // 3. Try ATS patterns (high confidence for known ATS)
        self.extract_ats_patterns(&document, &mut posting, &raw.source_url, raw, &mut confidence);

        // 4. Meta tags (medium confidence)
        self.extract_meta_tags(&document, &mut posting, &mut confidence);

        // 5. Fallback: readability-style content extraction (low confidence)
        if posting.field_is_empty("title") || posting.field_is_empty("company") || posting.field_is_empty("description") {
            self.extract_fallback(&document, &mut posting, &mut confidence);
        }

        // 6. Extract application steps
        self.extract_application_steps(&document, &mut posting, &mut confidence);

        // 7. Extract skills from description text patterns
        self.extract_skills_from_text(&mut posting, &mut confidence);

        // Calculate overall confidence from per-field scores
        let overall_confidence = confidence.average();
        let missing = self.identify_missing(&posting);

        Ok(ExtractionResult {
            posting,
            extraction_method: ExtractionMethod::Rules,
            confidence: overall_confidence,
            missing_fields: missing,
            warnings: Vec::new(),
            field_confidence: confidence,
            metrics: ExtractionMetrics::default(),
        })
    }

    /// Extract JSON-LD structured data
    fn extract_json_ld(&self, document: &Html) -> Option<Value> {
        let selector = Selector::parse("script[type='application/ld+json']").ok()?;
        for script in document.select(&selector) {
            let content = script.inner_html();
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                let items = if json.is_array() { 
                    json.as_array().unwrap() 
                } else { 
                    &vec![json] 
                };
                for item in items {
                    if self.is_job_posting(item) {
                        return Some(item.clone());
                    }
                }
            }
        }
        None
    }

    fn is_job_posting(&self, item: &Value) -> bool {
        match item.get("@type") {
            Some(Value::String(s)) => s == "JobPosting",
            Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some("JobPosting")),
            _ => false,
        }
    }

    /// Apply JSON-LD JobPosting data to posting with confidence scores
    fn apply_json_ld(&self, posting: &mut JobPosting, ld: &Value, confidence: &mut ConfidenceMap) {
        // Title - high confidence
        if let Some(title) = ld.get("title").and_then(|v| v.as_str()) {
            if !title.is_empty() {
                posting.title = title.to_string();
                confidence.set("title", 0.95);
            }
        }

        // Company - high confidence
        if let Some(company) = ld.get("hiringOrganization")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str()) {
            if !company.is_empty() {
                posting.company = company.to_string();
                confidence.set("company", 0.95);
            }
        }

        // Description - high confidence
        if let Some(desc) = ld.get("description").and_then(|v| v.as_str()) {
            if !desc.is_empty() {
                posting.description = desc.to_string();
                confidence.set("description", 0.95);
            }
        }

        // Location - high confidence
        if let Some(loc) = self.extract_location_from_json_ld(ld) {
            posting.location = Some(loc.clone());
            confidence.set("location", 0.9);
        }

        // Job type
        if let Some(jt) = ld.get("employmentType").and_then(|v| v.as_str()) {
            if !jt.is_empty() {
                posting.job_type = Some(jt.to_string());
                confidence.set("job_type", 0.9);
            }
        }

        // Salary
        if let Some(salary) = self.extract_salary_from_json_ld(ld) {
            posting.salary_range = Some(salary);
            confidence.set("salary_range", 0.9);
        }

        // Posted date
        if let Some(date) = ld.get("datePosted").and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc)) {
            posting.posted_date = Some(date);
            confidence.set("posted_date", 0.9);
        }

        // Deadline
        if let Some(date) = ld.get("validThrough").and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc)) {
            posting.deadline = Some(date);
            confidence.set("deadline", 0.9);
        }

        // Remote policy
        if let Some(remote) = ld.get("jobLocationType").and_then(|v| v.as_str()) {
            posting.remote_policy = Some(remote.to_string());
            confidence.set("remote_policy", 0.9);
        }

        // Skills
        if let Some(skills) = ld.get("skills").and_then(|v| v.as_array()) {
            let skill_list: Vec<String> = skills.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !skill_list.is_empty() {
                posting.skills = sqlx::types::Json(skill_list);
                confidence.set("skills", 0.85);
            }
        }

        // Experience requirements
        if let Some(exp) = ld.get("experienceRequirements").and_then(|v| v.as_str()) {
            if !exp.is_empty() {
                posting.experience_required = Some(exp.to_string());
                confidence.set("experience_required", 0.85);
            }
        }

        // Education requirements
        if let Some(edu) = ld.get("educationRequirements").and_then(|v| v.as_str()) {
            if !edu.is_empty() {
                posting.education_required = Some(edu.to_string());
                confidence.set("education_required", 0.85);
            }
        }

        // Qualifications (requirements/benefits)
        if let Some(qual) = ld.get("qualifications").and_then(|v| v.as_str()) {
            if !qual.is_empty() {
                // Could be requirements or benefits - add to description if empty
                if posting.description.is_empty() {
                    posting.description = qual.to_string();
                }
            }
        }

        // Responsibilities (requirements)
        if let Some(resp) = ld.get("responsibilities").and_then(|v| v.as_str()) {
            if !resp.is_empty() && posting.description.is_empty() {
                posting.description = resp.to_string();
            }
        }
    }

    fn extract_location_from_json_ld(&self, ld: &Value) -> Option<String> {
        // Handle different location formats
        let job_location = ld.get("jobLocation")?;
        
        // Single location object
        if let Some(address) = job_location.get("address") {
            if let Some(locality) = address.get("addressLocality").and_then(|v| v.as_str()) {
                let mut parts = vec![locality.to_string()];
                if let Some(region) = address.get("addressRegion").and_then(|v| v.as_str()) {
                    parts.push(region.to_string());
                }
                if let Some(country) = address.get("addressCountry").and_then(|v| v.as_str()) {
                    parts.push(country.to_string());
                }
                return Some(parts.join(", "));
            }
        }

        // Array of locations
        if let Some(arr) = job_location.as_array() {
            for loc in arr {
                if let Some(addr) = loc.get("address") {
                    if let Some(locality) = addr.get("addressLocality").and_then(|v| v.as_str()) {
                        let mut parts = vec![locality.to_string()];
                        if let Some(region) = addr.get("addressRegion").and_then(|v| v.as_str()) {
                            parts.push(region.to_string());
                        }
                        if let Some(country) = addr.get("addressCountry").and_then(|v| v.as_str()) {
                            parts.push(country.to_string());
                        }
                        return Some(parts.join(", "));
                    }
                }
            }
        }

        None
    }

    fn extract_salary_from_json_ld(&self, ld: &Value) -> Option<String> {
        let base_salary = ld.get("baseSalary")?;
        let value = base_salary.get("value")?;
        
        let min = value.get("minValue").and_then(|v| v.as_str());
        let max = value.get("maxValue").and_then(|v| v.as_str());
        let currency = base_salary.get("currency").and_then(|v| v.as_str()).unwrap_or("USD");
        let unit = value.get("unitText").and_then(|v| v.as_str()).unwrap_or("YEAR");

        match (min, max) {
            (Some(min), Some(max)) => Some(format!("{} {} - {} {} per {}", currency, min, currency, max, unit.to_lowercase())),
            (Some(min), None) => Some(format!("{} {} per {}", currency, min, unit.to_lowercase())),
            (None, Some(max)) => Some(format!("Up to {} {} per {}", currency, max, unit.to_lowercase())),
            _ => None,
        }
    }

    /// Extract HTML microdata (itemprop)
    fn extract_microdata(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let mappings = [
            ("title", "[itemprop='title']", 0.9),
            ("company", "[itemprop='hiringOrganization'] [itemprop='name']", 0.9),
            ("description", "[itemprop='description']", 0.9),
            ("location", "[itemprop='jobLocation'] [itemprop='addressLocality']", 0.85),
            ("job_type", "[itemprop='employmentType']", 0.85),
            ("salary_range", "[itemprop='baseSalary'] [itemprop='minValue']", 0.85),
            ("experience_required", "[itemprop='experienceRequirements']", 0.8),
            ("education_required", "[itemprop='educationRequirements']", 0.8),
            ("remote_policy", "[itemprop='jobLocationType']", 0.85),
        ];

        for (field, selector_str, conf) in mappings {
            if posting.field_is_empty(field) {
                if let Ok(sel) = Selector::parse(selector_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            posting.set_field(field, text);
                            confidence.set(field, conf);
                        }
                    }
                }
            }
        }

        // Handle location with full address
        if posting.location.is_none() {
            if let Ok(sel) = Selector::parse("[itemprop='jobLocation'] [itemprop='address']") {
                for el in document.select(&sel) {
                    let mut parts = Vec::new();
                    if let Ok(locality_sel) = Selector::parse("[itemprop='addressLocality']") {
                        if let Some(e) = el.select(&locality_sel).next() {
                            parts.push(e.text().collect::<String>().trim().to_string());
                        }
                    }
                    if let Ok(region_sel) = Selector::parse("[itemprop='addressRegion']") {
                        if let Some(e) = el.select(&region_sel).next() {
                            parts.push(e.text().collect::<String>().trim().to_string());
                        }
                    }
                    if let Ok(country_sel) = Selector::parse("[itemprop='addressCountry']") {
                        if let Some(e) = el.select(&country_sel).next() {
                            parts.push(e.text().collect::<String>().trim().to_string());
                        }
                    }
                    if !parts.is_empty() {
                        posting.location = Some(parts.join(", "));
                        confidence.set("location", 0.85);
                        break;
                    }
                }
            }
        }
    }

    /// Extract ATS-specific patterns
    fn extract_ats_patterns(&self, document: &Html, posting: &mut JobPosting, url: &str, raw: &RawContent, confidence: &mut ConfidenceMap) {
        let url_lower = url.to_lowercase();
        let content_lower = raw.content.to_lowercase();

        // Greenhouse
        if url_lower.contains("greenhouse.io") || content_lower.contains("greenhouse") {
            self.extract_greenhouse(document, posting, confidence);
        }

        // Lever
        if url_lower.contains("lever.co") || content_lower.contains("lever") {
            self.extract_lever(document, posting, confidence);
        }

        // Workable
        if url_lower.contains("workable.com") || content_lower.contains("workable") {
            self.extract_workable(document, posting, confidence);
        }

        // SmartRecruiters
        if url_lower.contains("smartrecruiters.com") || content_lower.contains("smartrecruiters") {
            self.extract_smartrecruiters(document, posting, confidence);
        }

        // BambooHR
        if url_lower.contains("bamboohr.com") || content_lower.contains("bamboohr") {
            self.extract_bamboohr(document, posting, confidence);
        }

        # [cfg(test)]
        {
            // For testing: allow override via env var
            if std::env::var("TEST_ATS").unwrap_or_default() == "greenhouse" {
                self.extract_greenhouse(document, posting, confidence);
            }
        }
    }

    /// Greenhouse ATS extraction
    fn extract_greenhouse(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        // Greenhouse often has structured data in script tags
        let selectors = [
            "script#job-search-results-data",
            "script[data-job-id]",
            "script:contains('GREENHOUSE')",
        ];

        for sel_str in selectors {
            if let Ok(sel) = Selector::parse(sel_str) {
                for script in document.select(&sel) {
                    let content = script.inner_html();
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        // Try to find job data
                        if let Some(job) = json.get("job").or_else(|| json.get("jobs").and_then(|v| v.as_array()).and_then(|a| a.first())) {
                            self.apply_greenhouse_job(posting, job, confidence);
                            return;
                        }
                    }
                }
            }
        }

        // Fallback: look for Greenhouse-specific meta tags
        if let Ok(sel) = Selector::parse("meta[name='greenhouse:job_title']") {
            if let Some(el) = document.select(&sel).next() {
                if let Some(content) = el.value().attr("content") {
                    posting.title = content.to_string();
                    confidence.set("title", 0.9);
                }
            }
        }
    }

    fn apply_greenhouse_job(&self, posting: &mut JobPosting, job: &Value, confidence: &mut ConfidenceMap) {
        if let Some(title) = job.get("title").and_then(|v| v.as_str()) {
            posting.title = title.to_string();
            confidence.set("title", 0.95);
        }
        if let Some(company) = job.get("company").and_then(|v| v.get("name")).and_then(|v| v.as_str()) {
            posting.company = company.to_string();
            confidence.set("company", 0.95);
        }
        if let Some(desc) = job.get("content").and_then(|v| v.as_str()).or_else(|| job.get("description").and_then(|v| v.as_str())) {
            posting.description = desc.to_string();
            confidence.set("description", 0.95);
        }
        if let Some(loc) = job.get("location").and_then(|v| v.get("name")).and_then(|v| v.as_str()) {
            posting.location = Some(loc.to_string());
            confidence.set("location", 0.9);
        }
        if let Some(jt) = job.get("employment_type").and_then(|v| v.as_str()) {
            posting.job_type = Some(jt.to_string());
            confidence.set("job_type", 0.9);
        }
        if let Some(remote) = job.get("remote").and_then(|v| v.as_bool()) {
            posting.remote_policy = Some(if remote { "remote" } else { "onsite" }.to_string());
            confidence.set("remote_policy", 0.9);
        }
        if let Some(req) = job.get("requirements").and_then(|v| v.as_str()) {
            posting.experience_required = Some(req.to_string());
            confidence.set("experience_required", 0.85);
        }
    }

    /// Lever ATS extraction
    fn extract_lever(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let mappings = [
            ("title", "[data-qa='job-title']", 0.95),
            ("company", "[data-qa='company-name']", 0.95),
            ("location", "[data-qa='location']", 0.9),
            ("description", "[data-qa='description']", 0.95),
            ("job_type", "[data-qa='commitment']", 0.9),
            ("experience_required", "[data-qa='experience']", 0.85),
        ];

        for (field, selector_str, conf) in mappings {
            if posting.field_is_empty(field) {
                if let Ok(sel) = Selector::parse(selector_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            posting.set_field(field, text);
                            confidence.set(field, conf);
                        }
                    }
                }
            }
        }

        // Also check for JSON-LD in Lever pages
        if let Some(ld) = self.extract_json_ld(document) {
            if self.is_lever_job_posting(&ld) {
                self.apply_json_ld(posting, &ld, confidence);
            }
        }
    }

    fn is_lever_job_posting(&self, ld: &Value) -> bool {
        ld.get("@context").and_then(|v| v.as_str()).map(|s| s.contains("lever")).unwrap_or(false)
    }

    /// Workable ATS extraction
    fn extract_workable(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let mappings = [
            ("title", "[data-ui='job-title']", 0.95),
            ("company", "[data-ui='company-name']", 0.95),
            ("location", "[data-ui='location']", 0.9),
            ("description", "[data-ui='job-description']", 0.95),
            ("job_type", "[data-ui='employment-type']", 0.9),
            ("remote_policy", "[data-ui='workplace-type']", 0.9),
        ];

        for (field, selector_str, conf) in mappings {
            if posting.field_is_empty(field) {
                if let Ok(sel) = Selector::parse(selector_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            posting.set_field(field, text);
                            confidence.set(field, conf);
                        }
                    }
                }
            }
        }

        // Check for Workable's JSON data
        if let Ok(sel) = Selector::parse("script#workable-job-data") {
            if let Some(script) = document.select(&sel).next() {
                if let Ok(json) = serde_json::from_str::<Value>(&script.inner_html()) {
                    self.apply_workable_job(posting, &json, confidence);
                }
            }
        }
    }

    fn apply_workable_job(&self, posting: &mut JobPosting, job: &Value, confidence: &mut ConfidenceMap) {
        if let Some(title) = job.get("title").and_then(|v| v.as_str()) {
            posting.title = title.to_string();
            confidence.set("title", 0.95);
        }
        if let Some(company) = job.get("company").and_then(|v| v.as_str()) {
            posting.company = company.to_string();
            confidence.set("company", 0.95);
        }
        if let Some(desc) = job.get("description").and_then(|v| v.as_str()) {
            posting.description = desc.to_string();
            confidence.set("description", 0.95);
        }
        if let Some(loc) = job.get("location").and_then(|v| v.as_str()) {
            posting.location = Some(loc.to_string());
            confidence.set("location", 0.9);
        }
    }

    /// SmartRecruiters ATS extraction
    fn extract_smartrecruiters(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let mappings = [
            ("title", "[data-qa='job-title'], h1.job-title", 0.95),
            ("company", "[data-qa='company-name'], .company-name", 0.95),
            ("location", "[data-qa='location'], .job-location", 0.9),
            ("description", "[data-qa='job-description'], .job-description", 0.95),
            ("job_type", "[data-qa='job-type'], .job-type", 0.9),
            ("remote_policy", "[data-qa='remote-type'], .remote-type", 0.9),
            ("salary_range", "[data-qa='salary'], .salary", 0.85),
        ];

        for (field, selector_str, conf) in mappings {
            if posting.field_is_empty(field) {
                for sel_part in selector_str.split(',') {
                    let sel_part = sel_part.trim();
                    if let Ok(sel) = Selector::parse(sel_part) {
                        if let Some(el) = document.select(&sel).next() {
                            let text = el.text().collect::<String>().trim().to_string();
                            if !text.is_empty() {
                                posting.set_field(field, text);
                                confidence.set(field, conf);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // SmartRecruiters often has JSON-LD
        if let Some(ld) = self.extract_json_ld(document) {
            self.apply_json_ld(posting, &ld, confidence);
        }
    }

    /// BambooHR ATS extraction
    fn extract_bamboohr(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let mappings = [
            ("title", ".job-title, h1.title, #JobTitle", 0.95),
            ("company", ".company-name, .org-name", 0.95),
            ("location", ".location, .job-location, #Location", 0.9),
            ("description", ".job-description, .description, #JobDescription", 0.95),
            ("job_type", ".job-type, .employment-type, #JobType", 0.9),
            ("remote_policy", ".remote-policy, .work-type", 0.9),
        ];

        for (field, selector_str, conf) in mappings {
            if posting.field_is_empty(field) {
                for sel_part in selector_str.split(',') {
                    let sel_part = sel_part.trim();
                    if let Ok(sel) = Selector::parse(sel_part) {
                        if let Some(el) = document.select(&sel).next() {
                            let text = el.text().collect::<String>().trim().to_string();
                            if !text.is_empty() {
                                posting.set_field(field, text);
                                confidence.set(field, conf);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Extract standard HTML meta tags (Open Graph, Twitter, etc.)
    fn extract_meta_tags(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let meta_selectors = [
            // Open Graph
            ("title", "meta[property='og:title']", 0.7),
            ("description", "meta[property='og:description']", 0.7),
            ("company", "meta[property='og:site_name']", 0.6),
            // Twitter
            ("title", "meta[name='twitter:title']", 0.6),
            ("description", "meta[name='twitter:description']", 0.6),
            // Standard meta
            ("description", "meta[name='description']", 0.5),
            // Job-specific meta tags
            ("title", "meta[name='job-title'], meta[property='job:title']", 0.8),
            ("company", "meta[name='job-company'], meta[property='job:company']", 0.8),
            ("location", "meta[name='job-location'], meta[property='job:location']", 0.8),
            ("salary_range", "meta[name='job-salary'], meta[property='job:salary']", 0.8),
            ("job_type", "meta[name='job-type'], meta[property='job:employment_type']", 0.8),
            ("posted_date", "meta[name='job-date'], meta[property='job:date_posted']", 0.8),
            ("remote_policy", "meta[name='job-remote'], meta[property='job:remote']", 0.8),
        ];

        for (field, selector_str, conf) in meta_selectors {
            if posting.field_is_empty(field) {
                for sel_part in selector_str.split(',') {
                    let sel_part = sel_part.trim();
                    if let Ok(sel) = Selector::parse(sel_part) {
                        if let Some(el) = document.select(&sel).next() {
                            let content = el.value().attr("content").unwrap_or("").trim();
                            if !content.is_empty() {
                                if field == "posted_date" {
                                    if let Ok(dt) = DateTime::parse_from_rfc3339(content) {
                                        posting.posted_date = Some(dt.with_timezone(&Utc));
                                        confidence.set(field, conf);
                                    }
                                } else {
                                    posting.set_field(field, content.to_string());
                                    confidence.set(field, conf);
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Fallback extraction using common job posting patterns
    fn extract_fallback(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        // Title - try common selectors
        if posting.field_is_empty("title") {
            let title_selectors = [
                "h1.job-title", "h1.title", ".job-title h1", "[data-test='job-title']",
                "h1[itemprop='title']", ".job-header h1", ".position-title",
                "header h1", "h1:first-of-type",
            ];
            for sel_str in title_selectors {
                if let Ok(sel) = Selector::parse(sel_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() && text.len() < 200 {
                            posting.title = text;
                            confidence.set("title", 0.5);
                            break;
                        }
                    }
                }
            }
        }

        // Company
        if posting.field_is_empty("company") {
            let company_selectors = [
                ".company-name", ".employer-name", ".company", "[itemprop='hiringOrganization']",
                ".job-company", ".org-name", "[data-company]",
            ];
            for sel_str in company_selectors {
                if let Ok(sel) = Selector::parse(sel_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() && text.len() < 100 {
                            posting.company = text;
                            confidence.set("company", 0.5);
                            break;
                        }
                    }
                }
            }
        }

        // Description
        if posting.field_is_empty("description") {
            let desc_selectors = [
                ".job-description", ".description", "[itemprop='description']",
                ".job-details", ".job-content", "#job-description",
                "section.description", "div.description",
            ];
            for sel_str in desc_selectors {
                if let Ok(sel) = Selector::parse(sel_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() && text.len() > 50 {
                            posting.description = text;
                            confidence.set("description", 0.5);
                            break;
                        }
                    }
                }
            }
        }

        // Location
        if posting.location.is_none() {
            let loc_selectors = [
                ".location", ".job-location", "[itemprop='jobLocation']",
                ".job-location", "[data-location]", ".location-text",
            ];
            for sel_str in loc_selectors {
                if let Ok(sel) = Selector::parse(sel_str) {
                    if let Some(el) = document.select(&sel).next() {
                        let text = el.text().collect::<String>().trim().to_string();
                        if !text.is_empty() && text.len() < 100 {
                            posting.location = Some(text);
                            confidence.set("location", 0.5);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Extract application steps (links, emails, forms)
    fn extract_application_steps(&self, document: &Html, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        let mut steps = Vec::new();
        let mut order = 1;

        // Apply buttons/links
        let apply_selectors = [
            "a[href*='apply']", "button[data-apply]", "[data-qa='apply-button']",
            "a.apply-button", ".apply-link", "form[action*='apply']",
            "a[href*='application']", "a[href*='careers']",
        ];

        for sel_str in apply_selectors {
            if let Ok(sel) = Selector::parse(sel_str) {
                for el in document.select(&sel) {
                    let href = el.value().attr("href").or_else(|| el.value().attr("action"));
                    let text = el.text().collect::<String>().trim().to_string();

                    if let Some(url) = href {
                        // Convert relative URLs
                        let full_url = if url.starts_with("http") {
                            url.to_string()
                        } else if let Ok(base) = url::Url::parse(&posting.application_url.clone().unwrap_or_default()) {
                            base.join(url).ok().map(|u| u.to_string()).unwrap_or_else(|| url.to_string())
                        } else {
                            url.to_string()
                        };

                        steps.push(ApplicationStep {
                            order,
                            description: if text.is_empty() { "Apply via link".to_string() } else { text },
                            action_type: ActionType::Link,
                            target: full_url,
                            deadline: None,
                            required: true,
                            completed: false,
                        });
                        order += 1;
                    }
                }
                if !steps.is_empty() { break; }
            }
        }

        // Email application
        let email_selector = Selector::parse("a[href^='mailto:']").ok();
        if let Some(sel) = email_selector {
            for el in document.select(&sel) {
                if let Some(href) = el.value().attr("href") {
                    let email = href.strip_prefix("mailto:").unwrap_or(href);
                    if email.contains('@') {
                        steps.push(ApplicationStep {
                            order,
                            description: "Apply via email".to_string(),
                            action_type: ActionType::Email,
                            target: email.to_string(),
                            deadline: None,
                            required: true,
                            completed: false,
                        });
                        order += 1;
                    }
                }
            }
        }

        // Form application
        let form_selector = Selector::parse("form[id*='apply'], form[class*='apply'], form[action*='apply']").ok();
        if let Some(sel) = form_selector {
            for el in document.select(&sel) {
                let action = el.value().attr("action").unwrap_or("");
                let form_id = el.value().attr("id").unwrap_or("");
                steps.push(ApplicationStep {
                    order,
                    description: "Fill application form".to_string(),
                    action_type: ActionType::Form,
                    target: if !action.is_empty() { action.to_string() } else { format!("#{}", form_id) },
                    deadline: None,
                    required: true,
                    completed: false,
                });
                break;
            }
        }

        if !steps.is_empty() {
            posting.application_steps = sqlx::types::Json(steps);
            confidence.set("application_steps", 0.7);
        }
    }

    /// Extract skills from description text using common patterns
    fn extract_skills_from_text(&self, posting: &mut JobPosting, confidence: &mut ConfidenceMap) {
        if posting.description.is_empty() {
            return;
        }

        let desc_lower = posting.description.to_lowercase();
        let mut skills = Vec::new();

        // Common tech skills patterns
        let skill_patterns = [
            // Programming languages
            "rust", "python", "javascript", "typescript", "java", "go", "golang", "c++", "c#", "php",
            "ruby", "swift", "kotlin", "scala", "clojure", "haskell", "elixir", "erlang", "r", "matlab",
            // Web frameworks
            "react", "vue", "angular", "svelte", "next.js", "nuxt", "django", "flask", "fastapi",
            "spring", "spring boot", "express", "nestjs", "actix", "axum", "rocket", "laravel",
            "rails", "phoenix", "gin", "echo", "fiber",
            // Databases
            "postgresql", "postgres", "mysql", "mongodb", "redis", "elasticsearch", "cassandra",
            "dynamodb", "sqlite", "mariadb", "oracle", "sql server",
            // Cloud & DevOps
            "aws", "azure", "gcp", "google cloud", "docker", "kubernetes", "k8s", "terraform",
            "ansible", "jenkins", "gitlab", "github actions", "ci/cd", "helm", "prometheus",
            "grafana", "datadog", "cloudflare", "vercel", "netlify",
            // Tools
            "git", "linux", "bash", "vim", "vscode", "intellij", "jira", "confluence", "slack",
            // Data & ML
            "tensorflow", "pytorch", "scikit-learn", "pandas", "numpy", "spark", "kafka", "airflow",
            "tableau", "power bi", "looker",
            // Mobile
            "ios", "android", "react native", "flutter", "swiftui", "kotlin multiplatform",
            // Other
            "graphql", "rest", "grpc", "microservices", "serverless", "websocket", "oauth",
            "jwt", "sso", "kubernetes",
        ];

        for skill in skill_patterns {
            // Match whole words only
            let pattern = format!(r"\b{}\b", regex::escape(skill));
            if regex::Regex::new(&pattern).map(|re| re.is_match(&desc_lower)).unwrap_or(false) {
                skills.push(skill.to_string());
            }
        }

        // Also check for "skills:", "requirements:", "technologies:" sections
        let section_markers = ["skills:", "technologies:", "requirements:", "tech stack:", "stack:"];
        for marker in section_markers {
            if let Some(idx) = desc_lower.find(marker) {
                let section = &desc_lower[idx..std::cmp::min(idx + 500, desc_lower.len())];
                // Extract comma-separated items
                for part in section.split([',', ';', '\n', '•', '-']) {
                    let part = part.trim().trim_start_matches(&['•', '-', '*'][..]);
                    if part.len() > 1 && part.len() < 50 {
                        skills.push(part.to_string());
                    }
                }
            }
        }

        // Deduplicate
        skills.sort();
        skills.dedup();

        if !skills.is_empty() {
            posting.skills = sqlx::types::Json(skills);
            confidence.set("skills", 0.6);
        }
    }

    async fn extract_from_rss(&self, raw: &RawContent, source_id: Uuid) -> Result<ExtractionResult> {
        let feed = feed_rs::parser::parse(raw.content.as_bytes())?;
        let mut results = Vec::new();

        for entry in feed.entries {
            let description = entry.content
                .and_then(|c| c.body)
                .or_else(|| entry.summary.map(|s| s.content))
                .unwrap_or_default();

            let mut posting = JobPosting::new(
                source_id,
                entry.id.clone(),
                entry.title.map(|t| t.content).unwrap_or_default(),
                entry.authors.first().map(|a| a.name.clone()).unwrap_or_default(),
                description,
                entry.published.or(entry.updated).unwrap_or(raw.fetched_at),
                raw.content_hash.clone(),
            );

            // Extract link as application URL
            if let Some(link) = entry.links.first() {
                posting.application_url = Some(link.href.clone());
                posting.application_steps = sqlx::types::Json(vec![ApplicationStep {
                    order: 1,
                    description: "View original posting".to_string(),
                    action_type: ActionType::Link,
                    target: link.href.clone(),
                    deadline: None,
                    required: true,
                    completed: false,
                }]);
            }

            let confidence = self.calculate_confidence(&posting);
            let missing = self.identify_missing(&posting);

            results.push(ExtractionResult {
                posting,
                extraction_method: ExtractionMethod::Rules,
                confidence,
                missing_fields: missing,
                warnings: Vec::new(),
                field_confidence: ConfidenceMap::new(),
                metrics: ExtractionMetrics::default(),
            });
        }

        // Return first result for now (pipeline expects single)
        Ok(results.into_iter().next().unwrap_or_else(|| ExtractionResult {
            posting: JobPosting::new(source_id, raw.content_hash.clone(), "RSS Entry".to_string(), "Unknown".to_string(), raw.content.clone(), raw.fetched_at, raw.content_hash.clone()),
            extraction_method: ExtractionMethod::Rules,
            confidence: 0.3,
            missing_fields: vec!["title".to_string(), "company".to_string(), "description".to_string()],
            warnings: vec!["No RSS entries found".to_string()],
            field_confidence: ConfidenceMap::new(),
            metrics: ExtractionMetrics::default(),
        }))
    }

    async fn extract_from_email(&self, raw: &RawContent, source_id: Uuid) -> Result<ExtractionResult> {
        let html_content = raw.metadata.get("html_body").and_then(|v| v.as_str()).unwrap_or(&raw.content);
        let document = Html::parse_document(html_content);

        let mut posting = JobPosting::new(
            source_id,
            raw.metadata.get("message_id").and_then(|v| v.as_str()).unwrap_or(&raw.content_hash).to_string(),
            String::new(),
            String::new(),
            raw.content.clone(),
            raw.fetched_at,
            raw.content_hash.clone(),
        );
        posting.raw_html = Some(raw.content.clone());

        let mut confidence = ConfidenceMap::new();

        // Extract from email HTML
        self.extract_json_ld(&document).map(|ld| self.apply_json_ld(&mut posting, &ld, &mut confidence));
        self.extract_microdata(&document, &mut posting, &mut confidence);
        self.extract_meta_tags(&document, &mut posting, &mut confidence);
        self.extract_fallback(&document, &mut posting, &mut confidence);
        self.extract_application_steps(&document, &mut posting, &mut confidence);
        self.extract_skills_from_text(&mut posting, &mut confidence);

        // Email-specific: subject as title fallback
        if posting.title.is_empty() {
            posting.title = raw.metadata.get("subject").and_then(|v| v.as_str()).unwrap_or("Email Job Alert").to_string();
            confidence.set("title", 0.4);
        }

        // From address as company fallback
        if posting.company.is_empty() {
            posting.company = raw.metadata.get("from").and_then(|v| v.as_str()).unwrap_or("Unknown Sender").to_string();
            confidence.set("company", 0.3);
        }

        let overall_confidence = confidence.average();
        let missing = self.identify_missing(&posting);

        Ok(ExtractionResult {
            posting,
            extraction_method: ExtractionMethod::Rules,
            confidence: overall_confidence,
            missing_fields: missing,
            warnings: Vec::new(),
            field_confidence: confidence,
            metrics: ExtractionMetrics::default(),
        })
    }

    async fn extract_from_json(&self, raw: &RawContent, source_id: Uuid) -> Result<ExtractionResult> {
        let json: Value = serde_json::from_str(&raw.content)?;
        
        let mut posting = JobPosting::new(
            source_id,
            raw.content_hash.clone(),
            "API Entry".to_string(),
            "API".to_string(),
            raw.content.clone(),
            raw.fetched_at,
            raw.content_hash.clone(),
        );

        let mut confidence = ConfidenceMap::new();

        // Try to extract common fields from JSON
        if let Some(title) = json.get("title").or_else(|| json.get("job_title")).and_then(|v| v.as_str()) {
            posting.title = title.to_string();
            confidence.set("title", 0.8);
        }
        if let Some(company) = json.get("company").or_else(|| json.get("company_name")).and_then(|v| v.as_str()) {
            posting.company = company.to_string();
            confidence.set("company", 0.8);
        }
        if let Some(desc) = json.get("description").or_else(|| json.get("job_description")).and_then(|v| v.as_str()) {
            posting.description = desc.to_string();
            confidence.set("description", 0.8);
        }
        if let Some(loc) = json.get("location").or_else(|| json.get("job_location")).and_then(|v| v.as_str()) {
            posting.location = Some(loc.to_string());
            confidence.set("location", 0.8);
        }
        if let Some(url) = json.get("apply_url").or_else(|| json.get("application_url")).and_then(|v| v.as_str()) {
            posting.application_url = Some(url.to_string());
        }
        if let Some(salary) = json.get("salary").or_else(|| json.get("salary_range")).and_then(|v| v.as_str()) {
            posting.salary_range = Some(salary.to_string());
            confidence.set("salary_range", 0.8);
        }
        if let Some(jt) = json.get("job_type").or_else(|| json.get("employment_type")).and_then(|v| v.as_str()) {
            posting.job_type = Some(jt.to_string());
            confidence.set("job_type", 0.8);
        }
        if let Some(remote) = json.get("remote").or_else(|| json.get("remote_policy")).and_then(|v| v.as_str()) {
            posting.remote_policy = Some(remote.to_string());
            confidence.set("remote_policy", 0.8);
        }
        if let Some(skills) = json.get("skills").and_then(|v| v.as_array()) {
            let skill_list: Vec<String> = skills.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            if !skill_list.is_empty() {
                posting.skills = sqlx::types::Json(skill_list);
                confidence.set("skills", 0.8);
            }
        }

        let overall_confidence = confidence.average();
        let missing = self.identify_missing(&posting);

        Ok(ExtractionResult {
            posting,
            extraction_method: ExtractionMethod::Rules,
            confidence: overall_confidence,
            missing_fields: missing,
            warnings: vec!["JSON extraction uses generic field mapping".to_string()],
            field_confidence: confidence,
            metrics: ExtractionMetrics::default(),
        })
    }

    fn calculate_confidence(&self, posting: &JobPosting) -> f32 {
        // Legacy method for backward compatibility
        let mut score = 0.0;
        let mut total = 0.0;

        for (field, weight) in [
            ("title", 0.25), ("company", 0.20), ("description", 0.20),
            ("location", 0.10), ("skills", 0.10), ("application_steps", 0.15),
        ] {
            total += weight;
            if !posting.field_is_empty(field) {
                score += weight;
            }
        }

        score / total
    }

    fn identify_missing(&self, posting: &JobPosting) -> Vec<String> {
        let mut missing = Vec::new();
        if posting.title.is_empty() { missing.push("title".to_string()); }
        if posting.company.is_empty() { missing.push("company".to_string()); }
        if posting.description.is_empty() { missing.push("description".to_string()); }
        if posting.location.is_none() { missing.push("location".to_string()); }
        if posting.skills.0.is_empty() { missing.push("skills".to_string()); }
        if posting.application_steps.0.is_empty() { missing.push("application_steps".to_string()); }
        if posting.salary_range.is_none() { missing.push("salary_range".to_string()); }
        if posting.job_type.is_none() { missing.push("job_type".to_string()); }
        if posting.remote_policy.is_none() { missing.push("remote_policy".to_string()); }
        if posting.experience_required.is_none() { missing.push("experience_required".to_string()); }
        if posting.education_required.is_none() { missing.push("education_required".to_string()); }
        missing
    }
}

impl Default for RuleExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;
    use crate::scrapers::pipeline::{RawContent, ContentType};

    fn create_test_raw(html: &str) -> RawContent {
        RawContent {
            source_url: "https://example.com/job/1".to_string(),
            content: html.to_string(),
            content_type: ContentType::Html,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_extract_json_ld_job_posting() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <head>
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org",
                    "@type": "JobPosting",
                    "title": "Senior Rust Developer",
                    "hiringOrganization": {
                        "@type": "Organization",
                        "name": "RustCorp"
                    },
                    "description": "We are looking for a Rust expert.",
                    "jobLocation": {
                        "@type": "Place",
                        "address": {
                            "@type": "PostalAddress",
                            "addressLocality": "Tunis",
                            "addressCountry": "TN"
                        }
                    },
                    "employmentType": "FULL_TIME",
                    "baseSalary": {
                        "@type": "MonetaryAmount",
                        "currency": "TND",
                        "value": {
                            "@type": "QuantitativeValue",
                            "minValue": "50000",
                            "maxValue": "80000",
                            "unitText": "YEAR"
                        }
                    },
                    "datePosted": "2025-01-15T10:00:00Z",
                    "validThrough": "2025-06-15T23:59:59Z",
                    "jobLocationType": "REMOTE",
                    "skills": ["Rust", "PostgreSQL", "Docker"],
                    "experienceRequirements": "3+ years Rust experience",
                    "educationRequirements": "BS Computer Science"
                }
                </script>
            </head>
            <body></body>
        </html>
        "#;

        let raw = create_test_raw(html);
        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "Senior Rust Developer");
        assert_eq!(result.posting.company, "RustCorp");
        assert_eq!(result.posting.description, "We are looking for a Rust expert.");
        assert_eq!(result.posting.location, Some("Tunis, TN".to_string()));
        assert_eq!(result.posting.job_type, Some("FULL_TIME".to_string()));
        assert!(result.posting.salary_range.as_ref().unwrap().contains("50000"));
        assert!(result.posting.salary_range.as_ref().unwrap().contains("80000"));
        assert!(result.posting.posted_date.is_some());
        assert!(result.posting.deadline.is_some());
        assert_eq!(result.posting.remote_policy, Some("REMOTE".to_string()));
        assert_eq!(result.posting.skills.0, vec!["Rust", "PostgreSQL", "Docker"]);
        assert_eq!(result.posting.experience_required, Some("3+ years Rust experience".to_string()));
        assert_eq!(result.posting.education_required, Some("BS Computer Science".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_extract_microdata() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1 itemprop="title">Software Engineer</h1>
                <div itemprop="hiringOrganization" itemscope itemtype="https://schema.org/Organization">
                    <span itemprop="name">TechCorp</span>
                </div>
                <div itemprop="description">Great job opportunity.</div>
                <div itemprop="jobLocation" itemscope itemtype="https://schema.org/Place">
                    <div itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
                        <span itemprop="addressLocality">San Francisco</span>
                        <span itemprop="addressRegion">CA</span>
                        <span itemprop="addressCountry">US</span>
                    </div>
                </div>
                <span itemprop="employmentType">FULL_TIME</span>
            </body>
        </html>
        "#;

        let raw = create_test_raw(html);
        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "Software Engineer");
        assert_eq!(result.posting.company, "TechCorp");
        assert_eq!(result.posting.description, "Great job opportunity.");
        assert_eq!(result.posting.location, Some("San Francisco, CA, US".to_string()));
        assert_eq!(result.posting.job_type, Some("FULL_TIME".to_string()));
        assert!(result.confidence > 0.7);
    }

    #[tokio::test]
    async fn test_extract_greenhouse_ats() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <script id="job-search-results-data">
                {
                    "job": {
                        "title": "Backend Engineer",
                        "company": { "name": "Greenhouse Inc" },
                        "content": "Build scalable systems.",
                        "location": { "name": "Remote" },
                        "employment_type": "full-time",
                        "remote": true,
                        "requirements": "5 years experience"
                    }
                }
                </script>
            </body>
        </html>
        "#;

        let raw = RawContent {
            source_url: "https://boards.greenhouse.io/jobs/123".to_string(),
            content: html.to_string(),
            content_type: ContentType::Html,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "Backend Engineer");
        assert_eq!(result.posting.company, "Greenhouse Inc");
        assert_eq!(result.posting.description, "Build scalable systems.");
        assert_eq!(result.posting.location, Some("Remote".to_string()));
        assert_eq!(result.posting.job_type, Some("full-time".to_string()));
        assert_eq!(result.posting.remote_policy, Some("remote".to_string()));
    }

    #[tokio::test]
    async fn test_extract_lever_ats() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1 data-qa="job-title">Frontend Developer</h1>
                <div data-qa="company-name">Lever Corp</div>
                <div data-qa="location">New York, NY</div>
                <div data-qa="description">React and TypeScript role.</div>
                <span data-qa="commitment">Full-time</span>
            </body>
        </html>
        "#;

        let raw = RawContent {
            source_url: "https://jobs.lever.co/levercorp/123".to_string(),
            content: html.to_string(),
            content_type: ContentType::Html,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "Frontend Developer");
        assert_eq!(result.posting.company, "Lever Corp");
        assert_eq!(result.posting.location, Some("New York, NY".to_string()));
        assert_eq!(result.posting.description, "React and TypeScript role.");
        assert_eq!(result.posting.job_type, Some("Full-time".to_string()));
    }

    #[tokio::test]
    async fn test_extract_workable_ats() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1 data-ui="job-title">DevOps Engineer</h1>
                <div data-ui="company-name">Workable Ltd</div>
                <div data-ui="location">London, UK</div>
                <div data-ui="job-description">Kubernetes and AWS.</div>
                <span data-ui="employment-type">Contract</span>
                <span data-ui="workplace-type">Hybrid</span>
            </body>
        </html>
        "#;

        let raw = RawContent {
            source_url: "https://apply.workable.com/j/123".to_string(),
            content: html.to_string(),
            content_type: ContentType::Html,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "DevOps Engineer");
        assert_eq!(result.posting.company, "Workable Ltd");
        assert_eq!(result.posting.location, Some("London, UK".to_string()));
        assert_eq!(result.posting.job_type, Some("Contract".to_string()));
        assert_eq!(result.posting.remote_policy, Some("Hybrid".to_string()));
    }

    #[tokio::test]
    async fn test_extract_meta_tags() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <head>
                <meta property="og:title" content="Meta Tag Job">
                <meta property="og:description" content="Found via meta tags">
                <meta property="og:site_name" content="MetaCorp">
                <meta name="job-location" content="Paris, France">
                <meta name="job-salary" content="€60k-€80k">
                <meta name="job-type" content="full-time">
                <meta name="job-remote" content="hybrid">
            </head>
            <body></body>
        </html>
        "#;

        let raw = create_test_raw(html);
        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "Meta Tag Job");
        assert_eq!(result.posting.description, "Found via meta tags");
        assert_eq!(result.posting.company, "MetaCorp");
        assert_eq!(result.posting.location, Some("Paris, France".to_string()));
        assert_eq!(result.posting.salary_range, Some("€60k-€80k".to_string()));
        assert_eq!(result.posting.job_type, Some("full-time".to_string()));
        assert_eq!(result.posting.remote_policy, Some("hybrid".to_string()));
    }

    #[tokio::test]
    async fn test_extract_application_steps() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1>Test Job</h1>
                <a href="/apply/123" class="apply-button">Apply Now</a>
                <a href="mailto:jobs@example.com">Email Us</a>
                <form action="/submit-application" id="apply-form">
                    <input type="text" name="name">
                </form>
            </body>
        </html>
        "#;

        let raw = create_test_raw(html);
        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert!(!result.posting.application_steps.0.is_empty());
        let steps = &result.posting.application_steps.0;
        
        // Should have link, email, and form steps
        assert!(steps.iter().any(|s| s.action_type == ActionType::Link));
        assert!(steps.iter().any(|s| s.action_type == ActionType::Email));
        assert!(steps.iter().any(|s| s.action_type == ActionType::Form));
    }

    #[tokio::test]
    async fn test_extract_skills_from_description() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1>Rust Engineer</h1>
                <div class="company">RustCorp</div>
                <div class="description">
                    We need a Rust expert with PostgreSQL and Docker experience.
                    Skills: Rust, PostgreSQL, Docker, Kubernetes, AWS.
                    Technologies: Git, Linux, CI/CD.
                </div>
            </body>
        </html>
        "#;

        let raw = create_test_raw(html);
        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert!(!result.posting.skills.0.is_empty());
        let skills: Vec<String> = result.posting.skills.0.iter().map(|s| s.to_lowercase()).collect();
        assert!(skills.iter().any(|s| s.contains("rust")));
        assert!(skills.iter().any(|s| s.contains("postgresql")));
        assert!(skills.iter().any(|s| s.contains("docker")));
    }

    #[tokio::test]
    async fn test_rss_extraction() {
        let extractor = RuleExtractor::new();
        let rss = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
            <channel>
                <title>Job Feed</title>
                <item>
                    <title>RSS Job</title>
                    <link>https://example.com/job/1</link>
                    <description>RSS description</description>
                    <pubDate>Mon, 15 Jan 2025 10:00:00 GMT</pubDate>
                    <guid>job-1</guid>
                </item>
            </channel>
        </rss>
        "#;

        let raw = RawContent {
            source_url: "https://example.com/feed.xml".to_string(),
            content: rss.to_string(),
            content_type: ContentType::Rss,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "RSS Job");
        assert_eq!(result.posting.description, "RSS description");
        assert_eq!(result.posting.application_url, Some("https://example.com/job/1".to_string()));
        assert!(!result.posting.application_steps.0.is_empty());
    }

    #[tokio::test]
    async fn test_confidence_map() {
        let mut map = ConfidenceMap::new();
        map.set("title", 0.9);
        map.set("company", 0.8);
        map.set("description", 0.7);

        assert_eq!(map.get("title"), 0.9);
        assert_eq!(map.get("company"), 0.8);
        assert_eq!(map.get("description"), 0.7);
        assert_eq!(map.get("missing"), 0.0);

        let avg = map.average();
        assert!((avg - 0.8).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_smartrecruiters_extraction() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1 data-qa="job-title">SmartRecruiters Job</h1>
                <div data-qa="company-name">SmartRecruiters Inc</div>
                <div data-qa="location">Berlin, Germany</div>
                <div data-qa="job-description">Description here.</div>
                <span data-qa="job-type">Full-time</span>
                <span data-qa="remote-type">Remote</span>
            </body>
        </html>
        "#;

        let raw = RawContent {
            source_url: "https://careers.smartrecruiters.com/company/123".to_string(),
            content: html.to_string(),
            content_type: ContentType::Html,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "SmartRecruiters Job");
        assert_eq!(result.posting.company, "SmartRecruiters Inc");
        assert_eq!(result.posting.location, Some("Berlin, Germany".to_string()));
        assert_eq!(result.posting.job_type, Some("Full-time".to_string()));
        assert_eq!(result.posting.remote_policy, Some("Remote".to_string()));
    }

    #[tokio::test]
    async fn test_bamboohr_extraction() {
        let extractor = RuleExtractor::new();
        let html = r#"
        <html>
            <body>
                <h1 class="job-title">BambooHR Position</h1>
                <div class="company-name">BambooHR Corp</div>
                <div class="job-location">Remote</div>
                <div class="job-description">Job description content.</div>
                <span class="job-type">Full-time</span>
            </body>
        </html>
        "#;

        let raw = RawContent {
            source_url: "https://company.bamboohr.com/jobs/123".to_string(),
            content: html.to_string(),
            content_type: ContentType::Html,
            fetched_at: Utc::now(),
            content_hash: "test_hash".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

        assert_eq!(result.posting.title, "BambooHR Position");
        assert_eq!(result.posting.company, "BambooHR Corp");
        assert_eq!(result.posting.location, Some("Remote".to_string()));
        assert_eq!(result.posting.job_type, Some("Full-time".to_string()));
    }
}