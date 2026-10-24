"""
Seed the database with default posting sources.
"""

import asyncio
import uuid
from datetime import datetime, timezone

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker
from sqlalchemy import func

from app.core.db import engine, AsyncSessionLocal
from app.models.posting import PostingSource

# Default sources for Tunisia
DEFAULT_SOURCES = [
    {
        "name": "ANETI",
        "source_type": "official",
        "url": "https://www.emploi.nat.tn/",
        "schedule": "1d",
        "interval_minutes": 1440,
        "enabled": True,
        "parser_name": "aneti",
        "notes": "Agence Nationale pour l'Emploi et le Travail Indépendant",
    },
    {
        "name": "ANETI Appels à Candidatures",
        "source_type": "official",
        "url": "https://candidatures.aneti.tn/",
        "schedule": "1d",
        "interval_minutes": 1440,
        "enabled": True,
        "parser_name": "aneti_candidatures",
        "notes": "ANETI platform for job calls",
    },
    {
        "name": "Tunisie Concours",
        "source_type": "commercial",
        "url": "https://tunisieconcours.tn/",
        "schedule": "12h",
        "interval_minutes": 720,
        "enabled": True,
        "parser_name": "tunisieconcours",
        "notes": "Daily updated concours and job postings",
    },
    {
        "name": "Tunisie Travail",
        "source_type": "commercial",
        "url": "https://www.tunisietravail.net/",
        "schedule": "12h",
        "interval_minutes": 720,
        "enabled": True,
        "parser_name": "tunisietravail",
        "notes": "Job offers and concours",
    },
    {
        "name": "TunisieJob",
        "source_type": "commercial",
        "url": "https://tunisiejob.tn/",
        "schedule": "1d",
        "interval_minutes": 1440,
        "enabled": True,
        "parser_name": "tunisiejob",
        "notes": "Recruitment platform",
    },
    {
        "name": "Emploitunisie",
        "source_type": "commercial",
        "url": "https://www.emploitunisie.com/",
        "schedule": "12h",
        "interval_minutes": 720,
        "enabled": True,
        "parser_name": "emploitunisie",
        "notes": "Job offers in Tunisia",
    },
    {
        "name": "Concours Finance",
        "source_type": "official",
        "url": "https://concours.finances.gov.tn/",
        "schedule": "1d",
        "interval_minutes": 1440,
        "enabled": True,
        "parser_name": "concours_finances",
        "notes": "Ministry of Finance concours announcements",
    },
    {
        "name": "Facebook - ANETI",
        "source_type": "social",
        "url": "https://www.facebook.com/ANETI.EMPLOI.NAT.TN/",
        "schedule": "6h",
        "interval_minutes": 360,
        "enabled": True,
        "parser_name": "facebook_aneti",
        "notes": "ANETI Facebook page for job postings",
    },
    {
        "name": "Facebook - Concours Gov",
        "source_type": "social",
        "url": "https://www.facebook.com/concours.gouv.tn/",
        "schedule": "6h",
        "interval_minutes": 360,
        "enabled": True,
        "parser_name": "facebook_concours",
        "notes": "Government concours Facebook page",
    },
    {
        "name": "Facebook - Tunisie Travail",
        "source_type": "social",
        "url": "https://www.facebook.com/tunisietravail/",
        "schedule": "6h",
        "interval_minutes": 360,
        "enabled": True,
        "parser_name": "facebook_tunisietravail",
        "notes": "Tunisie Travail Facebook page",
    },
    {
        "name": "ActuTn",
        "source_type": "commercial",
        "url": "https://actu.tn/",
        "schedule": "12h",
        "interval_minutes": 720,
        "enabled": True,
        "parser_name": "actu_rss",
        "notes": "Tunisia news and job postings (RSS + Playwright)",
    },
    {
        "name": "ANETI International",
        "source_type": "official",
        "url": "https://aneti-international.tn/",
        "schedule": "1d",
        "interval_minutes": 1440,
        "enabled": True,
        "parser_name": "aneti",
        "notes": "ANETI International job postings",
    },
]


async def seed_sources():
    """Seed the database with default sources."""
    async with AsyncSessionLocal() as session:
        # Check if sources already exist
        result = await session.execute(select(PostingSource))
        existing = result.scalars().all()
        
        if existing:
            print(f"Found {len(existing)} existing sources, skipping seed")
            return
        
        # Create default sources
        for src in DEFAULT_SOURCES:
            source = PostingSource(
                id=uuid.uuid4(),
                name=src["name"],
                source_type=src["source_type"],
                url=src["url"],
                schedule=src["schedule"],
                interval_minutes=src["interval_minutes"],
                enabled=src["enabled"],
                parser_name=src["parser_name"],
                notes=src["notes"],
            )
            session.add(source)
        
        await session.commit()
        print(f"Seeded {len(DEFAULT_SOURCES)} sources")


if __name__ == "__main__":
    asyncio.run(seed_sources())
