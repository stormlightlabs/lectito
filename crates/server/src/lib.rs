pub mod config;
pub mod db;
pub mod error;
pub mod routes;

use deadpool_postgres::Pool;

pub use config::Config;
pub use routes::build_app;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: Pool,
    pub version: &'static str,
}

impl AppState {
    #[must_use]
    pub fn new(config: Config, pool: Pool, version: &'static str) -> Self {
        Self { config, pool, version }
    }
}
