#!/usr/bin/env python3
"""
Python Sidecar for Social Media Scraping
Communicates via JSON-RPC over stdin/stdout with Tauri app
"""

import sys
import json
import asyncio
import logging
import os
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, asdict
from pathlib import Path

# Configure logging to stderr (not stdout!)
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger(__name__)

@dataclass
class ScrapedItem:
    url: str
    title: str
    content: str
    author: Optional[str] = None
    posted_at: Optional[str] = None
    metadata: Dict[str, Any] = None

    def __post_init__(self):
        if self.metadata is None:
            self.metadata = {}

    def to_dict(self) -> Dict[str, Any]:
        return {
            "url": self.url,
            "title": self.title,
            "content": self.content,
            "author": self.author,
            "posted_at": self.posted_at,
            "metadata": self.metadata
        }

class ScraperError(Exception):
    """Structured error for scraper operations"""
    def __init__(self, code: str, message: str, details: Optional[Dict] = None):
        self.code = code
        self.message = message
        self.details = details or {}
        super().__init__(message)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "message": self.message,
            "details": self.details
        }

def get_browser_args() -> List[str]:
    """Get browser launch arguments for anti-detection"""
    return [
        "--disable-blink-features=AutomationControlled",
        "--disable-features=IsolateOrigins,site-per-process",
        "--no-sandbox",
        "--disable-setuid-sandbox",
        "--disable-dev-shm-usage",
        "--disable-accelerated-2d-canvas",
        "--no-first-run",
        "--no-zygote",
        "--disable-gpu",
    ]

async def create_stealth_context(playwright, headless: bool = True):
    """Create a browser context with stealth settings"""
    from playwright_stealth import Stealth
    
    stealth = Stealth()
    
    browser = await playwright.chromium.launch(
        headless=headless,
        args=get_browser_args()
    )
    
    context = await browser.new_context(
        viewport={"width": 1280, "height": 800},
        user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        locale="en-US",
        timezone_id="America/New_York",
    )
    
    # Apply stealth to context (affects all pages)
    await stealth.apply_stealth_async(context)
    
    return browser, context

async def login_facebook(url: str) -> str:
    """Login to Facebook and return cookies as JSON string"""
    from playwright.async_api import async_playwright
    
    async with async_playwright() as p:
        browser, context = await create_stealth_context(p, headless=False)
        page = await context.new_page()
        
        logger.info(f"Opening Facebook login: {url}")
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=60000)
        except Exception as e:
            logger.warning(f"Initial navigation timeout: {e}")
        
        # Wait for user to complete login
        # Facebook redirects to feed/home after login
        logger.info("Waiting for user to log in (check browser window)...")
        try:
            # Wait for either feed, home, or profile - indicators of successful login
            await page.wait_for_url(
                lambda u: any(x in u for x in ["/feed", "/home", "/profile", "/me", "facebook.com/?"]),
                timeout=300000  # 5 minutes
            )
        except Exception:
            # Fallback: wait for any facebook.com URL change after initial load
            logger.info("Waiting for any Facebook navigation...")
            await page.wait_for_url("**/facebook.com/**", timeout=300000)
        
        # Additional wait for login to fully complete (cookies to set)
        await page.wait_for_timeout(5000)
        
        # Verify login by checking for feed content
        try:
            await page.wait_for_selector('[role="feed"], [data-pagelet="FeedUnit"], div[aria-label="Feed"]', timeout=10000)
            logger.info("Login verified - feed detected")
        except Exception:
            logger.warning("Feed not detected, but continuing with cookies")
        
        cookies = await context.cookies()
        await browser.close()
        
        # Filter for facebook.com cookies only
        fb_cookies = [c for c in cookies if ".facebook.com" in c.get("domain", "") or "facebook.com" == c.get("domain", "")]
        logger.info(f"Captured {len(fb_cookies)} Facebook cookies")
        
        return json.dumps(fb_cookies)

async def login_tiktok(url: str) -> str:
    """Login to TikTok and return cookies as JSON string"""
    from playwright.async_api import async_playwright
    
    async with async_playwright() as p:
        browser, context = await create_stealth_context(p, headless=False)
        page = await context.new_page()
        
        logger.info(f"Opening TikTok login: {url}")
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=60000)
        except Exception as e:
            logger.warning(f"Initial navigation timeout: {e}")
        
        logger.info("Waiting for user to log in (check browser window)...")
        try:
            # TikTok redirects to /explore, /following, or profile after login
            await page.wait_for_url(
                lambda u: any(x in u for x in ["/explore", "/following", "/upload", "/live", "/@", "tiktok.com/?"]),
                timeout=300000
            )
        except Exception:
            await page.wait_for_url("**/tiktok.com/**", timeout=300000)
        
        await page.wait_for_timeout(5000)
        
        # Verify login
        try:
            await page.wait_for_selector('[data-e2e="feed-video"], [data-e2e="user-post-item"], div[data-e2e="video-feed"]', timeout=10000)
            logger.info("Login verified - feed detected")
        except Exception:
            logger.warning("Feed not detected, but continuing with cookies")
        
        cookies = await context.cookies()
        await browser.close()
        
        tt_cookies = [c for c in cookies if ".tiktok.com" in c.get("domain", "") or "tiktok.com" == c.get("domain", "")]
        logger.info(f"Captured {len(tt_cookies)} TikTok cookies")
        
        return json.dumps(tt_cookies)

async def login_linkedin(url: str) -> str:
    """Login to LinkedIn and return cookies as JSON string"""
    from playwright.async_api import async_playwright
    
    async with async_playwright() as p:
        browser, context = await create_stealth_context(p, headless=False)
        page = await context.new_page()
        
        logger.info(f"Opening LinkedIn login: {url}")
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=60000)
        except Exception as e:
            logger.warning(f"Initial navigation timeout: {e}")
        
        logger.info("Waiting for user to log in (check browser window)...")
        try:
            # LinkedIn redirects to feed, mynetwork, or jobs after login
            await page.wait_for_url(
                lambda u: any(x in u for x in ["/feed", "/mynetwork", "/jobs", "/in/", "/messaging", "linkedin.com/?"]),
                timeout=300000
            )
        except Exception:
            await page.wait_for_url("**/linkedin.com/**", timeout=300000)
        
        await page.wait_for_timeout(5000)
        
        # Verify login
        try:
            await page.wait_for_selector('[data-test-feed], .feed-shared-update-v2, .jobs-search-results', timeout=10000)
            logger.info("Login verified - feed detected")
        except Exception:
            logger.warning("Feed not detected, but continuing with cookies")
        
        cookies = await context.cookies()
        await browser.close()
        
        li_cookies = [c for c in cookies if ".linkedin.com" in c.get("domain", "") or "linkedin.com" == c.get("domain", "")]
        logger.info(f"Captured {len(li_cookies)} LinkedIn cookies")
        
        return json.dumps(li_cookies)

async def scrape_facebook(target: Dict, cookies: str, scroll_depth: int = 5) -> List[Dict]:
    """Scrape Facebook group/page posts"""
    from playwright.async_api import async_playwright
    from bs4 import BeautifulSoup
    import re
    
    cookies_list = json.loads(cookies) if cookies else []
    
    async with async_playwright() as p:
        browser, context = await create_stealth_context(p, headless=True)
        
        if cookies_list:
            await context.add_cookies(cookies_list)
        
        page = await context.new_page()
        
        target_url = build_facebook_url(target)
        logger.info(f"Scraping Facebook: {target_url}")
        
        try:
            await page.goto(target_url, wait_until="domcontentloaded", timeout=60000)
        except Exception as e:
            logger.warning(f"Navigation timeout: {e}")
        
        # Wait for feed to load
        try:
            await page.wait_for_selector('[role="feed"], [data-pagelet^="FeedUnit"], div[aria-label="Feed"]', timeout=15000)
        except Exception:
            logger.warning("Feed selector not found, continuing anyway")
        
        # Scroll to load more posts
        for i in range(scroll_depth):
            logger.debug(f"Scroll {i+1}/{scroll_depth}")
            await page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
            await page.wait_for_timeout(3000)
            
            # Check for "See more" buttons and click them
            try:
                see_more_buttons = await page.query_selector_all('div[role="button"]:has-text("See more"), span:has-text("See more"), a:has-text("See more")')
                for btn in see_more_buttons[:5]:  # Limit clicks per scroll
                    try:
                        await btn.click()
                        await page.wait_for_timeout(500)
                    except Exception:
                        pass
            except Exception:
                pass
        
        html = await page.content()
        await browser.close()
        
        return parse_facebook_posts(html, target_url)

async def scrape_tiktok(target: Dict, cookies: str, scroll_depth: int = 5) -> List[Dict]:
    """Scrape TikTok hashtag/user posts"""
    from playwright.async_api import async_playwright
    from bs4 import BeautifulSoup
    import re
    
    cookies_list = json.loads(cookies) if cookies else []
    
    async with async_playwright() as p:
        browser, context = await create_stealth_context(p, headless=True)
        
        if cookies_list:
            await context.add_cookies(cookies_list)
        
        page = await context.new_page()
        
        target_url = build_tiktok_url(target)
        logger.info(f"Scraping TikTok: {target_url}")
        
        try:
            await page.goto(target_url, wait_until="domcontentloaded", timeout=60000)
        except Exception as e:
            logger.warning(f"Navigation timeout: {e}")
        
        # Wait for video feed
        try:
            await page.wait_for_selector('[data-e2e="feed-video"], [data-e2e="user-post-item"], div[data-e2e="video-feed"]', timeout=15000)
        except Exception:
            logger.warning("Video feed selector not found, continuing anyway")
        
        for i in range(scroll_depth):
            logger.debug(f"Scroll {i+1}/{scroll_depth}")
            await page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
            await page.wait_for_timeout(3000)
        
        html = await page.content()
        await browser.close()
        
        return parse_tiktok_posts(html, target_url)

async def scrape_linkedin(target: Dict, cookies: str, scroll_depth: int = 5) -> List[Dict]:
    """Scrape LinkedIn job posts"""
    from playwright.async_api import async_playwright
    from bs4 import BeautifulSoup
    import re
    
    cookies_list = json.loads(cookies) if cookies else []
    
    async with async_playwright() as p:
        browser, context = await create_stealth_context(p, headless=True)
        
        if cookies_list:
            await context.add_cookies(cookies_list)
        
        page = await context.new_page()
        
        target_url = build_linkedin_url(target)
        logger.info(f"Scraping LinkedIn: {target_url}")
        
        try:
            await page.goto(target_url, wait_until="domcontentloaded", timeout=60000)
        except Exception as e:
            logger.warning(f"Navigation timeout: {e}")
        
        # Wait for job cards
        try:
            await page.wait_for_selector('[data-test-job-card], .job-search-card, .base-card, li[data-occludable-job-id]', timeout=15000)
        except Exception:
            logger.warning("Job card selector not found, continuing anyway")
        
        for i in range(scroll_depth):
            logger.debug(f"Scroll {i+1}/{scroll_depth}")
            await page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
            await page.wait_for_timeout(3000)
        
        html = await page.content()
        await browser.close()
        
        return parse_linkedin_posts(html, target_url)

def build_facebook_url(target: Dict) -> str:
    """Build Facebook URL from target config"""
    t = target.get("type", "group")
    if t == "group":
        gid = target.get("id", "")
        return f"https://www.facebook.com/groups/{gid}"
    elif t == "page":
        pid = target.get("id", "")
        return f"https://www.facebook.com/{pid}"
    elif t == "search":
        query = target.get("query", "jobs Tunisia")
        return f"https://www.facebook.com/search/posts/?q={query}"
    return "https://www.facebook.com"

def build_tiktok_url(target: Dict) -> str:
    """Build TikTok URL from target config"""
    t = target.get("type", "hashtag")
    if t == "hashtag":
        tag = target.get("tag", "jobs")
        return f"https://www.tiktok.com/tag/{tag}"
    elif t == "user":
        username = target.get("username", "")
        return f"https://www.tiktok.com/@{username}"
    elif t == "search":
        query = target.get("query", "jobs")
        return f"https://www.tiktok.com/search?q={query}"
    return "https://www.tiktok.com"

def build_linkedin_url(target: Dict) -> str:
    """Build LinkedIn URL from target config"""
    t = target.get("type", "search")
    if t == "search":
        query = target.get("query", "jobs")
        location = target.get("location", "Tunisia")
        return f"https://www.linkedin.com/jobs/search/?keywords={query}&location={location}"
    elif t == "company":
        cid = target.get("id", "")
        return f"https://www.linkedin.com/company/{cid}/jobs"
    return "https://www.linkedin.com/jobs"

def parse_facebook_posts(html: str, base_url: str) -> List[Dict]:
    """Parse Facebook posts from HTML"""
    from bs4 import BeautifulSoup
    import re
    
    soup = BeautifulSoup(html, 'html.parser')
    posts = []
    
    # Try multiple selectors for Facebook posts
    selectors = [
        'div[data-pagelet^="FeedUnit"]',
        'div[role="article"]',
        'div[data-ad-comet-preview="message"]',
        'div.x1iorvi4.x1pi30zi.x1l90r2v.x1swvt13',
    ]
    
    post_elements = []
    for sel in selectors:
        post_elements = soup.select(sel)
        if post_elements:
            logger.debug(f"Found {len(post_elements)} posts with selector: {sel}")
            break
    
    if not post_elements:
        # Fallback: any div with substantial text content
        post_elements = soup.find_all('div', string=re.compile(r'.{100,}'))
    
    for elem in post_elements[:50]:
        try:
            # Extract text content
            text_candidates = [
                elem.find('div', {'data-ad-comet-preview': 'message'}),
                elem.find('div', class_=re.compile(r'x1iorvi4|xdj266r|x11i5rnm')),
                elem.find('span', class_=re.compile(r'x193iq5w|xeuugli')),
            ]
            
            content = ""
            for tc in text_candidates:
                if tc and tc.get_text(strip=True):
                    content = tc.get_text(strip=True)
                    break
            
            if not content:
                content = elem.get_text(strip=True)
            
            # Find author
            author_elem = elem.find('a', class_=re.compile(r'x1i10hfl|x1qjc9v5|x1yr1y7q'))
            author = author_elem.get_text(strip=True) if author_elem else None
            
            # Find link
            link_elem = elem.find('a', href=re.compile(r'/posts/|/photos/|/videos/|/reel/|story_fbid='))
            url = link_elem['href'] if link_elem and link_elem.get('href') else base_url
            if url.startswith('/'):
                url = f"https://www.facebook.com{url}"
            
            # Filter: must have meaningful content
            if content and len(content) > 50:
                posts.append({
                    "url": url,
                    "title": content[:120],
                    "content": content,
                    "author": author,
                    "posted_at": None,
                    "metadata": {"platform": "facebook"}
                })
        except Exception as e:
            logger.warning(f"Failed to parse Facebook post: {e}")
            continue
    
    logger.info(f"Parsed {len(posts)} Facebook posts")
    return posts

def parse_tiktok_posts(html: str, base_url: str) -> List[Dict]:
    """Parse TikTok posts from HTML"""
    from bs4 import BeautifulSoup
    import re
    
    soup = BeautifulSoup(html, 'html.parser')
    posts = []
    
    # TikTok video container selectors
    selectors = [
        'div[data-e2e="feed-video"]',
        'div[data-e2e="user-post-item"]',
        'div.DivItemContainer',
        'div[class*="DivItemContainer"]',
        'article[class*="VideoCard"]',
    ]
    
    video_elements = []
    for sel in selectors:
        video_elements = soup.select(sel)
        if video_elements:
            logger.debug(f"Found {len(video_elements)} videos with selector: {sel}")
            break
    
    for elem in video_elements[:50]:
        try:
            link_elem = elem.find('a', href=re.compile(r'/video/\d+|/@[\w.]+/video/'))
            if not link_elem:
                link_elem = elem.find('a', href=True)
            
            url = link_elem['href'] if link_elem and link_elem.get('href') else base_url
            if url.startswith('/'):
                url = f"https://www.tiktok.com{url}"
            
            # Description
            desc_elem = (
                elem.find('div', class_=re.compile(r'Desc|desc|caption')) or
                elem.find('span', class_=re.compile(r'Desc|desc|caption')) or
                elem.find('div', {'data-e2e': re.compile(r'video-desc|description')})
            )
            content = desc_elem.get_text(strip=True) if desc_elem else ""
            
            # Author
            author_elem = (
                elem.find('a', class_=re.compile(r'Author|author|UserName')) or
                elem.find('span', class_=re.compile(r'Author|author|UserName')) or
                elem.find('a', {'data-e2e': re.compile(r'video-author|author')})
            )
            author = author_elem.get_text(strip=True) if author_elem else None
            
            # Hashtags from description
            hashtags = re.findall(r'#(\w+)', content)
            
            if content or author or url != base_url:
                posts.append({
                    "url": url,
                    "title": content[:120] or "TikTok Video",
                    "content": content,
                    "author": author,
                    "posted_at": None,
                    "metadata": {"platform": "tiktok", "hashtags": hashtags}
                })
        except Exception as e:
            logger.warning(f"Failed to parse TikTok post: {e}")
            continue
    
    logger.info(f"Parsed {len(posts)} TikTok posts")
    return posts

def parse_linkedin_posts(html: str, base_url: str) -> List[Dict]:
    """Parse LinkedIn job posts from HTML"""
    from bs4 import BeautifulSoup
    import re
    
    soup = BeautifulSoup(html, 'html.parser')
    posts = []
    
    # LinkedIn job card selectors
    selectors = [
        'li[data-occludable-job-id]',
        'div.job-search-card',
        'div.base-card',
        'div[data-test-job-card]',
        'ul.jobs-search__results-list > li',
    ]
    
    job_cards = []
    for sel in selectors:
        job_cards = soup.select(sel)
        if job_cards:
            logger.debug(f"Found {len(job_cards)} jobs with selector: {sel}")
            break
    
    for card in job_cards[:50]:
        try:
            # Title
            title_elem = (
                card.find('h3', class_=re.compile(r'base-search-card__title|job-title')) or
                card.find('a', class_=re.compile(r'job-title|base-card__full-link')) or
                card.find('h3')
            )
            title = title_elem.get_text(strip=True) if title_elem else ""
            
            # Company
            company_elem = (
                card.find('h4', class_=re.compile(r'base-search-card__subtitle|job-company')) or
                card.find('a', class_=re.compile(r'hidden-nested-link|job-company')) or
                card.find('h4')
            )
            company = company_elem.get_text(strip=True) if company_elem else ""
            
            # Location
            location_elem = (
                card.find('span', class_=re.compile(r'job-search-card__location|job-location')) or
                card.find('li', class_=re.compile(r'job-search-card__location'))
            )
            location = location_elem.get_text(strip=True) if location_elem else ""
            
            # URL
            link_elem = (
                card.find('a', class_=re.compile(r'base-card__full-link|job-link')) or
                card.find('a', href=re.compile(r'/jobs/view/|/jobs/'))
            )
            url = link_elem['href'] if link_elem and link_elem.get('href') else base_url
            if url.startswith('/'):
                url = f"https://www.linkedin.com{url}"
            
            # Description/snippet
            desc_elem = (
                card.find('div', class_=re.compile(r'job-search-card__description|job-snippet')) or
                card.find('p', class_=re.compile(r'job-snippet|description'))
            )
            content = desc_elem.get_text(strip=True) if desc_elem else f"{title} at {company}"
            
            # Posted date
            posted_elem = (
                card.find('time', class_=re.compile(r'job-search-card__listdate|posted-date')) or
                card.find('time')
            )
            posted_at = posted_elem.get('datetime') if posted_elem else None
            if not posted_at and posted_elem:
                posted_at = posted_elem.get_text(strip=True)
            
            # Job ID from data attribute
            job_id = card.get('data-occludable-job-id') or card.get('data-job-id')
            
            if title:
                posts.append({
                    "url": url,
                    "title": title,
                    "content": content,
                    "author": company,
                    "posted_at": posted_at,
                    "metadata": {
                        "platform": "linkedin",
                        "company": company,
                        "location": location,
                        "job_id": job_id
                    }
                })
        except Exception as e:
            logger.warning(f"Failed to parse LinkedIn post: {e}")
            continue
    
    logger.info(f"Parsed {len(posts)} LinkedIn posts")
    return posts

async def handle_command(command: Dict) -> Dict:
    """Handle incoming command from Tauri"""
    cmd = command.get("command")
    request_id = command.get("id")
    
    def make_response(success: bool, data: Any = None, error: Any = None) -> Dict:
        resp = {"success": success}
        if request_id is not None:
            resp["id"] = request_id
        if success:
            resp["data"] = data
        else:
            resp["error"] = error if isinstance(error, dict) else {"code": "UNKNOWN_ERROR", "message": str(error)}
        return resp
    
    try:
        if cmd == "login":
            platform = command.get("platform")
            url = command.get("url")
            
            if not platform or not url:
                return make_response(False, error={"code": "INVALID_PARAMS", "message": "platform and url required"})
            
            logger.info(f"Login request for {platform}")
            
            if platform == "facebook":
                cookies = await login_facebook(url)
            elif platform == "tiktok":
                cookies = await login_tiktok(url)
            elif platform == "linkedin":
                cookies = await login_linkedin(url)
            else:
                return make_response(False, error={"code": "UNSUPPORTED_PLATFORM", "message": f"Unknown platform: {platform}"})
            
            return make_response(True, {"type": "login", "cookies": cookies})
        
        elif cmd == "scrape":
            platform = command.get("platform")
            target = command.get("target")
            cookies = command.get("cookies", "")
            scroll_depth = command.get("scroll_depth", 5)
            
            if not platform or not target:
                return make_response(False, error={"code": "INVALID_PARAMS", "message": "platform and target required"})
            
            logger.info(f"Scrape request for {platform}, target: {target}, scroll_depth: {scroll_depth}")
            
            if platform == "facebook":
                items = await scrape_facebook(target, cookies, scroll_depth)
            elif platform == "tiktok":
                items = await scrape_tiktok(target, cookies, scroll_depth)
            elif platform == "linkedin":
                items = await scrape_linkedin(target, cookies, scroll_depth)
            else:
                return make_response(False, error={"code": "UNSUPPORTED_PLATFORM", "message": f"Unknown platform: {platform}"})
            
            return make_response(True, {"type": "scrape", "items": items})
        
        elif cmd == "ping":
            return make_response(True, {"type": "pong", "message": "sidecar alive"})
        
        else:
            return make_response(False, error={"code": "UNKNOWN_COMMAND", "message": f"Unknown command: {cmd}"})
    
    except ScraperError as e:
        logger.exception("Scraper error")
        return make_response(False, error=e.to_dict())
    except Exception as e:
        logger.exception("Command failed")
        return make_response(False, error={"code": "INTERNAL_ERROR", "message": str(e)})

async def main():
    """Main loop - read JSON commands from stdin, write responses to stdout"""
    logger.info("Python sidecar started")
    
    loop = asyncio.get_event_loop()
    reader = asyncio.StreamReader()
    protocol = asyncio.StreamReaderProtocol(reader)
    await loop.connect_read_pipe(lambda: protocol, sys.stdin)
    
    # Use direct stdout write for simplicity
    stdout = sys.stdout
    
    while True:
        try:
            line = await reader.readline()
            if not line:
                logger.info("Stdin closed, exiting")
                break
            
            line = line.decode('utf-8').strip()
            if not line:
                continue
            
            try:
                command = json.loads(line)
            except json.JSONDecodeError as e:
                logger.warning(f"Invalid JSON received: {line[:100]}")
                error_response = make_response(False, error={"code": "PARSE_ERROR", "message": "Invalid JSON"})
                print(json.dumps(error_response), flush=True)
                continue
            
            response = await handle_command(command)
            
            # Write response to stdout
            print(json.dumps(response), flush=True)
            
        except Exception as e:
            logger.exception("Error in main loop")
            error_response = make_response(False, error={"code": "INTERNAL_ERROR", "message": str(e)})
            print(json.dumps(error_response), flush=True)

def make_response(success: bool, data: Any = None, error: Any = None) -> Dict:
    resp = {"success": success}
    if success:
        resp["data"] = data
    else:
        resp["error"] = error if isinstance(error, dict) else {"code": "UNKNOWN_ERROR", "message": str(error)}
    return resp

if __name__ == "__main__":
    # Skip playwright install check - browsers are installed during build
    # (via build.rs or venv setup)
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("Sidecar stopped by user")
    except Exception as e:
        logger.exception("Fatal error")
        sys.exit(1)