use anyhow::Result;
use clap::{Parser, Subcommand};
use jcp_core::{Envelope, LocalSigner, Subject};
use jmcp_approval_telegram::{TelegramBotClient, TelegramConfig};
use serde_json::json;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command as StdCommand;

const DEFAULT_API_URL: &str = "http://127.0.0.1:18877";
const DEFAULT_API_BIND: &str = "127.0.0.1:18877";
const DEFAULT_COCKPIT_HOST: &str = "127.0.0.1";
const DEFAULT_COCKPIT_PORT: u16 = 15873;
const JERYU_PROTECTED_PORTS: &[u16] = &[2224, 8787, 8799, 8929, 18787, 18788, 19800];

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

async fn telegram_doctor(env_file: PathBuf, offset_file: PathBuf) -> Result<()> {
    let config = TelegramConfig::from_env_file_for_setup(&env_file)?;
    let mut failed = false;

    println!("JMCP_TELEGRAM_ENV={}", env_file.display());
    println!("telegram_token=loaded (redacted)");
    println!("telegram_api_base={}", config.api_base);
    println!(
        "telegram_allowlist=user_ids:{} chat_ids:{}",
        config.allowed_user_ids.len(),
        config.allowed_chat_ids.len()
    );
    println!("telegram_config={config:?}");

    if !config.has_allowlist() {
        eprintln!("error: telegram allowlist missing");
        failed = true;
    }

    let client = TelegramBotClient::new(config);
    match client.get_me().await {
        Ok(me) => {
            println!(
                "telegram_getMe=ok id:{} username:{}",
                me.id,
                me.username.unwrap_or_else(|| "(none)".to_owned())
            );
        }
        Err(err) => {
            eprintln!("error: telegram getMe failed: {err}");
            failed = true;
        }
    }

    match std::fs::read_to_string(&offset_file) {
        Ok(contents) => match contents.trim().parse::<i64>() {
            Ok(offset) => println!(
                "telegram_offset_file={} offset={offset}",
                offset_file.display()
            ),
            Err(_) => {
                eprintln!(
                    "error: telegram offset file is not a valid integer: {}",
                    offset_file.display()
                );
                failed = true;
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "telegram_offset_file={} status=absent",
                offset_file.display()
            );
        }
        Err(err) => {
            eprintln!(
                "error: telegram offset file could not be read: {}: {err}",
                offset_file.display()
            );
            failed = true;
        }
    }

    if failed {
        anyhow::bail!("Telegram setup is not ready");
    }
    println!("Telegram setup is ready");
    Ok(())
}

async fn telegram_discover_ids(env_file: PathBuf) -> Result<()> {
    let config = TelegramConfig::from_env_file_for_setup(&env_file)?;
    let client = TelegramBotClient::new(config);
    let updates = client.get_updates(None, 0).await?;
    let mut candidates = BTreeSet::new();
    for update in updates {
        if let Some(message) = update.message {
            if let Some(user) = message.from {
                candidates.insert(format!(
                    "user_id={} chat_id={} chat_type={} username={} first_name={}",
                    user.id,
                    message.chat.id,
                    message.chat.kind,
                    user.username.unwrap_or_else(|| "(none)".to_owned()),
                    user.first_name
                ));
            }
        }
    }

    if candidates.is_empty() {
        println!("No Telegram updates found. Send the bot a message, then rerun discover-ids.");
    } else {
        for candidate in candidates {
            println!("{candidate}");
        }
    }
    Ok(())
}

fn doctor_env(server: &str) -> Result<()> {
    let api_bind = std::env::var("JMCP_API_BIND").unwrap_or_else(|_| DEFAULT_API_BIND.to_owned());
    let cockpit_host =
        std::env::var("JMCP_COCKPIT_HOST").unwrap_or_else(|_| DEFAULT_COCKPIT_HOST.to_owned());
    let mut failed = false;
    let cockpit_port_value =
        std::env::var("JMCP_COCKPIT_PORT").unwrap_or_else(|_| DEFAULT_COCKPIT_PORT.to_string());
    let cockpit_port = match parse_port(&cockpit_port_value) {
        Ok(port) => port,
        Err(message) => {
            eprintln!("error: JMCP_COCKPIT_PORT {message}");
            failed = true;
            0
        }
    };

    println!("JMCP_API_BIND={api_bind}");
    println!("JMCP_API_URL={server}");
    println!("JMCP_COCKPIT_HOST={cockpit_host}");
    println!("JMCP_COCKPIT_PORT={cockpit_port}");

    if let Some(port) = port_from_bind(&api_bind) {
        if JERYU_PROTECTED_PORTS.contains(&port) {
            eprintln!("error: JMCP_API_BIND uses Jeryu protected port {port}");
            failed = true;
        }
        if let Some(owner) = listener_owner(port) {
            println!("api bind conflict on port {port}: {owner}");
        }
    } else {
        eprintln!("error: could not parse JMCP_API_BIND={api_bind}");
        failed = true;
    }

    if cockpit_port != 0 && JERYU_PROTECTED_PORTS.contains(&cockpit_port) {
        eprintln!("error: JMCP_COCKPIT_PORT uses Jeryu protected port {cockpit_port}");
        failed = true;
    }
    if cockpit_port != 0 {
        if let Some(owner) = listener_owner(cockpit_port) {
            println!("cockpit bind conflict on port {cockpit_port}: {owner}");
        }
    }

    for port in JERYU_PROTECTED_PORTS {
        if let Some(owner) = listener_owner(*port) {
            println!("Jeryu protected port {port} is occupied by: {owner}");
        }
    }

    if listener_owner(8799).is_none() && listener_owner(8787).is_none() {
        eprintln!("warning: Jeryu was not detected on 127.0.0.1:8799 or 127.0.0.1:8787");
    }

    if failed {
        anyhow::bail!("JMCP environment is not safe");
    }
    println!("JMCP environment is safe for Jeryu coexistence");
    Ok(())
}

fn parse_port(value: &str) -> Result<u16, String> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!("is not numeric: {value}"));
    }
    value
        .parse::<u16>()
        .map_err(|_| format!("is outside the valid TCP port range: {value}"))
}

fn port_from_bind(bind: &str) -> Option<u16> {
    parse_port(bind.rsplit_once(':')?.1).ok()
}

fn listener_owner(port: u16) -> Option<String> {
    let output = StdCommand::new("ss").args(["-ltnp"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find(|line| line.contains(&format!(":{port} ")))
        .map(|line| line.trim().to_owned())
}
