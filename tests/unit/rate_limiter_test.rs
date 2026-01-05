use sekha_controller::api::rate_limiter::RateLimiter;
use std::net::IpAddr;
use std::time::Duration;

#[tokio::test]
async fn test_rate_limiter_new() {
    let _limiter = RateLimiter::new(60); // 60 requests per minute
    // Should initialize successfully
}

#[tokio::test]
async fn test_rate_limiter_allows_first_request() {
    let limiter = RateLimiter::new(60);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    let allowed = limiter.check_rate_limit(ip).await;
    assert!(allowed);
}

#[tokio::test]
async fn test_rate_limiter_blocks_excess_requests() {
    let limiter = RateLimiter::new(2); // Only 2 requests
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    assert!(limiter.check_rate_limit(ip).await);
    assert!(limiter.check_rate_limit(ip).await);
    
    // Third request should be blocked
    let allowed = limiter.check_rate_limit(ip).await;
    assert!(!allowed);
}

#[tokio::test]
async fn test_rate_limiter_different_ips() {
    let limiter = RateLimiter::new(1); // Only 1 request per IP
    
    let ip1: IpAddr = "127.0.0.1".parse().unwrap();
    let ip2: IpAddr = "127.0.0.2".parse().unwrap();
    
    assert!(limiter.check_rate_limit(ip1).await);
    assert!(limiter.check_rate_limit(ip2).await); // Different IP
}

#[tokio::test]
async fn test_rate_limiter_reset_after_window() {
    let limiter = RateLimiter::new(1);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    assert!(limiter.check_rate_limit(ip).await);
    
    // Wait for window to reset (61 seconds to be safe)
    tokio::time::sleep(Duration::from_secs(61)).await;
    
    // Should allow again after window
    assert!(limiter.check_rate_limit(ip).await);
}

#[test]
fn test_rate_limit_calculation() {
    let per_minute = 60;
    let per_second = per_minute / 60;
    
    assert_eq!(per_second, 1);
}

#[test]
fn test_ip_address_parsing() {
    let valid_ips = vec!["127.0.0.1", "192.168.1.1", "10.0.0.1", "::1"];
    
    for ip_str in valid_ips {
        let parsed: Result<IpAddr, _> = ip_str.parse();
        assert!(parsed.is_ok());
    }
}
