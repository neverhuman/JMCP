use anyhow::Result;
use clap::{Parser, Subcommand};
use jcp_core::{Envelope, LocalSigner, Subject};
use serde_json::Value;
use std::process::Command as StdCommand;

const DEFAULT_API_URL: &str = "http://127.0.0.1:18877";
const DEFAULT_API_BIND: &str = "127.0.0.1:18877";
const DEFAULT_COCKPIT_HOST: &str = "127.0.0.1";
const DEFAULT_COCKPIT_PORT: u16 = 15873;
const JERYU_PROTECTED_PORTS: &[u16] = &[2224, 8787, 8929, 18787, 18788, 19800];

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
    Submit {
        subject: String,
        kind: String,
        #[arg(long, default_value = "{}")]
        payload: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    Env,
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
        Command::Doctor { command } => match command {
            DoctorCommand::Env => doctor_env(&args.server)?,
        },
        Command::Submit {
            subject,
            kind,
            payload,
        } => {
            let payload: Value = serde_json::from_str(&payload)?;
            let signer = LocalSigner::load_or_create_default()?;
            let envelope = signer.sign(Envelope::new(subject.parse::<Subject>()?, kind, payload));
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
