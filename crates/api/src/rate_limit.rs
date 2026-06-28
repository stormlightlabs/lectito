use std::net::{IpAddr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Method, Request};
use redis::aio::ConnectionManager;

const TOKEN_BUCKET_SCRIPT: &str = include_str!("bucket.lua");

#[derive(Debug, Eq, PartialEq)]
pub enum RateLimitDecision {
    Allowed,
    Limited { retry_after_secs: u64 },
}

impl RateLimiter {
    pub async fn new(config: &RateLimitConfig) -> redis::RedisResult<Self> {
        let client = redis::Client::open(config.redis_url.as_str())?;
        let redis = ConnectionManager::new(client).await?;
        Ok(Self { redis, prefix: config.prefix.clone(), trust_proxy_headers: config.trust_proxy_headers })
    }

    pub fn caller(&self, request: &Request<Body>) -> String {
        caller_ip(request.headers(), request.extensions(), self.trust_proxy_headers)
    }

    pub async fn check(&self, method: Method, path: String, caller: String) -> redis::RedisResult<RateLimitDecision> {
        let buckets = buckets_for(&method, &path);
        if buckets.is_empty() {
            return Ok(RateLimitDecision::Allowed);
        }

        for bucket in buckets {
            let decision = self.check_bucket(bucket, caller.as_str()).await?;
            if !matches!(decision, RateLimitDecision::Allowed) {
                return Ok(decision);
            }
        }

        Ok(RateLimitDecision::Allowed)
    }

    async fn check_bucket(&self, bucket: Bucket, caller: &str) -> redis::RedisResult<RateLimitDecision> {
        let mut redis = self.redis.clone();
        let now_ms = now_ms();
        let refill_per_ms = f64::from(bucket.capacity) / 60_000.0;
        let key = format!("{}:{}:{}", self.prefix, bucket.name, caller);
        let result: (i64, i64) = redis::Script::new(TOKEN_BUCKET_SCRIPT)
            .key(key)
            .arg(now_ms)
            .arg(bucket.capacity)
            .arg(refill_per_ms)
            .arg(120_000_u64)
            .invoke_async(&mut redis)
            .await?;

        match result.0 {
            1 => Ok(RateLimitDecision::Allowed),
            _ => Ok(RateLimitDecision::Limited { retry_after_secs: u64::try_from(result.1).unwrap_or(1).max(1) }),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub redis_url: String,
    pub prefix: String,
    pub trust_proxy_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redis_url: "redis://lectito-redis:6379".to_owned(),
            prefix: "lectito:api:rate".to_owned(),
            trust_proxy_headers: false,
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    redis: ConnectionManager,
    prefix: String,
    trust_proxy_headers: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Bucket {
    name: &'static str,
    capacity: u32,
}

fn buckets_for(method: &Method, path: &str) -> Vec<Bucket> {
    let mut buckets = Vec::with_capacity(2);

    match (method, path) {
        (&Method::GET, "/healthz") => {}
        (&Method::GET, "/openapi.json") => buckets.push(Bucket { name: "openapi", capacity: 60 }),
        (&Method::POST, "/v1/extract") => {
            buckets.push(Bucket { name: "post", capacity: 45 });
            buckets.push(Bucket { name: "extract", capacity: 5 });
        }
        (&Method::POST, "/v1/evaluate") => {
            buckets.push(Bucket { name: "post", capacity: 45 });
            buckets.push(Bucket { name: "evaluate", capacity: 10 });
        }
        (&Method::POST, "/v1/transform") => {
            buckets.push(Bucket { name: "post", capacity: 45 });
            buckets.push(Bucket { name: "transform", capacity: 30 });
        }
        (method, _) if method == Method::POST => buckets.push(Bucket { name: "post", capacity: 45 }),
        _ => {}
    }

    buckets
}

fn caller_ip(headers: &HeaderMap, extensions: &axum::http::Extensions, trust_proxy_headers: bool) -> String {
    if trust_proxy_headers {
        if let Some(ip) = header_ip(headers, "cf-connecting-ip") {
            return ip.to_string();
        }

        if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|value| value.to_str().ok()) {
            if let Some(ip) = forwarded_for
                .split(',')
                .find_map(|part| part.trim().parse::<IpAddr>().ok())
            {
                return ip.to_string();
            }
        }
    }

    extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn header_ip(headers: &HeaderMap, name: &'static str) -> Option<IpAddr> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use axum::http::Extensions;

    use super::*;

    #[test]
    fn skips_healthz() {
        assert!(buckets_for(&Method::GET, "/healthz").is_empty());
    }

    #[test]
    fn classifies_expensive_extract() {
        assert_eq!(
            buckets_for(&Method::POST, "/v1/extract"),
            vec![
                Bucket { name: "post", capacity: 45 },
                Bucket { name: "extract", capacity: 5 },
            ],
        );
    }

    #[test]
    fn trusts_cloudflare_ip_when_enabled() {
        let headers = HeaderMap::from_iter([("cf-connecting-ip".parse().unwrap(), "203.0.113.10".parse().unwrap())]);

        assert_eq!(caller_ip(&headers, &Extensions::new(), true), "203.0.113.10");
    }

    #[test]
    fn ignores_proxy_headers_when_disabled() {
        let headers = HeaderMap::from_iter([("cf-connecting-ip".parse().unwrap(), "203.0.113.10".parse().unwrap())]);

        assert_eq!(caller_ip(&headers, &Extensions::new(), false), "unknown");
    }
}
