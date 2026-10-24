"""
Posting models: job postings + concours (competitive exams).
Includes source registry and matching results.
"""

import uuid
from datetime import datetime, timezone
from sqlalchemy import String, Boolean, DateTime, Text, ForeignKey, Integer, JSON
from sqlalchemy.dialects.postgresql import UUID
from sqlalchemy.orm import Mapped, mapped_column
from sqlalchemy.ext.mutable import MutableDict, MutableList
from ..core.base import Base


class PostingSource(Base):
    """Registry of data sources (official, commercial, social)."""

    __tablename__ = "posting_sources"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    name: Mapped[str] = mapped_column(String(200), nullable=False, index=True)
    source_type: Mapped[str] = mapped_column(
        String(50), nullable=False
    )  # official | commercial | social
    url: Mapped[str] = mapped_column(String(1000), nullable=False)
    auth_type: Mapped[str] = mapped_column(
        String(50), default="none"
    )  # none | bearer | cookie
    auth_token: Mapped[str | None] = mapped_column(Text, nullable=True)
    schedule: Mapped[str] = mapped_column(
        String(50), default="1d", nullable=False
    )  # 1h, 6h, 12h, 1d
    interval_minutes: Mapped[int] = mapped_column(Integer, default=1440)
    enabled: Mapped[bool] = mapped_column(Boolean, default=True)
    parser_name: Mapped[str] = mapped_column(String(100), default="default")
    notes: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )


class JobPosting(Base):
    """Job / employment posting."""

    __tablename__ = "job_postings"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    source_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("posting_sources.id"), index=True
    )
    source_posting_id: Mapped[str] = mapped_column(
        String(500), index=True, nullable=False
    )  # external source ID for dedup
    title: Mapped[str] = mapped_column(String(500), nullable=False)
    description: Mapped[str] = mapped_column(Text, nullable=False)
    company: Mapped[str] = mapped_column(String(300), nullable=False)
    location: Mapped[str | None] = mapped_column(String(300), nullable=True)
    job_type: Mapped[str] = mapped_column(
        String(50), default="emploi", nullable=False
    )  # emploi | alternance | stage | cdi | cdd | autre
    raw: Mapped[dict] = mapped_column(
        MutableDict.as_mutable(JSON), default=dict, nullable=False
    )
    status: Mapped[str] = mapped_column(String(50), default="active", nullable=False)
    closed_at: Mapped[datetime | None] = mapped_column(
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


class ConcoursPosting(Base):
    """Concours / competitive exam posting."""

    __tablename__ = "concours_postings"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    source_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("posting_sources.id"), index=True
    )
    source_posting_id: Mapped[str] = mapped_column(
        String(500), index=True, nullable=False
    )
    title: Mapped[str] = mapped_column(String(500), nullable=False)
    description: Mapped[str] = mapped_column(Text, nullable=False)
    organization: Mapped[str] = mapped_column(String(300), nullable=False)
    exam_type: Mapped[str] = mapped_column(
        String(50), default="externe", nullable=False
    )  # interne | externe | nationale
    required_diploma: Mapped[str | None] = mapped_column(String(200), nullable=True)
    required_level: Mapped[str | None] = mapped_column(String(100), nullable=True)
    speciality: Mapped[str | None] = mapped_column(String(300), nullable=True)
    age_limit: Mapped[int | None] = mapped_column(Integer, nullable=True)
    raw: Mapped[dict] = mapped_column(
        MutableDict.as_mutable(JSON), default=dict, nullable=False
    )
    status: Mapped[str] = mapped_column(String(50), default="open", nullable=False)
    closed_at: Mapped[datetime | None] = mapped_column(
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


class MatchResult(Base):
    """Matching result between a user's profile and a posting."""

    __tablename__ = "matches"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id"), index=True, nullable=False
    )
    # Flexible reference (polymorphic)
    posting_id: Mapped[uuid.UUID] = mapped_column(UUID(as_uuid=True), nullable=False)
    posting_type: Mapped[str] = mapped_column(String(50), nullable=False)  # job_posting | concours_posting
    score: Mapped[float] = mapped_column(
        "score", nullable=False
    )  # 0-100
    matched_skills: Mapped[list[str]] = mapped_column(
        MutableList.as_mutable(JSON), default=list, nullable=False
    )
    reasons: Mapped[list[str]] = mapped_column(
        MutableList.as_mutable(JSON), default=list, nullable=False
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )


class Notification(Base):
    """User notifications."""

    __tablename__ = "notifications"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id"), index=True, nullable=False
    )
    type: Mapped[str] = mapped_column(
        String(50), nullable=False
    )  # new_posting | summary | match_score
    title: Mapped[str] = mapped_column(String(300), nullable=False)
    body: Mapped[str] = mapped_column(Text, nullable=False)
    link: Mapped[str | None] = mapped_column(String(1000), nullable=True)
    read_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )


class AISummary(Base):
    """LLM-generated summary for a posting matched to a user."""

    __tablename__ = "ai_summaries"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id"), index=True, nullable=False
    )
    posting_id: Mapped[uuid.UUID] = mapped_column(UUID(as_uuid=True), nullable=False)
    posting_type: Mapped[str] = mapped_column(String(50), nullable=False)
    summary: Mapped[str] = mapped_column(Text, nullable=False)
    required_documents: Mapped[list[str]] = mapped_column(
        MutableList.as_mutable(JSON), default=list, nullable=False
    )
    relevance: Mapped[float] = mapped_column(
        "relevance", nullable=False
    )  # 0-100
    is_relevant: Mapped[bool] = mapped_column(Boolean, default=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc),
    )
