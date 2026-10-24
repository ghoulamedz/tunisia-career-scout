"""
Scraping pipeline — orchestrates scrapers and stores results.
"""

import asyncio
import logging

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ...core.db import AsyncSessionLocal
from ...models.posting import PostingSource, JobPosting, ConcoursPosting
from .tunisia_scrapers import (
    AnetiScraper,
    TunisieConcoursScraper,
    TunisieTravailScraper,
    EmploitunisieScraper,
    FacebookScraper,
    ActuTnScraper,
)

logger = logging.getLogger(__name__)

# Source name → Scraper class mapping
SCRAPER_MAP = {
    "ANETI": AnetiScraper,
    "ANETI Appels à Candidatures": AnetiScraper,
    "Tunisie Concours": TunisieConcoursScraper,
    "Tunisie Travail": TunisieTravailScraper,
    "TunisieJob": TunisieTravailScraper,
    "Emploitunisie": EmploitunisieScraper,
    "Concours Finance": AnetiScraper,
    "Facebook - ANETI": FacebookScraper,
    "Facebook - Concours Gov": FacebookScraper,
    "Facebook - Tunisie Travail": FacebookScraper,
    "ActuTn": ActuTnScraper,
    "ANETI International": AnetiScraper,
}


async def sync_source(source: PostingSource, login_url: str = None) -> int:
    """Sync a single source. Returns number of postings found.

    login_url: For Facebook scraper, the URL to open for manual login.
    """
    scraper_class = SCRAPER_MAP.get(source.name)
    if not scraper_class:
        logger.warning(f"No scraper for source: {source.name}")
        return 0

    scraper = scraper_class()
    try:
        # Facebook scraper supports manual login
        if login_url and hasattr(scraper, 'fetch'):
            raw_postings = await scraper.fetch(login_url=login_url)
        else:
            raw_postings = await scraper.fetch()
    except Exception as e:
        logger.error(f"Scraper {source.name} failed: {e}")
        return 0

    if not raw_postings:
        logger.info(f"No postings from {source.name}")
        return 0

    # Store in DB
    count = 0
    async with AsyncSessionLocal() as db:
        for p in raw_postings:
            # Check for duplicates
            existing = await db.execute(
                select(JobPosting).where(
                    JobPosting.source_id == source.id,
                    JobPosting.source_posting_id == p["source_posting_id"],
                )
            )
            if existing.scalar_one_or_none():
                continue

            if p.get("job_type") == "concours":
                db_posting = ConcoursPosting(
                    source_id=source.id,
                    source_posting_id=p["source_posting_id"],
                    title=p["title"],
                    description=p.get("description", ""),
                    organization=p.get("company", ""),
                    exam_type=p.get("exam_type", "externe"),
                    required_diploma=p.get("required_diploma", ""),
                    speciality=p.get("speciality", ""),
                    raw=p.get("raw", {}),
                    status="open",
                )
                db.add(db_posting)
            else:
                db_posting = JobPosting(
                    source_id=source.id,
                    source_posting_id=p["source_posting_id"],
                    title=p["title"],
                    description=p.get("description", ""),
                    company=p.get("company", ""),
                    location=p.get("location", ""),
                    job_type=p.get("job_type", "emploi"),
                    raw=p.get("raw", {}),
                    status="active",
                )
                db.add(db_posting)

            count += 1

        await db.commit()

    logger.info(f"Synced {count} postings from {source.name}")
    return count


async def sync_all_sources() -> dict:
    """Sync all enabled sources. Returns summary."""
    results = {}

    async with AsyncSessionLocal() as db:
        result = await db.execute(
            select(PostingSource).where(PostingSource.enabled == True)
        )
        sources = result.scalars().all()

    for source in sources:
        count = await sync_source(source)
        results[source.name] = count

    total = sum(results.values())
    logger.info(f"Sync complete: {total} total postings from {len(results)} sources")
    return {"total": total, "sources": results}


if __name__ == "__main__":
    asyncio.run(sync_all_sources())
