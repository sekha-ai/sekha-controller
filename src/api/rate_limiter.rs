//! Rate limiting middleware for REST API

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter state tracking requests per IP
#[derive(Clone)]
pub struct RateLimiter {
    /// Maximum requests per minute
    max_requests: u32,
    /// Request tracking: IP -> (count, window_start)
    requests: Arc<RwLock<HashMap<IpAddr, (u32, Instant)>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            max_requests: requests_per_minute,
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if request is allowed for given IP
    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        match requests.get_mut(&ip) {
            Some((count, start)) => {
                // Check if window has expired
                if now.duration_since(*start) > window {
                    // Reset window
                    *count = 1;
                    *start = now;
                    true
                } else if *count < self.max_requests {
                    // Increment counter
                    *count += 1;
                    true
                } else {
                    // Rate limit exceeded
                    false
                }
            }
            None => {
                // First request from this IP
                requests.insert(ip, (1, now));
                true
            }
        }
    }

    /// Clean up expired entries (call periodically)
    pub async fn cleanup_expired(&self) {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        requests.retain(|_, (_, start)| now.duration_since(*start) <= window);
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    // Extract client IP
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

    // Check rate limit
    if limiter.check_rate_limit(ip).await {
        // Allow request
        next.run(request).await
    } else {
        // Rate limit exceeded
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_requests() {
        let limiter = RateLimiter::new(10);
        let ip = "127.0.0.1".parse().unwrap();

        // First 10 requests should succeed
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(ip).await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess() {
        let limiter = RateLimiter::new(2);
        let ip = "127.0.0.1".parse().unwrap();

        // First 2 requests succeed
        assert!(limiter.check_rate_limit(ip).await);
        assert!(limiter.check_rate_limit(ip).await);

        // Third request should fail
        assert!(!limiter.check_rate_limit(ip).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_per_ip() {
        let limiter = RateLimiter::new(2);
        let ip1 = "127.0.0.1".parse().unwrap();
        let ip2 = "192.168.1.1".parse().unwrap();

        // IP1 uses its quota
        assert!(limiter.check_rate_limit(ip1).await);
        assert!(limiter.check_rate_limit(ip1).await);
        assert!(!limiter.check_rate_limit(ip1).await);

        // IP2 should still have quota
        assert!(limiter.check_rate_limit(ip2).await);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let limiter = RateLimiter::new(100);
        let ip = "127.0.0.1".parse().unwrap();

        // Make request
        assert!(limiter.check_rate_limit(ip).await);
        assert_eq!(limiter.requests.read().await.len(), 1);

        // Cleanup should not remove recent entries
        limiter.cleanup_expired().await;
        assert_eq!(limiter.requests.read().await.len(), 1);
    }
}
