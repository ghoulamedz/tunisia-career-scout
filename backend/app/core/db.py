"""
Database setup: SQLAlchemy async engine, session, and base.
"""

import os
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession
from sqlalchemy.orm import DeclarativeBase

from .config import get_settings

settings = get_settings()

# Create async engine
# Use asyncpg for PostgreSQL
SQLALCHEMY_DATABASE_URL = settings.DATABASE_URL

# Convert postgresql:// to postgresql+asyncpg:// for async
if SQLALCHEMY_DATABASE_URL.startswith("postgresql://"):
    SQLALCHEMY_DATABASE_URL = SQLALCHEMY_DATABASE_URL.replace("postgresql://", "postgresql+asyncpg://", 1)

engine = create_async_engine(
    SQLALCHEMY_DATABASE_URL,
    pool_size=20,
    max_overflow=10,
    pool_timeout=30,
    pool_recycle=3600,
    echo=settings.DEBUG,
)

AsyncSessionLocal = async_sessionmaker(
    engine,
    class_=AsyncSession,
    expire_on_commit=False,
    autoflush=False,
    autocommit=False,
)


async def get_db() -> AsyncSession:
    """Dependency: yield a DB session, close after use."""
    async with AsyncSessionLocal() as session:
        yield session


async def init_db():
    """Create all tables (call once at startup)."""
    from ..core.base import Base as base  # noqa: F401
    async with engine.begin() as conn:
        await conn.run_sync(base.metadata.create_all)


async def close_db():
    """Close connections at shutdown."""
    await engine.dispose()
