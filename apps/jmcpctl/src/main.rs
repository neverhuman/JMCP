use anyhow::Result;
use clap::{Parser, Subcommand};
use jcp_core::{Envelope, LocalSigner, Subject};
use serde_json::json;
use serde_json::Value;
use std::path::PathBuf;

mod doctor;
mod telegram;

use doctor::doctor_env;
use telegram::{telegram_discover_ids, telegram_doctor};

const DEFAULT_API_URL: &str = "http://127.0.0.1:18877";

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "JMCP_API_URL", default_value = DEFAULT_API_URL)]
    server: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Health,
    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },
    Telegram {
        #[command(subcommand)]
        command: TelegramCommand,
    },
    Submit {
        subject: String,
        kind: String,
        #[arg(long, default_value = "{}")]
        payload: String,
    },
    Approve {
        token: String,
        #[arg(long, default_value = "local")]
        approver: String,
    },
    Deny {
        token: String,
        #[arg(long, default_value = "local")]
        approver: String,
    },
    WorkOrders,
    Evidence,
    Replay {
        #[arg(long)]
        now: bool,
    },
    Ecosystem,
    List,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    Env,
}

#[derive(Debug, Subcommand)]
enum TelegramCommand {
    Doctor {
        #[arg(long, env = "JMCP_TELEGRAM_ENV", default_value = "telegram.env")]
        env_file: PathBuf,
        #[arg(
            long,
            env = "JMCP_TELEGRAM_OFFSET_FILE",
            default_value = "jmcp.telegram.offset"
        )]
        offset_file: PathBuf,
    },
    DiscoverIds {
        #[arg(long, env = "JMCP_TELEGRAM_ENV", default_value = "telegram.env")]
        env_file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = reqwest::Client::new();
    match args.command {
        Command::Health => {
            let value = get_json(&client, &args.server, "/health").await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Doctor { command } => match command {
            DoctorCommand::Env => doctor_env(&args.server)?,
        },
        Command::Telegram { command } => match command {
            TelegramCommand::Doctor {
                env_file,
                offset_file,
            } => telegram_doctor(env_file, offset_file).await?,
            TelegramCommand::DiscoverIds { env_file } => telegram_discover_ids(env_file).await?,
        },
        Command::Submit {
            subject,
            kind,
            payload,
        } => {
            let payload: Value = serde_json::from_str(&payload)?;
            let signer = LocalSigner::load_or_create_default()?;
            let envelope = signer.sign(Envelope::new(subject.parse::<Subject>()?, kind, payload));
            let value = post_json(&client, &args.server, "/work-orders", &envelope).await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Approve { token, approver } => {
            let value = post_json(
                &client,
                &args.server,
                "/approvals/approve",
                &json!({ "token": token, "approver": approver }),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Deny { token, approver } => {
            let value = post_json(
                &client,
                &args.server,
                "/approvals/deny",
                &json!({ "token": token, "approver": approver }),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::WorkOrders | Command::List => {
            let value = get_json(&client, &args.server, "/work-orders").await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Evidence => {
            let value = get_json(&client, &args.server, "/evidence").await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Replay { now } => {
            let value = if now {
                post_json(&client, &args.server, "/replay", &json!({})).await?
            } else {
                get_json(&client, &args.server, "/replay").await?
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Ecosystem => {
            let value = get_json(&client, &args.server, "/ecosystem").await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

async fn get_json(client: &reqwest::Client, server: &str, path: &str) -> Result<Value> {
    let response = client.get(format!("{server}{path}")).send().await?;
    read_json_response(path, response).await
}

async fn post_json<T: serde::Serialize + ?Sized>(
    client: &reqwest::Client,
    server: &str,
    path: &str,
    body: &T,
) -> Result<Value> {
    let response = client
        .post(format!("{server}{path}"))
        .json(body)
        .send()
        .await?;
    read_json_response(path, response).await
}

async fn read_json_response(path: &str, response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("JMCP API {path} returned {status}: {text}");
    }
    Ok(serde_json::from_str(&text)?)
}
