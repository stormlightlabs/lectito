use lectito_server::{AppState, Config, build_app, db};
use std::error::Error;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let config = Config::from_env()?;
    let pool = db::create_pool(&config)?;
    db::run_migrations(&pool).await?;
    db::spawn_cleanup_task(pool.clone(), Duration::from_secs(config.cleanup_interval_secs));

    let state = AppState::new(config.clone(), pool, env!("CARGO_PKG_VERSION"));
    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    info!(
        "lectito-server listening on {} (web dir: {})",
        config.listen_addr,
        config.web_dir.display()
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        signal.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    info!("shutdown signal received");
}
