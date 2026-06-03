use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowSnapshot {
    pub generation: i64,
    pub captured_at: DateTime<Utc>,
    pub deck: Vec<PanePreview>,
    pub default_pane: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanePreview {
    pub id: String,
    pub kind: PaneKind,
    pub title: String,
    pub headline: String,
    pub chips: Vec<String>,
    pub counters: Vec<Counter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sparkline: Option<Vec<f64>>,
    pub rank: f64,
    pub focus_score: f64,
    pub confidence: f64,
    pub severity: RiskBand,
    pub status: PaneStatus,
    pub predicted_next: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counter {
    pub label: String,
    pub value: CounterValue,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CounterValue {
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub key: String,
    pub kind: PaneKind,
    pub mode: SceneMode,
    pub accent: Accent,
    pub title: String,
    pub layout: SceneLayout,
    pub status: PaneStatus,
    pub generation: i64,
    pub captured_at: DateTime<Utc>,
    pub cards: Vec<Card>,
    pub narration_hint: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub kind: CardKind,
    pub title: String,
    pub status: CardStatus,
    pub rank: f64,
    pub risk: RiskBand,
    pub why_now: String,
    pub rank_reason: RankReason,
    pub evidence_refs: Vec<DrilldownRef>,
    pub drilldowns: Vec<DrilldownRef>,
    pub actions: Vec<PreparedAction>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedAction {
    pub id: String,
    pub label: String,
    pub safety_class: SafetyClass,
    pub ready: bool,
    pub reason: String,
    pub target: String,
    pub method: ActionMethod,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrilldownRef {
    pub id: String,
    pub label: String,
    pub kind: DrilldownKind,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drilldown {
    pub id: String,
    pub label: String,
    pub kind: DrilldownKind,
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankReason {
    pub score: f64,
    pub factors: RankFactors,
    pub summary: String,
    pub dominant_factor: RankFactorKind,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankFactors {
    pub risk: f64,
    pub actionability: f64,
    pub freshness: f64,
    pub blast_radius: f64,
    pub lease_pressure: f64,
    pub user_relevance: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneStatus {
    Predicted,
    Incubating,
    Warm,
    Active,
    Degraded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneMode {
    Focus,
    Fan,
    Compare,
    Tunnel,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    Purple,
    Red,
    Amber,
    Green,
    Blue,
    Slate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneLayout {
    Stack,
    Timeline,
    Matrix,
    Drilldown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    QueueBlocker,
    Lease,
    Approval,
    Evidence,
    Incident,
    AdapterHealth,
    Replay,
    Autonomy,
    Memory,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    Probing,
    Ranked,
    Verified,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskBand {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClass {
    ReadOnly,
    BoundedAuto,
    ApprovalRequired,
    ManualOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DrilldownKind {
    Evidence,
    Lease,
    Approval,
    Replay,
    System,
    Raw,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMethod {
    Get,
    Post,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RankFactorKind {
    Risk,
    Actionability,
    Freshness,
    BlastRadius,
    LeasePressure,
    UserRelevance,
}
