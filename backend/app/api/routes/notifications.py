"""
Notifications routes — user notifications.
"""

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ...core.db import get_db
from ...core.dependencies import get_current_user
from ...models.user import User
from ...models.posting import Notification
from ...schemas.posting import NotificationResponse

router = APIRouter()


@router.get("")
async def list_notifications(
    unread_only: bool = False,
    limit: int = 50,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """List notifications for the current user."""
    query = select(Notification).where(Notification.user_id == current_user.id)
    if unread_only:
        query = query.where(Notification.read_at.is_(None))
    result = await db.execute(query.order_by(Notification.created_at.desc()).limit(limit))
    notifications = result.scalars().all()
    return [NotificationResponse.model_validate(n) for n in notifications]


@router.patch("/{notification_id}/read")
async def mark_read(
    notification_id: str,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Mark a notification as read."""
    result = await db.execute(
        select(Notification).where(
            Notification.id == notification_id,
            Notification.user_id == current_user.id,
        )
    )
    notification = result.scalar_one_or_none()
    if notification is None:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Notification not found",
        )
    notification.read_at = __import__("datetime").datetime.now(__import__("datetime").timezone.utc)
    await db.commit()
    return {"success": True}


@router.delete("")
async def clear_notifications(
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Delete all notifications for the current user."""
    await db.execute(select(Notification).where(Notification.user_id == current_user.id))
    await db.commit()
    return {"success": True, "message": "All notifications deleted"}
