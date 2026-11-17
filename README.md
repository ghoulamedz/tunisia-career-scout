# Tunisia Career Scout

A universal, local-first job scouting and CV management application built with **Tauri v2 (Rust + Svelte 5)**.

## Features

- 🔍 **Universal Job Scouting** - Configure sources: URLs, RSS feeds, Gmail (OAuth), Social Media (Facebook, TikTok, LinkedIn via Playwright)
- 🤖 **Two-Pass Extraction** - Deterministic rules (schema.org, microdata, ATS patterns) + LLM fallback (configurable: OpenAI, Ollama, llama.cpp)
- 📄 **CV Engine** - Structured CV data model with 3 professional templates (Modern, Classic, Minimal) → PDF via Typst
- 🎯 **Smart Matching** - TF-IDF + LLM reranking to match CVs against job postings
- 📱 **Cross-Platform** - Linux desktop + Android (APK) from single codebase
- 🔒 **Local-First** - All data stays on your device, SQLite database, no cloud required
- 🔔 **Push Notifications** - Android FCM for high-match job alerts

## Architecture

```
tunisia-career-scout/
├── app/                          # Tauri v2 Application
│   ├── src/                      # Rust Backend
│   │   ├── main.rs               # Entry point
│   │   ├── commands.rs           # Tauri command handlers
│   │   ├── db/                   # SQLite + SQLx (repositories, migrations)
│   │   ├── models/               # Domain models (Source, JobPosting, CVData, MatchResult)
│   │   ├── scrapers/             # Extraction pipeline + source adapters
│   │   ├── scheduler/            # Background polling (tokio-cron)
│   │   ├── cv/                   # CV engine (Typst templates, compiler)
│   │   ├── matching/             # TF-IDF + LLM matching
│   │   ├── gmail/                # Gmail API + OAuth2
│   │   ├── push/                 # FCM push notifications
│   │   └── sidecar/              # Python sidecar management
│   ├── frontend/                 # Svelte 5 + Vite + TailwindCSS
│   │   ├── src/
│   │   │   ├── routes/           # File-based routing (Dashboard, Sources, Postings, CV, Matches, Settings)
│   │   │   ├── components/       # UI + domain components
│   │   │   ├── lib/              # Stores, API, types, utils
│   │   │   └── styles/           # Global styles
│   │   └── static/               # Icons, fonts
│   ├── sidecar/                  # Python Playwright scrapers (bundled)
│   ├── migrations/               # SQLx migrations
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
├── templates/cv/                 # Typst CV templates (modern.typ, classic.typ, minimal.typ)
├── specs/SPEC.md                 # Complete specification
├── docs/agents/                  # Skill configurations
└── CLAUDE.md                     # Agent instructions
```

## Quick Start

### Prerequisites

- **Rust** 1.75+ (`rustup install stable`)
- **Node.js** 20+ (`nvm install 20`)
- **Python** 3.10+ (for Playwright sidecar)
- **System dependencies** (Linux):
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev \
    libssl-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev patchelf
  ```

### Development

```bash
# Clone and enter
cd tunisia-career-scout

# Install frontend deps
cd app/frontend && npm install

# Install Python sidecar deps
cd ../sidecar/python-scrapers && pip install -r requirements.txt
playwright install chromium

# Run in development mode (from app/)
cd ../..
cargo tauri dev
```

### Build for Production

```bash
# Desktop (Linux/macOS/Windows)
cargo tauri build

# Android
rustup target add aarch64-linux-android
cargo tauri android init
cargo tauri build --target aarch64-linux-android
```

## Configuration

### Environment Variables

Create `.env` in `app/`:

```env
# Database (SQLite - auto-managed)
DATABASE_URL=sqlite://./app.db?mode=rwc

# LLM (choose one)
LLM_PROVIDER=ollama          # ollama, openai, openai-compatible, llama-cpp
LLM_MODEL=llama3.2
LLM_BASE_URL=http://localhost:11434
LLM_API_KEY=                 # Required for OpenAI

# Gmail OAuth (for job alert emails)
GMAIL_CLIENT_ID=your-client-id
GMAIL_CLIENT_SECRET=your-secret
GMAIL_REDIRECT_URI=http://localhost:1420/auth/gmail/callback

# FCM Push (Android)
FCM_SERVER_KEY=your-fcm-server-key
FCM_PROJECT_ID=your-firebase-project-id
```

### Source Types

| Type | Description | Auth |
|------|-------------|------|
| **URL** | Any webpage with job listings | Optional headers |
| **RSS** | RSS/Atom feeds | None |
| **Gmail** | Job alert emails via Gmail API | OAuth2 |
| **Social** | Facebook Groups, TikTok hashtags, LinkedIn jobs | Cookies (captured via browser) |

## Development Workflow

### Project Structure (Key Files)

| Area | Files |
|------|-------|
| **Backend Commands** | `app/src/commands/*.rs` |
| **Database** | `app/src/db/repositories/*.rs`, `app/migrations/*.sql` |
| **Scrapers** | `app/src/scrapers/*.rs`, `app/src/scrapers/adapters/*.rs` |
| **Frontend Routes** | `app/frontend/src/routes/*.svelte` |
| **Frontend State** | `app/frontend/src/lib/stores.ts` |
| **Shared Types** | `app/frontend/src/lib/types.ts` |
| **CV Templates** | `templates/cv/*.typ` |
| **Python Sidecar** | `app/sidecar/python-scrapers/src/` |

### Adding a New Source Adapter

1. Create `app/src/scrapers/adapters/new_source.rs` implementing `SourceAdapter`
2. Add to `app/src/scrapers/adapters/mod.rs`
3. Register in `app/src/commands/source.rs::trigger_source_scan`

### Adding a CV Template

1. Create `templates/cv/new_template.typ`
2. Add to `app/src/cv/templates.rs`
3. Add `CVTemplate::NewTemplate` variant to `app/src/models/cv.rs`

## Testing

```bash
# Rust unit/integration tests
cd app && cargo test

# Frontend unit tests
cd app/frontend && npm test

# E2E tests
cd app/frontend && npx playwright test
```

## Deployment

### Android Play Store

1. Generate keystore: `keytool -genkey -v -keystore release.keystore -alias release -keyalg RSA -keysize 2048 -validity 10000`
2. Configure `app/src-tauri/gen/android/app/build.gradle.kts` with signing config
3. Build AAB: `cargo tauri build --target aarch64-linux-android -- --aab`
4. Upload to Play Console

### Linux (AppImage/Flatpak)

Tauri generates AppImage automatically with `cargo tauri build`.

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Follow the existing code style (run `cargo fmt` and `npm run lint`)
4. Write tests for new functionality
5. Submit a PR

## Acknowledgments

- **Tauri** - Cross-platform desktop/mobile framework
- **Svelte 5** - Reactive UI framework
- **Typst** - Modern typesetting for PDF generation
- **SQLx** - Compile-time checked SQL
- **Playwright** - Reliable web automation