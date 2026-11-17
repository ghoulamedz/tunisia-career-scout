# Architecture Decision Record Log

Index of all ADRs for Tunisia Career Scout. Each ADR documents a hard-to-reverse decision with genuine trade-offs.

---

## ADR Index

| ID | Title | Status | Date |
|----|-------|--------|------|
| 0001 | Local-First Tauri v2 Architecture | Accepted | 2025-11-16 |
| 0002 | SQLite (SQLx) for All Platforms | Accepted | 2025-11-16 |
| 0003 | Typst for CV PDF Generation | Accepted | 2025-11-16 |
| 0004 | llm-chain for LLM Provider Abstraction | Accepted | 2025-11-16 |
| 0005 | Python Sidecar for Playwright Social Scraping | Accepted | 2025-11-16 |
| 0006 | FCM for Android Push Notifications | Accepted | 2025-11-16 |
| 0007 | Two-Pass Extraction (Rules + LLM) | Accepted | 2025-11-16 |
| 0008 | Svelte 5 + Vite + TailwindCSS Frontend | Accepted | 2025-11-16 |
| 0009 | Repository Pattern for Data Access | Accepted | 2025-11-16 |
| 0010 | JSON-RPC for Tauri↔Python Communication | Accepted | 2025-11-16 |

---

## ADR Template

When creating a new ADR, use this format:

```markdown
# ADR-XXXX: <Title>

**Status**: Proposed | Accepted | Superseded | Deprecated
**Date**: YYYY-MM-DD
**Deciders**: <names>
**Technical Story**: <link to issue/ticket>

## Context
What is the issue that we're seeing that is motivating this decision?

## Decision
What is the change that we're proposing and/or doing?

## Consequences
### Positive
- Benefit 1
- Benefit 2

### Negative
- Drawback 1
- Drawback 2

### Risks
- Risk 1
- Risk 2

## Alternatives Considered
1. **Alternative A** — Why not chosen
2. **Alternative B** — Why not chosen

## References
- Link to discussion
- Link to related ADRs
```

---

## ADR-0001: Local-First Tauri v2 Architecture

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need a cross-platform app (Linux desktop + Android) that runs entirely on-device for privacy and residential IP scraping. No cloud backend desired for MVP.

## Decision
Use Tauri v2 with Rust backend + Svelte 5 frontend. Tauri provides:
- Native binaries for Linux/macOS/Windows
- APK/AAB for Android via `cargo tauri android build`
- Shared Rust codebase across platforms
- WebView frontend (system WebView on Android, WebView2 on Windows, WebKit on Linux/macOS)

## Consequences
### Positive
- Single codebase for desktop + mobile
- Rust for performance-critical scraping/matching
- Svelte 5 for excellent DX and small bundles
- No Electron bloat (~3-5MB binary vs ~100MB)
- System WebView on Android (evergreen, small APK)

### Negative
- Android WebView limitations (no background JS execution)
- Requires Rust learning curve
- Python sidecar needed for Playwright (no native Rust Playwright)
- Background tasks need native implementation (tokio-cron + Android foreground service)

### Risks
- Tauri v2 mobile is relatively new (but stabilizing)
- Android foreground service complexity for background scraping
- Python sidecar binary size (~150MB with Chromium)

---

## ADR-0002: SQLite (SQLx) for All Platforms

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need embedded database that works identically on Linux desktop and Android. No PostgreSQL on mobile.

## Decision
Use SQLite via SQLx with compile-time checked queries. Database file stored in platform-appropriate app data directory.

## Consequences
### Positive
- Zero-config, single file, works everywhere
- SQLx compile-time verification catches schema drift
- WAL mode for concurrent reads
- Same file format across platforms (enables sync later)
- No separate database process

### Negative
- Limited concurrent write throughput (but single-user app)
- No native JSONB indexing (SQLite JSON functions sufficient)
- Migration tooling less mature than PostgreSQL

### Risks
- SQLite on Android with WAL mode needs testing under storage pressure
- Large raw_html columns may bloat database (consider separate blob storage later)

---

## ADR-0003: Typst for CV PDF Generation

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need high-quality PDF output from CV templates. WeasyPrint is Python-only; want pure Rust.

## Decision
Use Typst (modern typesetting system) with templates in `templates/cv/*.typ`. Compile via `typst::compile()`.

## Consequences
### Positive
- Pure Rust, no Python dependency
- Fast compilation, excellent typography
- Templateable with `#let` functions
- Small binary size
- Active development, growing ecosystem

### Negative
- Learning curve for Typst syntax
- Less mature than LaTeX/WeasyPrint for complex layouts
- Fewer third-party templates available

### Risks
- Typst API may have breaking changes (pin version)
- Complex CV layouts (multi-column, advanced tables) may be challenging

---

## ADR-0004: llm-chain for LLM Provider Abstraction

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need configurable LLM providers (OpenAI, Ollama, llama.cpp, OpenAI-compatible) for extraction and matching.

## Decision
Use `llm-chain` crate for provider abstraction with executor pattern.

## Consequences
### Positive
- Single interface for multiple providers
- Supports OpenAI, Ollama, llama.cpp, OpenAI-compatible
- Async executor pattern fits Tauri commands
- Extensible for custom providers

### Negative
- Early-stage crate, API may change
- Less battle-tested than Python equivalents
- Some provider-specific features not exposed

### Risks
- May need to fork or contribute upstream for missing features
- Streaming responses not yet needed but may be later

---

## ADR-0005: Python Sidecar for Playwright Social Scraping

**Status**: Accepted
**Date**: 2025-11-16

## Context
Facebook, TikTok, LinkedIn require JavaScript rendering and have anti-bot measures. No mature Rust Playwright binding.

## Decision
Bundle Python + Playwright + Chromium as Tauri sidecar. Communicate via JSON-RPC over stdin/stdout.

## Consequences
### Positive
- Leverages mature Playwright ecosystem
- playwright-stealth for anti-detection
- User-assisted login (capture cookies after manual auth)
- Isolated from Rust process (crashes don't kill app)

### Negative
- Large binary size (~150MB with Chromium)
- Python startup latency (~2-3s cold start)
- Two runtimes to manage (Rust + Python)
- Sidecar distribution complexity (PyInstaller or bundled venv)

### Risks
- Playwright updates may break selectors
- Android sidecar distribution untested (may need separate APK or dynamic download)
- Memory usage with Chromium + Rust + WebView

---

## ADR-0006: FCM for Android Push Notifications

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need push notifications on Android for high-match job alerts. Self-hosted push (UnifiedPush, ntfy) requires server.

## Decision
Use Firebase Cloud Messaging (FCM) via `fcm` crate or direct HTTP. User provides their own Firebase project.

## Consequences
### Positive
- Reliable, free tier generous
- Works with Tauri (no Capacitor needed)
- User owns Firebase project (self-hosted friendly)
- Quiet hours, topics, data payloads supported

### Negative
- Requires Google account + Firebase project setup
- Not fully "offline-first" (needs internet for FCM)
- FCM token management (rotation, invalidation)

### Risks
- FCM deprecation risk (but FCM v1 API is stable)
- Android 13+ notification permission runtime request

---

## ADR-0007: Two-Pass Extraction (Rules + LLM)

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need high-quality structured extraction from diverse job sites. Pure LLM is slow/expensive; pure rules miss unstructured sites.

## Decision
Deterministic rules first (schema.org JSON-LD, microdata, ATS heuristics for Greenhouse/Lever/Workable) → on-demand LLM fallback for incomplete results.

## Consequences
### Positive
- Fast, free extraction for 60-70% of structured sites
- LLM only for difficult cases (user-triggered)
- Cost control (user chooses when to use LLM)
- Rules are auditable and debuggable

### Negative
- Rule maintenance burden for new ATS patterns
- LLM prompt engineering for consistent JSON output
- Two code paths to maintain

### Risks
- Schema.org adoption varies by site
- ATS heuristics brittle to UI changes
- LLM hallucination on sparse content

---

## ADR-0008: Svelte 5 + Vite + TailwindCSS Frontend

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need reactive, performant UI with small bundle for WebView. Team prefers Svelte over React/Vue.

## Decision
Svelte 5 (runes) + Vite + TailwindCSS. File-based routing in `src/routes/`.

## Consequences
### Positive
- Excellent DX with runes (`$state`, `$derived`, `$effect`)
- Tiny bundles, no virtual DOM overhead
- TailwindCSS for rapid styling
- Vite HMR fast even in Tauri dev
- Native TypeScript support

### Negative
- Smaller ecosystem than React
- File-based routing less flexible than SvelteKit
- Tauri integration requires manual `invoke()` wrappers

### Risks
- Svelte 5 is new (runes stabilized recently)
- Migration from Svelte 4 patterns if copying code

---

## ADR-0009: Repository Pattern for Data Access

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need clean separation between domain logic and database queries for testability and potential storage backend swap.

## Decision
Each entity gets a Repository (SourceRepository, PostingRepository, CVRepository, etc.) encapsulating all SQLx queries.

## Consequences
### Positive
- Single place for all queries per entity
- Easy to mock in tests
- Clear seam for future storage abstraction
- Consistent CRUD + custom query API

### Negative
- Boilerplate for simple CRUD
- Repository methods can become "god objects" if not careful

### Risks
- Over-abstraction for simple apps
- N+1 query problems if not careful with joins

---

## ADR-0010: JSON-RPC for Tauri↔Python Communication

**Status**: Accepted
**Date**: 2025-11-16

## Context
Need reliable, structured communication between Rust Tauri app and Python Playwright sidecar.

## Decision
Line-delimited JSON over stdin/stdout. Each request/response is a single JSON object.

## Consequences
### Positive
- Simple, language-agnostic
- Easy to debug (log stdout/stderr)
- Streaming friendly for long-running scrapes
- No external dependencies (gRPC, etc.)

### Negative
- No built-in schema validation (use serde)
- Manual correlation ID handling for concurrent requests
- Stdout must be clean (logging to stderr only)

### Risks
- Deadlocks if both sides write without reading
- Encoding issues with non-UTF8 output
- Process management complexity (startup, crashes, restart)