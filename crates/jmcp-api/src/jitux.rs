use crate::routes::internal_error;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    Json,
};
use chrono::Utc;
use jmcp_app::AppState;
use jmcp_domain::{
    DeckMode, DeckPatch, DeckRankFactors, DeckRankReason, JituxFrame, JituxFrameBase,
    JituxFrameSource, PaneRankReason, PaneVm,
};
use jmcp_now::{queue_blockers_projection, NowReads, QueueBlockersProjection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::{Arc, Mutex, OnceLock},
};
use tokio_stream::{self as stream, Stream};
use uuid::Uuid;

const MAX_SESSIONS: usize = 64;
const MAX_BACKLOG: usize = 64;

static JITUX_HUB: OnceLock<Arc<JituxHub>> = OnceLock::new();

pub(crate) fn hub() -> &'static Arc<JituxHub> {
    JITUX_HUB.get_or_init(|| Arc::new(JituxHub::default()))
}

#[derive(Default)]
pub(crate) struct JituxHub {
    sessions: Mutex<HashMap<String, JituxSession>>,
}

#[derive(Clone)]
struct JituxSession {
    backlog: Vec<JituxFrame>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateJituxSessionRequest {
    pub(crate) prompt: Option<String>,
    pub(crate) source: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateJituxSessionResponse {
    pub(crate) session_id: String,
    pub(crate) stream_url: String,
    pub(crate) ws_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JituxActionRequest {
    pub(crate) action_id: String,
}

impl JituxHub {
    fn create_session(
        &self,
        request: CreateJituxSessionRequest,
        projection: QueueBlockersProjection,
    ) -> CreateJituxSessionResponse {
        let session_id = format!("jitux_{}", Uuid::new_v4().simple());
        let frames = ignition_frames(
            &session_id,
            request.prompt.as_deref(),
            request.source.as_deref(),
            projection,
        );
        let mut sessions = self.sessions.lock().expect("jitux hub mutex poisoned");
        if sessions.len() >= MAX_SESSIONS {
            if let Some(oldest) = sessions.keys().next().cloned() {
                sessions.remove(&oldest);
            }
        }
        sessions.insert(session_id.clone(), JituxSession { backlog: frames });
        CreateJituxSessionResponse {
            stream_url: format!("/jitux/sessions/{session_id}/stream"),
            ws_url: format!("/jitux/sessions/{session_id}/ws"),
            session_id,
        }
    }

    pub(crate) fn backlog(&self, session_id: &str) -> Option<Vec<JituxFrame>> {
        self.sessions
            .lock()
            .expect("jitux hub mutex poisoned")
            .get(session_id)
            .map(|session| session.backlog.clone())
    }

    fn action_preview(&self, session_id: &str, action_id: &str) -> Option<Value> {
        self.backlog(session_id)?;
        Some(json!({
            "sessionId": session_id,
            "actionId": action_id,
            "mode": "preview_only",
            "committed": false,
            "message": "JITUX actions are previews unless committed through JMCP authority routes."
        }))
    }
}

pub(crate) async fn create_jitux_session(
    State(state): State<AppState>,
    Json(request): Json<CreateJituxSessionRequest>,
) -> Result<Json<CreateJituxSessionResponse>, (StatusCode, String)> {
    let reads = NowReads::from_state(&state).map_err(internal_error)?;
    let projection = queue_blockers_projection(&reads, Utc::now());
    Ok(Json(hub().create_session(request, projection)))
}

pub(crate) async fn jitux_session_stream(
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let frames = hub().backlog(&id).ok_or(StatusCode::NOT_FOUND)?;
    let stream = stream::iter(frames.into_iter().map(|frame| {
        let data = serde_json::to_string(&frame).unwrap_or_else(|_| "{}".to_owned());
        Ok(Event::default().event("jitux.frame").data(data))
    }));
    Ok(Sse::new(stream))
}

pub(crate) async fn jitux_session_ws(Path(id): Path<String>, ws: WebSocketUpgrade) -> Response {
    let Some(frames) = hub().backlog(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    ws.on_upgrade(move |socket| send_backlog(socket, frames))
        .into_response()
}

pub(crate) async fn jitux_session_action(
    Path(id): Path<String>,
    Json(request): Json<JituxActionRequest>,
) -> Result<Json<Value>, StatusCode> {
    let preview = hub()
        .action_preview(&id, &request.action_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(preview))
}

async fn send_backlog(mut socket: WebSocket, frames: Vec<JituxFrame>) {
    for frame in frames {
        let Ok(data) = serde_json::to_string(&frame) else {
            continue;
        };
        if socket.send(Message::Text(data)).await.is_err() {
            return;
        }
    }
}

fn ignition_frames(
    session_id: &str,
    prompt: Option<&str>,
    source: Option<&str>,
    projection: QueueBlockersProjection,
) -> Vec<JituxFrame> {
    let title = match prompt {
        Some(prompt) if prompt.to_lowercase().contains("queue") => "Scanning queue blockers",
        _ => "Preparing Mission Deck",
    };
    let mut seq = 1;
    let mut frames = VecDeque::with_capacity(MAX_BACKLOG);
    frames.push_back(JituxFrame::DeckPatch {
        base: base(session_id, seq, JituxFrameSource::Projection),
        deck: DeckPatch {
            title: title.to_owned(),
            active: true,
            mode: DeckMode::MissionDeck,
        },
    });
    seq += 1;

    let pane_ids = projection
        .panes
        .iter()
        .map(|pane| pane.id.clone())
        .collect::<Vec<_>>();
    let focus_pane_id = pane_ids.first().cloned();

    for (index, pane) in projection.panes.iter().enumerate() {
        if index == 0 {
            frames.push_back(JituxFrame::CardGhost {
                base: base(session_id, seq, JituxFrameSource::Projection),
                pane: pane.clone(),
            });
        } else {
            frames.push_back(JituxFrame::PanePrepare {
                base: base(session_id, seq, JituxFrameSource::Projection),
                pane: pane.clone(),
                reason: prepare_reason(source, pane, &projection.rank_reasons),
            });
        }
        seq += 1;
    }

    frames.push_back(JituxFrame::DeckRankChanged {
        base: base(session_id, seq, JituxFrameSource::Projection),
        ordered_pane_ids: pane_ids.clone(),
        reasons: projection.rank_reasons.clone(),
    });
    seq += 1;
    if let Some(focus_pane_id) = focus_pane_id {
        frames.push_back(JituxFrame::FocusChange {
            base: base(session_id, seq, JituxFrameSource::Projection),
            pane_id: focus_pane_id.clone(),
            reason: focus_reason(&focus_pane_id, &projection),
        });
        seq += 1;
    }

    for pane in &projection.panes {
        if let Some(evidence) = projection.evidence_refs.get(&pane.id) {
            if !evidence.is_empty() {
                frames.push_back(JituxFrame::EvidenceAttach {
                    base: base(session_id, seq, JituxFrameSource::Projection),
                    pane_id: pane.id.clone(),
                    evidence: evidence.clone(),
                    freshness_ms: pane.freshness_ms,
                    confidence: Some(pane.confidence),
                });
                seq += 1;
            }
        }
        if let Some(actions) = projection.prepared_actions.get(&pane.id) {
            for action in actions {
                frames.push_back(JituxFrame::ActionReady {
                    base: base(session_id, seq, JituxFrameSource::Projection),
                    pane_id: pane.id.clone(),
                    action: action.clone(),
                });
                seq += 1;
            }
        }
        frames.push_back(JituxFrame::CardHydrated {
            base: base(session_id, seq, JituxFrameSource::Projection),
            pane_id: pane.id.clone(),
            prepared_tabs: pane.prepared_tabs.clone(),
        });
        seq += 1;
    }

    frames.push_back(JituxFrame::SessionDone {
        base: base(session_id, seq, JituxFrameSource::Projection),
        summary: "Mission Deck ignition frames emitted.".to_owned(),
    });
    frames.into_iter().collect()
}

fn base(session_id: &str, seq: u64, source: JituxFrameSource) -> JituxFrameBase {
    JituxFrameBase {
        v: 1,
        session_id: session_id.to_owned(),
        seq,
        frame_id: format!("frame_{seq:04}"),
        emitted_at: Utc::now(),
        source,
        ttl_ms: Some(30_000),
    }
}

fn prepare_reason(source: Option<&str>, pane: &PaneVm, reasons: &[PaneRankReason]) -> String {
    let source = source.unwrap_or("local");
    reasons
        .iter()
        .find(|reason| reason.pane_id == pane.id)
        .map(|reason| {
            format!(
                "{} Warmed for {source} Mission Deck session.",
                reason.reason.explanation
            )
        })
        .unwrap_or_else(|| format!("{} warmed for {source} Mission Deck session.", pane.title))
}

fn focus_reason(pane_id: &str, projection: &QueueBlockersProjection) -> DeckRankReason {
    projection
        .rank_reasons
        .iter()
        .find(|reason| reason.pane_id == pane_id)
        .map(|reason| reason.reason.clone())
        .unwrap_or_else(|| DeckRankReason {
            score: 0.0,
            factors: DeckRankFactors {
                risk: 0.0,
                blockedness: 0.0,
                approval_expiry_pressure: 0.0,
                lease_pressure: 0.0,
                adapter_degraded_weight: 0.0,
                evidence_gap_weight: 0.0,
                user_query_relevance: 0.0,
                freshness: 0.0,
                downstream_blast_radius: 0.0,
            },
            explanation: "Focus pane came from the queue-blocker projection.".to_owned(),
        })
}
