#[path = "../src/jitux.rs"]
mod jitux;

mod routes {
    use axum::http::StatusCode;

    pub fn internal_error(err: impl std::fmt::Display) -> (StatusCode, String) {
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
    }
}

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, TimeZone, Utc};
use jitux::{
    create_jitux_session, hub, jitux_session_action, jitux_session_stream,
    CreateJituxSessionRequest, JituxActionRequest,
};
use jmcp_app::AppState;
use jmcp_domain::{JituxFrame, MicrotaskOverrides};
use jmcp_store::SqliteStore;
use std::{
    io::Write,
    process::{Command, Stdio},
};

const JITUX_FRAME_SCHEMA: &str =
    include_str!("../../../schemas/jitux/1.0.0/jitux-frame.schema.json");

fn test_state_with_blocker() -> AppState {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    state
        .submit_microtask("research.concept-scan", MicrotaskOverrides::default())
        .expect("microtask work order");
    state
}

fn validate_against_jitux_schema(instance: &serde_json::Value) {
    let schema: serde_json::Value =
        serde_json::from_str(JITUX_FRAME_SCHEMA).expect("canonical JITUX schema parses");
    let payload = serde_json::json!({
        "schema": schema,
        "instance": instance,
    });
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(
            r#"
import json
import jsonschema
import sys

payload = json.load(sys.stdin)
jsonschema.Draft202012Validator(payload["schema"]).validate(payload["instance"])
"#,
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 jsonschema validator starts");

    let mut stdin = child.stdin.take().expect("validator stdin");
    stdin
        .write_all(serde_json::to_string(&payload).unwrap().as_bytes())
        .expect("validator input write");
    drop(stdin);

    let output = child.wait_with_output().expect("validator exits");
    assert!(
        output.status.success(),
        "JITUX schema validation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn create_session_returns_descriptor_and_schema_valid_backlog_frames() {
    let response = create_jitux_session(
        State(test_state_with_blocker()),
        Json(CreateJituxSessionRequest {
            prompt: Some("what is blocking the queue?".to_owned()),
            source: Some("deck".to_owned()),
        }),
    )
    .await
    .unwrap()
    .0;

    assert!(response.session_id.starts_with("jitux_"));
    assert!(response.stream_url.ends_with("/stream"));
    assert!(response.ws_url.ends_with("/ws"));

    let frames = hub()
        .backlog(&response.session_id)
        .expect("session backlog");
    assert!(!frames.is_empty());
    assert!(matches!(frames.first(), Some(JituxFrame::DeckPatch { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::CardGhost { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::DeckRankChanged { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::FocusChange { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::ActionReady { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::CardHydrated { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::SessionDone { .. })));

    for frame in &frames {
        let value = serde_json::to_value(frame).expect("frame json");
        validate_against_jitux_schema(&value);
        let decoded: JituxFrame = serde_json::from_value(value).expect("frame round trip");
        assert_eq!(&decoded, frame);
    }
}

#[tokio::test]
async fn action_preview_is_non_committal() {
    let response = create_jitux_session(
        State(test_state_with_blocker()),
        Json(CreateJituxSessionRequest {
            prompt: None,
            source: None,
        }),
    )
    .await
    .unwrap()
    .0;

    let Json(preview) = jitux_session_action(
        Path(response.session_id.clone()),
        Json(JituxActionRequest {
            action_id: "show_evidence".to_owned(),
        }),
    )
    .await
    .unwrap();

    assert_eq!(preview["sessionId"], response.session_id);
    assert_eq!(preview["actionId"], "show_evidence");
    assert_eq!(preview["mode"], "preview_only");
    assert_eq!(preview["committed"], false);
}

#[tokio::test]
async fn unknown_session_routes_return_not_found() {
    let stream_error = jitux_session_stream(Path("missing".to_owned()))
        .await
        .unwrap_err();
    let action_error = jitux_session_action(
        Path("missing".to_owned()),
        Json(JituxActionRequest {
            action_id: "show_evidence".to_owned(),
        }),
    )
    .await
    .unwrap_err();

    assert_eq!(stream_error, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(action_error, axum::http::StatusCode::NOT_FOUND);
}
