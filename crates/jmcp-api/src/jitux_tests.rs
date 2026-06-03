use crate::jitux::{
    create_jitux_session, hub, jitux_session_action, jitux_session_stream,
    CreateJituxSessionRequest, JituxActionRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use jmcp_app::AppState;
use jmcp_domain::{JituxFrame, MicrotaskOverrides};
use jmcp_store::SqliteStore;

fn test_state_with_blocker() -> AppState {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    state
        .submit_microtask("research.concept-scan", MicrotaskOverrides::default())
        .expect("microtask work order");
    state
}

#[tokio::test]
async fn creating_session_returns_stream_and_ws_paths() {
    let response = create_jitux_session(
        State(test_state_with_blocker()),
        Json(CreateJituxSessionRequest {
            prompt: Some("What's blocking the queue right now?".to_owned()),
            source: Some("text".to_owned()),
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
    assert!(matches!(frames.first(), Some(JituxFrame::DeckPatch { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::CardGhost { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::FocusChange { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::ActionReady { .. })));
    assert!(frames
        .iter()
        .any(|frame| matches!(frame, JituxFrame::CardHydrated { .. })));
    assert!(frames.iter().any(|frame| match frame {
        JituxFrame::DeckRankChanged {
            ordered_pane_ids, ..
        } => ordered_pane_ids
            .iter()
            .any(|pane_id| pane_id.starts_with("queue_blockers:")),
        _ => false,
    }));
}

#[tokio::test]
async fn action_route_is_preview_only() {
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
        Path(response.session_id),
        Json(JituxActionRequest {
            action_id: "show_evidence".to_owned(),
        }),
    )
    .await
    .unwrap();

    assert_eq!(preview["mode"], "preview_only");
    assert_eq!(preview["committed"], false);
}

#[tokio::test]
async fn unknown_session_stream_is_not_found() {
    let error = jitux_session_stream(Path("missing".to_owned()))
        .await
        .unwrap_err();

    assert_eq!(error, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_session_action_is_not_found() {
    let error = jitux_session_action(
        Path("missing".to_owned()),
        Json(JituxActionRequest {
            action_id: "show_evidence".to_owned(),
        }),
    )
    .await
    .unwrap_err();

    assert_eq!(error, StatusCode::NOT_FOUND);
}
