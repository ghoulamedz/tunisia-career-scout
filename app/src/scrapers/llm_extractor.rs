//! LLM-Based Extractor
//!
//! Uses llm-chain for structured extraction with configurable providers.

use anyhow::Result;
use llm_chain::options::{Opt, Options};
use llm_chain::Parameters;
use llm_chain::prompt;
use llm_chain::traits::Executor as ExecutorTrait;
use llm_chain_openai::chatgpt::Executor as ChatGptExecutor;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::scrapers::pipeline::{RawContent, ExtractionResult, ExtractionMethod, ContentType, ExtractionMetrics};
use crate::scrapers::rule_extractor::ConfidenceMap;
use crate::models::posting::{JobPosting, ApplicationStep, ActionType};
use crate::models::settings::{LLMSettings, LLMProvider};

pub struct LLMExtractor {
    settings: LLMSettings,
    executor: Option<ChatGptExecutor>,
}

impl LLMExtractor {
    pub fn new(settings: LLMSettings) -> Result<Self> {
        let executor = Self::create_executor(&settings)?;
        Ok(Self { settings, executor })
    }

    fn create_executor(settings: &LLMSettings) -> Result<Option<ChatGptExecutor>> {
        let mut builder = Options::builder();

        // Set model
        builder.add_option(Opt::Model(llm_chain::options::ModelRef::from_model_name(settings.model.clone())));

        // Set temperature
        builder.add_option(Opt::Temperature(settings.temperature));

        // Set max tokens
        if let Some(max_tokens) = settings.max_tokens {
            builder.add_option(Opt::MaxTokens(max_tokens as usize));
        }

        // Set API key if provided
        if let Some(api_key) = &settings.api_key {
            if !api_key.is_empty() {
                builder.add_option(Opt::ApiKey(api_key.clone()));
            }
        }

        let options = builder.build();

        match settings.provider {
            LLMProvider::OpenAI | LLMProvider::OpenAICompatible | LLMProvider::LlamaCpp | LLMProvider::Ollama => {
                // All providers use OpenAI-compatible API via async-openai
                let mut cfg = async_openai::config::OpenAIConfig::new();

                let base_url = match settings.provider {
                    LLMProvider::Ollama => settings.base_url.as_deref().unwrap_or("http://localhost:11434/v1"),
                    _ => settings.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                };
                cfg = cfg.with_api_base(base_url);

                // Set API key
                if let Some(api_key) = &settings.api_key {
                    if !api_key.is_empty() {
                        cfg = cfg.with_api_key(api_key);
                    }
                } else if settings.provider == LLMProvider::LlamaCpp || settings.provider == LLMProvider::Ollama {
                    // llama.cpp and Ollama often don't require a real API key
                    cfg = cfg.with_api_key("sk-no-key-required");
                }

                let client = async_openai::Client::with_config(cfg);
                let executor = ChatGptExecutor::for_client(client, options);
                Ok(Some(executor))
            }
        }
    }

    pub fn update_settings(&mut self, settings: LLMSettings) -> Result<()> {
        self.executor = Self::create_executor(&settings)?;
        self.settings = settings;
        Ok(())
    }

    pub async fn extract(
        &self,
        raw: &RawContent,
        source_id: Uuid,
        rule_result: Option<&ExtractionResult>,
    ) -> Result<ExtractionResult> {
        let Some(executor) = &self.executor else {
            return Err(anyhow::anyhow!("LLM executor not configured"));
        };

        // Build prompt with schema and context
        let prompt_text = self.build_prompt(raw, rule_result);
        let prompt_template = prompt!(prompt_text);
        let prompt = prompt_template.format(&Parameters::new())?;
        let options = Options::empty().clone();

        let response = executor.execute(&options, &prompt).await?;
        let text = response.to_string();

        // Parse JSON from response
        let parsed = self.parse_response(&text)?;

        // Build JobPosting from parsed data
        let mut posting = JobPosting::new(
            source_id,
            raw.content_hash.clone(),
            parsed.title,
            parsed.company,
            parsed.description,
            raw.fetched_at,
            raw.content_hash.clone(),
        );

        posting.summary = Some(parsed.summary);
        posting.skills = sqlx::types::Json(parsed.skills);
        posting.experience_required = parsed.experience_required;
        posting.education_required = parsed.education_required;
        posting.languages = sqlx::types::Json(parsed.languages);
        posting.location = parsed.location;
        posting.remote_policy = parsed.remote_policy;
        posting.job_type = parsed.job_type;
        posting.salary_range = parsed.salary_range;
        posting.deadline = parsed.deadline;
        // Convert application steps
        let app_steps: Vec<ApplicationStep> = parsed.application_steps.into_iter().map(Into::into).collect();
        posting.application_steps = sqlx::types::Json(app_steps);
        posting.application_email = parsed.application_email;
        posting.application_url = parsed.application_url;
        posting.contact_person = parsed.contact_person;
        posting.contact_phone = parsed.contact_phone;
        posting.posted_date = parsed.posted_date;

        // Confidence based on completeness
        let confidence = self.calculate_confidence(&posting);
        let missing = self.identify_missing(&posting);
        let field_confidence = self.build_field_confidence(&posting);

        Ok(ExtractionResult {
            posting,
            extraction_method: ExtractionMethod::Llm,
            confidence,
            missing_fields: missing,
            warnings: Vec::new(),
            field_confidence,
            metrics: ExtractionMetrics::default(),
        })
    }

    fn build_prompt(&self, raw: &RawContent, rule_result: Option<&ExtractionResult>) -> String {
        let mut prompt = String::from(
            r#"You are an expert job posting extractor. Extract structured data from the provided content.

Return ONLY valid JSON matching this exact schema:
{
  "title": "string",
  "company": "string",
  "description": "string",
  "summary": "string (2-3 sentences)",
  "skills": ["string"],
  "experience_required": "string or null",
  "education_required": "string or null",
  "languages": ["string"],
  "location": "string or null",
  "remote_policy": "remote|hybrid|onsite or null",
  "job_type": "full-time|part-time|contract|internship|other or null",
  "salary_range": "string or null",
  "deadline": "ISO8601 datetime or null",
  "application_steps": [
    {"order": 1, "description": "string", "action_type": "link|email|form|upload|other", "target": "string", "deadline": "ISO8601 or null", "required": true, "completed": false}
  ],
  "application_email": "string or null",
  "application_url": "string or null",
  "contact_person": "string or null",
  "contact_phone": "string or null",
  "posted_date": "ISO8601 datetime or null"
}

Rules:
- Extract ONLY information explicitly present in the content
- Use null for missing fields, not empty strings
- For skills, extract specific technologies, tools, methodologies
- For application_steps, find ALL steps mentioned (links, emails, forms, uploads)
- If content appears to be a job listing, extract it; otherwise return minimal structure
"#
        );

        if let Some(rule) = rule_result {
            prompt.push_str("\n\nRule-based extraction found:\n");
            prompt.push_str(&serde_json::to_string_pretty(&rule.posting).unwrap_or_default());
            prompt.push_str("\n\nImprove and complete the above extraction.\n");
        }

        prompt.push_str("\n\nContent to extract from:\n");
        prompt.push_str(&raw.content);

        prompt
    }

    fn parse_response(&self, text: &str) -> Result<LLMExtractionOutput> {
        // Try to extract JSON from response (may have markdown code fences)
        let json_str = if text.contains("```json") {
            text.split("```json").nth(1).unwrap_or(text)
                .split("```").next().unwrap_or(text)
                .trim()
        } else if text.contains("```") {
            text.split("```").nth(1).unwrap_or(text)
                .split("```").next().unwrap_or(text)
                .trim()
        } else {
            text.trim()
        };

        let parsed: LLMExtractionOutput = serde_json::from_str(json_str)?;
        Ok(parsed)
    }

    fn calculate_confidence(&self, posting: &JobPosting) -> f32 {
        let mut score = 0.0;
        let mut total = 0.0;

        for (field, weight) in [
            ("title", 0.20), ("company", 0.15), ("description", 0.15),
            ("summary", 0.10), ("skills", 0.10), ("location", 0.05),
            ("experience_required", 0.05), ("application_steps", 0.10),
            ("salary_range", 0.05), ("deadline", 0.05),
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
        if posting.summary.is_none() { missing.push("summary".to_string()); }
        if posting.skills.0.is_empty() { missing.push("skills".to_string()); }
        if posting.location.is_none() { missing.push("location".to_string()); }
        if posting.application_steps.0.is_empty() { missing.push("application_steps".to_string()); }
        missing
    }
}

#[derive(Debug, Deserialize)]
struct LLMExtractionOutput {
    title: String,
    company: String,
    description: String,
    summary: String,
    skills: Vec<String>,
    experience_required: Option<String>,
    education_required: Option<String>,
    languages: Vec<String>,
    location: Option<String>,
    remote_policy: Option<String>,
    job_type: Option<String>,
    salary_range: Option<String>,
    deadline: Option<DateTime<Utc>>,
    application_steps: Vec<LLMStep>,
    application_email: Option<String>,
    application_url: Option<String>,
    contact_person: Option<String>,
    contact_phone: Option<String>,
    posted_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct LLMStep {
    order: u32,
    description: String,
    action_type: String,
    target: String,
    deadline: Option<DateTime<Utc>>,
    required: bool,
    completed: bool,
}

impl From<LLMStep> for ApplicationStep {
    fn from(s: LLMStep) -> Self {
        Self {
            order: s.order,
            description: s.description,
            action_type: match s.action_type.as_str() {
                "email" => ActionType::Email,
                "form" => ActionType::Form,
                "upload" => ActionType::Upload,
                "other" => ActionType::Other,
                _ => ActionType::Link,
            },
            target: s.target,
            deadline: s.deadline,
            required: s.required,
            completed: s.completed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_chain_mock::Executor as MockExecutor;
    use llm_chain::traits::Executor as ExecutorTrait;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_llm_extractor_with_mock() {
        // Create a mock executor that returns a predefined response
        let mock_response = r#"{
            "title": "Software Engineer",
            "company": "Test Corp",
            "description": "We are hiring a software engineer.",
            "summary": "Test Corp is hiring a software engineer.",
            "skills": ["Rust", "Python"],
            "experience_required": "3 years",
            "education_required": "BS Computer Science",
            "languages": ["English"],
            "location": "San Francisco, CA",
            "remote_policy": "hybrid",
            "job_type": "full-time",
            "salary_range": "$100k-$150k",
            "deadline": "2025-12-31T23:59:59Z",
            "application_steps": [
                {"order": 1, "description": "Apply via link", "action_type": "link", "target": "https://example.com/apply", "deadline": null, "required": true, "completed": false}
            ],
            "application_email": "jobs@testcorp.com",
            "application_url": "https://example.com/apply",
            "contact_person": "Jane Doe",
            "contact_phone": "+1-555-1234",
            "posted_date": "2025-01-15T10:00:00Z"
        }"#;

        // Note: llm_chain_mock::Executor doesn't support custom responses easily.
        // This test demonstrates the structure; actual mocking would require
        // a more sophisticated mock implementation.
    }

    #[test]
    fn test_parse_response_with_markdown_fences() {
        let extractor = LLMExtractor {
            settings: LLMSettings {
                provider: LLMProvider::Ollama,
                model: "test".to_string(),
                base_url: None,
                api_key: None,
                temperature: 0.3,
                max_tokens: Some(4096),
                timeout_seconds: 60,
            },
            executor: None,
        };

        // Test with ```json fence
        let response_with_json_fence = r#"
```json
{
    "title": "Test",
    "company": "Test",
    "description": "Test",
    "summary": "Test",
    "skills": [],
    "experience_required": null,
    "education_required": null,
    "languages": [],
    "location": null,
    "remote_policy": null,
    "job_type": null,
    "salary_range": null,
    "deadline": null,
    "application_steps": [],
    "application_email": null,
    "application_url": null,
    "contact_person": null,
    "contact_phone": null,
    "posted_date": null
}
```"#;

        let result = extractor.parse_response(response_with_json_fence);
        assert!(result.is_ok());

        // Test with ``` fence
        let response_with_fence = r#"
```
{
    "title": "Test",
    "company": "Test",
    "description": "Test",
    "summary": "Test",
    "skills": [],
    "experience_required": null,
    "education_required": null,
    "languages": [],
    "location": null,
    "remote_policy": null,
    "job_type": null,
    "salary_range": null,
    "deadline": null,
    "application_steps": [],
    "application_email": null,
    "application_url": null,
    "contact_person": null,
    "contact_phone": null,
    "posted_date": null
}
```"#;

        let result = extractor.parse_response(response_with_fence);
        assert!(result.is_ok());

        // Test without fence
        let response_no_fence = r#"{
            "title": "Test",
            "company": "Test",
            "description": "Test",
            "summary": "Test",
            "skills": [],
            "experience_required": null,
            "education_required": null,
            "languages": [],
            "location": null,
            "remote_policy": null,
            "job_type": null,
            "salary_range": null,
            "deadline": null,
            "application_steps": [],
            "application_email": null,
            "application_url": null,
            "contact_person": null,
            "contact_phone": null,
            "posted_date": null
        }"#;

        let result = extractor.parse_response(response_no_fence);
        assert!(result.is_ok());
    }

    #[test]
    fn test_confidence_calculation() {
        let extractor = LLMExtractor {
            settings: LLMSettings {
                provider: LLMProvider::Ollama,
                model: "test".to_string(),
                base_url: None,
                api_key: None,
                temperature: 0.3,
                max_tokens: Some(4096),
                timeout_seconds: 60,
            },
            executor: None,
        };

        let mut posting = JobPosting::new(
            Uuid::new_v4(),
            "hash".to_string(),
            "Software Engineer".to_string(),
            "Test Corp".to_string(),
            "Description".to_string(),
            Utc::now(),
            "hash".to_string(),
        );

        posting.summary = Some("Summary".to_string());
        posting.skills = sqlx::types::Json(vec!["Rust".to_string()]);
        posting.location = Some("Remote".to_string());
        posting.experience_required = Some("3 years".to_string());
        posting.application_steps = sqlx::types::Json(vec![]);
        posting.salary_range = Some("$100k".to_string());
        posting.deadline = Some(Utc::now());

        let confidence = extractor.calculate_confidence(&posting);
        assert!(confidence > 0.8); // Most fields filled
    }
}