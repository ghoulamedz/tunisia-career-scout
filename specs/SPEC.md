# Tunisia Career Scout — Specification

## Problem Statement

Job seekers in Tunisia (and globally) struggle to find relevant opportunities because:
1. Job postings are scattered across many sources (company sites, job boards, social media, email newsletters, government portals)
2. Existing aggregators are limited to fixed sources and don't allow user-extensible scraping
3. CV creation tools are either LaTeX-based (complex), paid, or produce poor PDF output
4. No unified tool exists that scouts jobs, builds CVs, and matches them locally on-device
5. Mobile apps for job search are either web wrappers or require cloud accounts

**Target User**: A job seeker who wants a self-contained, privacy-first app that runs on their Linux desktop and Android phone, scrapes configured sources on-device using their residential IP, builds tailored CVs, and matches jobs to their profile — all without cloud dependency.

## Solution

A **local-first, cross-platform application** built with **Tauri v2 (Rust backend + Svelte 5 frontend)** that:

1. **Scouts jobs** from user-configured sources: URLs, RSS feeds, Gmail (OAuth + Gmail API), and social media (Facebook, TikTok, LinkedIn via Playwright sidecar)
2. **Extracts structured data** via a two-pass pipeline: deterministic rules (schema.org JSON-LD, microdata, ATS heuristics for Greenhouse/Lever/Workable) → on-demand LLM fallback (configurable: OpenAI-compatible, Ollama, llama.cpp)
3. **Manages CVs** with a structured data model (CVData), 3 professional templates (Modern, Classic, Minimal) rendered via **Typst** to PDF
4. **Matches CVs to jobs** using TF-IDF + LLM reranking, displaying results sorted by relevance
5. **Runs entirely on-device**: Desktop (Linux) and Android (APK) each run the full stack; scraping happens locally via residential IP
6. **Syncs optionally** via local LAN/QR export-import (Phase 2+)

## User Stories

### Source Management
1. As a job seeker, I want to add a URL source (company careers page), so that the app scrapes it for new postings
2. As a job seeker, I want to add an RSS feed source, so that I get updates from job board feeds
3. As a job seeker, I want to connect my Gmail account via OAuth, so that the app fetches job alert emails via Gmail API
4. As a job seeker, I want to configure a social media source (Facebook/TikTok/LinkedIn), so that the app scrapes job posts from groups/pages
5. As a job seeker, I want to set a scan schedule per source (hourly, daily, weekly), so that scraping runs automatically
6. As a job seeker, I want to manually trigger a scan, so that I get fresh results on demand
7. As a job seeker, I want to see scan logs and errors, so that I can debug failing sources
8. As a job seeker, I want to enable/disable sources without deleting them, so that I can pause scraping

### Job Posting Extraction & Display
9. As a job seeker, I want the app to extract structured data (title, company, description, skills, experience, salary, deadline, application steps) from any source, so that I see consistent job cards
10. As a job seeker, I want deterministic rule-based extraction to run first (schema.org, microdata, ATS patterns), so that common sites are parsed fast and free
11. As a job seeker, I want a "Re-extract with LLM" button on any posting, so that I can get better extraction for tricky sites
12. As a job seeker, I want to configure my LLM provider (OpenAI, Ollama, llama.cpp) and model, so that extraction works with my preferred setup
13. As a job seeker, I want to browse job postings in a searchable, filterable list, so that I can find relevant opportunities
14. As a job seeker, I want to view full posting details including application steps (links, emails, forms), so that I know exactly how to apply
15. As a job seeker, I want to mark postings as applied/saved/hidden, so that I track my progress

### CV Engine
16. As a job seeker, I want to create/edit my CV in a structured form (personal info, experience, education, skills, languages, projects, certifications), so that my data is reusable
17. As a job seeker, I want to choose from 3 professional templates (Modern, Classic, Minimal), so that my CV looks polished
18. As a job seeker, I want a live PDF preview while editing, so that I see the final output instantly
19. As a job seeker, I want to export my CV as a high-quality PDF, so that I can submit applications
20. As a job seeker, I want to save multiple CV versions for different roles, so that I can tailor applications

### Matching & Relevance
21. As a job seeker, I want the app to score job postings against my CV (skills, experience, location, keywords), so that I see the most relevant first
22. As a job seeker, I want to see match reasons (matched skills, missing requirements, location fit), so that I understand the score
23. As a job seeker, I want to filter/sort by match score, date, source, so that I prioritize applications

### Mobile & Desktop Experience
24. As a job seeker, I want the same app on my Linux desktop and Android phone, so that I can use it anywhere
25. As a job seeker, I want scraping to run on-device using my residential IP, so that I avoid blocks and preserve privacy
26. As a job seeker, I want background scanning on Android (foreground service), so that I get fresh results without opening the app
27. As a job seeker, I want push notifications for high-match postings on Android, so that I never miss urgent opportunities
28. As a job seeker, I want a clean, responsive UI that works on mobile and desktop, so that the experience is consistent

### Settings & Configuration
29. As a job seeker, I want to configure LLM provider/model per extraction task, so that I optimize cost/quality
30. As a job seeker, I want to manage stored cookies for social sources, so that I stay logged in
31. As a job seeker, I want to export/import my configuration (sources, CV, settings), so that I can migrate devices

## Implementation Decisions

### Architecture
- **Tauri v2** for cross-platform (Desktop + Android) with Rust backend + WebView frontend
- **Svelte 5 + Vite** for frontend (signals reactivity, excellent DX, small bundles, TailwindCSS for styling)
- **Rust** for all backend logic: scraping, extraction, scheduling, database, matching, PDF generation
- **SQLite (sqlx)** for embedded database — same file format across platforms, zero config
- **Local-first**: No required cloud services; all data stays on device

### Database Schema (SQLite via SQLx Migrations)
- `sources` — id, name, type (url|rss|gmail|social), config (JSON), schedule, enabled, auth_data (encrypted), created_at, updated_at
- `postings` — id, source_id, source_posting_id, title, company, description, summary, skills (JSON), experience_required, education_required, languages (JSON), location, remote_policy, job_type, salary_range, deadline, application_steps (JSON), application_email, application_url, contact_person, contact_phone, posted_date, scraped_at, content_hash, raw_html, status, created_at, updated_at
- `cv_profiles` — id, name, data (JSON — full CVData), template_name, theme, is_default, created_at, updated_at
- `match_results` — id, cv_profile_id, posting_id, score, matched_skills (JSON), missing_skills (JSON), reasons (JSON), created_at
- `settings` — key, value (JSON) — LLM provider config, push token, sync preferences
- `scan_logs` — id, source_id, status, postings_found, postings_new, error_message, started_at, completed_at

### Source Adapters (Plugin Pattern)
- **Trait**: `SourceAdapter` with `fetch(&self, config: &SourceConfig) -> Result<Vec<RawContent>>`
- **URL Adapter**: HTTP GET + HTML parsing (selectors, feed-rs for RSS)
- **RSS Adapter**: feed-rs parser
- **Gmail Adapter**: OAuth2 + Gmail API (list messages by label/query, fetch full message)
- **Social Adapter**: Delegates to Python sidecar (Playwright) via stdin/stdout JSON protocol

### Extraction Pipeline
1. **RuleExtractor** (deterministic, fast):
   - schema.org JSON-LD parsing (`JobPosting` type)
   - HTML microdata (`itemprop`)
   - ATS heuristics: Greenhouse (`/jobs/` + `data-mapped`), Lever (`/jobs/` + `data-`), Workable (`/jobs/` + `data-ui`)
   - Fallback: readability-style content extraction
2. **LLMExtractor** (on-demand, configurable):
   - Structured prompt with `JobPosting` schema definition
   - Uses `llm-chain` with provider abstraction (OpenAI-compatible, Ollama)
   - Pydantic-equivalent validation via `serde` + custom validators
   - User-triggered via "Re-extract" button

### CV Engine & PDF (Typst)
- **CVData** struct (mirrors frontend form): personal, experience[], education[], skills[], languages[], projects[], certifications[], template, theme
- **Templates**: `modern.typ`, `classic.typ`, `minimal.typ` — Typst template files with `#let` functions for sections
- **Compiler**: `typst::compile()` → PDF bytes
- **Live Preview**: Frontend requests PDF on debounced form change; returned as base64 for `<embed>` or blob URL

### Matching Engine
- **TF-IDF Vectorizer**: Fit on all posting descriptions + CV skills/experience text
- **Cosine Similarity** → initial score (0-100)
- **LLM Rerank** (optional): Top-N sent to LLM for semantic relevance + reasoning
- **Output**: `MatchResult` with score, matched_skills, missing_skills, reasons

### Scheduler (Background Polling)
- **tokio-cron-scheduler** for cron-like schedules
- **Foreground Service** on Android (dataSync type) for background execution
- **Poller**: For each enabled source, run `ExtractionPipeline` → store new postings → trigger matching if CV exists

### Push Notifications (Android)
- **FCM** via `fcm` crate (Rust) or Tauri plugin
- Device token stored in settings
- Trigger: New posting with match score > threshold (configurable)

### Python Sidecar (Social Scraping)
- Bundled as Tauri sidecar binary (PyInstaller) or standalone Python + deps
- Protocol: JSON-RPC over stdin/stdout
- Commands: `login(url) -> cookies`, `scrape(url, cookies) -> RawContent[]`
- Scrapers: Facebook (group/page posts), TikTok (hashtag/user), LinkedIn (jobs/search)

### Frontend Architecture (Svelte 5 + Vite)
- **Routes** (file-based): `/` (dashboard), `/sources`, `/sources/new`, `/postings`, `/postings/:id`, `/cv`, `/cv/preview`, `/cv/templates`, `/matches`, `/settings`
- **Stores** (Svelte 5 signals): `sourcesStore`, `postingsStore`, `cvStore`, `matchesStore`, `settingsStore`
- **API Layer**: `invoke()` wrappers in `lib/api.ts` with TypeScript types mirroring Rust structs
- **Components**: Reusable UI (`Button`, `Card`, `Modal`, `Table`, `FormField`, `Toast`), domain-specific (`PostingCard`, `CVEditor`, `SourceForm`)
- **Styling**: TailwindCSS + custom CSS variables for theming

### Testing Strategy
- **Rust Unit Tests**: `#[cfg(test)]` modules in each crate — test extractors, validators, repositories
- **Rust Integration Tests**: `tests/rust/` — spin up test DB, test full pipelines with fixture HTML
- **Frontend Unit Tests**: Vitest + Svelte Testing Library — test stores, components
- **E2E Tests**: Playwright (separate `tests/e2e/`) — test critical user flows (add source → scan → view posting → create CV → match → export PDF)

## Testing Decisions

- **Seam 1: SourceAdapter trait** — Test each adapter with recorded HTML fixtures (no live network)
- **Seam 2: ExtractionPipeline** — Test rule extractor against known sites; test LLM extractor with mocked provider
- **Seam 3: Repository pattern** — Test DB operations with sqlx test fixtures
- **Seam 4: CV Compiler** — Test Typst compilation produces valid PDF bytes
- **Seam 5: Matching Engine** — Test TF-IDF scoring with known vectors; test LLM rerank with mocked responses
- **Seam 6: Scheduler** — Test job enqueueing and execution order (mock time)
- **Seam 7: Tauri Commands** — Test command handlers with mocked services

**Good Test Criteria**: Tests verify behavior through public interfaces; use independent expected values (fixtures, literals); survive refactors; run fast (<100ms unit, <5s integration).

## Out of Scope

- Cloud synchronization / multi-device sync (Phase 2+)
- User authentication / multi-user support (single-user local app)
- Web deployment (Vercel/Netlify) — this is a local app
- iOS support (Tauri v2 iOS is experimental)
- Windows desktop (Linux + Android only for MVP)
- Social media APIs (official APIs require approval; using Playwright scraping)
- Advanced CV features (cover letters, portfolio sites, AI writing assistant)
- Job application automation (auto-fill forms, submit applications)
- Team/sharing features
- Analytics / telemetry

## Further Notes

### Key Technical Risks
1. **Android Python sidecar size** — Playwright + Chromium ~150MB. Mitigation: Use `fantoccini` + `headless_chrome` Rust crate instead if viable.
2. **Background execution on Android** — Foreground service + WorkManager integration needs testing.
3. **Typst template complexity** — Typst is powerful but learning curve for complex layouts.
4. **LLM provider abstraction** — `llm-chain` is early; may need custom provider adapters.
5. **SQLite on Android** — File locking and WAL mode work but need stress testing.

### Migration from Old Codebase
- Old `backend/app/models/` → new `app/src/models/`
- Old `backend/app/services/matching.py` → new `app/src/matching/`
- Old `backend/app/services/summary.py` → extraction pipeline LLM step
- Old `backend/app/workers/scrapers/` → new `app/src/scrapers/adapters/`
- Old `frontend/src/` → new `app/frontend/src/`

### Folder Reference
```
tunisia-career-scout/
├── app/                          # MAIN TAURI APP
│   ├── src/                      # RUST BACKEND SOURCE
│   │   ├── main.rs               # Entry point
│   │   ├── lib.rs                # Public API
│   │   ├── commands.rs           # Tauri command handlers
│   │   ├── db/                   # Database (connection, migrations, repositories)
│   │   ├── models/               # Domain models (Source, JobPosting, CVData, MatchResult)
│   │   ├── scrapers/             # Extraction pipeline + source adapters
│   │   ├── scheduler/            # Background polling (tokio-cron)
│   │   ├── cv/                   # CV engine (Typst templates, compiler)
│   │   ├── matching/             # TF-IDF + LLM matching
│   │   ├── gmail/                # Gmail API + OAuth
│   │   ├── push/                 # FCM push notifications
│   │   └── sidecar/              # Python sidecar management
│   ├── frontend/                 # SVELTE 5 FRONTEND
│   │   ├── src/
│   │   │   ├── routes/           # File-based routing (SvelteKit-style)
│   │   │   ├── lib/              # API, stores, types, utils
│   │   │   ├── components/       # UI + domain components
│   │   │   └── styles/           # Global styles
│   │   ├── static/               # Icons, fonts
│   │   ├── package.json
│   │   ├── vite.config.ts
│   │   ├── svelte.config.js
│   │   ├── tailwind.config.js
│   │   └── tsconfig.json
│   ├── sidecar/                  # PYTHON SIDECAR (bundled)
│   │   └── python-scrapers/
│   ├── migrations/               # SQLX MIGRATION FILES
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
├── specs/
│   └── SPEC.md                   # THIS FILE
├── tests/
│   ├── rust/                     # Rust integration tests
│   ├── frontend/                 # Frontend unit tests (Vitest)
│   └── e2e/                      # Playwright E2E tests
├── docs/
│   └── agents/                   # Skill configs
├── CLAUDE.md
├── GLOSSARY.md
├── ADR-LOG.md
└── README.md
```

### Generated Android Files (Do Not Commit)
```
app/src-tauri/gen/android/
├── app/
│   ├── src/main/
│   │   ├── AndroidManifest.xml
│   │   ├── res/
│   │   └── java/com/tunisiacareerscout/app/MainActivity.kt
│   └── build.gradle.kts
└── build.gradle.kts
```

Generated by `cargo tauri android init` and `cargo tauri build --target aarch64-linux-android`.