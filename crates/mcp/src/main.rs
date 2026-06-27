use lectito_mcp::{Config, Server, ddg::DuckDuckGoSearch};

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let search = DuckDuckGoSearch::new().expect("failed to initialize DuckDuckGo search client");
    let server = Server::new(search, config);

    if let Err(err) = server.run_stdio().await {
        eprintln!("lectito-mcp: {err}");
        std::process::exit(1);
    }
}
