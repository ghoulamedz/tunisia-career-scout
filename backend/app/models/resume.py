"""
Resume model for uploaded CVs.
"""

import uuid
from datetime import datetime, timezone
from sqlalchemy import String, DateTime, Text, LargeBinary
from sqlalchemy.dialects.postgresql import UUID
from sqlalchemy.orm import Mapped, mapped_column
from sqlalchemy.ext.mutable import MutableDict, MutableList
from ..core.base import Base as base


class Resume(base):
    """Uploaded resume / CV."""

    __tablename__ = "resumes"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), nullable=False, index=True
    )
    # Parsed CV data
    parsed: Mapped[dict] = mapped_column(MutableDict.as_mutable(Text), nullable=False, default=dict)
    file_name: Mapped[str] = mapped_column(String(500), nullable=False)
    file_path: Mapped[str] = mapped_column(String(1000), nullable=False)
    file_size: Mapped[int] = mapped_column(DateTime, default=0)
    mime_type: Mapped[str] = mapped_column(String(100), default="application/pdf")
    status: Mapped[str] = mapped_column(
        String(50), default="pending", nullable=False
    )  # pending | parsing | parsed | error
    error_message: Mapped[str | None] = mapped_column(Text, nullable=True)
    parsed_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc),
    )
