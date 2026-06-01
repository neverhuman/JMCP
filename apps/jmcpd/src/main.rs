use anyhow::Result;
use clap::Parser;
use jmcp_api::router;
use jmcp_app::AppState;
use jmcp_store::SqliteStore;
use std::net::SocketAddr;

const DEFAULT_API_BIND: &str = "127.0.0.1:18877";
const JERYU_PROTECTED_PORTS: &[u16] = &[2224, 8787, 8929, 18787, 18788, 19800];

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "JMCP_API_BIND", default_value = DEFAULT_API_BIND)]
    listen: SocketAddr,
    #[arg(long, default_value = "jmcp.db")]
    database: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if JERYU_PROTECTED_PORTS.contains(&args.listen.port()) {
        anyhow::bail!(
            "JMCP_API_BIND must not use Jeryu protected port {}",
            args.listen.port()
        );
    }
    let store = SqliteStore::open(&args.database)?;
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    println!("jmcpd listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router(AppState::new(store))).await?;
    Ok(())
}
