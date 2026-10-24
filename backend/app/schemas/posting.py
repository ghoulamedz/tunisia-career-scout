"""
Pydantic schemas for postings and matching.
"""

import uuid
from datetime import datetime
from typing import Optional, List
from enum import Enum
from pydantic import BaseModel, Field, field_validator


class JobType(str, Enum):
    emploi = "emploi"
    alternance = "alternance"
    stage = "stage"
    cdi = "cdi"
    cdd = "cdd"
    autre = "autre"


class PostingStatus(str, Enum):
    active = "active"
    closed = "closed"


class ExamType(str, Enum):
    interne = "interne"
    externe = "externe"
    nationale = "nationale"


class SourceType(str, Enum):
    official = "official"
    commercial = "commercial"
    social = "social"


class SourceResponse(BaseModel):
    id: str
    name: str
    source_type: str
    url: str
    schedule: str
    enabled: bool
    parser_name: str

    @field_validator("id", mode="before")
    @classmethod
    def serialize_uuid(cls, v):
        if isinstance(v, uuid.UUID):
            return str(v)
        return v

    class Config:
        from_attributes = True


class JobPostingResponse(BaseModel):
    id: str
    source_id: str
    source_posting_id: str
    title: str
    description: str
    company: str
    location: Optional[str] = None
    job_type: str
    status: str
    created_at: datetime
    source_name: Optional[str] = None
    source_url: Optional[str] = None

    @field_validator("id", "source_id", mode="before")
    @classmethod
    def serialize_uuid(cls, v):
        if isinstance(v, uuid.UUID):
            return str(v)
        return v

    class Config:
        from_attributes = True


class ConcoursPostingResponse(BaseModel):
    id: str
    source_id: str
    source_posting_id: str
    title: str
    description: str
    organization: str
    exam_type: str
    status: str
    location: Optional[str] = None
    created_at: datetime
    source_name: Optional[str] = None
    source_url: Optional[str] = None

    @field_validator("id", "source_id", mode="before")
    @classmethod
    def serialize_uuid(cls, v):
        if isinstance(v, uuid.UUID):
            return str(v)
        return v

    class Config:
        from_attributes = True


class MatchResultResponse(BaseModel):
    id: str
    score: float
    matched_skills: List[str] = []
    reasons: List[str] = []
    created_at: datetime

    @field_validator("id", mode="before")
    @classmethod
    def serialize_uuid(cls, v):
        if isinstance(v, uuid.UUID):
            return str(v)
        return v

    class Config:
        from_attributes = True


class MatchScoreResponse(BaseModel):
    """Response for a posting with a score for the current user."""
    id: str
    title: str
    description: str
    company: str
    score: Optional[float] = None
    matched_skills: Optional[List[str]] = None
    reasons: Optional[List[str]] = None
    is_relevant: Optional[bool] = None

    class Config:
        from_attributes = True


class SummarizeRequest(BaseModel):
    user_profile: dict = Field(default_factory=dict)
    context: str = ""  # "job" or "concours"


class SummarizeResponse(BaseModel):
    success: bool
    data: Optional[dict] = None
    error: Optional[str] = None


class NotificationResponse(BaseModel):
    id: str
    type: str
    title: str
    body: str
    link: Optional[str] = None
    read_at: Optional[datetime] = None
    created_at: datetime

    @field_validator("id", mode="before")
    @classmethod
    def serialize_uuid(cls, v):
        if isinstance(v, uuid.UUID):
            return str(v)
        return v

    class Config:
        from_attributes = True
