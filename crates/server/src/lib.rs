pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod rate_limit;
pub mod routes;
pub mod spam;

use deadpool_postgres::Pool;

pub use config::Config;
pub use rate_limit::RateLimiter;
pub use routes::build_app;
pub use spam::SpamFilter;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: Pool,
    pub rate_limiter: RateLimiter,
    pub spam_filter: SpamFilter,
    pub version: &'static str,
}

impl AppState {
    #[must_use]
    pub fn new(config: Config, pool: Pool, version: &'static str) -> Self {
        let rate_limiter = RateLimiter::new(&config);
        let spam_filter = SpamFilter::new(&config);
        Self { config, pool, rate_limiter, spam_filter, version }
    }
}
