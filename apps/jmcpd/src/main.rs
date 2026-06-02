use anyhow::Result;
use chrono::Duration as ChronoDuration;
use clap::Parser;
use jcp_core::{Envelope, LocalSigner, Subject};
use jmcp_api::router;
use jmcp_app::{telegram_actor, AppState, ApprovalDecisionError};
use jmcp_approval_telegram::{
    render_prompt, TelegramApprovalChallenge, TelegramBotClient, TelegramConfig, TelegramMessage,
};
use jmcp_domain::{ApprovalDecision, WorkOrder};
use jmcp_store::SqliteStore;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use uuid::Uuid;

#[cfg(test)]
mod tests;

const DEFAULT_API_BIND: &str = "127.0.0.1:18877";
const JERYU_PROTECTED_PORTS: &[u16] = &[2224, 8787, 8799, 8929, 18787, 18788, 19800];

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "JMCP_API_BIND", default_value = DEFAULT_API_BIND)]
    listen: SocketAddr,
    #[arg(long, default_value = "jmcp.db")]
    database: String,
    #[arg(long, env = "JMCP_TELEGRAM_POLL", default_value_t = false)]
    telegram_poll: bool,
    #[arg(long, env = "JMCP_TELEGRAM_ENV", default_value = "telegram.env")]
    telegram_env: PathBuf,
    #[arg(
        long,
        env = "JMCP_TELEGRAM_OFFSET_FILE",
        default_value = "jmcp.telegram.offset"
    )]
    telegram_offset_file: PathBuf,
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
    let state = AppState::new(store);
    if args.telegram_poll {
        let config = TelegramConfig::from_env_file(&args.telegram_env)?;
        let telegram_state = state.clone();
        let telegram_offset_file = args.telegram_offset_file.clone();
        tokio::spawn(async move {
            if let Err(err) = telegram_poll_loop(config, telegram_state, telegram_offset_file).await
            {
                eprintln!("telegram runtime stopped: {err}");
            }
        });
    }
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    println!("jmcpd listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router(state)).await?;
    Ok(())
}

async fn telegram_poll_loop(
    config: TelegramConfig,
    state: AppState,
    offset_file: PathBuf,
) -> Result<()> {
    let client = TelegramBotClient::new(config);
    let mut offset = read_telegram_offset(&offset_file)?;
    loop {
        let updates = match client.get_updates(offset, 25).await {
            Ok(updates) => updates,
            Err(_) => {
                eprintln!("telegram getUpdates failed; retrying");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        for update in updates {
            let next_offset = update.update_id + 1;
            if persist_telegram_offset(&offset_file, next_offset).is_err() {
                eprintln!("telegram offset persistence failed; retrying");
                tokio::time::sleep(Duration::from_secs(5)).await;
                break;
            }
            offset = Some(next_offset);
            if let Some(message) = update.message {
                if handle_telegram_message(&client, &state, message)
                    .await
                    .is_err()
                {
                    eprintln!("telegram message handling failed");
                }
            }
        }
    }
}

fn read_telegram_offset(path: &Path) -> Result<Option<i64>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(trimmed.parse()?))
}

fn persist_telegram_offset(path: &Path, offset: i64) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, offset.to_string())?;
    Ok(())
}

async fn handle_telegram_message(
    client: &TelegramBotClient,
    state: &AppState,
    message: TelegramMessage,
) -> Result<()> {
    let Some(user) = message.from.as_ref() else {
        return Ok(());
    };
    if !client.config().is_allowed(user.id, message.chat.id) {
        let _ = client
            .send_message(
                message.chat.id,
                "JMCP: this Telegram user or chat is not allowed.",
            )
            .await;
        return Ok(());
    }

    let Some(text) = message.text.as_deref() else {
        return Ok(());
    };
    if text == "/start" || text == "/help" {
        client
            .send_message(
                message.chat.id,
                "JMCP commands: /submit <subject> <kind> <json>, /status <work_order_id>, /approve <token>, /deny <token>.",
            )
            .await?;
        return Ok(());
    }

    if let Some(rest) = text.strip_prefix("/submit ") {
        let mut parts = rest.splitn(3, ' ');
        let subject = parts.next();
        let kind = parts.next();
        let payload = parts.next().unwrap_or("{}");
        let response = match (subject, kind) {
            (Some(subject), Some(kind)) => submit_from_telegram(state, subject, kind, payload),
            _ => Err(anyhow::anyhow!(
                "usage: /submit <tenant/service/entity> <kind> <json>"
            )),
        };
        match response {
            Ok(work_order) => {
                let challenge = state.create_telegram_approval_challenge(
                    work_order.id,
                    user.id,
                    message.chat.id,
                    Some(ChronoDuration::minutes(15)),
                )?;
                let prompt = render_prompt(&TelegramApprovalChallenge {
                    work_order_id: work_order.id,
                    approver_user_id: user.id,
                    token: challenge.token,
                    expires_at: challenge.challenge.expires_at,
                });
                client
                    .send_message(
                        message.chat.id,
                        &format!("JMCP work order submitted: {}\n{prompt}", work_order.id),
                    )
                    .await?;
            }
            Err(err) => {
                eprintln!("telegram submit rejected");
                let message_text = if err.downcast_ref::<serde_json::Error>().is_some() {
                    "JMCP submit rejected: malformed JSON payload."
                } else {
                    "JMCP submit rejected; check subject, kind, and payload."
                };
                client.send_message(message.chat.id, message_text).await?;
            }
        }
        return Ok(());
    }

    if let Some(rest) = text.strip_prefix("/status ") {
        let response = match Uuid::parse_str(rest.trim()) {
            Ok(id) => status_from_telegram(state, id),
            Err(_) => "JMCP status rejected: invalid work order id.".to_owned(),
        };
        client.send_message(message.chat.id, &response).await?;
        return Ok(());
    }

    if let Some(token) = text.strip_prefix("/approve ") {
        let response = decide_from_telegram(
            state,
            token,
            user.id,
            message.chat.id,
            ApprovalDecision::Approved,
        );
        client.send_message(message.chat.id, &response).await?;
        return Ok(());
    }

    if let Some(token) = text.strip_prefix("/deny ") {
        let response = decide_from_telegram(
            state,
            token,
            user.id,
            message.chat.id,
            ApprovalDecision::Rejected,
        );
        client.send_message(message.chat.id, &response).await?;
    }
    Ok(())
}

fn submit_from_telegram(
    state: &AppState,
    subject: &str,
    kind: &str,
    payload: &str,
) -> Result<WorkOrder> {
    let payload = serde_json::from_str(payload)?;
    let signer = LocalSigner::load_or_create_default()?;
    let envelope = signer.sign(Envelope::new(
        Subject::from_str(subject)?,
        kind.to_owned(),
        payload,
    ));
    Ok(state.submit_envelope(envelope)?)
}

fn status_from_telegram(state: &AppState, id: Uuid) -> String {
    match state.work_order(id) {
        Ok(Some(work_order)) => {
            let attention = if work_order.attention.is_empty() {
                "none".to_owned()
            } else {
                work_order
                    .attention
                    .iter()
                    .map(|item| item.reason.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "JMCP work order {}: {:?}; attention: {}; evidence: {}.",
                work_order.id,
                work_order.status,
                attention,
                work_order.evidence.len()
            )
        }
        Ok(None) => "JMCP status rejected: unknown work order id.".to_owned(),
        Err(_) => "JMCP status unavailable: state could not be read.".to_owned(),
    }
}

fn decide_from_telegram(
    state: &AppState,
    token: &str,
    user_id: i64,
    chat_id: i64,
    decision: ApprovalDecision,
) -> String {
    match state.decide_approval_by_token(token.trim(), telegram_actor(user_id, chat_id), decision) {
        Ok(outcome) => format!(
            "JMCP approval {:?} for work order {}.",
            outcome.approval.decision.unwrap_or(decision),
            outcome.work_order.id
        ),
        Err(ApprovalDecisionError::UnknownToken) => {
            "JMCP approval rejected: unknown token.".to_owned()
        }
        Err(ApprovalDecisionError::Expired) => "JMCP approval rejected: expired token.".to_owned(),
        Err(ApprovalDecisionError::AlreadyUsed) => {
            "JMCP approval rejected: token already used.".to_owned()
        }
        Err(ApprovalDecisionError::WrongApprover) => {
            "JMCP approval rejected: wrong Telegram approver.".to_owned()
        }
        Err(ApprovalDecisionError::UnavailableState(_)) => {
            "JMCP approval unavailable: state could not be updated.".to_owned()
        }
    }
}
