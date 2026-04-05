use std::time::Duration;

use deadpool_postgres::{
    Config as DeadpoolConfig, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime, Timeouts,
};
use tokio_postgres::NoTls;
use tracing::{info, warn};

use crate::Config;

const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");

pub fn create_pool(config: &Config) -> Result<Pool, deadpool_postgres::CreatePoolError> {
    let mut cfg = DeadpoolConfig::new();
    cfg.url = Some(config.database_url.clone());
    cfg.application_name = Some("lectito-server".to_string());
    cfg.connect_timeout = Some(Duration::from_secs(config.db_connect_timeout_secs));
    cfg.keepalives = Some(true);
    cfg.keepalives_idle = Some(Duration::from_secs(config.db_idle_timeout_secs));
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });
    cfg.pool = Some(PoolConfig {
        max_size: config.db_max_connections,
        timeouts: Timeouts {
            wait: Some(Duration::from_secs(config.db_wait_timeout_secs)),
            create: Some(Duration::from_secs(config.db_create_timeout_secs)),
            recycle: Some(Duration::from_secs(config.db_recycle_timeout_secs)),
        },
        queue_mode: Default::default(),
    });
    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
}

pub async fn run_migrations(pool: &Pool) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    client.batch_execute(MIGRATION_001).await?;
    info!("Migrations applied successfully");
    Ok(())
}

pub async fn ping(pool: &Pool) -> Result<(), String> {
    let client = pool
        .get()
        .await
        .map_err(|err| format!("failed to get DB connection: {err}"))?;
    client
        .query_one("SELECT 1", &[])
        .await
        .map(|_| ())
        .map_err(|err| format!("database ping failed: {err}"))
}

/// Spawns a background task that deletes expired cache and rate-limit rows every 15 minutes.
pub fn spawn_cleanup_task(pool: Pool, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            match pool.get().await {
                Ok(client) => {
                    match client
                        .execute("DELETE FROM extracted_articles WHERE expires_at < now()", &[])
                        .await
                    {
                        Ok(n) => {
                            if n > 0 {
                                info!("Cache cleanup: removed {n} expired articles");
                            }
                        }
                        Err(e) => warn!("Cache cleanup failed: {e}"),
                    }

                    match client
                        .execute(
                            "DELETE FROM rate_limits \
                             WHERE window_start + (window_seconds || ' seconds')::interval < now()",
                            &[],
                        )
                        .await
                    {
                        Ok(n) => {
                            if n > 0 {
                                info!("Rate-limit cleanup: removed {n} expired rows");
                            }
                        }
                        Err(e) => warn!("Rate-limit cleanup failed: {e}"),
                    }
                }
                Err(e) => warn!("Cleanup task: could not acquire DB connection: {e}"),
            }
        }
    });
}
