"""
Scraper base class and registry.
"""

from abc import ABC, abstractmethod
from typing import Any, List, Optional
import logging
import time
import re

import httpx

from ...core.config import get_settings

logger = logging.getLogger(__name__)


class BaseScraper(ABC):
    """Base scraper for all sources."""

    name: str = "base"
    source_type: str = "official"
    url: str = ""
    interval_minutes: int = 1440  # 24h default

    def __init__(self):
        self._last_fetch: float = 0
        self._settings = get_settings()

    @abstractmethod
    async def fetch(self) -> List[dict]:
        """Return a list of raw posting dicts."""
        pass

    @abstractmethod
    async def parse(self, raw_data: str) -> List[dict]:
        """Parse raw HTML/text into structured posting dicts."""
        pass

    async def fetch_and_parse(self) -> List[dict]:
        """Fetch then parse."""
        raw = await self.fetch()
        return await self.parse(raw)

    async def health_check(self) -> dict:
        """Basic health check."""
        return {
            "name": self.name,
            "source_type": self.source_type,
            "url": self.url,
            "status": "ok",
            "interval_minutes": self.interval_minutes,
        }

    async def can_fetch(self, url: str) -> bool:
        """Check robots.txt before fetching."""
        try:
            from urllib.parse import urlparse
            parsed = urlparse(url)
            robots_url = f"{parsed.scheme}://{parsed.netloc}/robots.txt"

            async with httpx.AsyncClient(timeout=10, verify=False) as client:
                r = await client.get(robots_url)
                if r.status_code != 200:
                    return True  # No robots.txt = allow all

                text = r.text
                # Simple robots.txt parser
                user_agent = "*"
                disallow_paths = []
                in_our_section = False

                for line in text.split("\n"):
                    line = line.strip()
                    if line.lower().startswith("user-agent:"):
                        ua = line.split(":", 1)[1].strip()
                        in_our_section = ua == "*" or ua == self._settings.SCRAPER_USER_AGENT
                    elif line.lower().startswith("disallow:") and in_our_section:
                        path = line.split(":", 1)[1].strip()
                        disallow_paths.append(path)

                for path in disallow_paths:
                    if path == "/" or path == "":
                        return False
                    if url.startswith(f"{parsed.scheme}://{parsed.netloc}{path}"):
                        return False

                return True

        except Exception as e:
            logger.warning(f"robots.txt check failed for {url}: {e}")
            return True  # Fail open
