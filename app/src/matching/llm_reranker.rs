//! LLM Reranker
//!
//! Implements LLM-based reranking on top of TF-IDF cosine similarity.
//!
//! Pipeline:
//! 1. TF-IDF cosine similarity → get top N (default 50)
//! 2. LLM rerank with structured prompt → score 0-100 with reasoning
//! 3. Combine scores (weighted 0.4 TF-IDF / 0.6 LLM)
//! 4. Return top K (default 10)
//!
//! Caching: LLM responses cached for 24h by (cv_hash + job_ids_hash)

use anyhow::Result;
use llm_chain::options::{Opt, Options};
use llm_chain::Parameters;
use llm_chain::prompt;
use llm_chain_openai::chatgpt::Executor as ChatGptExecutor;
use async_openai::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use crate::models::cv::CVData;
use crate::models::posting::JobPosting;
use crate::models::match_result::{MatchResult, MatchReason, MatchCategory};
use crate::models::settings::{LLMSettings, LLMProvider};
use crate::matching::tfidf::MatchingEngine;

/// Configuration for LLM reranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerConfig {
    pub enabled: bool,
    pub provider: LLMProvider,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub timeout_seconds: u32,
    pub top_k_before_rerank: usize,  // Default 50
    pub final_top_k: usize,           // Default 10
    pub tfidf_weight: f32,            // Default 0.4
    pub llm_weight: f32,              // Default 0.6
    pub cache_ttl_hours: u32,         // Default 24
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: LLMProvider::Ollama,
            model: "llama3.2".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            api_key: None,
            temperature: 0.1,
            max_tokens: Some(2048),
            timeout_seconds: 60,
            top_k_before_rerank: 50,
            final_top_k: 10,
            tfidf_weight: 0.4,
            llm_weight: 0.6,
            cache_ttl_hours: 24,
        }
    }
}

impl From<&LLMSettings> for RerankerConfig {
    fn from(settings: &LLMSettings) -> Self {
        Self {
            enabled: true,
            provider: settings.provider.clone(),
            model: settings.model.clone(),
            base_url: settings.base_url.clone(),
            api_key: settings.api_key.clone(),
            temperature: settings.temperature,
            max_tokens: settings.max_tokens,
            timeout_seconds: settings.timeout_seconds,
            top_k_before_rerank: 50,
            final_top_k: 10,
            tfidf_weight: 0.4,
            llm_weight: 0.6,
            cache_ttl_hours: 24,
        }
    }
}

/// Cached LLM rerank response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedRerankResponse {
    cv_hash: String,
    job_ids_hash: String,
    responses: Vec<LLMRerankOutput>,
    cached_at: DateTime<Utc>,
}

/// LLM rerank output for a single job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LLMRerankOutput {
    job_id: Uuid,
    score: f32,           // 0-100
    reasoning: String,
}

/// LLM Reranker with caching and provider abstraction
pub struct LLMReranker {
    config: RerankerConfig,
    executor: Option<ChatGptExecutor>,
    cache: Arc<RwLock<HashMap<String, CachedRerankResponse>>>,
}

impl LLMReranker {
    /// Create a new LLM reranker with the given configuration
    pub fn new(config: RerankerConfig) -> Result<Self> {
        let executor = if config.enabled {
            Some(Self::create_executor(&config)?)
        } else {
            None
        };

        Ok(Self {
            config,
            executor,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create the LLM executor based on provider config
    fn create_executor(config: &RerankerConfig) -> Result<ChatGptExecutor> {
        let mut builder = Options::builder();

        builder.add_option(Opt::Model(llm_chain::options::ModelRef::from_model_name(config.model.clone())));
        builder.add_option(Opt::Temperature(config.temperature));

        if let Some(max_tokens) = config.max_tokens {
            builder.add_option(Opt::MaxTokens(max_tokens as usize));
        }

        let options = builder.build();

        let mut cfg = async_openai::config::OpenAIConfig::new();

        let base_url = match config.provider {
            LLMProvider::Ollama => config.base_url.as_deref().unwrap_or("http://localhost:11434/v1"),
            _ => config.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
        };
        cfg = cfg.with_api_base(base_url);

        if let Some(api_key) = &config.api_key {
            if !api_key.is_empty() {
                cfg = cfg.with_api_key(api_key);
            }
        } else if config.provider == LLMProvider::LlamaCpp || config.provider == LLMProvider::Ollama {
            cfg = cfg.with_api_key("sk-no-key-required");
        }

        let client = Client::with_config(cfg);
        let executor = ChatGptExecutor::for_client(client, options);
        Ok(executor)
    }

    /// Update configuration and recreate executor if needed
    pub fn update_config(&mut self, config: RerankerConfig) -> Result<()> {
        self.executor = if config.enabled {
            Some(Self::create_executor(&config)?)
        } else {
            None
        };
        self.config = config;
        Ok(())
    }

    /// Main rerank function implementing the full pipeline
    pub async fn rerank(
        &self,
        cv: &CVData,
        jobs: Vec<JobPosting>,
        top_k: Option<usize>,
    ) -> Result<Vec<MatchResult>> {
        if jobs.is_empty() {
            return Ok(Vec::new());
        }

        let final_top_k = top_k.unwrap_or(self.config.final_top_k);
        let top_k_before = self.config.top_k_before_rerank.min(jobs.len());

        // Step 1: TF-IDF matching to get initial scores
        let engine = MatchingEngine::new(Uuid::new_v4(), cv);
        let mut tfidf_results: Vec<MatchResult> = jobs.iter()
            .map(|job| engine.match_posting(job))
            .collect();

        // Sort by TF-IDF score descending
        tfidf_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N for LLM reranking
        let top_candidates = tfidf_results.into_iter().take(top_k_before).collect::<Vec<_>>();
        let candidate_postings: Vec<JobPosting> = top_candidates.iter()
            .filter_map(|m| jobs.iter().find(|j| j.id == m.posting_id).cloned())
            .collect();

        if candidate_postings.is_empty() {
            return Ok(Vec::new());
        }

        // Step 2: LLM reranking (with caching)
        let llm_scores = self.rerank_with_llm(cv, &candidate_postings).await?;

        // Step 3: Combine scores
        let mut combined_results = Vec::new();
        for (tfidf_result, posting) in top_candidates.into_iter().zip(candidate_postings.iter()) {
            if let Some(llm_output) = llm_scores.iter().find(|o| o.job_id == posting.id) {
                let tfidf_norm = tfidf_result.tfidf_score.unwrap_or(0.0); // 0-1
                let llm_norm = llm_output.score / 100.0; // 0-1

                let combined = self.config.tfidf_weight * tfidf_norm + self.config.llm_weight * llm_norm;
                let final_score = (combined * 100.0).round().clamp(0.0, 100.0);

                let mut result = tfidf_result;
                result.score = final_score;
                result.llm_score = Some(llm_output.score);

                // Add LLM reasoning as a match reason
                result.reasons.0.push(MatchReason {
                    category: MatchCategory::Other,
                    description: format!("LLM reasoning: {}", llm_output.reasoning),
                    weight: self.config.llm_weight,
                    matched: true,
                });

                combined_results.push(result);
            } else {
                // No LLM score available, keep TF-IDF only
                combined_results.push(tfidf_result);
            }
        }

        // Sort by combined score descending
        combined_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top K
        combined_results.truncate(final_top_k);
        Ok(combined_results)
    }

    /// Perform LLM reranking with caching
    async fn rerank_with_llm(&self, cv: &CVData, postings: &[JobPosting]) -> Result<Vec<LLMRerankOutput>> {
        let Some(executor) = &self.executor else {
            return Ok(Vec::new());
        };

        // Generate cache key
        let cv_hash = self.compute_cv_hash(cv);
        let job_ids_hash = self.compute_job_ids_hash(postings);
        let cache_key = format!("{}:{}", cv_hash, job_ids_hash);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                let age = Utc::now() - cached.cached_at;
                if age < Duration::hours(self.config.cache_ttl_hours as i64) {
                    return Ok(cached.responses.clone());
                }
            }
        }

        // Build prompt
        let prompt_text = self.build_rerank_prompt(cv, postings);
        let prompt_template = prompt!(prompt_text);
        let prompt = prompt_template.format(&Parameters::new())?;
        let options = Options::empty().clone();

        // Execute LLM
        let response = executor.execute(&options, &prompt).await?;
        let text = response.to_string();

        // Parse JSON response
        let outputs = self.parse_rerank_response(&text, postings)?;

        // Cache the response
        let cached = CachedRerankResponse {
            cv_hash,
            job_ids_hash,
            responses: outputs.clone(),
            cached_at: Utc::now(),
        };
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, cached);
        }

        Ok(outputs)
    }

    /// Build the reranking prompt
    fn build_rerank_prompt(&self, cv: &CVData, postings: &[JobPosting]) -> String {
        let cv_summary = self.summarize_cv(cv);
        
        let mut postings_json = String::new();
        postings_json.push_str("[\n");
        for (i, posting) in postings.iter().enumerate() {
            let skills = posting.skills.0.join(", ");
            let location = posting.location.as_deref().unwrap_or("Not specified");
            let remote = posting.remote_policy.as_deref().unwrap_or("Not specified");
            let salary = posting.salary_range.as_deref().unwrap_or("Not specified");
            let experience = posting.experience_required.as_deref().unwrap_or("Not specified");
            let education = posting.education_required.as_deref().unwrap_or("Not specified");

            postings_json.push_str(&format!(r#"  {{
    "job_id": "{}",
    "title": "{}",
    "company": "{}",
    "skills": "{}",
    "location": "{}",
    "remote_policy": "{}",
    "salary_range": "{}",
    "experience_required": "{}",
    "education_required": "{}",
    "description": "{}"
  }}"#, 
                posting.id,
                posting.title.replace('"', "\\\""),
                posting.company.replace('"', "\\\""),
                skills.replace('"', "\\\""),
                location.replace('"', "\\\""),
                remote.replace('"', "\\\""),
                salary.replace('"', "\\\""),
                experience.replace('"', "\\\""),
                education.replace('"', "\\\""),
                posting.description.chars().take(500).collect::<String>().replace('"', "\\\"")
            ));

            if i < postings.len() - 1 {
                postings_json.push_str(",");
            }
            postings_json.push_str("\n");
        }
        postings_json.push_str("]");

        format!(r#"You are an expert job matching evaluator. Score how well the CV matches each job posting.

CV SUMMARY:
{}

JOB POSTINGS:
{}

For EACH job, score 0-100 how well this CV matches. Consider:
- Skills overlap (required vs. possessed)
- Experience level match
- Location compatibility (remote policy, geographic)
- Salary expectations alignment
- Education requirements
- Overall role fit

Output ONLY valid JSON array:
[
  {{"job_id": "uuid", "score": 85, "reasoning": "Strong match: 8/10 required skills, remote-friendly, experience aligned"}},
  {{"job_id": "uuid", "score": 45, "reasoning": "Partial match: missing key skills (Kubernetes, AWS), onsite only"}}
]

No extra text, no markdown, just the JSON array."#, cv_summary, postings_json)
    }

    /// Summarize CV for prompt
    fn summarize_cv(&self, cv: &CVData) -> String {
        let mut parts = Vec::new();
        parts.push(format!("Name: {}", cv.full_name));
        if let Some(loc) = &cv.location {
            parts.push(format!("Location: {}", loc));
        }
        if let Some(summary) = &cv.summary {
            parts.push(format!("Summary: {}", summary));
        }

        let skills: Vec<String> = cv.skills.iter().map(|s| s.name.clone()).collect();
        parts.push(format!("Skills: {}", skills.join(", ")));

        let exp_summary: Vec<String> = cv.experience.iter()
            .map(|e| format!("{} at {} ({})", e.title, e.company, e.description.chars().take(100).collect::<String>()))
            .collect();
        parts.push(format!("Experience: {}", exp_summary.join("; ")));

        let edu_summary: Vec<String> = cv.education.iter()
            .map(|e| format!("{} in {} from {}", e.degree, e.field_of_study.as_deref().unwrap_or(""), e.institution))
            .collect();
        parts.push(format!("Education: {}", edu_summary.join("; ")));

        let langs: Vec<String> = cv.languages.iter().map(|l| format!("{} ({:?})", l.name, l.proficiency)).collect();
        parts.push(format!("Languages: {}", langs.join(", ")));

        parts.join("\n")
    }

    /// Parse LLM response into structured outputs
    fn parse_rerank_response(&self, text: &str, postings: &[JobPosting]) -> Result<Vec<LLMRerankOutput>> {
        // Extract JSON from response (handle markdown fences)
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

        let parsed: Value = serde_json::from_str(json_str)?;
        let arr = parsed.as_array().ok_or_else(|| anyhow::anyhow!("Expected JSON array"))?;

        let mut outputs = Vec::new();
        let posting_ids: std::collections::HashSet<Uuid> = postings.iter().map(|p| p.id).collect();

        for item in arr {
            let job_id_str = item.get("job_id").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("Missing job_id"))?;
            let job_id = Uuid::parse_str(job_id_str)?;
            
            if !posting_ids.contains(&job_id) {
                continue; // Skip unknown job IDs
            }

            let score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let reasoning = item.get("reasoning").and_then(|v| v.as_str()).unwrap_or("").to_string();

            outputs.push(LLMRerankOutput {
                job_id,
                score: score.clamp(0.0, 100.0),
                reasoning,
            });
        }

        Ok(outputs)
    }

    /// Compute hash of CV for caching
    fn compute_cv_hash(&self, cv: &CVData) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        cv.full_name.hash(&mut hasher);
        cv.location.hash(&mut hasher);
        cv.summary.hash(&mut hasher);
        for skill in &cv.skills {
            skill.name.hash(&mut hasher);
        }
        for exp in &cv.experience {
            exp.title.hash(&mut hasher);
            exp.company.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Compute hash of job IDs for caching
    fn compute_job_ids_hash(&self, postings: &[JobPosting]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut ids: Vec<Uuid> = postings.iter().map(|p| p.id).collect();
        ids.sort();
        
        let mut hasher = DefaultHasher::new();
        for id in ids {
            id.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache stats
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), cache.capacity())
    }
}

/// High-level reranker trait for dependency injection / testing
#[async_trait::async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(&self, cv: &CVData, jobs: Vec<JobPosting>, top_k: Option<usize>) -> Result<Vec<MatchResult>>;
}

#[async_trait::async_trait]
impl Reranker for LLMReranker {
    async fn rerank(&self, cv: &CVData, jobs: Vec<JobPosting>, top_k: Option<usize>) -> Result<Vec<MatchResult>> {
        self.rerank(cv, jobs, top_k).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cv::{CVData, Experience, Education, Skill, Language, Project, Certification, SkillLevel, LanguageProficiency, CVTemplate, CVTheme};
    use crate::models::posting::{JobPosting, PostingStatus, ApplicationStep, ActionType};
    use chrono::{DateTime, Utc, TimeZone};
    use uuid::Uuid;

    fn sample_cv() -> CVData {
        CVData {
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            phone: Some("+1-555-0123".to_string()),
            location: Some("San Francisco, CA".to_string()),
            linkedin: Some("linkedin.com/in/johndoe".to_string()),
            github: Some("github.com/johndoe".to_string()),
            portfolio: None,
            summary: Some("Senior software engineer with 8 years experience in Rust and Python.".to_string()),
            experience: vec![
                Experience {
                    id: Uuid::new_v4(),
                    title: "Senior Software Engineer".to_string(),
                    company: "TechCorp".to_string(),
                    location: Some("San Francisco, CA".to_string()),
                    start_date: Utc.with_ymd_and_hms(2020, 1, 15, 0, 0, 0).unwrap(),
                    end_date: None,
                    current: true,
                    description: "Building distributed systems in Rust and Python. Led team of 5.".to_string(),
                    skills: vec!["Rust".to_string(), "Python".to_string(), "Kubernetes".to_string(), "AWS".to_string()],
                },
            ],
            education: vec![
                Education {
                    id: Uuid::new_v4(),
                    institution: "Stanford University".to_string(),
                    degree: "Master of Science".to_string(),
                    field_of_study: Some("Computer Science".to_string()),
                    location: Some("Stanford, CA".to_string()),
                    start_date: Utc.with_ymd_and_hms(2015, 9, 1, 0, 0, 0).unwrap(),
                    end_date: Some(Utc.with_ymd_and_hms(2017, 6, 15, 0, 0, 0).unwrap()),
                    current: false,
                    description: None,
                    grade: Some("3.9/4.0".to_string()),
                },
            ],
            skills: vec![
                Skill { id: Uuid::new_v4(), name: "Rust".to_string(), category: Some("technical".to_string()), level: Some(SkillLevel::Expert), years: Some(5) },
                Skill { id: Uuid::new_v4(), name: "Python".to_string(), category: Some("technical".to_string()), level: Some(SkillLevel::Advanced), years: Some(8) },
                Skill { id: Uuid::new_v4(), name: "Kubernetes".to_string(), category: Some("technical".to_string()), level: Some(SkillLevel::Advanced), years: Some(3) },
                Skill { id: Uuid::new_v4(), name: "AWS".to_string(), category: Some("technical".to_string()), level: Some(SkillLevel::Advanced), years: Some(4) },
            ],
            languages: vec![
                Language { id: Uuid::new_v4(), name: "English".to_string(), proficiency: LanguageProficiency::Native },
            ],
            projects: vec![],
            certifications: vec![],
            template: CVTemplate::Modern,
            theme: CVTheme::Auto,
        }
    }

    fn sample_posting(id: Uuid, title: &str, skills: Vec<&str>) -> JobPosting {
        let now = Utc::now();
        JobPosting {
            id,
            source_id: Uuid::new_v4(),
            source_posting_id: format!("test-{}", id),
            title: title.to_string(),
            company: "TestCorp".to_string(),
            description: format!("We are looking for a {} with experience in {}.", title, skills.join(", ")),
            summary: Some(format!("{} role", title)),
            skills: skills.into_iter().map(|s| s.to_string()).collect::<Vec<_>>().into(),
            experience_required: Some("5+ years".to_string()),
            education_required: Some("MS Computer Science".to_string()),
            languages: vec!["English".to_string()].into(),
            location: Some("San Francisco, CA".to_string()),
            remote_policy: Some("Hybrid".to_string()),
            job_type: Some("Full-time".to_string()),
            salary_range: Some("$150k-$200k".to_string()),
            deadline: None,
            application_steps: vec![].into(),
            application_email: None,
            application_url: Some("https://testcorp.com/jobs/123".to_string()),
            contact_person: None,
            contact_phone: None,
            posted_date: Some(now),
            scraped_at: now,
            content_hash: "abc123".to_string(),
            raw_html: None,
            status: PostingStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_reranker_config_default() {
        let config = RerankerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.top_k_before_rerank, 50);
        assert_eq!(config.final_top_k, 10);
        assert_eq!(config.tfidf_weight, 0.4);
        assert_eq!(config.llm_weight, 0.6);
        assert_eq!(config.cache_ttl_hours, 24);
    }

    #[test]
    fn test_reranker_config_from_llm_settings() {
        let settings = crate::models::settings::LLMSettings {
            provider: LLMProvider::OpenAI,
            model: "gpt-4".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            api_key: Some("sk-test".to_string()),
            temperature: 0.2,
            max_tokens: Some(1024),
            timeout_seconds: 30,
        };
        let config: RerankerConfig = (&settings).into();
        assert_eq!(config.provider, LLMProvider::OpenAI);
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.2);
        assert_eq!(config.max_tokens, Some(1024));
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_compute_cv_hash_deterministic() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        let cv = sample_cv();
        
        let hash1 = reranker.compute_cv_hash(&cv);
        let hash2 = reranker.compute_cv_hash(&cv);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_job_ids_hash_deterministic() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let postings = vec![sample_posting(id1, "Job 1", vec!["Rust"]), sample_posting(id2, "Job 2", vec!["Python"])];
        
        let hash1 = reranker.compute_job_ids_hash(&postings);
        let hash2 = reranker.compute_job_ids_hash(&postings);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_job_ids_hash_order_independent() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let postings1 = vec![sample_posting(id1, "Job 1", vec!["Rust"]), sample_posting(id2, "Job 2", vec!["Python"])];
        let postings2 = vec![sample_posting(id2, "Job 2", vec!["Python"]), sample_posting(id1, "Job 1", vec!["Rust"])];
        
        let hash1 = reranker.compute_job_ids_hash(&postings1);
        let hash2 = reranker.compute_job_ids_hash(&postings2);
        assert_eq!(hash1, hash2); // Order shouldn't matter
    }

    #[test]
    fn test_summarize_cv() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        let cv = sample_cv();
        
        let summary = reranker.summarize_cv(&cv);
        assert!(summary.contains("John Doe"));
        assert!(summary.contains("San Francisco"));
        assert!(summary.contains("Rust"));
        assert!(summary.contains("Python"));
        assert!(summary.contains("Senior Software Engineer"));
        assert!(summary.contains("Stanford"));
    }

    #[test]
    fn test_parse_rerank_response_valid_json() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let postings = vec![sample_posting(id1, "Job 1", vec!["Rust"]), sample_posting(id2, "Job 2", vec!["Python"])];

        let response = format!(r#"[
            {{"job_id": "{}", "score": 85, "reasoning": "Good match"}},
            {{"job_id": "{}", "score": 45, "reasoning": "Partial match"}}
        ]"#, id1, id2);

        let outputs = reranker.parse_rerank_response(&response, &postings).unwrap();
        assert_eq!(outputs.len(), 2);
        assert!((outputs[0].score - 85.0).abs() < 1e-6);
        assert!((outputs[1].score - 45.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_rerank_response_with_markdown_fences() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        
        let id1 = Uuid::new_v4();
        let postings = vec![sample_posting(id1, "Job 1", vec!["Rust"])];

        let response = format!(r#"
```json
[
    {{"job_id": "{}", "score": 90, "reasoning": "Excellent match"}}
]
```"#, id1);

        let outputs = reranker.parse_rerank_response(&response, &postings).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!((outputs[0].score - 90.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_rerank_response_ignores_unknown_job_ids() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        
        let id1 = Uuid::new_v4();
        let unknown_id = Uuid::new_v4();
        let postings = vec![sample_posting(id1, "Job 1", vec!["Rust"])];

        let response = format!(r#"[
            {{"job_id": "{}", "score": 85, "reasoning": "Good match"}},
            {{"job_id": "{}", "score": 95, "reasoning": "Unknown job"}}
        ]"#, id1, unknown_id);

        let outputs = reranker.parse_rerank_response(&response, &postings).unwrap();
        assert_eq!(outputs.len(), 1); // Unknown job should be ignored
        assert_eq!(outputs[0].job_id, id1);
    }

    #[test]
    fn test_parse_rerank_response_score_clamping() {
        let config = RerankerConfig::default();
        let reranker = LLMReranker { config, executor: None, cache: Arc::new(RwLock::new(HashMap::new())) };
        
        let id1 = Uuid::new_v4();
        let postings = vec![sample_posting(id1, "Job 1", vec!["Rust"])];

        let response = format!(r#"[
            {{"job_id": "{}", "score": 150, "reasoning": "Over 100"}},
            {{"job_id": "{}", "score": -10, "reasoning": "Negative"}}
        ]"#, id1, id1);

        let outputs = reranker.parse_rerank_response(&response, &postings).unwrap();
        assert_eq!(outputs.len(), 2);
        assert!((outputs[0].score - 100.0).abs() < 1e-6); // Clamped to 100
        assert!((outputs[1].score - 0.0).abs() < 1e-6); // Clamped to 0
    }
}