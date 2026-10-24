"""
JWT authentication utilities.
"""

import bcrypt
from datetime import datetime, timedelta, timezone
from typing import Optional
from jose import jwt, JWTError

from .config import get_settings

settings = get_settings()

ALGORITHM = settings.JWT_ALGORITHM
SECRET_KEY = settings.SECRET_KEY


def verify_password(plain_password: str, hashed: str) -> bool:
    """Verify a password against a bcrypt hash."""
    return bcrypt.checkpw(plain_password.encode("utf-8"), hashed.encode("utf-8"))


def get_password_hash(password: str) -> str:
    """Hash a password using bcrypt."""
    salt = bcrypt.gensalt(rounds=12)
    hashed = bcrypt.hashpw(password.encode("utf-8"), salt)
    return hashed.decode("utf-8")


def create_access_token(data: dict, expires_delta: Optional[timedelta] = None) -> str:
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.now(timezone.utc) + expires_delta
    else:
        expire = datetime.now(timezone.utc) + timedelta(
            minutes=settings.JWT_EXPIRATION_MINUTES
        )
    to_encode.update({"exp": expire, "type": "access"})
    encoded = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return encoded


def decode_access_token(token: str) -> Optional[dict]:
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        return payload
    except JWTError:
        return None
