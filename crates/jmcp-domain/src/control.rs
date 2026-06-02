use crate::{ApprovalDecision, AttentionLevel, Evidence, HealthLevel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSession {
    pub id: Uuid,
    pub work_order_id: Option<Uuid>,
    pub channel: String,
    pub transcript: String,
    pub confidence: f32,
    pub candidate: VoiceCandidate,
    pub confirmation_evidence: Vec<Evidence>,
    pub state: VoiceSessionState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCandidate {
    pub decision: ApprovalDecision,
    pub risk: VoiceRiskLevel,
    pub confirmation_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceRiskLevel {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceSessionState {
    Candidate,
    Confirmed,
    Rejected,
    Quarantined,
    Expired,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionPacket {
    pub id: Uuid,
    pub work_order_id: Option<Uuid>,
    pub title: String,
    pub why_now: String,
    pub alternatives: Vec<String>,
    pub risk_delta: String,
    pub drill_down: String,
    pub level: AttentionLevel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecord {
    pub id: Uuid,
    pub lesson: String,
    pub scope: String,
    pub source_evidence: Vec<Evidence>,
    pub freshness: String,
    pub counterexamples: Vec<String>,
    pub poisoning_checks: Vec<String>,
    pub promotion_policy: String,
    pub state: MemoryPromotionState,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPromotionState {
    Shadow,
    Proposed,
    Promoted,
    Expired,
    Quarantined,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCard {
    pub id: Uuid,
    pub kind: InventoryCardKind,
    pub name: String,
    pub owner: String,
    pub allowed_uses: Vec<String>,
    pub disallowed_uses: Vec<String>,
    pub cost: String,
    pub tests: Vec<String>,
    pub safety_case: String,
    pub health: HealthLevel,
    pub repo: Option<String>,
    pub provider: Option<String>,
    pub queue: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryCardKind {
    Model,
    Tool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionDecision {
    pub id: Uuid,
    pub target_kind: String,
    pub target_name: String,
    pub gate: String,
    pub verdict: PromotionVerdict,
    pub verifier: String,
    pub rollback_plan: String,
    pub evidence_count: usize,
    pub created_at: DateTime<Utc>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionVerdict {
    Shadow,
    Proposed,
    Promoted,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentRecord {
    pub id: Uuid,
    pub title: String,
    pub severity: IncidentSeverity,
    pub state: IncidentState,
    pub quarantine_scope: String,
    pub containment: String,
    pub related_work_orders: Vec<Uuid>,
    pub notes: Vec<String>,
    pub opened_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentSeverity {
    Info,
    Warning,
    Major,
    Critical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentState {
    Open,
    Investigating,
    Quarantined,
    Mitigated,
    Closed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousActionCard {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub mode: AutonomousActionMode,
    pub subject: AutonomousActionSubject,
    pub work_order_kind: AutonomousWorkOrderKind,
    pub manifest: AutonomousActionManifestMetadata,
    pub safety: AutonomousActionSafetyDefaults,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousActionMode {
    FullAuto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct AutonomousActionSubject(pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct AutonomousWorkOrderKind(pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousActionManifestMetadata {
    pub path: String,
    pub manifest_id: String,
    pub name: String,
    pub objective: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousActionSafetyDefaults {
    pub evidence_oriented: bool,
    pub live: bool,
    pub max_stages: u32,
    pub time_budget_hours: f64,
    pub per_phase_timeout_secs: u64,
    pub poll_timeout_secs: u64,
    pub submitted_by: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutonomousActionOverrides {
    pub run_id: Option<String>,
    pub db: Option<String>,
    pub live: Option<bool>,
    pub max_stages: Option<u32>,
    pub time_budget_hours: Option<f64>,
    pub per_phase_timeout_secs: Option<u64>,
    pub poll_timeout_secs: Option<u64>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrotaskCard {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub kind: MicrotaskKind,
    pub subject: MicrotaskSubject,
    pub work_order_kind: MicrotaskWorkOrderKind,
    pub resource_intent: MicrotaskResourceIntent,
    pub safety: MicrotaskSafetyDefaults,
    pub inputs: MicrotaskInputDefaults,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MicrotaskKind(pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MicrotaskSubject(pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MicrotaskWorkOrderKind(pub String);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrotaskResourceIntent {
    pub network: MicrotaskResourceScope,
    pub gpu: MicrotaskResourceScope,
    pub speech: MicrotaskResourceScope,
    pub durable_mutation: MicrotaskResourceScope,
    pub evidence_goal: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicrotaskResourceScope {
    None,
    LocalOnly,
    InventoryOnly,
    EvidenceOnly,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrotaskSafetyDefaults {
    pub evidence_oriented: bool,
    pub live: bool,
    pub allow_network: bool,
    pub allow_gpu: bool,
    pub allow_external_durable_mutation: bool,
    pub max_stages: u32,
    pub timeout_secs: u64,
    pub submitted_by: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MicrotaskInputDefaults {
    pub repo: Option<String>,
    pub concept: Option<String>,
    pub model_roots: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MicrotaskOverrides {
    pub run_id: Option<String>,
    pub repo: Option<String>,
    pub concept: Option<String>,
    pub model_root: Option<String>,
    pub live: Option<bool>,
    pub allow_network: Option<bool>,
    pub allow_gpu: Option<bool>,
    pub allow_external_durable_mutation: Option<bool>,
    pub max_stages: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub metadata: Option<Value>,
}
