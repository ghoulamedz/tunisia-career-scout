"""
Tests for Tunisia Career Scout backend.
Run with: pytest tests/ -v
"""

import pytest
import asyncio
import uuid
from datetime import datetime, timezone

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.db import AsyncSessionLocal, engine
from app.core.base import Base
from app.core.security import (
    get_password_hash,
    verify_password,
    create_access_token,
    decode_access_token,
)
from app.models.user import User
from app.models.posting import PostingSource, JobPosting, ConcoursPosting
from app.services.matching import compute_matches
from app.services.resume_parser import parse_resume, normalize_skills
from app.services.summary import generate_summary


# -------- Fixtures --------

@pytest.fixture(scope="module")
async def db_session():
    """Create a test database session."""
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    async with AsyncSessionLocal() as session:
        yield session
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)


# -------- Security Tests --------

class TestSecurity:
    def test_hash_password(self):
        """Test password hashing."""
        password = "mysecretpassword"
        hashed = get_password_hash(password)
        assert hashed != password
        assert len(hashed) > 0

    def test_verify_password_correct(self):
        """Test password verification with correct password."""
        password = "mysecretpassword"
        hashed = get_password_hash(password)
        assert verify_password(password, hashed) is True

    def test_verify_password_wrong(self):
        """Test password verification with wrong password."""
        password = "mysecretpassword"
        hashed = get_password_hash(password)
        assert verify_password("wrongpassword", hashed) is False

    def test_create_access_token(self):
        """Test JWT token creation."""
        data = {"sub": str(uuid.uuid4())}
        token = create_access_token(data)
        assert token is not None
        assert isinstance(token, str)

    def test_decode_access_token(self):
        """Test JWT token decoding."""
        data = {"sub": str(uuid.uuid4())}
        token = create_access_token(data)
        decoded = decode_access_token(token)
        assert decoded is not None
        assert "sub" in decoded

    def test_decode_invalid_token(self):
        """Test decoding an invalid token."""
        decoded = decode_access_token("invalid_token")
        assert decoded is None


# -------- Model Tests --------

class TestModels:
    @pytest.mark.asyncio
    async def test_user_creation(self):
        """Test user model creation."""
        user_id = uuid.uuid4()
        async with AsyncSessionLocal() as session:
            user = User(
                id=user_id,
                email=f"model_{user_id}@test.com",
                password_hash="hash",
                full_name="Model User",
            )
            session.add(user)
            await session.commit()
            await session.refresh(user)

            assert user.id == user_id
            assert user.email == f"model_{user_id}@test.com"
            assert user.full_name == "Model User"
            assert user.country == "Tunisia"

            await session.delete(user)
            await session.commit()

    @pytest.mark.asyncio
    async def test_posting_source_creation(self):
        """Test posting source creation."""
        source_id = uuid.uuid4()
        async with AsyncSessionLocal() as session:
            source = PostingSource(
                id=source_id,
                name="Test Source",
                source_type="official",
                url="https://test.example.com",
            )
            session.add(source)
            await session.commit()
            await session.refresh(source)

            assert source.name == "Test Source"
            assert source.enabled is True

            await session.delete(source)
            await session.commit()

    @pytest.mark.asyncio
    async def test_job_posting_creation(self):
        """Test job posting creation."""
        source_id = uuid.uuid4()
        async with AsyncSessionLocal() as session:
            source = PostingSource(
                id=source_id,
                name="Test Source",
                source_type="commercial",
                url="https://test.example.com",
            )
            session.add(source)
            await session.commit()
            await session.refresh(source)

            posting_id = uuid.uuid4()
            posting = JobPosting(
                id=posting_id,
                source_id=source.id,
                source_posting_id="test_123",
                title="Test Job",
                description="Test description",
                company="Test Company",
                job_type="emploi",
                status="active",
            )
            session.add(posting)
            await session.commit()
            await session.refresh(posting)

            assert posting.title == "Test Job"
            assert posting.company == "Test Company"

            await session.delete(posting)
            await session.delete(source)
            await session.commit()

    @pytest.mark.asyncio
    async def test_concours_posting_creation(self):
        """Test concours posting creation."""
        source_id = uuid.uuid4()
        async with AsyncSessionLocal() as session:
            source = PostingSource(
                id=source_id,
                name="Test Source",
                source_type="commercial",
                url="https://test.example.com",
            )
            session.add(source)
            await session.commit()
            await session.refresh(source)

            posting_id = uuid.uuid4()
            posting = ConcoursPosting(
                id=posting_id,
                source_id=source.id,
                source_posting_id="concours_123",
                title="Test Concours",
                description="Test description",
                organization="Test Org",
                exam_type="externe",
                status="open",
            )
            session.add(posting)
            await session.commit()
            await session.refresh(posting)

            assert posting.title == "Test Concours"
            assert posting.organization == "Test Org"

            await session.delete(posting)
            await session.delete(source)
            await session.commit()


# -------- Resume Parser Tests --------

class TestResumeParser:
    def test_normalize_skills(self):
        """Test skill normalization."""
        text = "J'ai experiencé en python, Django, et machine learning"
        skills = normalize_skills(text)
        assert "python" in skills
        assert "django" in skills

    def test_normalize_skills_empty(self):
        """Test skill normalization with empty text."""
        skills = normalize_skills("")
        assert skills == []

    def test_parse_resume_text(self, tmp_path):
        """Test parsing a text resume."""
        resume_file = tmp_path / "resume.txt"
        resume_file.write_text("Python Developer\n3 years experience\nLicence in Computer Science")

        result = parse_resume(str(resume_file))
        assert "raw_text" in result
        assert "skills" in result
        assert "education" in result
        assert "experience" in result

    def test_parse_resume_nonexistent(self):
        """Test parsing a non-existent file."""
        result = parse_resume("/nonexistent/file.pdf")
        assert result["raw_text"] == ""
        assert result["skills"] == []


# -------- Matching Engine Tests --------

class TestMatchingEngine:
    def test_compute_matches_empty_resume(self):
        """Test matching with empty resume data."""
        results = compute_matches(
            resume_data={},
            job_postings=[],
            concours_postings=[],
            user_id=str(uuid.uuid4()),
        )
        assert results == []

    def test_compute_matches_no_postings(self):
        """Test matching with no postings."""
        results = compute_matches(
            resume_data={"skills": ["python"], "education": [], "experience": []},
            job_postings=[],
            concours_postings=[],
            user_id=str(uuid.uuid4()),
        )
        assert results == []

    def test_compute_matches_with_postings(self):
        """Test matching with sample postings."""
        resume_data = {
            "skills": ["python", "django", "react"],
            "education": ["Licence"],
            "experience": ["3 ans"],
        }

        job_postings = [
            {
                "id": uuid.uuid4(),
                "title": "Python Developer",
                "description": "We need a Python developer with Django experience",
                "company": "Test Corp",
                "source_name": "Test",
                "required_skills": ["python", "django"],
            },
            {
                "id": uuid.uuid4(),
                "title": "Frontend Developer",
                "description": "React developer needed",
                "company": "Test Corp",
                "source_name": "Test",
                "required_skills": ["react"],
            },
        ]

        results = compute_matches(
            resume_data=resume_data,
            job_postings=job_postings,
            concours_postings=[],
            user_id=str(uuid.uuid4()),
        )

        assert len(results) == 2
        assert all(0 <= r["score"] <= 100 for r in results)
        assert results[0]["score"] >= results[1]["score"]


# -------- Summary Service Tests --------

class TestSummaryService:
    @pytest.mark.asyncio
    async def test_generate_summary_basic(self):
        """Test basic summary generation."""
        posting_data = {
            "title": "Python Developer",
            "description": "We need a Python developer with 3 years experience",
            "company": "Test Corp",
            "raw": {},
        }
        user_profile = {
            "skills": ["python", "django"],
            "education": ["Licence"],
            "experience": ["3 ans"],
        }

        result = await generate_summary(posting_data, user_profile, posting_type="job")
        assert "summary" in result
        assert "required_documents" in result
        assert "relevance" in result
        assert isinstance(result["required_documents"], list)

    @pytest.mark.asyncio
    async def test_generate_summary_concours(self):
        """Test summary generation for concours."""
        posting_data = {
            "title": "Concours Ingénieur",
            "description": "Concours pour ingénieur",
            "organization": "Ministère",
            "raw": {"required_diploma": "Licence", "exam_type": "externe"},
        }
        user_profile = {
            "skills": [],
            "education": ["Licence"],
            "experience": [],
        }

        result = await generate_summary(posting_data, user_profile, posting_type="concours")
        assert "summary" in result
        assert "relevance" in result


# -------- Database Tests --------

class TestDatabase:
    @pytest.mark.asyncio
    async def test_init_db(self):
        """Test database initialization."""
        from app.core.db import init_db
        await init_db()
        async with AsyncSessionLocal() as session:
            result = await session.execute(select(User))
            users = result.scalars().all()
            assert isinstance(users, list)

    @pytest.mark.asyncio
    async def test_user_crud(self):
        """Test user CRUD operations."""
        user_id = uuid.uuid4()
        async with AsyncSessionLocal() as session:
            user = User(
                id=user_id,
                email=f"crud_{user_id}@test.com",
                password_hash="hash",
                full_name="CRUD User",
            )
            session.add(user)
            await session.commit()
            await session.refresh(user)
            assert user.id is not None

            result = await session.execute(select(User).where(User.email == f"crud_{user_id}@test.com"))
            found = result.scalar_one_or_none()
            assert found is not None
            assert found.full_name == "CRUD User"

            found.full_name = "Updated User"
            await session.commit()
            await session.refresh(found)
            assert found.full_name == "Updated User"

            await session.delete(found)
            await session.commit()