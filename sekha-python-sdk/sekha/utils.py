"""
Utility functions for the SDK
"""

import asyncio
import time
from typing import Callable, Any, TypeVar
from functools import wraps
import json
from datetime import datetime

T = TypeVar("T")


def json_serializer(obj: Any) -> str:
    """Custom JSON serializer for common types"""
    if hasattr(obj, "dict"):
        return json.dumps(obj.dict(), default=str)
    return json.dumps(obj, default=str)


class RateLimiter:
    """Simple token bucket rate limiter"""

    def __init__(self, max_requests: int, window_seconds: float = 60.0):
        self.max_requests = max_requests
        self.window_seconds = window_seconds
        self.requests: list[float] = []
        self._lock = asyncio.Lock()

    async def acquire(self):
        """Acquire a token, waiting if necessary"""
        async with self._lock:
            now = time.time()

            # Remove old requests outside window
            self.requests = [
                req for req in self.requests if now - req < self.window_seconds
            ]

            # If at limit, wait
            if len(self.requests) >= self.max_requests:
                wait_time = self.window_seconds - (now - self.requests[0])
                if wait_time > 0:
                    await asyncio.sleep(wait_time)

            # Record this request
            self.requests.append(now)


class ExponentialBackoff:
    """Exponential backoff with jitter"""

    def __init__(
        self, base_delay: float = 0.1, max_delay: float = 60.0, factor: float = 2.0
    ):
        self.base_delay = base_delay
        self.max_delay = max_delay
        self.factor = factor
        self.attempt = 0

    async def wait(self):
        """Wait for the next backoff period"""
        delay = min(self.base_delay * (self.factor**self.attempt), self.max_delay)
        # Add jitter to prevent thundering herd
        jitter = delay * 0.1 * (hash(id(asyncio.current_task())) % 10) / 10
        await asyncio.sleep(delay + jitter)
        self.attempt += 1

    def reset(self):
        """Reset the backoff counter"""
        self.attempt = 0


def validate_api_key(api_key: str) -> bool:
    """Validate API key format (basic check)"""
    if not api_key:
        raise ValueError("API key is required")

    if len(api_key) < 20:
        raise ValueError("API key appears to be too short")

    if not api_key.startswith("sk-sekha-"):
        raise ValueError("API key must start with 'sk-sekha-'")

    return True


def parse_iso_datetime(dt_str: str) -> datetime:
    """Parse ISO 8601 datetime string to datetime object"""
    # Try with microseconds first
    try:
        return datetime.fromisoformat(dt_str.replace("Z", "+00:00"))
    except ValueError:
        # Fallback to without microseconds
        return datetime.strptime(dt_str, "%Y-%m-%d %H:%M:%S")


def format_bytes(n: int) -> str:
    """Format bytes to human readable"""
    for unit in ["B", "KB", "MB", "GB"]:
        if n < 1024.0:
            return f"{n:.1f} {unit}"
        # Convert n to float for calculations and return formatted string
        size = float(n)
        for unit in ["B", "KB", "MB", "GB"]:
            if size < 1024.0:
                return f"{size:.1f} {unit}"
            size /= 1024.0
        return f"{size:.1f} TB"
    return f"{n:.1f} TB"
