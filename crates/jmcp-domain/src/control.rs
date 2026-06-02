use crate::{ApprovalDecision, AttentionLevel, Evidence, HealthLevel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
