use anyhow::Result;
use clap::{Parser, Subcommand};
use jcp_core::{Envelope, LocalSigner, Subject};
use serde_json::json;
use serde_json::Value;
use std::path::PathBuf;

mod doctor;
mod telegram;
mod voice_demo;

use doctor::doctor_env;
use telegram::{telegram_discover_ids, telegram_doctor};
use voice_demo::VoiceDemo;

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
    VoiceDemo {
        #[command(subcommand)]
        command: VoiceDemoCommand,
    },
}

#[derive(Debug, Subcommand)]
enum VoiceDemoCommand {
    Discover {
        #[arg(long, env = "JMCP_TELEGRAM_ENV", default_value = "telegram.env")]
        env_file: PathBuf,
    },
    Send {
        #[arg(long, env = "JMCP_TELEGRAM_ENV", default_value = "telegram.env")]
        env_file: PathBuf,
        chat_id: i64,
        text: String,
    },
    Listen {
        #[arg(long, env = "JMCP_TELEGRAM_ENV", default_value = "telegram.env")]
        env_file: PathBuf,
        #[arg(long)]
        reply_voice: bool,
        #[arg(long, default_value_t = 60)]
        seconds: u64,
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
            TelegramCommand::VoiceDemo { command } => match command {
                VoiceDemoCommand::Discover { env_file } => {
                    let demo = VoiceDemo::from_env_file(env_file)?;
                    let result = demo.discover().await?;
                    println!("updates: {}", result.update_count);
                    for chat_id in result.chat_ids {
                        println!("  chat_id={chat_id}");
                    }
                }
                VoiceDemoCommand::Send {
                    env_file,
                    chat_id,
                    text,
                } => {
                    let demo = VoiceDemo::from_env_file(env_file)?;
                    let result = demo.send(chat_id, &text).await?;
                    println!(
                        "[send] sendVoice message_id={} bytes={} chars={}",
                        result.message_id, result.bytes, result.chars
                    );
                }
                VoiceDemoCommand::Listen {
                    env_file,
                    reply_voice,
                    seconds,
                } => {
                    let demo = VoiceDemo::from_env_file(env_file)?;
                    println!(
                        "[listen] waiting up to {seconds}s for a voice note... send one to the bot now."
                    );
                    match demo.listen(reply_voice, seconds).await? {
                        Some(result) => {
                            println!(
                                "[recv] voice_duration={} -> ASR: {:?}",
                                format_voice_duration(result.voice_duration),
                                result.transcript
                            );
                            println!(
                                "[recv] replied_to_chat_id={} voice_reply={}",
                                result.chat_id, result.reply_voice_sent
                            );
                        }
                        None => {
                            println!("[listen] timed out with no voice note.");
                        }
                    }
                }
            },
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

fn format_voice_duration(duration: Option<i64>) -> String {
    match duration {
        Some(seconds) => format!("{seconds}s"),
        None => "not-measured".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_voice_duration;

    #[test]
    fn formats_measured_voice_duration() {
        assert_eq!(format_voice_duration(Some(7)), "7s");
    }

    #[test]
    fn formats_unmeasured_voice_duration() {
        assert_eq!(format_voice_duration(None), "not-measured");
    }
}
