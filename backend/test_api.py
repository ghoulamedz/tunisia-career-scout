#!/usr/bin/env python3
"""Start the backend server and test it."""

import subprocess
import time
import sys
import os
import signal
import urllib.request
import json
import time

env = os.environ.copy()
env["DATABASE_URL"] = "postgresql+asyncpg://easyerp:easyerp@localhost:55432/easyerp"
env["PYTHONPATH"] = "/home/medo/Desktop/projects/tunisia-career-scout/backend"

proc = subprocess.Popen(
    [sys.executable, "-m", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000", "--log-level", "error"],
    env=env,
    cwd="/home/medo/Desktop/projects/tunisia-career-scout/backend",
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

time.sleep(3)

try:
    # Health check
    with urllib.request.urlopen("http://localhost:8000/health") as resp:
        print("Health:", json.loads(resp.read()))

    # Register
    data = json.dumps({"email": f"test{int(time.time())}@example.com", "password": "test123456", "full_name": "Test User"}).encode()
    req = urllib.request.Request("http://localhost:8000/api/auth/register", data=data, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as resp:
        result = json.loads(resp.read())
        print("Register:", result)
        token = result["access_token"]

    # Sources
    with urllib.request.urlopen("http://localhost:8000/api/postings/sources") as resp:
        print("Sources:", json.loads(resp.read()))

    # Sources with auth
    req = urllib.request.Request("http://localhost:8000/api/postings/sources", headers={"Authorization": f"Bearer {token}"})
    with urllib.request.urlopen(req) as resp:
        print("Sources (auth):", json.loads(resp.read()))

    print("\nALL TESTS PASSED")
except Exception as e:
    print(f"ERROR: {e}")
finally:
    proc.terminate()
    proc.wait(timeout=5)
