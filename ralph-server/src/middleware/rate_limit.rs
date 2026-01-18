//! Rate limiting middleware for ralph-server
//!
//! This module provides rate limiting for API endpoints using the token bucket algorithm.
//! It protects the API from abuse by limiting requests per IP address based on configurable rate limits.

use axum::{
    extract::Request,
    extract::State,
    http::{HeaderMap, StatusCode, header::RETRY_AFTER},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Rate limiter configuration loaded from environment variables
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
        }
    }
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        let requests_per_minute = std::env::var("RATE_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        Self {
            requests_per_minute,
        }
    }
}

/// Token bucket for a single client IP
///
/// Uses the token bucket algorithm to rate limit requests.
/// Each bucket has a maximum capacity and refills at a constant rate.
#[derive(Debug)]
struct TokenBucket {
    tokens: u32,
    capacity: u32,
    last_refill: Instant,
    refill_rate: f64,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            last_refill: Instant::now(),
            refill_rate,
        }
    }

    /// Attempts to consume a token from the bucket
    ///
    /// Returns Ok(()) if a token was consumed, Err(retry_after) if rate limited.
    /// The retry_after value is the estimated time in seconds until the next token is available.
    fn consume(&mut self) -> Result<(), u64> {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            Ok(())
        } else {
            let time_until_token = (1.0 / self.refill_rate).ceil() as u64;
            Err(time_until_token)
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed();
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate) as u32;

        self.tokens = std::cmp::min(self.capacity, self.tokens + tokens_to_add);
        self.last_refill = Instant::now();
    }
}

/// In-memory rate limiter using token bucket algorithm
///
/// Tracks token buckets per client IP and enforces rate limits.
/// Thread-safe implementation using Arc<Mutex<>> for async compatibility.
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Creates a rate limiter with configuration from environment variables
    pub fn from_env() -> Self {
        Self::new(RateLimitConfig::from_env())
    }

    /// Checks if the client IP is rate limited
    ///
    /// Returns Ok(()) if the request is allowed, Err(retry_after) if rate limited.
    /// The retry_after value is in seconds.
    pub async fn check_rate_limit(&self, client_ip: &str) -> Result<(), u64> {
        let mut buckets = self.buckets.lock().await;

        let bucket = buckets.entry(client_ip.to_string()).or_insert_with(|| {
            let refill_rate = self.config.requests_per_minute as f64 / 60.0;
            TokenBucket::new(self.config.requests_per_minute, refill_rate)
        });

        bucket.consume()
    }
}

/// Extracts client IP address from request headers
fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(forwarded_for) = headers.get("x-forwarded-for")
        && let Ok(forwarded_str) = forwarded_for.to_str()
        && let Some(client_ip) = forwarded_str.split(',').next()
    {
        return client_ip.trim().to_string();
    }

    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(real_ip_str) = real_ip.to_str()
    {
        return real_ip_str.to_string();
    }

    "unknown".to_string()
}

/// Rate limiting middleware implementation
///
/// Checks if the client has exceeded their rate limit based on IP address.
/// Returns 429 (Too Many Requests) with Retry-After header if limit exceeded.
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let client_ip = extract_client_ip(request.headers());

    match rate_limiter.check_rate_limit(&client_ip).await {
        Ok(_) => Ok(next.run(request).await),
        Err(retry_after) => {
            tracing::warn!(client_ip = %client_ip, retry_after_seconds = retry_after, "Rate limit exceeded");

            let mut response = (StatusCode::TOO_MANY_REQUESTS, ()).into_response();
            response
                .headers_mut()
                .insert(RETRY_AFTER, retry_after.to_string().parse().unwrap());

            Ok(response)
        }
    }
}

/// Convenience function to create a rate limiter from environment variables
/// for use in router setup
pub fn create_rate_limiter_from_env() -> Arc<RateLimiter> {
    Arc::new(RateLimiter::from_env())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 100);
    }

    #[test]
    fn test_rate_limit_config_from_env() {
        unsafe {
            std::env::set_var("RATE_LIMIT", "50");
        }

        let config = RateLimitConfig::from_env();
        assert_eq!(config.requests_per_minute, 50);

        unsafe {
            std::env::remove_var("RATE_LIMIT");
        }
    }

    #[test]
    fn test_token_bucket_consume_tokens() {
        let mut bucket = TokenBucket::new(10, 1.0);

        for i in 0..10 {
            assert!(
                bucket.consume().is_ok(),
                "Token {} should be available",
                i + 1
            );
        }

        assert!(
            bucket.consume().is_err(),
            "11th token should be rate limited"
        );
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(5, 60.0);

        for _ in 0..5 {
            bucket.consume().unwrap();
        }

        assert!(bucket.consume().is_err());

        bucket.last_refill = Instant::now() - Duration::from_secs(60);
        bucket.refill();

        assert!(bucket.consume().is_ok());
    }

    #[test]
    fn test_token_bucket_capacity_limit() {
        let mut bucket = TokenBucket::new(3, 10.0);

        bucket.consume().unwrap();
        bucket.consume().unwrap();

        bucket.last_refill = Instant::now() - Duration::from_secs(10);
        bucket.refill();

        assert_eq!(bucket.tokens, 3);
    }

    #[test]
    fn test_token_bucket_retry_after() {
        let mut bucket = TokenBucket::new(2, 30.0);

        bucket.consume().unwrap();
        bucket.consume().unwrap();

        let retry_after = bucket.consume().expect_err("Should be rate limited");
        assert_eq!(retry_after, 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_enforces_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 3,
        });

        for i in 0..3 {
            assert!(
                limiter.check_rate_limit("client-1").await.is_ok(),
                "Request {} should succeed",
                i + 1
            );
        }

        assert!(
            limiter.check_rate_limit("client-1").await.is_err(),
            "4th request should be rate limited"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_different_clients() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 2,
        });

        for _ in 0..2 {
            assert!(limiter.check_rate_limit("client-1").await.is_ok());
        }

        assert!(limiter.check_rate_limit("client-1").await.is_err());

        for _ in 0..2 {
            assert!(limiter.check_rate_limit("client-2").await.is_ok());
        }

        assert!(limiter.check_rate_limit("client-2").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_refill_after_delay() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 2,
        });

        for _ in 0..2 {
            limiter.check_rate_limit("client-1").await.unwrap();
        }

        assert!(limiter.check_rate_limit("client-1").await.is_err());

        tokio::time::sleep(tokio::time::Duration::from_millis(61000)).await;

        assert!(limiter.check_rate_limit("client-1").await.is_ok());
    }

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "192.168.1.100, 10.0.0.1".parse().unwrap(),
        );

        let client_ip = extract_client_ip(&headers);
        assert_eq!(client_ip, "192.168.1.100");
    }

    #[test]
    fn test_extract_client_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "192.168.1.100".parse().unwrap());

        let client_ip = extract_client_ip(&headers);
        assert_eq!(client_ip, "192.168.1.100");
    }

    #[test]
    fn test_extract_client_ip_fallback() {
        let headers = HeaderMap::new();
        let client_ip = extract_client_ip(&headers);
        assert_eq!(client_ip, "unknown");
    }

    #[test]
    fn test_create_rate_limiter_from_env() {
        unsafe {
            std::env::set_var("RATE_LIMIT", "75");
        }

        let limiter = create_rate_limiter_from_env();
        assert_eq!(limiter.config.requests_per_minute, 75);

        unsafe {
            std::env::remove_var("RATE_LIMIT");
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::{Router, body::Body, routing::get};
    use tower::ServiceExt;

    fn create_test_app(rate_limiter: Arc<RateLimiter>) -> Router {
        Router::new().route("/test", get(test_handler)).route_layer(
            axum::middleware::from_fn_with_state(rate_limiter.clone(), rate_limit_middleware),
        )
    }

    async fn test_handler() -> &'static str {
        "Success"
    }

    #[tokio::test]
    async fn test_rate_limit_enforced() {
        let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig {
            requests_per_minute: 3,
        }));
        let app = create_test_app(rate_limiter);

        for i in 0..3 {
            let response = app
                .clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} should succeed",
                i + 1
            );
        }

        let response = app
            .clone()
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_429_includes_retry_after_header() {
        let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig {
            requests_per_minute: 2,
        }));
        let app = create_test_app(rate_limiter);

        for _ in 0..2 {
            app.clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();
        }

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = response.headers().get(RETRY_AFTER);
        assert!(
            retry_after.is_some(),
            "Retry-After header should be present"
        );
    }

    #[tokio::test]
    async fn test_rate_limit_resets_after_window() {
        let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig {
            requests_per_minute: 2,
        }));
        let app = create_test_app(rate_limiter);

        for _ in 0..2 {
            app.clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();
        }

        let response = app
            .clone()
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        tokio::time::sleep(tokio::time::Duration::from_millis(61000)).await;

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limit_from_env() {
        unsafe {
            std::env::set_var("RATE_LIMIT", "5");
        }

        let rate_limiter = Arc::new(RateLimiter::from_env());
        let app = create_test_app(rate_limiter);

        for i in 0..5 {
            let response = app
                .clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} should succeed",
                i + 1
            );
        }

        let response = app
            .clone()
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        unsafe {
            std::env::remove_var("RATE_LIMIT");
        }
    }
}
