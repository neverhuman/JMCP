use anyhow::Result;
use clap::Parser;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use serde_json::Value;
use std::io;

const DEFAULT_API_URL: &str = "http://127.0.0.1:18877";

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "JMCP_API_URL", default_value = DEFAULT_API_URL)]
    server: String,
    #[arg(long)]
    once: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = reqwest::Client::new();
    let value = serde_json::json!({
        "systems": get_json(&client, &args.server, "/systems").await?,
        "work_orders": get_json(&client, &args.server, "/work-orders").await?,
        "approvals": get_json(&client, &args.server, "/approvals").await?,
        "evidence": get_json(&client, &args.server, "/evidence").await?,
        "replay": get_json(&client, &args.server, "/replay").await?,
    });
    if args.once {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| {
        let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "[]".to_owned());
        frame.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .title("JMCP Work Orders")
                    .borders(Borders::ALL),
            ),
            frame.size(),
        );
    })?;
    Ok(())
}

async fn get_json(client: &reqwest::Client, server: &str, path: &str) -> Result<Value> {
    Ok(client
        .get(format!("{server}{path}"))
        .send()
        .await?
        .json()
        .await?)
}
