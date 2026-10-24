"""
Pytest configuration for Tunisia Career Scout tests.
"""

import pytest
import asyncio
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.db import engine
from app.core.base import Base


@pytest.fixture(scope="session")
def event_loop():
    """Create an event loop for the test session."""
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    yield loop
    loop.close()


@pytest.fixture(scope="session")
async def db_session():
    """Create a test database session."""
    # Create all tables
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    async with AsyncSessionLocal() as session:
        yield session

    # Clean up
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)