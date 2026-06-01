use anyhow::Result;
use clap::Parser;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use serde_json::Value;
use std::io;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:8787")]
    server: String,
    #[arg(long)]
    once: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let value: Value = reqwest::get(format!("{}/work-orders", args.server))
        .await?
        .json()
        .await?;
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
