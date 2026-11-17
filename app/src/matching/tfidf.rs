//! TF-IDF Matching Engine
//!
//! Computes cosine similarity between CV and job postings.
//!
//! # TF-IDF Vectorization
//!
//! - Tokenization: lowercase, alphanumeric only, length 2-30
//! - Vocabulary: sorted for deterministic ordering
//! - IDF: log((N + 1) / (df + 1)) + 1 (smoothed)
//! - TF: raw count normalized by document length
//! - L2 normalization on final TF-IDF vectors
//!
//! # Matching
//!
//! - Cosine similarity between CV and job posting vectors (0-100)
//! - Skill set comparison (case-insensitive)
//! - Reason generation with MatchCategory categorization

use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use crate::models::cv::CVData;
use crate::models::posting::JobPosting;
use crate::models::match_result::{MatchResult, MatchReason, MatchCategory};

/// TF-IDF vectorizer for text similarity.
///
/// Vocabulary is built from a corpus and sorted for deterministic token-to-index mapping.
/// IDF uses smoothed log((N + 1) / (df + 1)) + 1 to avoid zero weights for unseen terms.
#[derive(Debug, Clone)]
pub struct TfIdfVectorizer {
    vocabulary: HashMap<String, usize>,
    idf: Vec<f32>,
    doc_count: usize,
}

impl TfIdfVectorizer {
    /// Create a new empty vectorizer.
    pub fn new() -> Self {
        Self {
            vocabulary: HashMap::new(),
            idf: Vec::new(),
            doc_count: 0,
        }
    }

    /// Fit the vectorizer on a corpus of documents.
    ///
    /// Builds vocabulary from all unique tokens across documents,
    /// computes document frequencies, then computes smoothed IDF weights.
    pub fn fit(&mut self, documents: &[String]) {
        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        let mut vocab_set = HashSet::new();

        for doc in documents {
            let tokens = Self::tokenize(doc);
            let unique_tokens: HashSet<_> = tokens.into_iter().collect();
            for token in unique_tokens {
                *doc_freq.entry(token.clone()).or_insert(0) += 1;
                vocab_set.insert(token);
            }
        }

        // Build vocabulary: sorted for deterministic ordering
        let mut vocab: Vec<String> = vocab_set.into_iter().collect();
        vocab.sort();
        self.vocabulary = vocab.iter().enumerate().map(|(i, t)| (t.clone(), i)).collect();

        // Compute IDF with smoothing: log((N + 1) / (df + 1)) + 1
        self.doc_count = documents.len();
        self.idf = vec![0.0; self.vocabulary.len()];
        let n_plus_1 = (self.doc_count + 1) as f32;
        for (token, idx) in &self.vocabulary {
            let df = doc_freq.get(token).copied().unwrap_or(0);
            let df_plus_1 = (df + 1) as f32;
            self.idf[*idx] = (n_plus_1 / df_plus_1).ln() + 1.0;
        }
    }

    /// Transform a document into a TF-IDF vector.
    ///
    /// Returns a vector of length vocabulary_size with L2-normalized TF-IDF weights.
    /// Tokens not in vocabulary are ignored.
    pub fn transform(&self, document: &str) -> Vec<f32> {
        let tokens = Self::tokenize(document);
        let mut tf = vec![0.0; self.vocabulary.len()];

        // Term frequency: raw count
        for token in tokens {
            if let Some(&idx) = self.vocabulary.get(&token) {
                tf[idx] += 1.0;
            }
        }

        // Normalize TF by document length (L1 norm)
        let total: f32 = tf.iter().sum();
        if total > 0.0 {
            for v in &mut tf {
                *v /= total;
            }
        }

        // Apply IDF weights
        for i in 0..tf.len() {
            tf[i] *= self.idf[i];
        }

        // L2 normalize the final TF-IDF vector
        let norm: f32 = tf.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut tf {
                *v /= norm;
            }
        }

        tf
    }

    /// Tokenize text into lowercase alphanumeric tokens of length 2-30.
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 2 && s.len() <= 30)
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.vocabulary.len()
    }

    /// Get the IDF weight for a token, if in vocabulary.
    pub fn idf_for(&self, token: &str) -> Option<f32> {
        self.vocabulary.get(token).map(|&idx| self.idf[idx])
    }
}

impl Default for TfIdfVectorizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Main matching engine for CV-to-job matching.
///
/// Stores the CV data, builds a TF-IDF vectorizer fitted on the CV text,
/// and computes match scores against job postings.
#[derive(Debug)]
pub struct MatchingEngine {
    vectorizer: TfIdfVectorizer,
    cv_profile_id: Uuid,
    cv_text: String,
    cv_vector: Vec<f32>,
    cv_data: CVData,
}

impl MatchingEngine {
    /// Create a new matching engine for a CV profile.
    ///
    /// The vectorizer is fitted on the CV text alone, so IDF weights
    /// reflect term rarity within the CV (terms appearing in CV get IDF=1.0+).
    pub fn new(cv_profile_id: Uuid, cv: &CVData) -> Self {
        let cv_text = Self::cv_to_text(cv);
        let mut vectorizer = TfIdfVectorizer::new();
        vectorizer.fit(&[cv_text.clone()]);
        let cv_vector = vectorizer.transform(&cv_text);

        Self {
            vectorizer,
            cv_profile_id,
            cv_text,
            cv_vector,
            cv_data: cv.clone(),
        }
    }

    /// Convert CV data to a single text string for vectorization.
    fn cv_to_text(cv: &CVData) -> String {
        let mut parts = Vec::new();

        parts.push(cv.full_name.clone());
        if let Some(summary) = &cv.summary {
            parts.push(summary.clone());
        }

        for exp in &cv.experience {
            parts.push(exp.title.clone());
            parts.push(exp.company.clone());
            parts.push(exp.description.clone());
            parts.extend(exp.skills.clone());
        }

        for edu in &cv.education {
            parts.push(edu.degree.clone());
            parts.push(edu.field_of_study.clone().unwrap_or_default());
            parts.push(edu.institution.clone());
        }

        for skill in &cv.skills {
            parts.push(skill.name.clone());
            if let Some(cat) = &skill.category {
                parts.push(cat.clone());
            }
        }

        for lang in &cv.languages {
            parts.push(lang.name.clone());
        }

        for proj in &cv.projects {
            parts.push(proj.name.clone());
            parts.push(proj.description.clone());
            parts.extend(proj.technologies.clone());
        }

        for cert in &cv.certifications {
            parts.push(cert.name.clone());
            parts.push(cert.issuer.clone());
        }

        parts.join(" ")
    }

    /// Convert job posting to a single text string for vectorization.
    fn posting_to_text(posting: &JobPosting) -> String {
        let mut parts = Vec::new();
        parts.push(posting.title.clone());
        parts.push(posting.company.clone());
        parts.push(posting.description.clone());
        if let Some(summary) = &posting.summary {
            parts.push(summary.clone());
        }
        parts.extend(posting.skills.0.clone());
        if let Some(exp) = &posting.experience_required {
            parts.push(exp.clone());
        }
        if let Some(edu) = &posting.education_required {
            parts.push(edu.clone());
        }
        parts.extend(posting.languages.0.clone());
        if let Some(loc) = &posting.location {
            parts.push(loc.clone());
        }
        if let Some(remote) = &posting.remote_policy {
            parts.push(remote.clone());
        }
        if let Some(jtype) = &posting.job_type {
            parts.push(jtype.clone());
        }
        parts.join(" ")
    }

    /// Compute cosine similarity between two L2-normalized vectors.
    /// Returns value in [0, 1].
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        // Vectors are already L2-normalized, so norm = 1
        dot.clamp(0.0, 1.0)
    }

    /// Compare CV skills vs posting skills (case-insensitive).
    /// Returns (matched_skills, missing_skills) with original casing from posting.
    fn compare_skills(&self, posting: &JobPosting) -> (Vec<String>, Vec<String>) {
        let cv_skills: HashSet<String> = self
            .cv_data
            .skills
            .iter()
            .map(|s| s.name.to_lowercase())
            .collect();

        let mut matched = Vec::new();
        let mut missing = Vec::new();

        for skill in &posting.skills.0 {
            if cv_skills.contains(&skill.to_lowercase()) {
                matched.push(skill.clone());
            } else {
                missing.push(skill.clone());
            }
        }

        (matched, missing)
    }

    /// Generate match reasons with categories and weights.
    fn generate_reasons(
        &self,
        posting: &JobPosting,
        matched_skills: &[String],
        missing_skills: &[String],
        _score: f32,
    ) -> Vec<MatchReason> {
        let mut reasons = Vec::new();

        // Skills match
        if !matched_skills.is_empty() {
            reasons.push(MatchReason {
                category: MatchCategory::Skills,
                description: format!(
                    "Matched {} skill(s): {}",
                    matched_skills.len(),
                    matched_skills.join(", ")
                ),
                weight: 0.4,
                matched: true,
            });
        }

        // Missing skills
        if !missing_skills.is_empty() {
            reasons.push(MatchReason {
                category: MatchCategory::Skills,
                description: format!(
                    "Missing {} skill(s): {}",
                    missing_skills.len(),
                    missing_skills.join(", ")
                ),
                weight: -0.2,
                matched: false,
            });
        }

        // Location match
        if let (Some(cv_loc), Some(job_loc)) = (&self.cv_data.location, &posting.location) {
            let cv_lower = cv_loc.to_lowercase();
            let job_lower = job_loc.to_lowercase();
            if cv_lower.contains(&job_lower) || job_lower.contains(&cv_lower) {
                reasons.push(MatchReason {
                    category: MatchCategory::Location,
                    description: format!("Location match: {}", job_loc),
                    weight: 0.15,
                    matched: true,
                });
            }
        }

        // Remote policy
        if let Some(remote) = &posting.remote_policy {
            let remote_lower = remote.to_lowercase();
            if remote_lower.contains("remote") {
                reasons.push(MatchReason {
                    category: MatchCategory::Other,
                    description: "Remote position available".to_string(),
                    weight: 0.1,
                    matched: true,
                });
            } else if remote_lower.contains("hybrid") {
                reasons.push(MatchReason {
                    category: MatchCategory::Other,
                    description: "Hybrid work policy".to_string(),
                    weight: 0.05,
                    matched: true,
                });
            }
        }

        // Experience match (keyword-based)
        if let Some(exp_req) = &posting.experience_required {
            let exp_lower = exp_req.to_lowercase();
            let cv_exp_text = self.cv_data.experience.iter()
                .map(|e| format!("{} {}", e.title, e.description))
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if cv_exp_text.contains(&exp_lower) || exp_lower.contains(&cv_exp_text) {
                reasons.push(MatchReason {
                    category: MatchCategory::Experience,
                    description: format!("Experience requirement matched: {}", exp_req),
                    weight: 0.1,
                    matched: true,
                });
            }
        }

        // Education match
        if let Some(edu_req) = &posting.education_required {
            let edu_lower = edu_req.to_lowercase();
            let cv_edu_text = self.cv_data.education.iter()
                .map(|e| format!("{} {}", e.degree, e.field_of_study.as_deref().unwrap_or("")))
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if cv_edu_text.contains(&edu_lower) || edu_lower.contains(&cv_edu_text) {
                reasons.push(MatchReason {
                    category: MatchCategory::Education,
                    description: format!("Education requirement matched: {}", edu_req),
                    weight: 0.1,
                    matched: true,
                });
            }
        }

        // Language match
        let cv_langs: HashSet<String> = self.cv_data.languages.iter()
            .map(|l| l.name.to_lowercase())
            .collect();
        let matched_langs: Vec<String> = posting.languages.0.iter()
            .filter(|l| cv_langs.contains(&l.to_lowercase()))
            .cloned()
            .collect();
        if !matched_langs.is_empty() {
            reasons.push(MatchReason {
                category: MatchCategory::Language,
                description: format!("Language match: {}", matched_langs.join(", ")),
                weight: 0.1,
                matched: true,
            });
        }

        // Keyword match (from description)
        let cv_keywords: HashSet<String> = TfIdfVectorizer::tokenize(&self.cv_text).into_iter().collect();
        let posting_keywords: HashSet<String> = TfIdfVectorizer::tokenize(&Self::posting_to_text(posting)).into_iter().collect();
        let common_keywords: Vec<String> = cv_keywords.intersection(&posting_keywords).cloned().collect();
        if !common_keywords.is_empty() && common_keywords.len() > 3 {
            reasons.push(MatchReason {
                category: MatchCategory::Keywords,
                description: format!("{} shared keywords", common_keywords.len()),
                weight: 0.05,
                matched: true,
            });
        }

        reasons
    }

    /// Compute match score for a single job posting.
    ///
    /// Returns a MatchResult with score (0-100), matched/missing skills, and reasons.
    pub fn match_posting(&self, posting: &JobPosting) -> MatchResult {
        let posting_text = Self::posting_to_text(posting);
        let posting_vector = self.vectorizer.transform(&posting_text);

        // Cosine similarity (0-1) scaled to 0-100
        let tfidf_score = self.cosine_similarity(&self.cv_vector, &posting_vector);
        let score = (tfidf_score * 100.0).round().clamp(0.0, 100.0);

        // Extract matched/missing skills
        let (matched_skills, missing_skills) = self.compare_skills(posting);

        // Generate reasons
        let reasons = self.generate_reasons(posting, &matched_skills, &missing_skills, score);

        let mut result = MatchResult::new(
            self.cv_profile_id,
            posting.id,
            score,
            matched_skills,
            missing_skills,
            reasons,
        );
        result.tfidf_score = Some(tfidf_score);
        result
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
                Experience {
                    id: Uuid::new_v4(),
                    title: "Software Engineer".to_string(),
                    company: "StartupInc".to_string(),
                    location: Some("Remote".to_string()),
                    start_date: Utc.with_ymd_and_hms(2017, 6, 1, 0, 0, 0).unwrap(),
                    end_date: Some(Utc.with_ymd_and_hms(2019, 12, 31, 0, 0, 0).unwrap()),
                    current: false,
                    description: "Full-stack web development with React and Node.js.".to_string(),
                    skills: vec!["JavaScript".to_string(), "React".to_string(), "Node.js".to_string(), "PostgreSQL".to_string()],
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
                Skill { id: Uuid::new_v4(), name: "JavaScript".to_string(), category: Some("technical".to_string()), level: Some(SkillLevel::Intermediate), years: Some(3) },
                Skill { id: Uuid::new_v4(), name: "React".to_string(), category: Some("technical".to_string()), level: Some(SkillLevel::Intermediate), years: Some(3) },
                Skill { id: Uuid::new_v4(), name: "Leadership".to_string(), category: Some("soft".to_string()), level: Some(SkillLevel::Advanced), years: Some(3) },
            ],
            languages: vec![
                Language { id: Uuid::new_v4(), name: "English".to_string(), proficiency: LanguageProficiency::Native },
                Language { id: Uuid::new_v4(), name: "Spanish".to_string(), proficiency: LanguageProficiency::Conversational },
            ],
            projects: vec![
                Project {
                    id: Uuid::new_v4(),
                    name: "Distributed Cache".to_string(),
                    description: "High-performance distributed cache in Rust".to_string(),
                    url: None,
                    github: Some("github.com/johndoe/cache".to_string()),
                    technologies: vec!["Rust".to_string(), "Tokio".to_string(), "Redis".to_string()],
                    start_date: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
                    end_date: Some(Utc.with_ymd_and_hms(2021, 6, 30, 0, 0, 0).unwrap()),
                },
            ],
            certifications: vec![
                Certification {
                    id: Uuid::new_v4(),
                    name: "AWS Solutions Architect".to_string(),
                    issuer: "Amazon Web Services".to_string(),
                    date: Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap(),
                    expiry: Some(Utc.with_ymd_and_hms(2025, 3, 15, 0, 0, 0).unwrap()),
                    url: None,
                    credential_id: None,
                },
            ],
            template: CVTemplate::Modern,
            theme: CVTheme::Auto,
        }
    }

    fn sample_posting() -> JobPosting {
        let now = Utc::now();
        JobPosting {
            id: Uuid::new_v4(),
            source_id: Uuid::new_v4(),
            source_posting_id: "test-123".to_string(),
            title: "Senior Rust Engineer".to_string(),
            company: "SystemsInc".to_string(),
            description: "We are looking for a Senior Rust Engineer to build high-performance distributed systems. Experience with Kubernetes and AWS required.".to_string(),
            summary: Some("Senior Rust role with Kubernetes and AWS".to_string()),
            skills: vec!["Rust".to_string(), "Kubernetes".to_string(), "AWS".to_string(), "Python".to_string(), "Distributed Systems".to_string()].into(),
            experience_required: Some("5+ years Rust".to_string()),
            education_required: Some("MS Computer Science".to_string()),
            languages: vec!["English".to_string()].into(),
            location: Some("San Francisco, CA".to_string()),
            remote_policy: Some("Hybrid".to_string()),
            job_type: Some("Full-time".to_string()),
            salary_range: Some("$150k-$200k".to_string()),
            deadline: None,
            application_steps: vec![].into(),
            application_email: None,
            application_url: Some("https://systemsinc.com/jobs/123".to_string()),
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
    fn test_tokenize() {
        let tokens = TfIdfVectorizer::tokenize("Hello, World! This is a test.");
        assert_eq!(tokens, vec!["hello", "world", "this", "test"]);
    }

    #[test]
    fn test_tokenize_filters_short_long() {
        let tokens = TfIdfVectorizer::tokenize("a ab abc abcdefghijklmnopqrstuvwxyzabcde");
        assert_eq!(tokens, vec!["ab", "abc"]); // "a" too short, long one too long
    }

    #[test]
    fn test_fit_transform_deterministic() {
        let mut v = TfIdfVectorizer::new();
        v.fit(&["hello world".to_string(), "world rust".to_string()]);
        
        let vec1 = v.transform("hello world");
        let vec2 = v.transform("hello world");
        assert_eq!(vec1, vec2); // deterministic
        
        // Vocabulary should be sorted: ["hello", "rust", "world"]
        assert_eq!(v.vocab_size(), 3);
    }

    #[test]
    fn test_idf_smoothing() {
        let mut v = TfIdfVectorizer::new();
        v.fit(&["hello world".to_string()]); // N=1
        
        // df(hello)=1, df(world)=1
        // IDF = log((1+1)/(1+1)) + 1 = log(1) + 1 = 1.0
        let idf_hello = v.idf_for("hello").unwrap();
        let idf_world = v.idf_for("world").unwrap();
        assert!((idf_hello - 1.0).abs() < 1e-6);
        assert!((idf_world - 1.0).abs() < 1e-6);
        
        // Term not in any doc but in vocab (not possible with current fit)
        // But if we fit with more docs:
        let mut v2 = TfIdfVectorizer::new();
        v2.fit(&["hello".to_string(), "world".to_string()]); // N=2
        // df(hello)=1, df(world)=1
        // IDF = log((2+1)/(1+1)) + 1 = log(1.5) + 1 ≈ 1.405
        let idf = v2.idf_for("hello").unwrap();
        assert!((idf - (1.5_f32.ln() + 1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_l2_normalization() {
        let mut v = TfIdfVectorizer::new();
        v.fit(&["hello world rust".to_string()]);
        let vec = v.transform("hello world rust");
        
        // L2 norm should be 1.0
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut v = TfIdfVectorizer::new();
        v.fit(&["hello world".to_string()]);
        let a = v.transform("hello world");
        let b = v.transform("hello world");
        
        let sim = MatchingEngine::cosine_similarity_static(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut v = TfIdfVectorizer::new();
        v.fit(&["hello world".to_string(), "rust python".to_string()]);
        let a = v.transform("hello world");
        let b = v.transform("rust python");
        
        let sim = MatchingEngine::cosine_similarity_static(&a, &b);
        assert!(sim < 0.01); // nearly orthogonal
    }

    #[test]
    fn test_matching_engine_basic() {
        let cv = sample_cv();
        let engine = MatchingEngine::new(Uuid::new_v4(), &cv);
        let posting = sample_posting();
        
        let result = engine.match_posting(&posting);
        
        assert!(result.score >= 0.0 && result.score <= 100.0);
        assert!(!result.matched_skills.0.is_empty()); // Should match Rust, Kubernetes, AWS, Python
        assert_eq!(result.cv_profile_id, engine.cv_profile_id);
        assert_eq!(result.posting_id, posting.id);
        assert!(!result.reasons.0.is_empty());
    }

    #[test]
    fn test_skill_comparison_case_insensitive() {
        let cv = sample_cv(); // Has "Rust", "Python", "Kubernetes", "AWS"
        let engine = MatchingEngine::new(Uuid::new_v4(), &cv);
        
        let mut posting = sample_posting();
        posting.skills = vec!["rust".to_string(), "PYTHON".to_string(), "kubernetes".to_string(), "aws".to_string(), "Go".to_string()].into();
        
        let (matched, missing) = engine.compare_skills(&posting);
        
        assert_eq!(matched.len(), 4); // rust, python, kubernetes, aws (case insensitive)
        assert_eq!(missing.len(), 1); // Go
        assert!(missing.contains(&"Go".to_string()));
    }

    #[test]
    fn test_reason_generation_categories() {
        let cv = sample_cv();
        let engine = MatchingEngine::new(Uuid::new_v4(), &cv);
        let posting = sample_posting();
        
        let result = engine.match_posting(&posting);
        
        let categories: Vec<MatchCategory> = result.reasons.0.iter().map(|r| r.category.clone()).collect();
        
        // Should have Skills (matched), Skills (missing), Location, Other (hybrid), Experience, Education, Language, Keywords
        assert!(categories.contains(&MatchCategory::Skills));
        assert!(categories.contains(&MatchCategory::Location));
        assert!(categories.contains(&MatchCategory::Other));
        assert!(categories.contains(&MatchCategory::Experience));
        assert!(categories.contains(&MatchCategory::Education));
        assert!(categories.contains(&MatchCategory::Language));
        // Keywords may or may not appear depending on overlap
    }

    #[test]
    fn test_match_result_structure() {
        let cv = sample_cv();
        let engine = MatchingEngine::new(Uuid::new_v4(), &cv);
        let posting = sample_posting();
        
        let result = engine.match_posting(&posting);
        
        assert!(result.tfidf_score.is_some());
        assert!(result.tfidf_score.unwrap() >= 0.0 && result.tfidf_score.unwrap() <= 1.0);
        assert!(result.llm_score.is_none()); // Not set by TF-IDF engine
        
        // Matched skills should be subset of posting skills
        for skill in &result.matched_skills.0 {
            assert!(posting.skills.0.iter().any(|s| s.eq_ignore_ascii_case(skill)));
        }
        
        // Missing skills should be posting skills not in CV
        for skill in &result.missing_skills.0 {
            assert!(posting.skills.0.iter().any(|s| s.eq_ignore_ascii_case(skill)));
            assert!(!cv.skills.iter().any(|cs| cs.name.eq_ignore_ascii_case(skill)));
        }
    }

    #[test]
    fn test_known_vectors() {
        // Test with known simple corpus for exact vector values
        let mut v = TfIdfVectorizer::new();
        v.fit(&["a b c".to_string(), "b c d".to_string(), "c d e".to_string()]);
        
        // Vocabulary sorted: ["a", "b", "c", "d", "e"]
        // N=3
        // df(a)=1, df(b)=2, df(c)=3, df(d)=2, df(e)=1
        // IDF(a) = log(4/2)+1 = log(2)+1 ≈ 1.693
        // IDF(b) = log(4/3)+1 ≈ 1.288
        // IDF(c) = log(4/4)+1 = 1.0
        // IDF(d) = log(4/3)+1 ≈ 1.288
        // IDF(e) = log(4/2)+1 ≈ 1.693
        
        assert_eq!(v.vocab_size(), 5);
        
        let vec_a = v.transform("a");
        // TF: [1,0,0,0,0] -> normalized TF: [1,0,0,0,0]
        // TF-IDF: [1.693, 0, 0, 0, 0] -> L2 norm = 1.693 -> [1, 0, 0, 0, 0]
        assert!((vec_a[0] - 1.0).abs() < 1e-6);
        assert!(vec_a[1].abs() < 1e-6);
        assert!(vec_a[2].abs() < 1e-6);
        assert!(vec_a[3].abs() < 1e-6);
        assert!(vec_a[4].abs() < 1e-6);
        
        let vec_c = v.transform("c");
        // TF-IDF: [0, 0, 1.0, 0, 0] -> L2 norm = 1.0 -> [0, 0, 1, 0, 0]
        assert!((vec_c[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_remote_policy_reasons() {
        let cv = sample_cv();
        let engine = MatchingEngine::new(Uuid::new_v4(), &cv);
        
        let mut posting = sample_posting();
        posting.remote_policy = Some("Fully Remote".to_string());
        
        let result = engine.match_posting(&posting);
        
        let has_remote = result.reasons.0.iter().any(|r| 
            r.category == MatchCategory::Other && r.description.contains("Remote")
        );
        assert!(has_remote);
    }
}

impl MatchingEngine {
    /// Static cosine similarity for testing.
    fn cosine_similarity_static(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        dot.clamp(0.0, 1.0)
    }
}
