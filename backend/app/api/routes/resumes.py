"""
Resumes routes — upload and parse CVs.
"""

from fastapi import APIRouter, Depends, HTTPException, status, UploadFile, File, Form
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ...core.db import get_db
from ...core.dependencies import get_current_user
from ...models.user import User
from ...models.resume import Resume
from ...services.resume_parser import parse_resume

router = APIRouter()


@router.post("/upload")
async def upload_resume(
    file: UploadFile = File(...),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Upload a CV/PDF for the current user."""
    from tempfile import NamedTemporaryFile
    import os, tempfile

    # Save to temp file
    temp_path = None
    try:
        contents = await file.read()
        if not contents:
            raise HTTPException(status_code=400, detail="Empty file")

        suffix = os.path.splitext(file.filename or "resume.pdf")[1] or ".pdf"
        with tempfile.NamedTemporaryFile(delete=False, suffix=suffix, dir="/tmp") as f:
            f.write(contents)
            temp_path = f.name

        # Parse the resume
        parsed = parse_resume(temp_path)

        # Save to DB
        db_resume = Resume(
            user_id=current_user.id,
            parsed=parsed,
            file_name=file.filename or "resume.pdf",
            file_path=temp_path,
            mime_type=file.content_type or "application/pdf",
            status="parsed",
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
        if temp_path and os.path.exists(temp_path):
            os.unlink(temp_path)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to process resume: {str(e)}",
        )


@router.get("")
async def list_resumes(
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """List user's resumes."""
    result = await db.execute(
        select(Resume).where(Resume.user_id == current_user.id).order_by(Resume.created_at.desc())
    )
    resumes = result.scalars().all()
    return [
        {
            "id": str(r.id),
            "status": r.status,
            "file_name": r.file_name,
            "parsed_at": r.parsed_at,
            "created_at": r.created_at,
        }
        for r in resumes
    ]
