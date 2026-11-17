-- Initial schema for Tunisia Career Scout

-- Sources table
CREATE TABLE sources (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('url', 'rss', 'gmail', 'social')),
    config TEXT NOT NULL, -- JSON
    schedule TEXT NOT NULL DEFAULT '0 * * * *',
    enabled INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'error')),
    auth_data TEXT, -- JSON (encrypted)
    last_scan_at TEXT, -- ISO8601 datetime
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_sources_enabled ON sources(enabled);
CREATE INDEX idx_sources_status ON sources(status);

-- Postings table
CREATE TABLE postings (
    id TEXT PRIMARY KEY NOT NULL,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    source_posting_id TEXT NOT NULL,
    title TEXT NOT NULL,
    company TEXT NOT NULL,
    description TEXT NOT NULL,
    summary TEXT,
    skills TEXT NOT NULL DEFAULT '[]', -- JSON array
    experience_required TEXT,
    education_required TEXT,
    languages TEXT NOT NULL DEFAULT '[]', -- JSON array
    location TEXT,
    remote_policy TEXT,
    job_type TEXT,
    salary_range TEXT,
    deadline TEXT, -- ISO8601 datetime
    application_steps TEXT NOT NULL DEFAULT '[]', -- JSON array
    application_email TEXT,
    application_url TEXT,
    contact_person TEXT,
    contact_phone TEXT,
    posted_date TEXT, -- ISO8601 datetime
    scraped_at TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    raw_html TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'expired', 'filled', 'hidden', 'applied', 'saved')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_postings_source ON postings(source_id);
CREATE INDEX idx_postings_source_external ON postings(source_id, source_posting_id);
CREATE INDEX idx_postings_scraped ON postings(scraped_at DESC);
CREATE INDEX idx_postings_status ON postings(status);
CREATE INDEX idx_postings_content_hash ON postings(content_hash);
CREATE UNIQUE INDEX idx_postings_unique ON postings(source_id, source_posting_id);

-- CV Profiles table
CREATE TABLE cv_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON
    template TEXT NOT NULL DEFAULT 'modern' CHECK (template IN ('modern', 'classic', 'minimal')),
    theme TEXT NOT NULL DEFAULT 'auto' CHECK (theme IN ('light', 'dark', 'auto')),
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_cv_default ON cv_profiles(is_default);

-- Match Results table
CREATE TABLE match_results (
    id TEXT PRIMARY KEY NOT NULL,
    cv_profile_id TEXT NOT NULL REFERENCES cv_profiles(id) ON DELETE CASCADE,
    posting_id TEXT NOT NULL REFERENCES postings(id) ON DELETE CASCADE,
    score REAL NOT NULL,
    matched_skills TEXT NOT NULL DEFAULT '[]', -- JSON array
    missing_skills TEXT NOT NULL DEFAULT '[]', -- JSON array
    reasons TEXT NOT NULL DEFAULT '[]', -- JSON array
    tfidf_score REAL,
    llm_score REAL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_match_cv ON match_results(cv_profile_id);
CREATE INDEX idx_match_posting ON match_results(posting_id);
CREATE INDEX idx_match_score ON match_results(score DESC);
CREATE UNIQUE INDEX idx_match_unique ON match_results(cv_profile_id, posting_id);

-- Settings table
CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL, -- JSON
    updated_at TEXT NOT NULL
);

-- Scan Logs table
CREATE TABLE scan_logs (
    id TEXT PRIMARY KEY NOT NULL,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'partial')),
    postings_found INTEGER NOT NULL DEFAULT 0,
    postings_new INTEGER NOT NULL DEFAULT 0,
    postings_updated INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER,
    raw_response_sample TEXT
);

CREATE INDEX idx_scan_logs_source ON scan_logs(source_id);
CREATE INDEX idx_scan_logs_started ON scan_logs(started_at DESC);

-- Scheduler State table
CREATE TABLE scheduler_state (
    job_id TEXT PRIMARY KEY NOT NULL,
    job_type TEXT NOT NULL CHECK (job_type IN ('source_scan', 'match_rerank', 'notification_check')),
    source_id TEXT REFERENCES sources(id) ON DELETE CASCADE,
    cron_expression TEXT NOT NULL,
    next_run TEXT NOT NULL,
    last_run TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    metadata TEXT, -- JSON
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_scheduler_state_enabled ON scheduler_state(enabled);
CREATE INDEX idx_scheduler_state_job_type ON scheduler_state(job_type);
CREATE INDEX idx_scheduler_state_next_run ON scheduler_state(next_run);