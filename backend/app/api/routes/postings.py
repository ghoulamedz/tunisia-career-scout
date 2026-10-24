"""
Postings routes: listing, detail, sources, sync.
"""

from fastapi import APIRouter, Depends, HTTPException, status, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ...core.db import get_db
from ...models.posting import (
    JobPosting, ConcoursPosting, PostingSource,
)
from ...schemas.posting import (
    JobPostingResponse, ConcoursPostingResponse, SourceResponse,
)


async def _sync_endpoint():
    """Trigger a sync of all sources."""
    from ...workers.scrapers.pipeline import sync_all_sources
    return await sync_all_sources()

router = APIRouter()


def get_db_session():
    return get_db


@router.get("", response_model=list[JobPostingResponse])
async def list_job_postings(
    source: str | None = Query(None, description="Filter by source name"),
    status: str = Query("active", description="Only active postings"),
    db: AsyncSession = Depends(get_db),
):
    """List active job postings."""
    query = select(JobPosting).where(JobPosting.status == status)
    if source:
        result = await db.execute(
            select(PostingSource).where(PostingSource.name == source)
        )
        source_obj = result.scalar_one_or_none()
        if source_obj:
            query = query.where(JobPosting.source_id == source_obj.id)
    result = await db.execute(query)
    postings = result.scalars().all()

    # Get source names for each posting
    postings_with_origin = []
    for p in postings:
        item = JobPostingResponse.model_validate(p)
        # Source name from raw data
        if p.raw and isinstance(p.raw, dict):
            item.source_name = p.raw.get("source_name")
            item.source_url = p.raw.get("source_url")
        postings_with_origin.append(item)
    return postings_with_origin


@router.get("/concours", response_model=list[ConcoursPostingResponse])
async def list_concours_postings(
    source: str | None = Query(None, description="Filter by source name"),
    status: str = Query("open", description="Only open postings"),
    db: AsyncSession = Depends(get_db),
):
    """List open concours postings."""
    query = select(ConcoursPosting).where(ConcoursPosting.status == status)
    if source:
        result = await db.execute(
            select(PostingSource).where(PostingSource.name == source)
        )
        source_obj = result.scalar_one_or_none()
        if source_obj:
            query = query.where(ConcoursPosting.source_id == source_obj.id)
    result = await db.execute(query)
    postings = result.scalars().all()

    postings_with_origin = []
    for p in postings:
        item = ConcoursPostingResponse.model_validate(p)
        if p.raw and isinstance(p.raw, dict):
            item.source_name = p.raw.get("source_name")
            item.source_url = p.raw.get("source_url")
        postings_with_origin.append(item)
    return postings_with_origin


@router.get("/sources", response_model=list[SourceResponse])
async def list_sources(db: AsyncSession = Depends(get_db)):
    """List all registered posting sources."""
    result = await db.execute(select(PostingSource).order_by(PostingSource.created_at))
    sources = result.scalars().all()
    return [SourceResponse.model_validate(s) for s in sources]


@router.get("/job-types")
async def get_job_types():
    """Get available job type filter options."""
    return [
        {"value": "emploi", "label": "Emploi (CDI)"},
        {"value": "alternance", "label": "Alternance"},
        {"value": "stage", "label": "Stage"},
        {"value": "cdi", "label": "CDI"},
        {"value": "cdd", "label": "CDD"},
    ]


@router.get("/concours-types")
async def get_concours_types():
    """Get available concours type filter options."""
    return [
        {"value": "interne", "label": "Interne"},
        {"value": "externe", "label": "Externe"},
        {"value": "nationale", "label": "Nationale"},
    ]


@router.post("/sync", response_model=dict)
async def sync_postings():
    """Trigger a sync of all sources."""
    return await _sync_endpoint()
