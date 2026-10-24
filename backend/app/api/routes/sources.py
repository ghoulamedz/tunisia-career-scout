"""
Sources routes — registry of posting sources.
"""

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ...core.db import get_db
from ...models.posting import PostingSource
from ...schemas.posting import SourceResponse

router = APIRouter()


@router.get("")
async def list_sources(db: AsyncSession = Depends(get_db)):
    """List all registered posting sources."""
    result = await db.execute(select(PostingSource).order_by(PostingSource.created_at))
    sources = result.scalars().all()
    return [SourceResponse.model_validate(s) for s in sources]


@router.post("")
async def create_source(
    name: str,
    source_type: str,
    url: str,
    schedule: str = "1d",
    enabled: bool = True,
    parser_name: str = "default",
    db: AsyncSession = Depends(get_db),
):
    """Register a new posting source."""
    from ...models.posting import PostingSource
    from ...schemas.posting import SourceResponse

    # Check uniqueness
    check = await db.execute(select(PostingSource).where(PostingSource.name == name))
    if check.scalar_one_or_none():
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="A source with this name already exists",
        )

    source = PostingSource(
        name=name,
        source_type=source_type,
        url=url,
        schedule=schedule,
        enabled=enabled,
        parser_name=parser_name,
    )
    db.add(source)
    await db.commit()
    await db.refresh(source)
    return SourceResponse.model_validate(source)
