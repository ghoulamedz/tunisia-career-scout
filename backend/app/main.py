"""
FastAPI app with lifespan management.
"""

import time
import logging
from collections import defaultdict
from contextlib import asynccontextmanager
from fastapi import FastAPI, Request, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from starlette.middleware.base import BaseHTTPMiddleware

from .core.db import init_db, close_db
from .core.config import get_settings

# Import models to register them with SQLAlchemy metadata
from .models.user import User
from .models.posting import PostingSource, JobPosting, ConcoursPosting, MatchResult, Notification, AISummary
from .models.resume import Resume

settings = get_settings()
logger = logging.getLogger(__name__)

app = FastAPI(
    title=settings.APP_NAME,
    description="Tunisia Career Scout — scraping + matching + summarization",
    version="0.1.0",
)


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    print(f"DB connected: {settings.DATABASE_URL}")
    yield
    await close_db()


app.add_middleware(
    CORSMiddleware,
    allow_origins=[settings.FRONTEND_URL],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


class RateLimitMiddleware(BaseHTTPMiddleware):
    """Simple rate limiting middleware."""

    def __init__(self, app):
        super().__init__(app)
        self.requests: dict[str, list[float]] = defaultdict(list)
        self.max_requests = 100  # per minute
        self.window = 60.0

    async def dispatch(self, request: Request, call_next):
        client_ip = request.client.host if request.client else "unknown"
        now = time.time()

        # Clean old entries
        self.requests[client_ip] = [
            t for t in self.requests[client_ip]
            if now - t < self.window
        ]

        if len(self.requests[client_ip]) >= self.max_requests:
            raise HTTPException(
                status_code=429,
                detail="Rate limit exceeded. Try again later.",
            )

        self.requests[client_ip].append(now)
        return await call_next(request)


app.add_middleware(RateLimitMiddleware)

from .api.routes import api_router  # noqa: F401

app.include_router(api_router, prefix="/api")


@app.get("/robots.txt")
async def robots_txt():
    """Return robots.txt for scraping compliance."""
    return {
        "User-agent": "*",
        "Allow": "/api/",
        "Disallow": ["/admin/", "/internal/"],
        "Crawl-delay": "1",
    }


@app.get("/security-headers")
async def security_headers():
    """Return security header recommendations."""
    return {
        "X-Content-Type-Options": "nosniff",
        "X-Frame-Options": "DENY",
        "X-XSS-Protection": "1; mode=block",
        "Strict-Transport-Security": "max-age=31536000; includeSubDomains",
        "Content-Security-Policy": "default-src 'self'",
    }


@app.get("/health")
async def health_check():
    return {"status": "ok", "app": settings.APP_NAME}
