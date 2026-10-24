"""
Configuration management via Pydantic Settings.
Loads from environment variables (Docker) or .env file.
"""

from pydantic_settings import BaseSettings
from pydantic import Field
from functools import lru_cache


class Settings(BaseSettings):
    # App
    APP_NAME: str = "Tunisia Career Scout"
    APP_ENV: str = "development"
    DEBUG: bool = Field(default=True, description="Debug mode")

    # Database
    DATABASE_URL: str = "postgresql+asyncpg://postgres:postgres@localhost:5432/tunisia_career_scout"

    # Auth
    SECRET_KEY: str = Field(
        default="change-me-in-production-please-use-a-32-byte-random-key",
        description="JWT secret key (use a strong random key in production)",
    )
    JWT_ALGORITHM: str = "HS256"
    JWT_EXPIRATION_MINUTES: int = 60 * 24 * 7  # 7 days

    # Scrapers
    SCRAPING_LOG_FILE: str = "logs/scraping.log"
    SCRAPER_USER_AGENT: str = "TunisiaCareerScout/1.0 (+https://tunisia-career-scout.local)"

    # LLM
    LLM_PROVIDER: str = "ollama"  # "ollama" or "openai"
    LLM_BASE_URL: str = "http://localhost:11434"
    LLM_MODEL: str = "llama3.2"
    LLM_API_KEY: str = ""
    LLM_TEMPERATURE: float = 0.3

    # Redis
    REDIS_URL: str = "redis://localhost:6379/0"

    # Facebook OAuth
    FACEBOOK_APP_ID: str = ""
    FACEBOOK_APP_SECRET: str = ""
    FACEBOOK_REDIRECT_URI: str = "http://localhost:3000/api/auth/facebook/callback"

    # CORS
    FRONTEND_URL: str = "http://localhost:3000"

    class Config:
        env_file = ".env"
        case_sensitive = True
        extra = "allow"


@lru_cache
def get_settings() -> Settings:
    return Settings()
