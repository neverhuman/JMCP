use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
    },
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    Json,
};
use chrono::Utc;
use jmcp_domain::{
    CardLod, CounterValue, DeckMode, DeckPatch, DeckRankFactors, DeckRankReason, JituxFrame,
    JituxFrameBase, JituxFrameSource, PaneCounter, PaneKind, PanePreview, PaneRankReason, PaneRisk,
    PaneStatus, PaneVm, PreparedTab,
};
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
    fn create_session(&self, request: CreateJituxSessionRequest) -> CreateJituxSessionResponse {
        let session_id = format!("jitux_{}", Uuid::new_v4().simple());
        let frames = ignition_frames(
            &session_id,
            request.prompt.as_deref(),
            request.source.as_deref(),
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
    Json(request): Json<CreateJituxSessionRequest>,
) -> Json<CreateJituxSessionResponse> {
    Json(hub().create_session(request))
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

    let focus = pane(
        "pane:queue",
        PaneKind::Queue,
        "Queue blocker",
        "Finding blocked work, approvals, leases, and adapter pressure.",
        PaneStatus::Active,
        CardLod::Focus,
        1.0,
    );
    let warm_panes = [
        pane(
            "pane:jeryu",
            PaneKind::Jeryu,
            "Jeryu context",
            "Warming ecosystem and repo signals.",
            PaneStatus::Incubating,
            CardLod::Ghost,
            0.72,
        ),
        pane(
            "pane:jailgun",
            PaneKind::Jailgun,
            "Jailgun runs",
            "Warming run, capture, and deploy attention.",
            PaneStatus::Incubating,
            CardLod::Ghost,
            0.66,
        ),
        pane(
            "pane:replay",
            PaneKind::Replay,
            "Replay",
            "Warming recent events and checkpoints.",
            PaneStatus::Incubating,
            CardLod::Ghost,
            0.74,
        ),
        pane(
            "pane:approval",
            PaneKind::Approval,
            "Approvals",
            "Warming pending and expiring approvals.",
            PaneStatus::Incubating,
            CardLod::Ghost,
            0.78,
        ),
    ];

    frames.push_back(JituxFrame::CardGhost {
        base: base(session_id, seq, JituxFrameSource::Projection),
        pane: focus.clone(),
    });
    seq += 1;
    for pane in warm_panes {
        frames.push_back(JituxFrame::PanePrepare {
            base: base(session_id, seq, JituxFrameSource::Projection),
            pane,
            reason: format!(
                "Likely drilldown for {} Mission Deck session.",
                source.unwrap_or("local")
            ),
        });
        seq += 1;
    }
    let reason = queue_rank_reason();
    frames.push_back(JituxFrame::DeckRankChanged {
        base: base(session_id, seq, JituxFrameSource::Projection),
        ordered_pane_ids: vec![
            "pane:queue".to_owned(),
            "pane:approval".to_owned(),
            "pane:replay".to_owned(),
            "pane:jeryu".to_owned(),
            "pane:jailgun".to_owned(),
        ],
        reasons: vec![PaneRankReason {
            pane_id: "pane:queue".to_owned(),
            reason: reason.clone(),
        }],
    });
    seq += 1;
    frames.push_back(JituxFrame::FocusChange {
        base: base(session_id, seq, JituxFrameSource::Projection),
        pane_id: "pane:queue".to_owned(),
        reason,
    });
    seq += 1;
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

fn pane(
    id: &str,
    kind: PaneKind,
    title: &str,
    headline: &str,
    status: PaneStatus,
    lod: CardLod,
    rank: f32,
) -> PaneVm {
    PaneVm {
        id: id.to_owned(),
        kind,
        title: title.to_owned(),
        rank,
        risk: PaneRisk::Medium,
        status,
        lod,
        confidence: rank.min(1.0),
        freshness_ms: Some(0),
        preview: PanePreview {
            headline: headline.to_owned(),
            chips: vec!["warming".to_owned()],
            counters: vec![PaneCounter {
                label: "prepared".to_owned(),
                value: CounterValue::Text("yes".to_owned()),
            }],
        },
        prepared_tabs: vec![
            PreparedTab::Evidence,
            PreparedTab::Replay,
            PreparedTab::Systems,
            PreparedTab::Actions,
        ],
    }
}

fn queue_rank_reason() -> DeckRankReason {
    DeckRankReason {
        score: 7.2,
        factors: DeckRankFactors {
            risk: 0.8,
            blockedness: 1.0,
            approval_expiry_pressure: 0.8,
            lease_pressure: 0.7,
            adapter_degraded_weight: 0.6,
            evidence_gap_weight: 0.8,
            user_query_relevance: 1.0,
            freshness: 0.8,
            downstream_blast_radius: 0.7,
        },
        explanation:
            "Queue blocker is active because the prompt asks for blocked work and related approval pressure."
                .to_owned(),
    }
}
