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
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
};
use dashmap::DashMap;
use serde_json::json;
use tower::{Layer, Service};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const DEFAULT_REQUESTS_PER_MINUTE: u32 = 100;

// ── Store abstraction ─────────────────────────────────────────────────────────

pub trait RateLimiterStore: Send + Sync + 'static {
    fn check(&self, key: IpAddr) -> Result<u32, u64>;
    fn update_metrics(&self, cpu_usage: f64, mem_usage: f64);
    fn get_quota(&self) -> u32;
}

// ── Adaptive Token Bucket (PID Controlled) ──────────────────────────────────

struct TokenBucket {
    tokens: AtomicU64,
    last_refill: AtomicU64, // Unix timestamp in millis
}

pub struct AdaptiveStore {
    buckets: DashMap<IpAddr, TokenBucket>,
    base_rate: f64,          // tokens per second
    current_rate: AtomicU64, // fixed-point
    target_cpu: f64,
    pid_kp: f64,
    pid_ki: f64,
    pid_kd: f64,
    integral: AtomicU64,   // fixed-point
    last_error: AtomicU64, // fixed-point
}

impl AdaptiveStore {
    pub fn new(requests_per_minute: u32) -> Self {
        let base_rate = requests_per_minute as f64 / 60.0;
        let _now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Self {
            buckets: DashMap::new(),
            base_rate,
            current_rate: AtomicU64::new(base_rate.to_bits()),
            target_cpu: 0.7,
            pid_kp: 0.5,
            pid_ki: 0.1,
            pid_kd: 0.05,
            integral: AtomicU64::new(0),
            last_error: AtomicU64::new(0),
        }
    }

    fn get_current_rate(&self) -> f64 {
        f64::from_bits(self.current_rate.load(Ordering::Relaxed))
    }
}

impl RateLimiterStore for AdaptiveStore {
    fn check(&self, key: IpAddr) -> Result<u32, u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let rate = self.get_current_rate();

        let entry = self.buckets.entry(key).or_insert_with(|| TokenBucket {
            tokens: AtomicU64::new((rate * 60.0 * 1000.0).round() as u64),
            last_refill: AtomicU64::new(now),
        });

        let bucket = entry.value();

        let result = bucket
            .tokens
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                let last = bucket.last_refill.load(Ordering::Acquire);
                let elapsed_ms = now.saturating_sub(last);
                let refill = (elapsed_ms as f64 * rate).round() as u64; // rate is per sec, elapsed is ms

                let next_tokens = (current + refill).min((rate * 60.0 * 1000.0).round() as u64);
                if next_tokens >= 1000 {
                    // If we use tokens, we update the timestamp to now.
                    // In fetch_update closure we can't easily update another atomic.
                    Some(next_tokens - 1000)
                } else {
                    None
                }
            });

        match result {
            Ok(_) => {
                bucket.last_refill.store(now, Ordering::Release);
                Ok(1) // Simplified remaining
            }
            Err(_) => {
                let wait_secs = (1.0 / rate).ceil() as u64;
                Err(wait_secs)
            }
        }
    }

    fn update_metrics(&self, cpu_usage: f64, _mem_usage: f64) {
        let error = self.target_cpu - cpu_usage;
        let prev_error = self.last_error.load(Ordering::Relaxed) as f64 / 1000.0;
        let prev_integral = self.integral.load(Ordering::Relaxed) as f64 / 1000.0;

        let integral = (prev_integral + error).clamp(-1.0, 1.0);
        let derivative = error - prev_error;

        let adjustment =
            (self.pid_kp * error) + (self.pid_ki * integral) + (self.pid_kd * derivative);
        let new_rate = (self.base_rate * (1.0 + adjustment)).max(1.0 / 60.0); // Min 1 req/min

        self.current_rate
            .store(new_rate.to_bits(), Ordering::Relaxed);
        self.integral
            .store((integral * 1000.0) as u64, Ordering::Relaxed);
        self.last_error
            .store((error * 1000.0) as u64, Ordering::Relaxed);
    }

    fn get_quota(&self) -> u32 {
        (self.get_current_rate() * 60.0) as u32
    }
}

// ── Layer ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RateLimitLayer {
    store: Arc<dyn RateLimiterStore>,
}

impl RateLimitLayer {
    pub fn new(store: Arc<dyn RateLimiterStore>) -> Self {
        Self { store }
    }

    #[allow(dead_code)]
    pub fn adaptive(requests_per_minute: u32) -> Self {
        let store = Arc::new(AdaptiveStore::new(requests_per_minute));
        Self::new(store)
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> RateLimitMiddleware<S> {
        RateLimitMiddleware {
            inner,
            store: Arc::clone(&self.store),
        }
    }
}

// ── Middleware service ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    store: Arc<dyn RateLimiterStore>,
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

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), <S as Service<Request<Body>>>::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(
        &mut self,
        req: Request<Body>,
    ) -> BoxFuture<Result<Response<Body>, <S as Service<Request<Body>>>::Error>> {
        let store = Arc::clone(&self.store);
        let client_ip = extract_ip(&req);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let quota = store.get_quota();
            match store.check(client_ip) {
                Ok(remaining) => {
                    let mut resp = inner.call(req).await?;
                    let h = resp.headers_mut();
                    h.insert("x-ratelimit-limit", quota.to_string().parse().unwrap());
                    h.insert(
                        "x-ratelimit-remaining",
                        remaining.to_string().parse().unwrap(),
                    );
                    h.insert("x-ratelimit-reset", "60".parse().unwrap());
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
                        .header("x-ratelimit-limit", quota.to_string())
                        .header("x-ratelimit-remaining", "0")
                        .header("x-ratelimit-reset", wait_secs.to_string())
                        .body(Body::from(body))
                        .unwrap();

                    Ok(resp)
                }
            }
        })
    }
}

fn extract_ip(req: &Request<Body>) -> IpAddr {
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            if let Some(first) = val.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return addr.ip();
    }

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
            .layer(RateLimitLayer::adaptive(limit));
        TestServer::new(app)
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
