use anyhow::Result;
use clap::Parser;
use jmcp_api::router;
use jmcp_app::AppState;
use jmcp_store::SqliteStore;
use std::net::SocketAddr;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8787")]
    listen: SocketAddr,
    #[arg(long, default_value = "jmcp.db")]
    database: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let store = SqliteStore::open(&args.database)?;
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    println!("jmcpd listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router(AppState::new(store))).await?;
    Ok(())
}
