#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check_no_auth() {
        // Health check should not require auth
        let app = create_test_app();
        
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_requires_auth() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/conversations")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_with_valid_auth() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/conversations")
                    .header(header::AUTHORIZATION, "Bearer test_key_12345678901234567890123456789012")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        // Should not be 401 Unauthorized
        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        // Create limiter with very low limit for testing
        let limiter = create_rate_limiter(2);
        
        // First request should succeed
        assert!(limiter.check().is_ok());
        
        // Second request should succeed
        assert!(limiter.check().is_ok());
        
        // Third request should fail (rate limited)
        assert!(limiter.check().is_err());
    }
}
