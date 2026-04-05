use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::IpAddr;
use std::sync::Arc;

use axum::http::HeaderMap;
use time::{Duration, OffsetDateTime};
use tokio::sync::RwLock;
use tracing::warn;
use url::Url;

use crate::Config;
use crate::error::AppError;

type ViolationStore = Arc<RwLock<HashMap<IpAddr, ViolationState>>>;
type BanStore = Arc<RwLock<HashMap<IpAddr, OffsetDateTime>>>;

#[derive(Clone)]
pub struct SpamFilter {
    blocked_domains: Arc<HashSet<String>>,
    admin_token: Option<String>,
    auto_ban_threshold: u32,
    auto_ban_window: Duration,
    auto_ban_duration: Duration,
    violations: ViolationStore,
    bans: BanStore,
}

#[derive(Debug, Clone)]
struct ViolationState {
    count: u32,
    last_violation_at: OffsetDateTime,
}

impl SpamFilter {
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            blocked_domains: Arc::new(load_blocked_domains(config.blocked_domains_path.as_deref())),
            admin_token: config.admin_token.clone(),
            auto_ban_threshold: config.auto_ban_threshold,
            auto_ban_window: Duration::seconds(config.auto_ban_window_secs as i64),
            auto_ban_duration: Duration::seconds(config.auto_ban_duration_secs as i64),
            violations: Arc::default(),
            bans: Arc::default(),
        }
    }

    pub async fn validate_extract_url(&self, pool: &deadpool_postgres::Pool, url: &str) -> Result<Url, AppError> {
        if url.len() > 2048 {
            return Err(AppError::BadRequest("URL must be 2048 characters or fewer".to_string()));
        }

        let parsed = Url::parse(url).map_err(|err| AppError::BadRequest(format!("Invalid URL: {err}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(AppError::BadRequest(
                "Only http:// and https:// URLs are allowed".to_string(),
            ));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(AppError::BadRequest(
                "URLs with embedded credentials are not allowed".to_string(),
            ));
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| AppError::BadRequest("URL must include a host".to_string()))?;
        let normalized_host = host.to_ascii_lowercase();

        if is_blocked_host(&normalized_host) {
            return Err(AppError::Forbidden("Target host is blocked".to_string()));
        }
        if self.blocked_domains.contains(&normalized_host) || self.is_domain_blocked_in_db(pool, &normalized_host).await
        {
            return Err(AppError::Forbidden("Target domain is blocked".to_string()));
        }

        Ok(parsed)
    }

    pub async fn check_ip_ban(&self, pool: &deadpool_postgres::Pool, ip: IpAddr) -> Result<(), AppError> {
        let now = OffsetDateTime::now_utc();
        if self.is_banned_in_memory(ip, now).await {
            return Err(AppError::TooManyRequests { retry_after: 60 });
        }
        if let Some(retry_after) = self.is_banned_in_db(pool, ip, now).await {
            return Err(AppError::TooManyRequests { retry_after });
        }
        Ok(())
    }

    pub async fn record_rate_limit_violation(&self, pool: &deadpool_postgres::Pool, ip: IpAddr) {
        let now = OffsetDateTime::now_utc();
        let mut violations = self.violations.write().await;
        let state = violations
            .entry(ip)
            .or_insert(ViolationState { count: 0, last_violation_at: now });

        if now - state.last_violation_at > self.auto_ban_window {
            state.count = 0;
        }
        state.count += 1;
        state.last_violation_at = now;

        if state.count >= self.auto_ban_threshold {
            drop(violations);
            self.ban_ip(pool, ip, "Auto-ban after repeated rate-limit violations", now)
                .await;
        }
    }

    pub async fn clear_rate_limit_violations(&self, ip: IpAddr) {
        self.violations.write().await.remove(&ip);
    }

    pub fn require_admin(&self, headers: &HeaderMap) -> Result<(), AppError> {
        let token = self
            .admin_token
            .as_deref()
            .ok_or_else(|| AppError::ServiceUnavailable("ADMIN_TOKEN is not configured".to_string()))?;
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Forbidden("Missing Authorization header".to_string()))?;

        match auth_header.strip_prefix("Bearer ") {
            Some(value) if value == token => Ok(()),
            _ => Err(AppError::Forbidden("Invalid admin token".to_string())),
        }
    }

    pub async fn insert_blocked_domain(
        &self, pool: &deadpool_postgres::Pool, domain: &str, reason: Option<&str>,
    ) -> Result<(), AppError> {
        let normalized = domain.trim().to_ascii_lowercase();
        let client = pool.get().await?;
        client
            .execute(
                "INSERT INTO blocked_domains (domain, reason) VALUES ($1, $2)
                 ON CONFLICT (domain) DO UPDATE SET reason = EXCLUDED.reason, blocked_at = now()",
                &[&normalized, &reason],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_blocked_domain(&self, pool: &deadpool_postgres::Pool, domain: &str) -> Result<(), AppError> {
        let normalized = domain.trim().to_ascii_lowercase();
        let client = pool.get().await?;
        client
            .execute("DELETE FROM blocked_domains WHERE domain = $1", &[&normalized])
            .await?;
        Ok(())
    }

    pub async fn insert_ip_ban(
        &self, pool: &deadpool_postgres::Pool, ip: IpAddr, reason: &str, duration_hours: u64,
    ) -> Result<(), AppError> {
        let now = OffsetDateTime::now_utc();
        self.write_ip_ban(pool, ip, reason, now + Duration::hours(duration_hours as i64))
            .await
    }

    async fn is_domain_blocked_in_db(&self, pool: &deadpool_postgres::Pool, domain: &str) -> bool {
        let client = match pool.get().await {
            Ok(client) => client,
            Err(err) => {
                warn!("blocked_domains lookup skipped because DB connection failed: {err}");
                return false;
            }
        };

        match client
            .query_opt("SELECT 1 FROM blocked_domains WHERE domain = $1", &[&domain])
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(err) => {
                warn!("blocked_domains lookup failed: {err}");
                false
            }
        }
    }

    async fn is_banned_in_db(&self, pool: &deadpool_postgres::Pool, ip: IpAddr, now: OffsetDateTime) -> Option<u32> {
        let client = match pool.get().await {
            Ok(client) => client,
            Err(err) => {
                warn!("ip_bans lookup skipped because DB connection failed: {err}");
                return None;
            }
        };

        match client
            .query_opt(
                "SELECT expires_at FROM ip_bans WHERE ip = $1 AND (expires_at IS NULL OR expires_at > now())",
                &[&ip],
            )
            .await
        {
            Ok(Some(row)) => row
                .get::<_, Option<OffsetDateTime>>("expires_at")
                .map(|expiry| (expiry - now).whole_seconds().max(1) as u32)
                .or(Some(60)),
            Ok(None) => None,
            Err(err) => {
                warn!("ip_bans lookup failed: {err}");
                None
            }
        }
    }

    async fn is_banned_in_memory(&self, ip: IpAddr, now: OffsetDateTime) -> bool {
        let mut bans = self.bans.write().await;
        bans.retain(|_, expiry| *expiry > now);
        bans.get(&ip).is_some_and(|expiry| *expiry > now)
    }

    async fn ban_ip(&self, pool: &deadpool_postgres::Pool, ip: IpAddr, reason: &str, now: OffsetDateTime) {
        let expires_at = now + self.auto_ban_duration;
        self.bans.write().await.insert(ip, expires_at);

        if let Err(err) = self.write_ip_ban(pool, ip, reason, expires_at).await {
            warn!("failed to persist ip ban: {:?}", err);
        }
    }

    async fn write_ip_ban(
        &self, pool: &deadpool_postgres::Pool, ip: IpAddr, reason: &str, expires_at: OffsetDateTime,
    ) -> Result<(), AppError> {
        let client = pool.get().await?;
        client
            .execute(
                "INSERT INTO ip_bans (ip, reason, expires_at)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (ip) DO UPDATE SET reason = EXCLUDED.reason, banned_at = now(), expires_at = EXCLUDED.expires_at",
                &[&ip, &reason, &expires_at],
            )
            .await?;
        Ok(())
    }
}

fn load_blocked_domains(path: Option<&str>) -> HashSet<String> {
    let mut domains = HashSet::new();
    if let Some(path) = path {
        match fs::read_to_string(path) {
            Ok(contents) => {
                for line in contents.lines() {
                    let domain = line.trim();
                    if domain.is_empty() || domain.starts_with('#') {
                        continue;
                    }
                    domains.insert(domain.to_ascii_lowercase());
                }
            }
            Err(err) => warn!("failed to read blocked domains file {path}: {err}"),
        }
    }
    domains
}

fn is_blocked_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<IpAddr>().is_ok_and(is_blocked_ip)
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.octets()[0] == 0,
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unspecified() || ip.is_unicast_link_local(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            database_url: "postgres://localhost/lectito".to_string(),
            listen_addr: "127.0.0.1:3000".parse().unwrap(),
            cache_ttl_secs: 86_400,
            rate_limit_per_min: 60,
            rate_limit_per_hour: 600,
            rate_limit_per_day: 5_000,
            blocked_domains_path: None,
            fetch_timeout_secs: 30,
            web_dir: std::path::PathBuf::from("web/dist"),
            db_max_connections: 16,
            db_connect_timeout_secs: 10,
            db_wait_timeout_secs: 5,
            db_create_timeout_secs: 10,
            db_recycle_timeout_secs: 5,
            db_idle_timeout_secs: 600,
            cleanup_interval_secs: 900,
            trust_proxy_headers: false,
            request_timeout_secs: 60,
            admin_token: Some("secret".to_string()),
            auto_ban_threshold: 5,
            auto_ban_window_secs: 600,
            auto_ban_duration_secs: 3600,
        }
    }

    #[test]
    fn blocks_private_hosts() {
        assert!(is_blocked_host("localhost"));
        assert!(is_blocked_host("127.0.0.1"));
        assert!(is_blocked_host("10.0.0.1"));
        assert!(is_blocked_host("192.168.1.2"));
        assert!(!is_blocked_host("example.com"));
    }

    #[test]
    fn rejects_invalid_admin_token() {
        let filter = SpamFilter::new(&config());
        let headers = HeaderMap::new();
        let err = filter.require_admin(&headers).unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }
}
