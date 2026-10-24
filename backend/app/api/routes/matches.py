"""
Matches routes: computing and listing matching results.
Also handles resume upload and summary generation.
"""

from fastapi import APIRouter, Depends, HTTPException, status, UploadFile, File, Form, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload
from tempfile import NamedTemporaryFile
import os, tempfile

from ...core.db import get_db
from ...core.dependencies import get_current_user
from ...models.user import User
from ...models.posting import MatchResult
from ...models.resume import Resume
from ...services.matching import compute_matches
from ...services.summary import generate_summary

router = APIRouter()


@router.get("")
async def list_matches(
    limit: int = Query(50, ge=1, le=200),
    offset: int = Query(0, ge=0, le=200),
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """List ranked matches for the current user."""
    result = await db.execute(
        select(MatchResult)
        .where(MatchResult.user_id == current_user.id)
        .order_by(MatchResult.score.desc())
        .limit(limit)
        .offset(offset)
    )
    matches = result.scalars().all()
    return [MatchResultResponse.model_validate(m) for m in matches]


@router.get("/{match_id}")
async def get_match(
    match_id: str,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Get a single match result."""
    result = await db.execute(
        select(MatchResult)
        .where(MatchResult.id == match_id, MatchResult.user_id == current_user.id)
    )
    match = result.scalar_one_or_none()
    if match is None:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Match not found",
        )
    return MatchResultResponse.model_validate(match)


@router.post("/recompute")
async def recompute_all_matches(
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """
    Recompute all matches for the current user against all active postings.
    Returns the updated ranked list.
    """
    # Gather all active job + concours postings
    job_result = await db.execute(
        select(JobPosting).where(JobPosting.status == "active")
    )
    jobs = job_result.scalars().all()

    concour_result = await db.execute(
        select(ConcoursPosting).where(ConcoursPosting.status == "open")
    )
    concours = concour_result.scalars().all()

    # Get user's resume
    result = await db.execute(
        select(Resume).where(Resume.user_id == current_user.id, Resume.status == "parsed")
    )
    resume_records = result.scalars().all()

    if not resume_records:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="No parsed resume found. Upload a CV first.",
        )

    resume_data = resume_records[0].parsed

    # Compute matches
    results = compute_matches(
        resume_data=resume_data,
        job_postings=jobs,
        concours_postings=concours,
        user_id=current_user.id,
    )

    # Delete old matches and insert new
    await db.execute(select(MatchResult).where(MatchResult.user_id == current_user.id))
    await db.commit()

    return {
        "message": f"Recomputed {len(results)} matches",
        "count": len(results),
        "ranked": results,
    }


@router.post("/upload")
async def upload_resume(
    file: UploadFile = File(...),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Upload a CV/PDF for the current user.
    Temporary: store in temp dir, parse, then save.
    """
    from ...services.resume_parser import parse_resume

    # Validate user (current authenticated user)
    user = current_user

    # Save to temp file
    temp_path = None
    try:
        contents = await file.read()
        if not contents:
            raise HTTPException(status_code=400, detail="Empty file")

        suffix = os.path.splitext(file.filename or "resume")[1] or ".pdf"
        with tempfile.NamedTemporaryFile(delete=False, suffix=suffix, dir="/tmp") as f:
            f.write(contents)
            temp_path = f.name

        # Parse the resume
        parsed = parse_resume(temp_path)

        # Save to DB
        db_resume = Resume(
            user_id=user.id,
            parsed=parsed,
            file_name=file.filename or "resume",
            file_path=temp_path,
            mime_type=file.content_type or "application/pdf",
            status="parsed",
            parsed_at=None,
        )
        db.add(db_resume)
        await db.commit()
        await db.refresh(db_resume)

        return {
            "success": True,
            "data": {
                "id": str(db_resume.id),
                "status": db_resume.status,
                "parsed_at": db_resume.parsed_at,
                "parsed_data": db_resume.parsed,
            },
        }
    except Exception as e:
        # Clean up temp file
        if temp_path and os.path.exists(temp_path):
            os.unlink(temp_path)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to process resume: {str(e)}",
        )
