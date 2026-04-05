use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    pub cache_ttl_secs: u64,
    pub rate_limit_per_min: u32,
    pub rate_limit_per_hour: u32,
    pub rate_limit_per_day: u32,
    pub blocked_domains_path: Option<String>,
    pub fetch_timeout_secs: u64,
    pub web_dir: PathBuf,
    pub db_max_connections: usize,
    pub db_connect_timeout_secs: u64,
    pub db_wait_timeout_secs: u64,
    pub db_create_timeout_secs: u64,
    pub db_recycle_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub cleanup_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let vars = env::vars().collect::<HashMap<_, _>>();
        Self::from_map(&vars)
    }

    fn from_map(vars: &HashMap<String, String>) -> Result<Self, String> {
        let database_url = vars
            .get("DATABASE_URL")
            .cloned()
            .ok_or_else(|| "DATABASE_URL is required".to_string())?;
        if database_url.trim().is_empty() {
            return Err("DATABASE_URL must not be empty".to_string());
        }

        let listen_addr = vars
            .get("LISTEN_ADDR")
            .cloned()
            .unwrap_or_else(|| "0.0.0.0:3000".to_string())
            .parse::<SocketAddr>()
            .map_err(|_| {
                let raw = vars.get("LISTEN_ADDR").map_or("0.0.0.0:3000", String::as_str);
                format!("LISTEN_ADDR '{raw}' is not a valid socket address")
            })?;

        let web_dir = vars.get("WEB_DIR").map(PathBuf::from).unwrap_or_else(default_web_dir);
        if web_dir.as_os_str().is_empty() {
            return Err("WEB_DIR must not be empty".to_string());
        }

        Ok(Self {
            database_url,
            listen_addr,
            cache_ttl_secs: parse_u64(vars, "CACHE_TTL_SECS", 86_400)?,
            rate_limit_per_min: parse_u32(vars, "RATE_LIMIT_PER_MIN", 60)?,
            rate_limit_per_hour: parse_u32(vars, "RATE_LIMIT_PER_HOUR", 600)?,
            rate_limit_per_day: parse_u32(vars, "RATE_LIMIT_PER_DAY", 5_000)?,
            blocked_domains_path: vars.get("BLOCKED_DOMAINS").cloned(),
            fetch_timeout_secs: parse_u64(vars, "FETCH_TIMEOUT_SECS", 30)?,
            web_dir,
            db_max_connections: parse_usize(vars, "DB_MAX_CONNECTIONS", 16)?,
            db_connect_timeout_secs: parse_u64(vars, "DB_CONNECT_TIMEOUT_SECS", 10)?,
            db_wait_timeout_secs: parse_u64(vars, "DB_WAIT_TIMEOUT_SECS", 5)?,
            db_create_timeout_secs: parse_u64(vars, "DB_CREATE_TIMEOUT_SECS", 10)?,
            db_recycle_timeout_secs: parse_u64(vars, "DB_RECYCLE_TIMEOUT_SECS", 5)?,
            db_idle_timeout_secs: parse_u64(vars, "DB_IDLE_TIMEOUT_SECS", 600)?,
            cleanup_interval_secs: parse_u64(vars, "CLEANUP_INTERVAL_SECS", 900)?,
        })
    }
}

fn default_web_dir() -> PathBuf {
    [
        PathBuf::from("web/dist"),
        PathBuf::from("web/build"),
        PathBuf::from("web/.svelte-kit/output/client"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .unwrap_or_else(|| PathBuf::from("web/dist"))
}

fn parse_u64(vars: &HashMap<String, String>, key: &str, default: u64) -> Result<u64, String> {
    match vars.get(key) {
        Some(value) => parse_positive(value, key),
        None => Ok(default),
    }
}

fn parse_u32(vars: &HashMap<String, String>, key: &str, default: u32) -> Result<u32, String> {
    match vars.get(key) {
        Some(value) => parse_positive(value, key),
        None => Ok(default),
    }
}

fn parse_usize(vars: &HashMap<String, String>, key: &str, default: usize) -> Result<usize, String> {
    match vars.get(key) {
        Some(value) => parse_positive(value, key),
        None => Ok(default),
    }
}

fn parse_positive<T>(value: &str, key: &str) -> Result<T, String>
where
    T: std::str::FromStr + PartialEq + Default,
{
    let parsed = value
        .parse::<T>()
        .map_err(|_| format!("{key} must be a positive integer, got '{value}'"))?;
    if parsed == T::default() {
        return Err(format!("{key} must be greater than zero"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_map() -> HashMap<String, String> {
        HashMap::from([
            ("DATABASE_URL".to_string(), "postgres://localhost/lectito".to_string()),
            ("LISTEN_ADDR".to_string(), "127.0.0.1:4000".to_string()),
        ])
    }

    #[test]
    fn parses_defaults() {
        let config = Config::from_map(&config_map()).unwrap();

        assert_eq!(config.listen_addr, "127.0.0.1:4000".parse().unwrap());
        assert_eq!(config.cache_ttl_secs, 86_400);
        assert_eq!(config.db_max_connections, 16);
        assert_eq!(config.cleanup_interval_secs, 900);
    }

    #[test]
    fn rejects_empty_database_url() {
        let mut vars = config_map();
        vars.insert("DATABASE_URL".to_string(), "   ".to_string());

        let err = Config::from_map(&vars).unwrap_err();
        assert!(err.contains("DATABASE_URL must not be empty"));
    }

    #[test]
    fn rejects_invalid_listen_addr() {
        let mut vars = config_map();
        vars.insert("LISTEN_ADDR".to_string(), "nope".to_string());

        let err = Config::from_map(&vars).unwrap_err();
        assert!(err.contains("LISTEN_ADDR"));
    }

    #[test]
    fn rejects_zero_values() {
        let mut vars = config_map();
        vars.insert("DB_MAX_CONNECTIONS".to_string(), "0".to_string());

        let err = Config::from_map(&vars).unwrap_err();
        assert!(err.contains("DB_MAX_CONNECTIONS must be greater than zero"));
    }
}
