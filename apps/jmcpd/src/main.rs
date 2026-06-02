use anyhow::Result;
use chrono::Duration as ChronoDuration;
use clap::Parser;
use jmcp_api::router;
use jmcp_app::AppState;
use jmcp_approval_telegram::{
    render_prompt, TelegramApprovalChallenge, TelegramBotClient, TelegramConfig, TelegramMessage,
};
use jmcp_domain::ApprovalDecision;
use jmcp_store::SqliteStore;
use serde_json::json;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

mod telegram_helpers;
#[cfg(test)]
mod tests;

use crate::telegram_helpers::{
    decide_from_telegram, emit_structured_event, status_from_telegram, submit_from_telegram,
};

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
                emit_structured_event(
                    "error",
                    "runtime.stopped",
                    json!({
                        "component": "telegram",
                        "reason": "poll_loop_failed",
                        "error": err.to_string(),
                    }),
                );
            }
        });
    }
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    emit_structured_event(
        "info",
        "runtime.started",
        json!({
            "listen": listener.local_addr()?.to_string(),
            "database": args.database,
            "telegramPoll": args.telegram_poll,
        }),
    );
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
                emit_structured_event(
                    "warn",
                    "telegram.poll.failed",
                    json!({
                        "operation": "getUpdates",
                        "offset": offset,
                        "retryInSeconds": 5,
                    }),
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        for update in updates {
            let next_offset = update.update_id + 1;
            if persist_telegram_offset(&offset_file, next_offset).is_err() {
                emit_structured_event(
                    "error",
                    "telegram.offset.persist.failed",
                    json!({
                        "updateId": update.update_id,
                        "nextOffset": next_offset,
                    }),
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
                break;
            }
            emit_structured_event(
                "info",
                "telegram.offset.persisted",
                json!({
                    "updateId": update.update_id,
                    "offset": next_offset,
                }),
            );
            offset = Some(next_offset);
            if let Some(message) = update.message {
                if handle_telegram_message(&client, &state, message)
                    .await
                    .is_err()
                {
                    emit_structured_event(
                        "error",
                        "telegram.message.failed",
                        json!({
                            "updateId": update.update_id,
                            "operation": "handleMessage",
                        }),
                    );
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
        emit_structured_event(
            "info",
            "telegram.message.handled",
            json!({
                "action": "help",
                "chatId": message.chat.id,
                "userId": user.id,
            }),
        );
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
                emit_structured_event(
                    "info",
                    "telegram.message.handled",
                    json!({
                        "action": "submit",
                        "chatId": message.chat.id,
                        "userId": user.id,
                        "workOrderId": work_order.id,
                        "result": "accepted",
                    }),
                );
            }
            Err(err) => {
                let message_text = if err.downcast_ref::<serde_json::Error>().is_some() {
                    "JMCP submit rejected: malformed JSON payload."
                } else {
                    "JMCP submit rejected; check subject, kind, and payload."
                };
                emit_structured_event(
                    "warn",
                    "telegram.submit.rejected",
                    json!({
                        "action": "submit",
                        "chatId": message.chat.id,
                        "userId": user.id,
                        "result": "rejected",
                        "reason": message_text,
                    }),
                );
                emit_structured_event(
                    "info",
                    "telegram.message.handled",
                    json!({
                        "action": "submit",
                        "chatId": message.chat.id,
                        "userId": user.id,
                        "result": "rejected",
                    }),
                );
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
        emit_structured_event(
            "info",
            "telegram.message.handled",
            json!({
                "action": "status",
                "chatId": message.chat.id,
                "userId": user.id,
            }),
        );
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
        emit_structured_event(
            "info",
            "telegram.message.handled",
            json!({
                "action": "approve",
                "chatId": message.chat.id,
                "userId": user.id,
            }),
        );
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
        emit_structured_event(
            "info",
            "telegram.message.handled",
            json!({
                "action": "deny",
                "chatId": message.chat.id,
                "userId": user.id,
            }),
        );
    }
    Ok(())
}
