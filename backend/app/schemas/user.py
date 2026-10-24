"""
Pydantic schemas for User and Resume.
"""

import uuid
from datetime import datetime
from typing import Optional
from pydantic import BaseModel, EmailStr, Field, field_validator


# ---------- Auth ----------

class UserRegister(BaseModel):
    email: EmailStr
    password: str = Field(min_length=8, max_length=128)
    full_name: str = Field(default="", max_length=255)
    country: str = Field(default="Tunisia", max_length=100)


class UserLogin(BaseModel):
    email: EmailStr
    password: str


class UserResponse(BaseModel):
    id: str
    email: str
    full_name: str
    country: str
    created_at: datetime

    @field_validator("id", mode="before")
    @classmethod
    def serialize_uuid(cls, v):
        if isinstance(v, uuid.UUID):
            return str(v)
        return v

    class Config:
        from_attributes = True


class TokenResponse(BaseModel):
    access_token: str
    token_type: str
    user: UserResponse


class AuthResponse(BaseModel):
    """Response after login or registration."""
    success: bool
    data: Optional[UserResponse] = None
    token: Optional[str] = None
    error: Optional[str] = None


# ---------- Resume ----------

class ResumeUploadResponse(BaseModel):
    """Response after uploading a resume."""
    success: bool
    data: Optional[dict] = None
    error: Optional[str] = None


class ResumeResponse(BaseModel):
    """Response for resume list."""
    id: str
    user_id: str
    status: str
    file_name: str
    parsed_at: Optional[datetime] = None
    created_at: datetime

    class Config:
        from_attributes = True
