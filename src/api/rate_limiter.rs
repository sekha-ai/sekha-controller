//! Rate limiting middleware for REST API

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiter instance
pub type RateLimiter = Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a new rate limiter with specified requests per minute
pub fn create_rate_limiter(requests_per_minute: u32) -> RateLimiter {
    let quota = Quota::per_minute(
        NonZeroU32::new(requests_per_minute).expect("Rate limit must be > 0")
    );
    Arc::new(GovernorRateLimiter::direct(quota))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    limiter: RateLimiter,
    request: Request,
    next: Next,
) -> Response {
    // Check rate limit
    match limiter.check() {
        Ok(_) => {
            // Allow request
            next.run(request).await
        }
        Err(_) => {
            // Rate limit exceeded
            (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded. Please try again later.",
            ).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = create_rate_limiter(1000);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limit_exhaustion() {
        let limiter = create_rate_limiter(2);
        
        // First two requests should succeed
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        
        // Third request should fail
        assert!(limiter.check().is_err());
    }
}
