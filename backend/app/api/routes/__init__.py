from fastapi import APIRouter
from .auth import router as auth_router
from .postings import router as postings_router
from .matches import router as matches_router
from .resumes import router as resumes_router
from .notifications import router as notifications_router
from .sources import router as sources_router

# Create a root router and include all sub-routers with their prefixes
api_router = APIRouter()
api_router.include_router(auth_router, prefix="/auth", tags=["auth"])
api_router.include_router(postings_router, prefix="/postings", tags=["postings"])
api_router.include_router(matches_router, prefix="/matches", tags=["matches"])
api_router.include_router(resumes_router, prefix="/resumes", tags=["resumes"])
api_router.include_router(notifications_router, prefix="/notifications", tags=["notifications"])
api_router.include_router(sources_router, prefix="/sources", tags=["sources"])