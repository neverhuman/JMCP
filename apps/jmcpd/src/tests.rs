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

#[test]
fn structured_runtime_events_are_valid_json_and_escape_content() {
    let record = structured_event_record(
        "warn",
        "telegram.submit.rejected",
        json!({
            "reason": "bad \"payload\"\nand newline",
            "chatId": 99,
        }),
    );
    let text = record.to_string();
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["event"], "telegram.submit.rejected");
    assert_eq!(parsed["level"], "warn");
    assert_eq!(parsed["fields"]["chatId"], 99);
    assert_eq!(parsed["fields"]["reason"], "bad \"payload\"\nand newline");
}
