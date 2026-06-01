use anyhow::Result;
use clap::{Parser, Subcommand};
use jcp_core::{Envelope, Subject};
use serde_json::Value;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:8787")]
    server: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Health,
    Submit {
        subject: String,
        kind: String,
        #[arg(long, default_value = "{}")]
        payload: String,
    },
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = reqwest::Client::new();
    match args.command {
        Command::Health => {
            let value: Value = client
                .get(format!("{}/health", args.server))
                .send()
                .await?
                .json()
                .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Submit {
            subject,
            kind,
            payload,
        } => {
            let payload: Value = serde_json::from_str(&payload)?;
            let envelope =
                Envelope::new(subject.parse::<Subject>()?, kind, payload).sign_stub("jmcpctl");
            let value: Value = client
                .post(format!("{}/work-orders", args.server))
                .json(&envelope)
                .send()
                .await?
                .json()
                .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::List => {
            let value: Value = client
                .get(format!("{}/work-orders", args.server))
                .send()
                .await?
                .json()
                .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}
