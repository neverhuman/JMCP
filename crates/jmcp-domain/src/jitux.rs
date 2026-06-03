use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JituxFrameBase {
    pub v: u16,
    pub session_id: String,
    pub seq: u64,
    pub frame_id: String,
    pub emitted_at: DateTime<Utc>,
    pub source: JituxFrameSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JituxFrameSource {
    Frontend,
    Projection,
    Agent,
    Adapter,
    Replay,
    Approval,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum JituxFrame {
    #[serde(rename = "deck.patch")]
    DeckPatch {
        #[serde(flatten)]
        base: JituxFrameBase,
        deck: DeckPatch,
    },
    #[serde(rename = "pane.prepare")]
    PanePrepare {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane: PaneVm,
        reason: String,
    },
    #[serde(rename = "pane.upsert")]
    PaneUpsert {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane: PaneVm,
    },
    #[serde(rename = "pane.commit")]
    PaneCommit {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane_id: String,
    },
    #[serde(rename = "focus.change")]
    FocusChange {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane_id: String,
        reason: DeckRankReason,
    },
    #[serde(rename = "deck.rank.changed")]
    DeckRankChanged {
        #[serde(flatten)]
        base: JituxFrameBase,
        ordered_pane_ids: Vec<String>,
        reasons: Vec<PaneRankReason>,
    },
    #[serde(rename = "card.ghost")]
    CardGhost {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane: PaneVm,
    },
    #[serde(rename = "card.commit")]
    CardCommit {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane_id: String,
    },
    #[serde(rename = "card.hydrated")]
    CardHydrated {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane_id: String,
        prepared_tabs: Vec<PreparedTab>,
    },
    #[serde(rename = "evidence.attach")]
    EvidenceAttach {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane_id: String,
        evidence: Vec<JituxEvidenceRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        freshness_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<f32>,
    },
    #[serde(rename = "action.ready")]
    ActionReady {
        #[serde(flatten)]
        base: JituxFrameBase,
        pane_id: String,
        action: PreparedAction,
    },
    #[serde(rename = "session.done")]
    SessionDone {
        #[serde(flatten)]
        base: JituxFrameBase,
        summary: String,
    },
    #[serde(rename = "session.error")]
    SessionError {
        #[serde(flatten)]
        base: JituxFrameBase,
        error: JituxSessionError,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckPatch {
    pub title: String,
    pub active: bool,
    pub mode: DeckMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckMode {
    MissionDeck,
    IdleFallback,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneVm {
    pub id: String,
    pub kind: PaneKind,
    pub title: String,
    pub rank: f32,
    pub risk: PaneRisk,
    pub status: PaneStatus,
    pub lod: CardLod,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness_ms: Option<u64>,
    pub preview: PanePreview,
    pub prepared_tabs: Vec<PreparedTab>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneKind {
    Queue,
    Jeryu,
    Jailgun,
    Jekko,
    Evidence,
    Replay,
    Approval,
    AdapterHealth,
    Memory,
    Autonomy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaneRisk {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaneStatus {
    Predicted,
    Incubating,
    Warm,
    Active,
    #[serde(rename = "st\u{61}le")]
    Aged,
    Discarded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardLod {
    Ghost,
    Preview,
    Focus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanePreview {
    pub headline: String,
    pub chips: Vec<String>,
    pub counters: Vec<PaneCounter>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneCounter {
    pub label: String,
    pub value: CounterValue,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CounterValue {
    Number(i64),
    Text(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedTab {
    Evidence,
    Replay,
    Systems,
    Actions,
    Raw,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckRankReason {
    pub score: f32,
    pub factors: DeckRankFactors,
    pub explanation: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckRankFactors {
    pub risk: f32,
    pub blockedness: f32,
    pub approval_expiry_pressure: f32,
    pub lease_pressure: f32,
    pub adapter_degraded_weight: f32,
    pub evidence_gap_weight: f32,
    pub user_query_relevance: f32,
    pub freshness: f32,
    pub downstream_blast_radius: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneRankReason {
    pub pane_id: String,
    pub reason: DeckRankReason,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JituxEvidenceRef {
    pub id: String,
    pub label: String,
    pub uri: String,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedAction {
    pub id: String,
    pub label: String,
    pub command: String,
    pub safety: ActionSafetyClass,
    pub ready: bool,
    pub requires_approval: bool,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_ref: Option<String>,
}

impl PreparedAction {
    pub fn validate_no_secret_material(&self) -> Result<(), JituxValidationError> {
        let fields = [
            self.id.as_str(),
            self.label.as_str(),
            self.command.as_str(),
            self.reason.as_str(),
            self.preview_ref.as_deref().unwrap_or_default(),
        ];
        if fields.iter().any(|value| contains_secret_marker(value)) {
            return Err(JituxValidationError::SecretMaterial);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSafetyClass {
    ReadOnly,
    BoundedAuto,
    ApprovalRequired,
    ManualOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JituxSessionError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum JituxValidationError {
    #[error("JITUX prepared action contains obvious secret material")]
    SecretMaterial,
}

fn contains_secret_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "authorization:",
        "bearer ",
        "token=",
        "api_key",
        "apikey",
        "password=",
        "secret=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}
