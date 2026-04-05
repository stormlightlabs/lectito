use std::cmp::Ordering;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Response};
use axum::middleware::Next;
use axum::response::IntoResponse;
use deadpool_postgres::Pool;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use tokio::sync::RwLock;
use tracing::warn;

use crate::AppState;
use crate::error::AppError;

const HEADER_LIMIT: &str = "x-ratelimit-limit";
const HEADER_REMAINING: &str = "x-ratelimit-remaining";
const HEADER_RESET: &str = "x-ratelimit-reset";
pub const WEB_CLIENT_HEADER: &str = "x-lectito-client";
pub const WEB_CLIENT_WEB_APP: &str = "web-app";

type RateLimitBucketStore = Arc<RwLock<HashMap<(IpAddr, i64, u32), u32>>>;

#[derive(Debug, Clone, Copy)]
struct RateLimitWindow {
    window_seconds: u32,
    limit: u32,
}

#[derive(Debug, Clone)]
pub struct RateLimitWindowState {
    pub window_seconds: u32,
    pub limit: u32,
    pub request_count: u32,
    pub remaining: u32,
    pub reset_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct RateLimitSnapshot {
    windows: Vec<RateLimitWindowState>,
}

#[derive(Debug, Clone)]
pub struct ClientRateLimitContext {
    pub client_ip: IpAddr,
    pub snapshot: RateLimitSnapshot,
    pub exempt: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimitsResponse {
    pub requests_remaining: u32,
    pub requests_limit: u32,
    pub window_seconds: u32,
    pub reset_at: String,
}

#[derive(Clone, Default)]
struct InMemoryRateLimitStore {
    buckets: RateLimitBucketStore,
}

#[derive(Clone)]
pub struct RateLimiter {
    trust_proxy_headers: bool,
    windows: [RateLimitWindow; 3],
    fallback_store: InMemoryRateLimitStore,
}

impl RateLimiter {
    #[must_use]
    pub fn new(config: &crate::Config) -> Self {
        Self {
            trust_proxy_headers: config.trust_proxy_headers,
            windows: [
                RateLimitWindow { window_seconds: 60, limit: config.rate_limit_per_min },
                RateLimitWindow { window_seconds: 60 * 60, limit: config.rate_limit_per_hour },
                RateLimitWindow { window_seconds: 60 * 60 * 24, limit: config.rate_limit_per_day },
            ],
            fallback_store: InMemoryRateLimitStore::default(),
        }
    }

    pub async fn check_and_increment(&self, pool: &Pool, ip: IpAddr) -> RateLimitSnapshot {
        match self.check_and_increment_db(pool, ip).await {
            Ok(snapshot) => snapshot,
            Err(message) => {
                warn!("rate limit DB unavailable, using in-memory fallback: {message}");
                self.check_and_increment_in_memory(ip).await
            }
        }
    }

    pub async fn current_snapshot(&self, pool: &Pool, ip: IpAddr) -> RateLimitSnapshot {
        match self.current_snapshot_db(pool, ip).await {
            Ok(snapshot) => snapshot,
            Err(message) => {
                warn!("rate limit DB unavailable while reading limits, using in-memory fallback: {message}");
                self.current_snapshot_in_memory(ip).await
            }
        }
    }

    #[must_use]
    pub fn client_ip_from_request(&self, request: &Request) -> IpAddr {
        if self.trust_proxy_headers
            && let Some(ip) = forwarded_ip(request.headers())
        {
            return ip;
        }

        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|connect_info| connect_info.0.ip())
            .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]))
    }

    async fn check_and_increment_db(&self, pool: &Pool, ip: IpAddr) -> Result<RateLimitSnapshot, String> {
        let client = pool
            .get()
            .await
            .map_err(|err| format!("failed to get DB connection: {err}"))?;
        let now = OffsetDateTime::now_utc();
        let mut states = Vec::with_capacity(self.windows.len());

        for window in self.windows {
            let start = window_start(now, window.window_seconds);
            let row = client
                .query_one(
                    "INSERT INTO rate_limits (ip, window_start, window_seconds, request_count)
                     VALUES ($1, $2, $3, 1)
                     ON CONFLICT (ip, window_start, window_seconds)
                     DO UPDATE SET request_count = rate_limits.request_count + 1
                     RETURNING request_count",
                    &[&ip, &start, &(window.window_seconds as i32)],
                )
                .await
                .map_err(|err| format!("failed to update rate limit row: {err}"))?;
            let request_count = row.get::<_, i32>("request_count") as u32;
            states.push(build_window_state(window, request_count, start));
        }

        Ok(RateLimitSnapshot { windows: states })
    }

    async fn current_snapshot_db(&self, pool: &Pool, ip: IpAddr) -> Result<RateLimitSnapshot, String> {
        let client = pool
            .get()
            .await
            .map_err(|err| format!("failed to get DB connection: {err}"))?;
        let now = OffsetDateTime::now_utc();
        let mut states = Vec::with_capacity(self.windows.len());

        for window in self.windows {
            let start = window_start(now, window.window_seconds);
            let row = client
                .query_opt(
                    "SELECT request_count
                     FROM rate_limits
                     WHERE ip = $1 AND window_start = $2 AND window_seconds = $3",
                    &[&ip, &start, &(window.window_seconds as i32)],
                )
                .await
                .map_err(|err| format!("failed to read rate limit row: {err}"))?;
            let request_count = row.map_or(0, |row| row.get::<_, i32>("request_count") as u32);
            states.push(build_window_state(window, request_count, start));
        }

        Ok(RateLimitSnapshot { windows: states })
    }

    async fn check_and_increment_in_memory(&self, ip: IpAddr) -> RateLimitSnapshot {
        let now = OffsetDateTime::now_utc();
        let mut buckets = self.fallback_store.buckets.write().await;
        prune_expired(&mut buckets, now);

        let mut states = Vec::with_capacity(self.windows.len());
        for window in self.windows {
            let start = window_start(now, window.window_seconds);
            let key = (ip, start.unix_timestamp(), window.window_seconds);
            let count = buckets.entry(key).and_modify(|count| *count += 1).or_insert(1);
            states.push(build_window_state(window, *count, start));
        }

        RateLimitSnapshot { windows: states }
    }

    async fn current_snapshot_in_memory(&self, ip: IpAddr) -> RateLimitSnapshot {
        let now = OffsetDateTime::now_utc();
        let mut buckets = self.fallback_store.buckets.write().await;
        prune_expired(&mut buckets, now);

        let mut states = Vec::with_capacity(self.windows.len());
        for window in self.windows {
            let start = window_start(now, window.window_seconds);
            let count = buckets
                .get(&(ip, start.unix_timestamp(), window.window_seconds))
                .copied()
                .unwrap_or(0);
            states.push(build_window_state(window, count, start));
        }

        RateLimitSnapshot { windows: states }
    }
}

impl RateLimitSnapshot {
    #[must_use]
    pub fn is_limited(&self) -> bool {
        self.windows.iter().any(|window| window.request_count > window.limit)
    }

    #[must_use]
    pub fn retry_after_seconds(&self) -> u32 {
        let now = OffsetDateTime::now_utc();
        self.windows
            .iter()
            .filter(|window| window.request_count > window.limit)
            .map(|window| (window.reset_at - now).whole_seconds().max(1) as u32)
            .min()
            .unwrap_or(1)
    }

    #[must_use]
    pub fn as_limits_response(&self) -> LimitsResponse {
        let window = self.header_window();
        LimitsResponse {
            requests_remaining: window.remaining,
            requests_limit: window.limit,
            window_seconds: window.window_seconds,
            reset_at: window
                .reset_at
                .format(&Rfc3339)
                .unwrap_or_else(|_| window.reset_at.to_string()),
        }
    }

    pub fn apply_headers(&self, headers: &mut HeaderMap) {
        let window = self.header_window();
        insert_header(headers, HEADER_LIMIT, &window.limit);
        insert_header(headers, HEADER_REMAINING, &window.remaining);
        insert_header(headers, HEADER_RESET, &window.reset_at.unix_timestamp());
    }

    fn header_window(&self) -> &RateLimitWindowState {
        self.windows
            .iter()
            .find(|window| window.request_count > window.limit)
            .unwrap_or_else(|| {
                self.windows
                    .iter()
                    .min_by(compare_window_pressure)
                    .expect("rate limit snapshot has at least one window")
            })
    }
}

pub async fn middleware(State(state): State<AppState>, mut request: Request, next: Next) -> Response<Body> {
    let ip = state.rate_limiter.client_ip_from_request(&request);
    let exempt = is_web_app_request(request.headers());

    if let Err(error) = state.spam_filter.check_ip_ban(&state.pool, ip).await {
        let snapshot = state.rate_limiter.current_snapshot(&state.pool, ip).await;
        let mut response = error.into_response();
        snapshot.apply_headers(response.headers_mut());
        return response;
    }

    if exempt {
        let snapshot = state.rate_limiter.current_snapshot(&state.pool, ip).await;
        request
            .extensions_mut()
            .insert(ClientRateLimitContext { client_ip: ip, snapshot, exempt: true });
        return next.run(request).await;
    }

    let snapshot = state.rate_limiter.check_and_increment(&state.pool, ip).await;
    if snapshot.is_limited() {
        state.spam_filter.record_rate_limit_violation(&state.pool, ip).await;
        let mut response = AppError::TooManyRequests { retry_after: snapshot.retry_after_seconds() }.into_response();
        snapshot.apply_headers(response.headers_mut());
        return response;
    }

    state.spam_filter.clear_rate_limit_violations(ip).await;
    request.extensions_mut().insert(ClientRateLimitContext {
        client_ip: ip,
        snapshot: snapshot.clone(),
        exempt: false,
    });

    let mut response = next.run(request).await;
    snapshot.apply_headers(response.headers_mut());
    response
}

#[must_use]
pub fn is_web_app_request(headers: &HeaderMap) -> bool {
    headers
        .get(WEB_CLIENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(WEB_CLIENT_WEB_APP))
}

fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|value| value.to_str().ok()) {
        for part in value.split(',') {
            if let Ok(ip) = part.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<IpAddr>().ok())
}

fn build_window_state(window: RateLimitWindow, request_count: u32, start: OffsetDateTime) -> RateLimitWindowState {
    RateLimitWindowState {
        window_seconds: window.window_seconds,
        limit: window.limit,
        request_count,
        remaining: window.limit.saturating_sub(request_count),
        reset_at: start + Duration::seconds(window.window_seconds as i64),
    }
}

fn prune_expired(buckets: &mut HashMap<(IpAddr, i64, u32), u32>, now: OffsetDateTime) {
    buckets.retain(|(_, start, seconds), _| *start + i64::from(*seconds) > now.unix_timestamp());
}

fn window_start(now: OffsetDateTime, window_seconds: u32) -> OffsetDateTime {
    let start = now.unix_timestamp() - (now.unix_timestamp() % i64::from(window_seconds));
    OffsetDateTime::from_unix_timestamp(start).expect("window start should be a valid timestamp")
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &impl ToString) {
    if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes())
        && let Ok(header_value) = HeaderValue::from_str(&value.to_string())
    {
        headers.insert(header_name, header_value);
    }
}

fn compare_window_pressure(left: &&RateLimitWindowState, right: &&RateLimitWindowState) -> Ordering {
    let left_ratio = left.remaining as f64 / left.limit as f64;
    let right_ratio = right.remaining as f64 / right.limit as f64;
    left_ratio
        .partial_cmp(&right_ratio)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.window_seconds.cmp(&right.window_seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_fallback_limits_requests() {
        let config = crate::Config {
            database_url: "postgres://127.0.0.1:1/lectito".to_string(),
            listen_addr: "127.0.0.1:3000".parse().unwrap(),
            cache_ttl_secs: 86_400,
            rate_limit_per_min: 2,
            rate_limit_per_hour: 10,
            rate_limit_per_day: 100,
            blocked_domains_path: None,
            fetch_timeout_secs: 30,
            web_dir: std::path::PathBuf::from("web/dist"),
            db_max_connections: 1,
            db_connect_timeout_secs: 1,
            db_wait_timeout_secs: 1,
            db_create_timeout_secs: 1,
            db_recycle_timeout_secs: 1,
            db_idle_timeout_secs: 1,
            cleanup_interval_secs: 900,
            trust_proxy_headers: false,
            request_timeout_secs: 60,
            admin_token: None,
            auto_ban_threshold: 5,
            auto_ban_window_secs: 600,
            auto_ban_duration_secs: 3600,
        };

        let limiter = RateLimiter::new(&config);
        let first = limiter
            .check_and_increment_in_memory(IpAddr::from([127, 0, 0, 1]))
            .await;
        let second = limiter
            .check_and_increment_in_memory(IpAddr::from([127, 0, 0, 1]))
            .await;
        let third = limiter
            .check_and_increment_in_memory(IpAddr::from([127, 0, 0, 1]))
            .await;

        assert!(!first.is_limited());
        assert!(!second.is_limited());
        assert!(third.is_limited());
        assert_eq!(third.as_limits_response().requests_limit, 2);
    }

    #[test]
    fn prefers_forwarded_ip_when_enabled() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.10, 10.0.0.1"));

        let ip = forwarded_ip(&headers).unwrap();
        assert_eq!(ip, IpAddr::from([203, 0, 113, 10]));
    }

    #[test]
    fn applies_rate_limit_headers() {
        let snapshot = RateLimitSnapshot {
            windows: vec![RateLimitWindowState {
                window_seconds: 60,
                limit: 60,
                request_count: 5,
                remaining: 55,
                reset_at: OffsetDateTime::now_utc() + Duration::seconds(10),
            }],
        };
        let mut headers = HeaderMap::new();
        snapshot.apply_headers(&mut headers);

        assert_eq!(headers.get(HEADER_LIMIT).unwrap(), "60");
        assert_eq!(headers.get(HEADER_REMAINING).unwrap(), "55");
        assert!(headers.get(HEADER_RESET).is_some());
    }

    #[test]
    fn recognizes_web_app_header() {
        let mut headers = HeaderMap::new();
        headers.insert(WEB_CLIENT_HEADER, HeaderValue::from_static(WEB_CLIENT_WEB_APP));

        assert!(is_web_app_request(&headers));
    }
}
