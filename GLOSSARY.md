# Glossary

Authoritative domain vocabulary for Tunisia Career Scout. Use these terms consistently in code, specs, tickets, and conversation.

---

## Core Concepts

### Source
A user-configured origin for job postings. Has a **SourceType** (url, rss, gmail, social), **SourceConfig** (type-specific settings), schedule (cron expression), and optional authentication.

### SourceType
Enumeration of supported source types:
- **Url** — Web page scraping via HTTP + HTML parsing
- **Rss** — RSS/Atom feed parsing
- **Gmail** — Gmail API access for job alert emails
- **Social** — Social media (Facebook, TikTok, LinkedIn) via Playwright sidecar

### SourceAdapter
Plugin interface that fetches raw content from a Source. Implements `fetch(config) → RawContent[]`. One adapter per SourceType.

### RawContent
Unstructured input fetched from a source: HTML, RSS XML, email MIME, or social media JSON. Includes source_url, fetched_at, content_hash, and metadata.

### ExtractionPipeline
Two-pass process converting RawContent → JobPosting:
1. **RuleExtractor** — Deterministic: schema.org JSON-LD, HTML microdata, ATS heuristics (Greenhouse, Lever, Workable)
2. **LLMExtractor** — Fallback: structured LLM prompt + Pydantic-equivalent validation. User-triggered via "Re-extract" button.

### JobPosting
Canonical structured job record. Fields:
- **Identity**: id, source_id, source_posting_id (external dedup key), content_hash
- **Core**: title, company, description, summary, skills[], experience_required, education_required, languages[]
- **Location/Type**: location, remote_policy (remote/hybrid/onsite), job_type (full-time/part-time/contract/internship), salary_range
- **Application**: deadline, application_steps[] (ordered: link/email/form/upload), application_email, application_url, contact_person, contact_phone
- **Metadata**: posted_date, scraped_at, raw_html (optional), status (active/expired/filled/hidden/applied/saved)

### ApplicationStep
Ordered step in a job application: order, description, action_type (link/email/form/upload/other), target (URL/email), deadline, required, completed.

### CVData
User's structured CV. Fields:
- **Personal**: full_name, email, phone, location, linkedin, github, portfolio, summary
- **Experience[]**: title, company, location, start_date, end_date, current, description, skills[]
- **Education[]**: institution, degree, field_of_study, location, start_date, end_date, current, grade
- **Skills[]**: name, category, level (beginner/intermediate/advanced/expert), years
- **Languages[]**: name, proficiency (native/fluent/professional/conversational/basic)
- **Projects[]**: name, description, url, github, technologies[], dates
- **Certifications[]**: name, issuer, date, expiry, url, credential_id
- **Rendering**: template (modern/classic/minimal), theme (light/dark/auto)

### CVProfile
A named, saved CV configuration (CVData + template + theme + is_default). Users can maintain multiple profiles for different roles.

### CVTemplate
Pre-defined visual styles for CV PDF output:
- **Modern** — Clean, contemporary with subtle accents
- **Classic** — Traditional professional layout
- **Minimal** — Minimalist, content-focused

### MatchResult
CV-to-JobPosting relevance score (0-100) with reasoning:
- score, matched_skills[], missing_skills[], reasons[] (category, description, weight, matched)
- tfidf_score, llm_score (when LLM reranking used)

### MatchingEngine
Computes relevance between CVData and JobPosting:
1. **TF-IDF Vectorizer** — Fit on CV text + posting descriptions, cosine similarity → initial score
2. **LLM Rerank** (optional) — Top-N sent to LLM for semantic relevance + reasoning

### ScanLog
Record of a source scanning operation: source_id, status (running/completed/failed/partial), postings_found/new/updated, error_message, duration_ms.

### Scheduler
Background job polling system (tokio-cron-scheduler). Runs ExtractionPipeline for each enabled Source on its schedule. On Android: foreground service with dataSync type.

### PythonSidecar
Bundled Playwright-based scraper for social media (Facebook, TikTok, LinkedIn). Communicates via JSON-RPC over stdin/stdout with Tauri app. Handles login (user-assisted cookie capture) and scraping.

### PushNotification
Android FCM notifications for high-match job postings. Threshold configurable via min_match_score. Quiet hours support.

---

## Technical Terms

### Tauri v2
Cross-platform framework: Rust backend + WebView frontend. Generates native binaries for Linux/macOS/Windows and APK/AAB for Android.

### Svelte 5
Reactive UI framework using runes (`$state`, `$derived`, `$effect`). File-based routing in `frontend/src/routes/`.

### SQLx
Compile-time checked SQL for Rust. SQLite database with migrations in `app/migrations/`.

### Typst
Modern typesetting system for PDF generation. CV templates in `templates/cv/*.typ`.

### llm-chain
Rust LLM abstraction supporting OpenAI-compatible, Ollama, llama.cpp providers.

### tokio-cron-scheduler
Cron-like job scheduler for async Rust. Used for background source polling.

### FCM
Firebase Cloud Messaging for Android push notifications.

### JSON-RPC
Lightweight RPC protocol used for Tauri↔Python sidecar communication over stdin/stdout.

---

## UI/UX Terms

### Dashboard
Main landing page showing stats, recent postings, top matches, quick actions.

### Source Management
CRUD interface for Sources: list, create, edit, delete, manual scan, view logs.

### Posting List/Detail
Searchable, filterable list of JobPostings with detail view showing full description, application steps, and actions (save, apply, re-extract).

### CV Editor
Multi-section form for CVData with live PDF preview and template/theme picker.

### Matches
Ranked list of MatchResults for selected CVProfile with score breakdown and reasons.

### Settings
Tabbed configuration: LLM, Push Notifications, General (theme, language, concurrency, retention), Advanced (danger zone, debug).