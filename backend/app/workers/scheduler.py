"""
Scheduler — scheduled scraping + post-processing tasks.
"""

import logging
from datetime import datetime, timezone

from celery import shared_task
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ..core.db import get_db
from ..models.posting import (
    JobPosting, ConcoursPosting, PostingSource,
)
from ..services.matching import compute_matches
from ..models.user import User
from ..models.resume import Resume

logger = logging.getLogger(__name__)


@shared_task
def health_check():
    """Ping all sources for health check."""
    result = {"sources": [], "timestamp": datetime.now(timezone.utc).isoformat()}

    async def _run():
        # In production, this would iterate over DB sources and call scrapers
        # For now, return a placeholder
        return result

    import asyncio
    return asyncio.run(_run())



@shared_task
def sync_all_postings():
    """
    Main periodic task: sync all sources with enabled=True.
    In production, this would iterate over DB sources and call each scraper.
    """
    logger.info("Starting full sync...")

    async def _run():
        async for db in get_db():
            # 1. Gather all enabled sources
            sources_result = await db.execute(
                select(PostingSource).where(PostingSource.enabled == True)
            )
            sources = sources_result.scalars().all()

            # 2. Collect newly discovered postings (in production, call scraper per source)
            # For now, we just return counts
            job_count = await db.scalar(select(JobPosting).where(JobPosting.status == "active"))
            concours_count = await db.scalar(
                select(ConcoursPosting).where(ConcoursPosting.status == "open")
            )

            logger.info(f"DB summary: {job_count} active jobs, {concours_count} open concours")

            return {
                "timestamp": datetime.now(timezone.utc).isoformat(),
                "sources_count": len(sources),
            }

    import asyncio
    result = asyncio.run(_run())

    # 3. Recompute matching for all users
    async for db in get_db():
        # Get all active users
        users_result = await db.execute(select(User).where(User.is_active == True))
        users = users_result.scalars().all()

        for user in users:
            # Get user's parsed resume
            resume_result = await db.execute(
                select(Resume).where(
                    Resume.user_id == user.id, Resume.status == "parsed"
                )
            )
            resume_records = resume_result.scalars().all()

            if not resume_records:
                continue

            resume_data = resume_records[0].parsed

            # Gather all postings
            job_all_result = await db.execute(select(JobPosting).where(JobPosting.status == "active"))
            jobs = job_all_result.scalars().all()

            concour_all_result = await db.execute(
                select(ConcoursPosting).where(ConcoursPosting.status == "open")
            )
            concours = concour_all_result.scalars().all()

            # Compute and store matches
            match_results = compute_matches(
                resume_data=resume_data,
                job_postings=jobs,
                concours_postings=concours,
                user_id=user.id,
            )

            # Delete old matches and store new
            await db.execute(select(JobPosting).where(JobPosting.user_id == user.id))
            await db.commit()

        logger.info("Matching recomputation completed")

    return result
