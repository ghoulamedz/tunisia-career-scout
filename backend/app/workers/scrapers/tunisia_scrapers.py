"""
Scrapers for Tunisia job/concours sources.
Uses httpx for simple sites, Playwright for JS-heavy/bot-protected sites.

Features:
- Residential IP (runs on user's device)
- Session persistence (cookies saved/reused)
- Rate limiting
- Browser fingerprint randomization
- Human-like scraping patterns
"""

import asyncio
import json
import logging
import os
import random
import time
from typing import Any
from pathlib import Path

import httpx
from bs4 import BeautifulSoup
from playwright.async_api import async_playwright

from ...core.config import get_settings
from .base import BaseScraper

logger = logging.getLogger(__name__)

# Cookie storage directory
COOKIE_DIR = Path("/tmp/tunisia_career_scout_cookies")
COOKIE_DIR.mkdir(exist_ok=True)

# Rate limiter state
_rate_limit_state: dict[str, list[float]] = {}
_rate_limit_lock = asyncio.Lock()

# Default rate limit: 30 requests per minute
DEFAULT_RATE_LIMIT = 30
DEFAULT_RATE_WINDOW = 60.0


async def rate_limit(source_name: str, max_requests: int = DEFAULT_RATE_LIMIT, window: float = DEFAULT_RATE_WINDOW):
    """Rate limit per source. Waits if needed."""
    async with _rate_limit_lock:
        now = time.time()
        if source_name not in _rate_limit_state:
            _rate_limit_state[source_name] = []

        # Clean old entries
        _rate_limit_state[source_name] = [
            t for t in _rate_limit_state[source_name]
            if now - t < window
        ]

        if len(_rate_limit_state[source_name]) >= max_requests:
            # Wait until oldest request expires
            oldest = _rate_limit_state[source_name][0]
            wait_time = window - (now - oldest) + 1
            if wait_time > 0:
                logger.info(f"Rate limit: {source_name} waiting {wait_time:.1f}s")
                await asyncio.sleep(wait_time)

        _rate_limit_state[source_name].append(time.time())


def _cookie_path(source_name: str) -> Path:
    """Get cookie file path for a source."""
    return COOKIE_DIR / f"{source_name.replace(' ', '_').lower()}.json"


async def load_cookies(source_name: str) -> list[dict]:
    """Load saved cookies for a source."""
    path = _cookie_path(source_name)
    if path.exists():
        try:
            with open(path) as f:
                return json.load(f)
        except Exception:
            pass
    return []


async def save_cookies(source_name: str, cookies: list[dict]):
    """Save cookies for a source."""
    path = _cookie_path(source_name)
    try:
        with open(path, "w") as f:
            json.dump(cookies, f)
    except Exception as e:
        logger.warning(f"Failed to save cookies for {source_name}: {e}")


def _random_fingerprint():
    """Generate random browser fingerprint."""
    viewports = [
        {"width": 1366, "height": 768},
        {"width": 1440, "height": 900},
        {"width": 1280, "height": 720},
        {"width": 1920, "height": 1080},
        {"width": 1536, "height": 864},
    ]
    locales = ["fr-FR", "fr", "en-US", "ar-TN"]
    timezones = ["Africa/Tunis", "Europe/Paris", "Europe/Madrid"]

    return {
        "viewport": random.choice(viewports),
        "locale": random.choice(locales),
        "timezone": random.choice(timezones),
        "user_agent": random.choice([
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ]),
    }


async def fetch_with_playwright(
    url: str,
    source_name: str = "",
    timeout: int = 20000,
    human_like: bool = True,
    wait_selector: str = None,
) -> str:
    """Fetch a page using Playwright browser with human-like behavior.

    Features:
    - Residential IP (runs on user's device)
    - Session persistence (cookies saved/reused)
    - Rate limiting
    - Browser fingerprint randomization
    - Human-like patterns (random delays, mouse, scroll)
    """
    # Rate limit
    if source_name:
        await rate_limit(source_name, max_requests=30, window=60.0)

    # Load saved cookies for session persistence
    saved_cookies = await load_cookies(source_name) if source_name else []

    html = ""
    pw = None
    try:
        pw = await async_playwright().start()

        # Random fingerprint
        fp = _random_fingerprint()

        browser = await pw.chromium.launch(
            headless=True,
            args=[
                "--disable-blink-features=AutomationControlled",
                "--disable-infobars",
                "--no-default-browser-check",
            ],
        )

        ctx = await browser.new_context(
            viewport=fp["viewport"],
            user_agent=fp["user_agent"],
            locale=fp["locale"],
            timezone_id=fp["timezone"],
        )

        # Restore saved cookies
        if saved_cookies:
            try:
                await ctx.add_cookies(saved_cookies)
                logger.info(f"Restored {len(saved_cookies)} cookies for {source_name}")
            except Exception as e:
                logger.warning(f"Cookie restore failed for {source_name}: {e}")

        # Add stealth headers
        await ctx.add_init_script("""
            Object.defineProperty(navigator, 'webdriver', {get: () => undefined});
            Object.defineProperty(navigator, 'plugins', {get: () => [1, 2, 3]});
            Object.defineProperty(navigator, 'languages', {get: () => ['fr-FR', 'fr', 'en-US', 'en']});
        """)

        page = await ctx.new_page()

        # Human-like: random delay before navigation
        if human_like:
            await asyncio.sleep(random.uniform(1.0, 2.5))

        await page.goto(url, wait_until="domcontentloaded", timeout=timeout)

        # Wait for body to load
        await page.wait_for_selector("body", timeout=5000)

        # Human-like: random scroll
        if human_like:
            await page.mouse.move(
                random.randint(200, 800),
                random.randint(200, 500),
            )
            await page.mouse.wheel(0, random.randint(100, 400))
            await asyncio.sleep(random.uniform(0.5, 1.5))

        # Wait for dynamic content
        if wait_selector:
            try:
                await page.wait_for_selector(wait_selector, timeout=10000)
            except Exception:
                pass

        await asyncio.sleep(random.uniform(2.0, 4.0) if human_like else 2.0)

        html = await page.content()

        # Save cookies for session persistence
        if source_name:
            try:
                cookies = await ctx.cookies()
                await save_cookies(source_name, cookies)
                logger.info(f"Saved {len(cookies)} cookies for {source_name}")
            except Exception as e:
                logger.warning(f"Cookie save failed for {source_name}: {e}")

        await ctx.close()
        await browser.close()
    except Exception as e:
        logger.error(f"Playwright fetch error for {url}: {e}")
    finally:
        if pw:
            try:
                await pw.stop()
            except Exception:
                pass

    return html


class AnetiScraper(BaseScraper):
    """Scraper for ANETI (Agence Nationale pour l'Emploi) using Playwright."""

    name = "ANETI"
    source_type = "official"
    url = "https://www.emploi.nat.tn/"
    interval_minutes = 1440

    async def fetch(self) -> list[dict]:
        """Fetch ANETI offers using Playwright."""
        offers = []
        try:
            html = await fetch_with_playwright(self.url, source_name="ANETI", wait_selector="body")
            if html:
                offers.extend(await self.parse(html))

            html2 = await fetch_with_playwright(
                "https://candidatures.aneti.tn/",
                source_name="ANETI",
                wait_selector="body",
            )
            if html2:
                offers.extend(await self.parse(html2, source_type="candidatures"))
        except Exception as e:
            logger.error(f"ANETI fetch error: {e}")

        return offers

    async def parse(self, html: str, source_type: str = "offres") -> list[dict]:
        """Parse ANETI HTML into structured postings."""
        soup = BeautifulSoup(html, "lxml")
        for tag in soup.select("script, style, nav, header, footer, iframe"):
            tag.decompose()

        postings = []
        for selector in [".offre", ".annonce", ".job-item", "article", ".listing-item", ".post"]:
            for el in soup.select(selector):
                title_el = el.select_one("h2, h3, h4, .title, .titre")
                company_el = el.select_one(".company, .organisme, .employeur")
                desc_el = el.select_one(".description, .content, p")
                url_el = el.select_one("a[href]")

                title = title_el.get_text(strip=True) if title_el else ""
                if not title:
                    continue

                postings.append({
                    "title": title,
                    "description": desc_el.get_text(strip=True) if desc_el else "",
                    "company": company_el.get_text(strip=True) if company_el else "ANETI",
                    "location": "",
                    "job_type": "emploi",
                    "source_posting_id": f"aneti_{hash(title)}",
                    "raw": {"html": str(el), "source_type": source_type},
                })

        # Fallback: extract all links with relevant text
        if not postings:
            for a in soup.select("a[href]"):
                text = a.get_text(strip=True)
                if any(kw in text.lower() for kw in ["concours", "recrutement", "emploi", "poste", "candidature"]):
                    if len(text) > 10:
                        postings.append({
                            "title": text[:200],
                            "description": text,
                            "company": "ANETI",
                            "location": "",
                            "job_type": "emploi",
                            "source_posting_id": f"aneti_link_{hash(text)}",
                            "raw": {"href": a.get("href", ""), "source_type": source_type},
                        })

        return postings


class TunisieConcoursScraper(BaseScraper):
    """Scraper for Tunisie Concours (tunisieconcours.tn) via WordPress REST API."""

    name = "Tunisie Concours"
    source_type = "commercial"
    url = "https://tunisieconcours.tn/"
    interval_minutes = 720

    async def fetch(self) -> list[dict]:
        """Fetch Tunisie Concours postings via WordPress REST API."""
        offers = []
        try:
            async with httpx.AsyncClient(timeout=30, follow_redirects=True, verify=False,
                                          headers={"User-Agent": "Mozilla/5.0"}) as client:
                api_url = "https://tunisieconcours.tn/wp-json/wp/v2/posts?per_page=20"
                r = await client.get(api_url)
                if r.status_code == 200:
                    offers = await self.parse_api(r.json())
                else:
                    html = await fetch_with_playwright(self.url, source_name="Tunisie Concours")
                    if html:
                        offers = await self.parse(html)
        except Exception as e:
            logger.error(f"Tunisie Concours fetch error: {e}")

        return offers

    async def parse_api(self, posts: list[dict]) -> list[dict]:
        """Parse WordPress REST API posts."""
        postings = []
        for post in posts:
            title = post.get("title", {}).get("rendered", "")
            if not title:
                continue
            desc = BeautifulSoup(post.get("content", {}).get("rendered", ""), "lxml").get_text(strip=True)
            link = post.get("link", "")
            slug = post.get("slug", "")
            date = post.get("date", "")

            postings.append({
                "title": title,
                "description": desc[:2000],
                "company": "Tunisie Concours",
                "location": "",
                "job_type": "concours",
                "source_posting_id": f"tunisieconcours_{post.get('id', slug)}",
                "raw": {"href": link, "slug": slug, "date": date},
            })
        return postings

    async def parse(self, html: str) -> list[dict]:
        """Parse Tunisie Concours HTML fallback."""
        soup = BeautifulSoup(html, "lxml")
        postings = []

        for el in soup.select("article, .post, .item, .concours-item, .job-card"):
            title_el = el.select_one("h2, h3, h4, .title, .entry-title")
            desc_el = el.select_one(".entry-content, .content, .description, p")
            link_el = el.select_one("a[href]")

            title = title_el.get_text(strip=True) if title_el else ""
            if not title:
                continue

            postings.append({
                "title": title,
                "description": desc_el.get_text(strip=True) if desc_el else "",
                "company": "Tunisie Concours",
                "location": "",
                "job_type": "concours",
                "source_posting_id": f"tunisieconcours_{hash(title)}",
                "raw": {"html": str(el), "link": link_el.get("href", "") if link_el else ""},
            })

        return postings


class TunisieTravailScraper(BaseScraper):
    """Scraper for Tunisie Travail (tunisietravail.net)."""

    name = "Tunisie Travail"
    source_type = "commercial"
    url = "https://www.tunisietravail.net/"
    interval_minutes = 720

    async def fetch(self) -> list[dict]:
        """Fetch Tunisie Travail postings."""
        offers = []
        try:
            async with httpx.AsyncClient(timeout=30, follow_redirects=True, verify=False,
                                          headers={"User-Agent": "Mozilla/5.0"}) as client:
                r = await client.get(self.url)
                r.raise_for_status()
                offers = await self.parse(r.text)
        except Exception as e:
            logger.error(f"Tunisie Travail fetch error: {e}")

        return offers

    async def parse(self, html: str) -> list[dict]:
        """Parse Tunisie Travail HTML."""
        soup = BeautifulSoup(html, "lxml")
        postings = []

        for el in soup.select("article, .post, .item, .offre, .job-listing"):
            title_el = el.select_one("h2, h3, h4, .title, .entry-title")
            desc_el = el.select_one(".entry-content, .content, .description")
            company_el = el.select_one(".company, .employeur, .entreprise")
            link_el = el.select_one("a[href]")

            title = title_el.get_text(strip=True) if title_el else ""
            if not title:
                continue

            postings.append({
                "title": title,
                "description": desc_el.get_text(strip=True) if desc_el else "",
                "company": company_el.get_text(strip=True) if company_el else "Tunisie Travail",
                "location": "",
                "job_type": "emploi",
                "source_posting_id": f"tunisietravail_{hash(title)}",
                "raw": {"html": str(el), "link": link_el.get("href", "") if link_el else ""},
            })

        return postings


class EmploitunisieScraper(BaseScraper):
    """Scraper for Emploitunisie (emploitunisie.com) using Playwright."""

    name = "Emploitunisie"
    source_type = "commercial"
    url = "https://www.emploitunisie.com/"
    interval_minutes = 720

    async def fetch(self) -> list[dict]:
        """Fetch Emploitunisie postings using Playwright."""
        offers = []
        try:
            html = await fetch_with_playwright(self.url, source_name="Emploitunisie", wait_selector="body")
            if html:
                offers = await self.parse(html)
        except Exception as e:
            logger.error(f"Emploitunisie fetch error: {e}")

        return offers

    async def parse(self, html: str) -> list[dict]:
        """Parse Emploitunisie HTML."""
        soup = BeautifulSoup(html, "lxml")
        postings = []

        for el in soup.select("article, .post, .item, .offre, .job-listing"):
            title_el = el.select_one("h2, h3, h4, .title, .entry-title")
            desc_el = el.select_one(".entry-content, .content, .description")
            company_el = el.select_one(".company, .employeur, .entreprise")
            link_el = el.select_one("a[href]")

            title = title_el.get_text(strip=True) if title_el else ""
            if not title:
                continue

            postings.append({
                "title": title,
                "description": desc_el.get_text(strip=True) if desc_el else "",
                "company": company_el.get_text(strip=True) if company_el else "Emploitunisie",
                "location": "",
                "job_type": "emploi",
                "source_posting_id": f"emploitunisie_{hash(title)}",
                "raw": {"html": str(el), "link": link_el.get("href", "") if link_el else ""},
            })

        return postings


class FacebookScraper(BaseScraper):
    """Scraper for Facebook pages using Playwright with manual browser login.

    Supports:
    - Manual browser login (opens visible window for user to log in)
    - Session persistence (cookies saved/reused)
    - Rate limiting
    """

    name = "Facebook"
    source_type = "social"
    url = ""
    interval_minutes = 360

    async def fetch(self, login_url: str = None) -> list[dict]:
        """Fetch Facebook posts using Playwright.

        If login_url is provided, opens a browser for manual login first.
        Then scrapes the ANETI, Concours Gov, and Tunisie Travail pages.
        """
        offers = []
        try:
            # If login URL provided, open browser for manual login
            if login_url:
                logger.info(f"Opening browser for Facebook login: {login_url}")
                await self._manual_login(login_url)

            pages = [
                {"name": "ANETI", "url": "https://www.facebook.com/ANETI.EMPLOI.NAT.TN/"},
                {"name": "Concours Gov", "url": "https://www.facebook.com/concours.gouv.tn/"},
                {"name": "Tunisie Travail", "url": "https://www.facebook.com/tunisietravail/"},
            ]

            for page in pages:
                html = await fetch_with_playwright(
                    page["url"],
                    source_name="Facebook",
                    wait_selector="body",
                )
                if html:
                    page_offers = await self.parse(html, page["name"])
                    offers.extend(page_offers)
        except Exception as e:
            logger.error(f"Facebook fetch error: {e}")

        return offers

    async def _manual_login(self, login_url: str):
        """Open a browser for manual Facebook login."""
        browser = None
        try:
            async with async_playwright() as pw:
                browser = await pw.chromium.launch(headless=False)  # Visible window
                ctx = await browser.new_context(
                    viewport={"width": 1366, "height": 768},
                    locale="fr-FR",
                    timezone_id="Africa/Tunis",
                )
                page = await ctx.new_page()
                await page.goto(login_url, wait_until="domcontentloaded")

                logger.info("Please log in to Facebook in the browser window...")

                # Wait for user to log in (poll for cookies)
                for _ in range(60):  # 5 min timeout
                    await asyncio.sleep(5)
                    cookies = await ctx.cookies()
                    fb_cookies = [c for c in cookies if c.get("name") in ["c_user", "xs"]]
                    if fb_cookies:
                        await save_cookies("Facebook", cookies)
                        logger.info("Login detected, cookies saved.")
                        break
                else:
                    logger.warning("Facebook login timeout.")

                await ctx.close()
                await browser.close()
        except KeyboardInterrupt:
            logger.info("Login interrupted by user.")
            if browser:
                await browser.close()
        except Exception as e:
            logger.error(f"Manual login error: {e}")
            if browser:
                await browser.close()

    async def parse(self, html: str, page_name: str = "Facebook") -> list[dict]:
        """Parse Facebook HTML posts."""
        soup = BeautifulSoup(html, "lxml")
        postings = []

        for el in soup.select("article, [role='article'], .userContentWrapper"):
            text_el = el.select_one("[data-ad-preview='message'], .userContent, .story_body")
            if not text_el:
                continue

            text = text_el.get_text(strip=True)
            if not text:
                continue

            if any(kw in text.lower() for kw in ["concours", "recrutement", "emploi", "poste", "candidature"]):
                postings.append({
                    "title": text[:200],
                    "description": text,
                    "company": page_name,
                    "location": "",
                    "job_type": "emploi",
                    "source_posting_id": f"fb_{page_name}_{hash(text)}",
                    "raw": {"source": "facebook_playwright"},
                })

        return postings


class ActuTnScraper(BaseScraper):
    """Scraper for actu.tn (Tunisia news/jobs/concours).

    Supports RSS feed and Playwright-based scraping.
    """

    name = "ActuTn"
    source_type = "commercial"
    url = "https://www.actu.tn/"
    interval_minutes = 720

    RSS_URLS = [
        "https://actu.tn/feed/",
        "https://actu.tn/rss/",
        "https://actu.tn/feed/rss/",
        "https://www.actu.tn/feed/",
        "https://www.actu.tn/rss/",
        "https://www.actu.tn/feed/rss/",
    ]

    PLAYWRIGHT_URLS = [
        "https://actu.tn/",
        "https://www.actu.tn/",
    ]

    async def fetch(self, use_rss: bool = True) -> list[dict]:
        """Fetch actu.tn postings via RSS or Playwright."""
        offers = []

        # Try RSS first
        if use_rss:
            for rss_url in self.RSS_URLS:
                try:
                    async with httpx.AsyncClient(timeout=15, verify=False) as client:
                        r = await client.get(rss_url)
                        if r.status_code == 200:
                            offers = self.parse_rss(r.text)
                            if offers:
                                logger.info(f"actu.tn: {len(offers)} from RSS: {rss_url}")
                                return offers
                except Exception as e:
                    logger.warning(f"RSS fetch error ({rss_url}): {e}")

        # Fallback to Playwright with alternative URLs
        for pu in self.PLAYWRIGHT_URLS:
            try:
                html = await fetch_with_playwright(pu, source_name="ActuTn")
                if html and len(html) > 500:
                    offers = await self.parse(html)
                    if offers:
                        logger.info(f"actu.tn: {len(offers)} from Playwright: {pu}")
                        return offers
            except Exception as e:
                logger.warning(f"actu.tn Playwright error ({pu}): {e}")

        return offers

    def parse_rss(self, xml_text: str) -> list[dict]:
        """Parse RSS/XML feed."""
        from xml.etree import ElementTree as ET
        postings = []
        try:
            root = ET.fromstring(xml_text)
            for item in root.iter("item"):
                title = item.findtext("title", "")
                link = item.findtext("link", "")
                desc = item.findtext("description", "")
                pub_date = item.findtext("pubDate", "")

                if title and len(title) > 5:
                    title_lower = title.lower()
                    is_relevant = any(
                        kw in title_lower
                        for kw in ["emploi", "recrutement", "concours", "poste", "candidature", "travail", "recrute"]
                    )
                    if not is_relevant:
                        continue

                    postings.append({
                        "title": title,
                        "description": desc or title,
                        "company": "ActuTn",
                        "location": "",
                        "job_type": "concours" if "concours" in title_lower else "emploi",
                        "source_posting_id": f"actu_{hash(title)}",
                        "raw": {"href": link, "pubDate": pub_date},
                    })
        except Exception as e:
            logger.error(f"RSS parse error: {e}")

        return postings

    async def parse(self, html: str) -> list[dict]:
        """Parse actu.tn HTML fallback."""
        soup = BeautifulSoup(html, "lxml")
        postings = []

        for el in soup.select("article, .post, .item, .entry"):
            title_el = el.select_one("h2, h3, h4, .title, .entry-title")
            desc_el = el.select_one(".entry-content, .content, .description")
            link_el = el.select_one("a[href]")

            title = title_el.get_text(strip=True) if title_el else ""
            if not title:
                continue

            postings.append({
                "title": title,
                "description": desc_el.get_text(strip=True) if desc_el else "",
                "company": "ActuTn",
                "location": "",
                "job_type": "emploi",
                "source_posting_id": f"actu_{hash(title)}",
                "raw": {"html": str(el), "link": link_el.get("href", "") if link_el else ""},
            })

        return postings