//! Sliding-window rate limiter middleware for the Axum API.
//!
//! Uses `governor`'s GCRA algorithm (a leaky-bucket variant that closely
//! approximates a sliding window) keyed on the client IP address.
//!
//! Every response gets three informational headers:
//!   - `X-RateLimit-Limit`     – total quota per window
//!   - `X-RateLimit-Remaining` – estimated remaining calls in the current window
//!   - `X-RateLimit-Reset`     – seconds until the quota fully replenishes
//!
//! When the quota is exhausted the middleware short-circuits with
//! `429 Too Many Requests` and a JSON error body.
//!
//! # Future Redis migration
//! `RateLimiterStore` is the seam for swapping the in-memory store for a
//! Redis-backed one without touching any middleware logic.

use std::{
    future::Future,
    net::IpAddr,
    num::NonZeroU32,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
};
use governor::{
    clock::{Clock, DefaultClock},
    middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
    Quota, RateLimiter,
};
use serde_json::json;
use tower::{Layer, Service};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default quota: 100 requests per 60-second window.
pub const DEFAULT_REQUESTS_PER_MINUTE: u32 = 100;

// ── Store abstraction (seam for Redis migration) ──────────────────────────────

/// Abstracts the rate-limit state store.
///
/// `Ok(remaining)` → request is within quota, `remaining` is an estimate of
/// how many more calls are allowed right now.
/// `Err(wait_secs)` → quota exceeded; caller should wait this many seconds.
pub trait RateLimiterStore: Send + Sync + 'static {
    fn check(&self, key: IpAddr) -> Result<u32, u64>;
}

// ── In-memory store ───────────────────────────────────────────────────────────

type GovernorLimiter =
    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock, NoOpMiddleware>;

pub struct InMemoryStore {
    limiter: GovernorLimiter,
    quota_per_minute: u32,
    clock: DefaultClock,
}

impl InMemoryStore {
    pub fn new(requests_per_minute: u32) -> Self {
        let rpm = NonZeroU32::new(requests_per_minute).expect("quota must be > 0");
        let quota = Quota::per_minute(rpm);
        Self {
            limiter: RateLimiter::keyed(quota),
            quota_per_minute: requests_per_minute,
            clock: DefaultClock::default(),
        }
    }
}

impl RateLimiterStore for InMemoryStore {
    fn check(&self, key: IpAddr) -> Result<u32, u64> {
        match self.limiter.check_key(&key) {
            Ok(_snapshot) => {
                // Estimate remaining without consuming additional tokens:
                // try check_key_n for decreasing n until one succeeds.
                // This is a read-only probe — we don't call check_key again.
                let remaining =
                    estimate_remaining(&self.limiter, key, self.quota_per_minute);
                Ok(remaining)
            }
            Err(not_until) => {
                let wait = not_until
                    .wait_time_from(self.clock.now())
                    .as_secs()
                    .max(1);
                Err(wait)
            }
        }
    }
}

/// Estimate how many more requests would be accepted right now without
/// consuming any tokens.
///
/// `governor`'s `check_key_n` is non-destructive when it returns `Err`
/// (the cell is not modified on failure).  We binary-search downward from
/// `quota` to find the largest `n` that would still be accepted.
fn estimate_remaining(limiter: &GovernorLimiter, key: IpAddr, quota: u32) -> u32 {
    if quota == 0 {
        return 0;
    }
    let mut lo = 0u32;
    let mut hi = quota;
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        // SAFETY: mid >= 1 because lo starts at 0 and we add at least 1.
        let n = NonZeroU32::new(mid).unwrap();
        // check_key_n does NOT modify state on Err — safe to probe.
        if limiter.check_key_n(&key, n).is_ok() {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

// ── Layer ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RateLimitLayer {
    store: Arc<dyn RateLimiterStore>,
    quota: u32,
    window_secs: u64,
}

impl RateLimitLayer {
    pub fn new(store: Arc<dyn RateLimiterStore>, quota: u32, window_secs: u64) -> Self {
        Self { store, quota, window_secs }
    }

    /// Convenience constructor using the default in-memory store.
    pub fn in_memory(requests_per_minute: u32) -> Self {
        let store = Arc::new(InMemoryStore::new(requests_per_minute));
        Self::new(store, requests_per_minute, 60)
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            store: Arc::clone(&self.store),
            quota: self.quota,
            window_secs: self.window_secs,
        }
    }
}

// ── Middleware service ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    store: Arc<dyn RateLimiterStore>,
    quota: u32,
    window_secs: u64,
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + Clone + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let store = Arc::clone(&self.store);
        let quota = self.quota;
        let window_secs = self.window_secs;
        let client_ip = extract_ip(&req);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match store.check(client_ip) {
                Ok(remaining) => {
                    let mut resp = inner.call(req).await?;
                    let h = resp.headers_mut();
                    h.insert("x-ratelimit-limit",   quota.to_string().parse().unwrap());
                    h.insert("x-ratelimit-remaining", remaining.to_string().parse().unwrap());
                    h.insert("x-ratelimit-reset",   window_secs.to_string().parse().unwrap());
                    Ok(resp)
                }
                Err(wait_secs) => {
                    let body = json!({
                        "error": "Rate limit exceeded. Please try again later."
                    })
                    .to_string();

                    let resp = Response::builder()
                        .status(StatusCode::TOO_MANY_REQUESTS)
                        .header("content-type", "application/json")
                        .header("x-ratelimit-limit",     quota.to_string())
                        .header("x-ratelimit-remaining", "0")
                        .header("x-ratelimit-reset",     wait_secs.to_string())
                        .body(Body::from(body))
                        .unwrap();

                    Ok(resp)
                }
            }
        })
    }
}

// ── IP extraction ─────────────────────────────────────────────────────────────

fn extract_ip(req: &Request<Body>) -> IpAddr {
    // 1. X-Forwarded-For (first entry — closest client behind a proxy)
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            if let Some(first) = val.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // 2. ConnectInfo set by axum::serve (requires make_into_service_with_connect_info)
    if let Some(ConnectInfo(addr)) = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
    {
        return addr.ip();
    }

    // 3. Fallback: loopback (e.g. in tests without ConnectInfo)
    IpAddr::from([127, 0, 0, 1])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use axum_test::TestServer;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn test_app(limit: u32) -> TestServer {
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(RateLimitLayer::in_memory(limit));
        TestServer::new(app).unwrap()
    }

    /// Sends `limit` requests (all must succeed) then asserts the next one
    /// returns 429 with the correct JSON body and headers.
    #[tokio::test]
    async fn test_rate_limiting_trigger() {
        let limit = 5u32;
        let server = test_app(limit);

        for i in 0..limit {
            let resp = server.get("/").await;
            assert_eq!(
                resp.status_code(),
                StatusCode::OK,
                "request {i} should be allowed"
            );
            assert!(
                resp.headers().contains_key("x-ratelimit-limit"),
                "missing X-RateLimit-Limit on request {i}"
            );
            assert!(
                resp.headers().contains_key("x-ratelimit-remaining"),
                "missing X-RateLimit-Remaining on request {i}"
            );
            assert!(
                resp.headers().contains_key("x-ratelimit-reset"),
                "missing X-RateLimit-Reset on request {i}"
            );
        }

        // The (limit + 1)th request must be rejected.
        let resp = server.get("/").await;
        assert_eq!(
            resp.status_code(),
            StatusCode::TOO_MANY_REQUESTS,
            "request {} should be rate-limited (429)",
            limit + 1
        );
        assert_eq!(
            resp.headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok()),
            Some("0"),
            "X-RateLimit-Remaining should be 0 on a 429"
        );
        let body: serde_json::Value = resp.json();
        assert_eq!(
            body["error"],
            "Rate limit exceeded. Please try again later."
        );
    }
}
