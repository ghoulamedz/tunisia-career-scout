# Tunisia Career Scout — Architecture Guide

## Overview
A multi-purpose webapp that **scrapes Tunisian concours (competitive exams) and job postings** from official, commercial, and social sources; **matches** them to a user's uploaded CV; **summarizes** the most relevant ones; and **presents** a clean dashboard with required documents checklist — targeted at Tunisian Francophone users.

## Tech Stack (Decided)
- **Frontend**: Next.js + TypeScript + Tailwind CSS
- **Backend API**: FastAPI (Python 3.12)
- **DB**: PostgreSQL (+ SQLAlchemy / Pydantic)
- **Scraping**: Scrapy + Playwright (for anti-bot sites)
- **Worker**: Celery + Celery Beat (scheduling)
- **NLP/Summarization**: Hybrid LLM (OpenAI/Anthropic API for quality + Ollama for local/privacy)
- **Auth**: JWT + bcrypt
- **Infra**: Docker + Docker Compose

## Directory Structure
```
tunisia-career-scout/
├── backend/                    # FastAPI app
│   ├── app/
│   │   ├── api/routes/         # Auth, Posts, Matches, Resumes, Notifications
│   │   ├── core/               # DB, security (JWT), deps
│   │   ├── models/             # SQLAlchemy models
│   │   ├── schemas/            # Pydantic v2 schemas
│   │   ├── services/           # Matching, Summarizer, Notifications
│   │   └── workers/            # Scrapers + scheduler
│   ├── tests/
│   └── requirements.txt
├── frontend/                   # Next.js app
│   ├── app/                    # Pages
│   ├── components/             # UI components
│   ├── lib/                    # API client + types
│   └── package.json
├── docker-compose.yml
├── .env.example
└── README.md
```

## Database Schema (PostgreSQL / SQLAlchemy)

### users
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| email | String unique | |
| password_hash | String | bcrypt |
| full_name | String | |
| country | String default "Tunisia" | |
| created_at | DateTime | |
| updated_at | DateTime | |

### resumes
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| user_id | UUID FK users | |
| parsed (JSON) | JSON | Extracted skills, education, experience, certs |
| file_path | String | Local upload path |
| parsed_at | DateTime | |
| created_at / updated_at | | |

### posting_sources
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| name | String | e.g. "ANETI", "Tunisie Concours" |
| type | String | "official", "commercial", "social" |
| url | String | Base URL |
| schedule | IntervalString | e.g. "1d", "12h" |
| enabled | Bool default true | |
| created_at | | |

### job_postings
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| source_id | UUID FK | |
| source_posting_id | String | For dedup |
| title | String | |
| description | Text | |
| company | String | |
| location | String | |
| job_type | String | "emploi", "alternance", "stage", "concours" |
| raw (JSON) | JSON | All scraped fields |
| status | String | "active", "closed" |
| closed_at | DateTime nullable | |
| created_at / updated_at | | |

### concours_postings
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| source_id | UUID FK | |
| source_id | UUID FK | |
| source_posting_id | String | For dedup |
| title | String | |
| description | Text | |
| organization | String | e.g., "BCT", "Ministère de l'Éducation" |
| exam_type | String | "interne", "externe", "nationale" |
| required_diploma | String | e.g. "Licence", "Master" |
| speciality | String | |
| age_limit | Integer nullable | |
| status | String | "open", "closed" |
| closed_at | DateTime nullable | |
| created_at / updated_at | | |

### matches
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| user_id | UUID FK | |
| posting_id | UUID FK | Flexible FK to job/concours |
| posting_type | String | "job_posting" / "concours_posting" |
| score | Float (0-100) | Similarity |
| matched_skills | JSON | Skills that matched |
| reasons | JSON | Human-readable explanations |
| created_at | | |

### notifications
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| user_id | UUID FK | |
| type | String | "new_posting", "summary", "match_score" |
| title | String | |
| body | Text | |
| link | String nullable | |
| read_at | DateTime nullable | |
| created_at | | |

### ai_summaries
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| posting_id | UUID FK | |
| posting_type | String | |
| user_id | UUID FK | |
| summary (Text) | Text | LLM-generated summary |
| required_documents (JSON) | JSON | Extracted docs list |
| match_score | Float | |
| created_at / updated_at | | |

## API Endpoints

### Auth
- `POST /api/auth/register` → {email, password, full_name} → user + JWT
- `POST /api/auth/login` → {email, password} → {token, user}
- `GET /api/auth/me` → current user

### Resumes
- `POST /api/resumes/upload` → multipart file (CV/PDF)
- `GET /api/resumes` → list of user's resumes
- `DELETE /api/resumes/:id`

### Postings (public + user-scoped)
- `GET /api/postings` → filter {source, job_type, location, status}
- `GET /api/postings/:id` → posting detail (+ matching score for current user if logged in)
- `GET /api/postings/sources` → list active sources

### Matches (user-scoped)
- `GET /api/matches` → ranked list for current user
- `GET /api/matches/:id` → detail
- `POST /api/matches/recompute` → recompute all scores for current user

### Summaries (user-scoped)
- `POST /api/postings/:id/summarize` → generate summary + docs
- `GET /api/matches/:id/summary` → get cached summary
- `DELETE /api/summaries/:id`

### Notifications (user-scoped)
- `GET /api/notifications` → list
- `PATCH /api/notifications/:id/read` → mark read
- `DELETE /api/notifications` → clear

## Scraping Pipeline Design

### Source Configs (both official + commercial + social)
Each source has:
- `name`: human label
- `type`: "official" / "commercial" / "social"
- `url`: base URL or page
- `auth`: none / bearer token / cookie
- `schedule`: "1h", "6h", "1d", "2d"
- `parser`: which parser function to use

### Parser Pattern
All parsers accept a raw HTML response and return a normalized JSON batch via the shared `parse_postings(html: str) -> list[dict]` interface.

### Anti-Bot Mitigation
- Playwright for JS-rendered pages (LinkedIn, possibly Facebook)
- Random user-agent rotation
- Respect `robots.txt`
- Rate limiting (critical — avoid IP bans)
- **Log all scraping activity** with source names for transparency

## Matching Engine

Uses **TF-IDF + cosine similarity** on:
- User resume parsed fields (skills, education, experience)
- Posting title + description + required skills

**Output:**
- Score (0–100)
- `matched_skills`: list of skill names that matched
- `reasons`: e.g., ["You have 3 years of React experience", "Master's degree matches required level"]

Re-score on POST /matches/recompute.

## Summarization Service

### Inputs (JSON)
- Title + description
- Company/organization
- Required documents (from text extraction)
- User profile (skills/education/experience)
- Rank in matches

### LLM Prompt (French, Francophone)
```
Vous êtes un assistant carrière pour les utilisateurs tunisiens.
Voici une offre d'emploi/concours et le profil de l'utilisateur.

CONCOURS/EMPLOI: {title}
ORGANISATION: {org}
DESCRIPTION: {desc}
DIPLÔMES REQUIS: {diplomas}
NIVEAU REQUIS: {level}

PROFIL UTILISATEUR:
- Compétences: {skills}
-Formation: {education}
-Expérience: {experience}

QUESTIONS:
1. Donnez un résumé en 3-4 phrases en français clair.
2. Dites précisément ce qui correspond au profil (1-2 phrases).
3. L'offre est-elle pertinente pour cet utilisateur? (oui/non)
4. Lister les documents requis (diplômes, attestations, etc.) — en liste à puce.
```

### Output
- `summary` (string)
- `required_documents` (list of strings)
- `relevance` (0–100)
- `is_relevant` (bool)

## Frontend Pages

1. **Landing** (`/`) — Hero, CTA "Upload your CV", features list, sources list
2. **Login / Register** (`/login`, `/register`)
3. **Dashboard** (`/`) — after login: show matches ranked, CTA to upload CV
4. **Posting Detail** (`/postings/[id]`) — full description, match score, summaries with docs checklist
5. **Matches** (`/matches`) — ranked list with filters
6. **Resume Upload** (`/resume`) — drag & drop, parsing status
7. **Settings** (`/settings`) — notifications, alert triggers

## Docker Compose

Services:
- `db` — PostgreSQL (via image)
- `backend` — FastAPI + Uvicorn
- `frontend` — Next.js
- `worker` — Celery worker (scrapers)
- `redis` — for Celery + caching

## Security & Legal

- **CNIL compliance**: log scraping, allow opt-out, aggregate where possible, no storing of personal data beyond CV for matching
- **robots.txt**: respect all sources
- **Rate limiting**: per-source and per-user
- **Source attribution**: always show source URL
- **Terms of service**: accept on register (scraping acknowledgment)

## Milestones

1. ✅ Project scaffold + Docker + DB
2. ✅ Auth (register/login) + JWT
3. ✅ Resume upload + parsing (extract skills/education/experience)
4. ✅ Posting schema + source configs (placeholder scrapers)
5. ✅ Matching engine (compute scores)
6. ✅ LLM summarization service
7. ✅ Frontend: landing + auth + dashboard
8. ✅ Frontend: posting detail + docs checklist
9. ✅ Tests + README + deploy guide
