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
    state.submit_envelope(envelope)
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::{Path as AxumPath, State as AxumState},
        routing::post,
        Json, Router,
    };
    use jmcp_approval_telegram::{TelegramChat, TelegramUpdate, TelegramUser};
    use jmcp_domain::WorkOrderStatus;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeBotState {
        sent_messages: Arc<Mutex<Vec<Value>>>,
    }

    async fn fake_bot_api() -> (String, Arc<Mutex<Vec<Value>>>) {
        let state = FakeBotState::default();
        let sent_messages = state.sent_messages.clone();
        let app = Router::new()
            .route("/*path", post(fake_bot_handler))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), sent_messages)
    }

    async fn fake_bot_handler(
        AxumPath(path): AxumPath<String>,
        AxumState(state): AxumState<FakeBotState>,
        Json(payload): Json<Value>,
    ) -> Json<Value> {
        let method = path.rsplit('/').next().unwrap_or_default();
        match method {
            "getMe" => Json(json!({
                "ok": true,
                "result": {
                    "id": 123,
                    "is_bot": true,
                    "first_name": "JMCP",
                    "username": "jmcp_test_bot"
                }
            })),
            "getUpdates" => Json(json!({
                "ok": true,
                "result": [{
                    "update_id": 10,
                    "message": {
                        "message_id": 1,
                        "from": {
                            "id": 42,
                            "is_bot": false,
                            "first_name": "Ada",
                            "username": "ada"
                        },
                        "chat": { "id": 99, "type": "private" },
                        "text": "/start"
                    }
                }]
            })),
            "sendMessage" => {
                state.sent_messages.lock().unwrap().push(payload.clone());
                Json(json!({
                    "ok": true,
                    "result": {
                        "message_id": state.sent_messages.lock().unwrap().len(),
                        "chat": {
                            "id": payload["chat_id"].as_i64().unwrap_or(0),
                            "type": "private"
                        },
                        "text": payload["text"].as_str().unwrap_or("")
                    }
                }))
            }
            _ => Json(json!({ "ok": false, "description": "unknown method" })),
        }
    }

    fn telegram_config(api_base: &str, allowlist: &str) -> TelegramConfig {
        TelegramConfig::from_env_contents(&format!(
            "JMCP_TELEGRAM_BOT_TOKEN=123:secret\nJMCP_TELEGRAM_API_BASE={api_base}\n{allowlist}\n"
        ))
        .unwrap()
    }

    fn telegram_message(text: &str, user_id: i64, chat_id: i64) -> TelegramMessage {
        TelegramMessage {
            message_id: 1,
            from: Some(TelegramUser {
                id: user_id,
                is_bot: false,
                first_name: "Ada".to_owned(),
                username: Some("ada".to_owned()),
            }),
            chat: TelegramChat {
                id: chat_id,
                kind: "private".to_owned(),
            },
            text: Some(text.to_owned()),
        }
    }

    fn last_sent_text(sent_messages: &Arc<Mutex<Vec<Value>>>) -> String {
        sent_messages
            .lock()
            .unwrap()
            .last()
            .and_then(|value| value.get("text"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    fn token_from_prompt(text: &str) -> String {
        let mut parts = text.split_whitespace();
        while let Some(part) = parts.next() {
            if part == "/approve" {
                return parts.next().unwrap().trim_end_matches('.').to_owned();
            }
        }
        panic!("approval token not found in prompt");
    }

    #[tokio::test]
    async fn fake_bot_api_supports_get_me_and_updates() {
        let (api_base, _sent) = fake_bot_api().await;
        let client = TelegramBotClient::new(telegram_config(
            &api_base,
            "JMCP_TELEGRAM_ALLOWED_USER_IDS=42",
        ));

        assert_eq!(
            client.get_me().await.unwrap().username.as_deref(),
            Some("jmcp_test_bot")
        );
        let updates: Vec<TelegramUpdate> = client.get_updates(None, 0).await.unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].update_id, 10);
    }

    #[tokio::test]
    async fn telegram_submit_delivers_challenge_and_approve_is_single_use() {
        let (api_base, sent) = fake_bot_api().await;
        let client = TelegramBotClient::new(telegram_config(
            &api_base,
            "JMCP_TELEGRAM_ALLOWED_USER_IDS=42\nJMCP_TELEGRAM_ALLOWED_CHAT_IDS=99",
        ));
        let state = AppState::new(SqliteStore::in_memory().unwrap());

        handle_telegram_message(
            &client,
            &state,
            telegram_message(
                "/submit tenant/service/entity demo.run {\"ok\":true}",
                42,
                99,
            ),
        )
        .await
        .unwrap();

        let prompt = last_sent_text(&sent);
        let token = token_from_prompt(&prompt);
        assert!(prompt.contains("JMCP approval requested"));
        assert_eq!(
            state.list_work_orders().unwrap()[0].status,
            WorkOrderStatus::AwaitingApproval
        );

        handle_telegram_message(
            &client,
            &state,
            telegram_message(&format!("/approve {token}"), 42, 99),
        )
        .await
        .unwrap();

        assert!(last_sent_text(&sent).contains("Approved"));
        assert_eq!(
            state.list_work_orders().unwrap()[0].status,
            WorkOrderStatus::Approved
        );

        handle_telegram_message(
            &client,
            &state,
            telegram_message(&format!("/approve {token}"), 42, 99),
        )
        .await
        .unwrap();

        assert!(last_sent_text(&sent).contains("already used"));
    }

    #[tokio::test]
    async fn telegram_deny_rejects_work_order() {
        let (api_base, sent) = fake_bot_api().await;
        let client = TelegramBotClient::new(telegram_config(
            &api_base,
            "JMCP_TELEGRAM_ALLOWED_USER_IDS=42\nJMCP_TELEGRAM_ALLOWED_CHAT_IDS=99",
        ));
        let state = AppState::new(SqliteStore::in_memory().unwrap());

        handle_telegram_message(
            &client,
            &state,
            telegram_message("/submit tenant/service/entity demo.run {}", 42, 99),
        )
        .await
        .unwrap();
        let token = token_from_prompt(&last_sent_text(&sent));

        handle_telegram_message(
            &client,
            &state,
            telegram_message(&format!("/deny {token}"), 42, 99),
        )
        .await
        .unwrap();

        assert!(last_sent_text(&sent).contains("Rejected"));
        assert_eq!(
            state.list_work_orders().unwrap()[0].status,
            WorkOrderStatus::Failed
        );
    }

    #[tokio::test]
    async fn telegram_rejects_unauthorized_and_malformed_submit() {
        let (api_base, sent) = fake_bot_api().await;
        let client = TelegramBotClient::new(telegram_config(
            &api_base,
            "JMCP_TELEGRAM_ALLOWED_USER_IDS=42\nJMCP_TELEGRAM_ALLOWED_CHAT_IDS=99",
        ));
        let state = AppState::new(SqliteStore::in_memory().unwrap());

        handle_telegram_message(
            &client,
            &state,
            telegram_message("/submit tenant/service/entity demo.run {}", 7, 99),
        )
        .await
        .unwrap();
        assert!(last_sent_text(&sent).contains("not allowed"));
        assert!(state.list_work_orders().unwrap().is_empty());

        handle_telegram_message(
            &client,
            &state,
            telegram_message("/submit tenant/service/entity demo.run {bad", 42, 99),
        )
        .await
        .unwrap();
        assert!(last_sent_text(&sent).contains("malformed JSON"));
        assert!(state.list_work_orders().unwrap().is_empty());
    }

    #[tokio::test]
    async fn telegram_reports_unknown_expired_and_wrong_approver_tokens() {
        let (api_base, sent) = fake_bot_api().await;
        let client = TelegramBotClient::new(telegram_config(
            &api_base,
            "JMCP_TELEGRAM_ALLOWED_CHAT_IDS=99",
        ));
        let state = AppState::new(SqliteStore::in_memory().unwrap());

        handle_telegram_message(&client, &state, telegram_message("/approve nope", 42, 99))
            .await
            .unwrap();
        assert!(last_sent_text(&sent).contains("unknown token"));

        let work_order =
            submit_from_telegram(&state, "tenant/service/entity", "demo.run", "{}").unwrap();
        let expired = state
            .create_telegram_approval_challenge(
                work_order.id,
                42,
                99,
                Some(ChronoDuration::seconds(-1)),
            )
            .unwrap();
        handle_telegram_message(
            &client,
            &state,
            telegram_message(&format!("/approve {}", expired.token), 42, 99),
        )
        .await
        .unwrap();
        assert!(last_sent_text(&sent).contains("expired token"));

        let work_order =
            submit_from_telegram(&state, "tenant/service/other", "demo.run", "{}").unwrap();
        let challenge = state
            .create_telegram_approval_challenge(work_order.id, 42, 99, None)
            .unwrap();
        handle_telegram_message(
            &client,
            &state,
            telegram_message(&format!("/approve {}", challenge.token), 43, 99),
        )
        .await
        .unwrap();
        assert!(last_sent_text(&sent).contains("wrong Telegram approver"));
    }

    #[test]
    fn telegram_offset_persists_and_reads() {
        let path = std::env::temp_dir().join(format!(
            "jmcp-telegram-offset-{}.txt",
            Uuid::new_v4().simple()
        ));

        assert_eq!(read_telegram_offset(&path).unwrap(), None);
        persist_telegram_offset(&path, 42).unwrap();
        assert_eq!(read_telegram_offset(&path).unwrap(), Some(42));
        let _ = std::fs::remove_file(path);
    }
}
