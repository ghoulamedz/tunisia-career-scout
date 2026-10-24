"""
Auth routes: register, login, get current user, Facebook OAuth.
"""

import json
import httpx
from fastapi import APIRouter, Depends, HTTPException, status, Request
from fastapi.security import OAuth2PasswordBearer
from fastapi.responses import RedirectResponse, JSONResponse
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from ...core.db import get_db
from ...core.security import verify_password, create_access_token, get_password_hash
from ...core.config import get_settings
from ...models.user import User
from ...models.posting import PostingSource
from ...schemas.user import UserRegister, UserLogin, UserResponse, TokenResponse

router = APIRouter()
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/api/auth/login", auto_error=False)


@router.post("/register", response_model=TokenResponse)
async def register(user_data: UserRegister, db: AsyncSession = Depends(get_db)):
    """Register a new user."""
    # Check email uniqueness
    result = await db.execute(select(User).where(User.email == user_data.email))
    if result.scalar_one_or_none():
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Email already registered",
        )

    # Create user
    hashed = get_password_hash(user_data.password)
    db_user = User(
        email=user_data.email,
        password_hash=hashed,
        full_name=user_data.full_name,
        country=user_data.country,
    )
    db.add(db_user)
    await db.commit()
    await db.refresh(db_user)

    # Create access token
    token = create_access_token(data={"sub": str(db_user.id)})

    return TokenResponse(
        access_token=token,
        token_type="bearer",
        user=UserResponse.model_validate(db_user),
    )


@router.post("/login", response_model=TokenResponse)
async def login(user_data: UserLogin, db: AsyncSession = Depends(get_db)):
    """Login and return JWT token."""
    result = await db.execute(select(User).where(User.email == user_data.email))
    user = result.scalar_one_or_none()

    if user is None or not verify_password(user_data.password, user.password_hash):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect email or password",
        )

    if not user.is_active:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Account is inactive",
        )

    token = create_access_token(data={"sub": str(user.id)})
    return TokenResponse(
        access_token=token,
        token_type="bearer",
        user=UserResponse.model_validate(user),
    )


@router.get("/me", response_model=UserResponse)
async def get_me(current_user: User = Depends(oauth2_scheme)):
    """
    Get current user's profile.
    Uses a custom dependency that extracts the user from the token.
    """
    from ...core.dependencies import get_current_user
    return UserResponse.model_validate(current_user)


@router.get("/facebook/login")
async def facebook_login():
    """
    Redirect to Facebook OAuth dialog (app-based).
    """
    settings = get_settings()
    if not settings.FACEBOOK_APP_ID or not settings.FACEBOOK_APP_SECRET:
        raise HTTPException(
            status_code=503,
            detail="Facebook OAuth not configured. Set FACEBOOK_APP_ID and FACEBOOK_APP_SECRET.",
        )

    params = {
        "client_id": settings.FACEBOOK_APP_ID,
        "redirect_uri": settings.FACEBOOK_REDIRECT_URI,
        "scope": "pages_show_list,pages_read_engagement,public_profile",
        "response_type": "code",
    }
    query = "&".join(f"{k}={v}" for k, v in params.items())
    url = f"https://www.facebook.com/v19.0/dialog/oauth?{query}"
    return RedirectResponse(url=url)


@router.post("/facebook/browser-login")
async def facebook_browser_login():
    """
    Start a Playwright browser for manual Facebook login.
    Returns the URL to open in the browser.
    """
    settings = get_settings()
    if not settings.FACEBOOK_APP_ID:
        raise HTTPException(
            status_code=503,
            detail="Facebook not configured. Set FACEBOOK_APP_ID.",
        )

    login_url = "https://www.facebook.com/login.php?next=/ANETI.EMPLOI.NAT.TN/"

    # Start browser in background for manual login
    async def _login_flow():
        try:
            from playwright.async_api import async_playwright
            pw = await async_playwright().start()
            browser = await pw.chromium.launch(headless=False)
            ctx = await browser.new_context(
                viewport={"width": 1366, "height": 768},
                locale="fr-FR",
                timezone_id="Africa/Tunis",
            )
            page = await ctx.new_page()
            await page.goto(login_url, wait_until="domcontentloaded")

            # Wait for user to log in (poll for cookies)
            for _ in range(60):  # 5 min timeout
                await asyncio.sleep(5)
                cookies = await ctx.cookies()
                fb_cookies = [c for c in cookies if c.get("name") in ["c_user", "xs"]]
                if fb_cookies:
                    # Save cookies
                    with open("/tmp/fb_cookies.json", "w") as f:
                        import json
                        json.dump(cookies, f)
                    logger.info("Facebook login detected, cookies saved.")
                    break
            else:
                logger.warning("Facebook login timeout.")

            await ctx.close()
            await browser.close()
            await pw.stop()
        except Exception as e:
            logger.error(f"Browser login error: {e}")

    # Run in background
    asyncio.create_task(_login_flow())

    return {
        "status": "browser_opening",
        "url": login_url,
        "message": "A browser window will open. Please log in to Facebook.",
    }


@router.get("/facebook/callback")
async def facebook_callback(code: str, db: AsyncSession = Depends(get_db)):
    """
    Handle Facebook OAuth callback.
    Exchanges code for access token and stores it in the Facebook sources.
    """
    settings = get_settings()
    if not settings.FACEBOOK_APP_ID or not settings.FACEBOOK_APP_SECRET:
        raise HTTPException(status_code=503, detail="Facebook OAuth not configured")

    # Exchange code for access token
    async with httpx.AsyncClient() as client:
        token_resp = await client.get("https://graph.facebook.com/v19.0/oauth/access_token", params={
            "client_id": settings.FACEBOOK_APP_ID,
            "redirect_uri": settings.FACEBOOK_REDIRECT_URI,
            "client_secret": settings.FACEBOOK_APP_SECRET,
            "code": code,
        })
        if token_resp.status_code != 200:
            raise HTTPException(status_code=400, detail="Failed to exchange code for token")
        token_data = token_resp.json()
        access_token = token_data.get("access_token")

    if not access_token:
        raise HTTPException(status_code=400, detail="No access token received")

    # Get user info
    async with httpx.AsyncClient() as client:
        user_resp = await client.get("https://graph.facebook.com/v19.0/me", params={
            "access_token": access_token,
            "fields": "id,name,email",
        })
        user_data = user_resp.json()

    # Store token in Facebook sources
    async with AsyncSessionLocal() as session:
        result = await session.execute(
            select(PostingSource).where(PostingSource.source_type == "social")
        )
        sources = result.scalars().all()
        for source in sources:
            source.auth_token = access_token
            await session.commit()

    # Create or update user
    email = user_data.get("email", f"fb_{user_data.get('id')}@facebook.com")
    result = await db.execute(select(User).where(User.email == email))
    user = result.scalar_one_or_none()
    if not user:
        user = User(
            id=user_data.get("id"),
            email=email,
            password_hash=get_password_hash("fb_default_password"),
            full_name=user_data.get("name", "Facebook User"),
            country="Tunisia",
        )
        db.add(user)
        await db.commit()
        await db.refresh(user)

    token = create_access_token(data={"sub": str(user.id)})
    return JSONResponse(content={
        "access_token": token,
        "token_type": "bearer",
        "user": UserResponse.model_validate(user).model_dump(),
        "facebook_name": user_data.get("name"),
    })
