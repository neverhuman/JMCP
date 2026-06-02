import type { AttentionLevel, Health, MemoryState, Risk, SystemNode, ToolAsset, VoiceState } from "./types";

export type ApiWorkOrder = {
  id: string;
  subject: string;
  status: string;
  task: { kind: string; payload?: unknown };
  evidence: unknown[];
  updated_at: string;
};

export type ApiEvidence = {
  kind: string;
  uri: string;
  captured_at: string;
};

export type ApiApproval = {
  work_order_id: string;
  approver: string;
  expires_at: string;
  decision?: string | null;
};

export type ApiApprovalChallenge = {
  id: string;
  work_order_id: string;
  approver: string;
  channel: string;
  token_hash: string;
  target_user_id?: number | null;
  target_chat_id?: number | null;
  expires_at: string;
  state: string;
  decision?: string | null;
};

export type ApiReplay = {
  events: number;
  checkpoints: Array<{ id: string; last_event_id: number; created_at: string }>;
};

export type ApiAttentionPacket = {
  attention_packet_id: string;
  work_order_id: string;
  attention_level: AttentionLevel;
  modality: "text" | "voice" | "ui-card" | "notification" | "api";
  user_visible_summary: string;
  why_now?: string | null;
  recommendation: string;
  decision_needed: boolean;
  options?: Array<{
    option_id: string;
    label: string;
    effect: string;
    risk: Risk;
  }>;
  alternatives?: Array<{
    option_id: string;
    label: string;
    effect: string;
    risk: Risk;
  }>;
  risk_delta?: {
    from: Risk;
    to: Risk;
    note: string;
  };
  drilldown_refs?: Array<{
    label: string;
    target: string;
    kind?: string;
  }>;
  created_at: string;
  expires_at?: string | null;
  incident?: {
    incident_id: string;
    title: string;
    severity: Risk;
    summary: string;
    quarantine: string;
    drilldown: string[];
  };
};

export type ApiVoiceThread = {
  interaction_id: string;
  channel: "voice" | "text";
  speaker_id: string;
  title: string;
  voice_state?: VoiceState | "draft" | null;
  transcript?: string;
  message?: string;
  intent: string;
  confidence?: number | null;
  confirmation_phrase?: string | null;
  requires_response: boolean;
  decision_options?: string[];
  updated_at: string;
  source_ref: string;
  summary?: string | null;
};

export type ApiMemoryProposal = {
  memory_id: string;
  scope: string;
  claim: string;
  lesson_state: MemoryState;
  confidence: number;
  retention: string;
  expiry: string;
  promotion: {
    status: string;
    gate: string;
    reviewed_by?: string | null;
    promoted_at?: string | null;
  };
  counterexamples: string[];
  source: string;
  rollback: string;
  incident?: {
    title: string;
    summary: string;
    quarantine: string;
    drilldown: string[];
  };
};

export type ApiAdapters = {
  service_cards: Array<{
    name: string;
    capabilities: string[];
    subjects: string[];
  }>;
  health: Array<{
    name: string;
    health: Health;
    endpoint?: string | null;
    detail: string;
  }>;
};

export type ApiEcosystem = {
  tools: ToolAsset[];
  live: boolean;
  degradedReason?: string;
};

export type ApiUniverse = {
  live: boolean;
  bootstrapTui: {
    live: boolean;
    observedCoverage: number;
    activeRepos: Array<{
      repo: string;
      toolCount: number;
      score: number;
      health: Health;
    }>;
    repoScores: Array<{
      repo: string;
      toolCount: number;
      score: number;
      coverage: number;
      currentTask: string;
      branch: string;
      pool: string;
      placement: string;
      health: Health;
      degradedReason?: string;
    }>;
    placements: Array<{
      agent: string;
      repo: string;
      currentTask: string;
      branch: string;
      pool: string;
      placement: string;
      score: number;
      health: Health;
      degradedReason?: string;
    }>;
    degradedSlices: Array<{
      name: string;
      live: boolean;
      coverage: number;
      degradedReason?: string;
    }>;
    degradedReason?: string;
  };
  ecosystem: ApiEcosystem;
};

export type EventBatch = Array<{ id?: number; event_type?: string }>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(isString);
}

function isHealth(value: unknown): value is Health {
  return value === "nominal" || value === "watch" || value === "blocked" || value === "degraded";
}

function isRisk(value: unknown): value is Risk {
  return value === "low" || value === "medium" || value === "high";
}

function isAttentionLevel(value: unknown): value is AttentionLevel {
  return value === "silent" || value === "digest" || value === "heads-up" || value === "decision" || value === "urgent" || value === "incident";
}

function isVoiceState(value: unknown): value is VoiceState {
  return value === "started" || value === "transcribed" || value === "intent_detected" || value === "confirmation_requested" || value === "confirmed" || value === "denied" || value === "ended";
}

function isMemoryState(value: unknown): value is MemoryState {
  return value === "shadow" || value === "proposed" || value === "quarantined" || value === "promoted" || value === "revoked";
}

export function isHealthResponse(value: unknown): value is { ok: boolean } {
  return isRecord(value) && typeof value.ok === "boolean";
}

function isWorkOrder(value: unknown): value is ApiWorkOrder {
  return (
    isRecord(value) &&
    isString(value.id) &&
    isString(value.subject) &&
    isString(value.status) &&
    isRecord(value.task) &&
    isString(value.task.kind) &&
    Array.isArray(value.evidence) &&
    isString(value.updated_at)
  );
}

export function isWorkOrderArray(value: unknown): value is ApiWorkOrder[] {
  return Array.isArray(value) && value.every(isWorkOrder);
}

function isEvidence(value: unknown): value is ApiEvidence {
  return isRecord(value) && isString(value.kind) && isString(value.uri) && isString(value.captured_at);
}

export function isEvidenceArray(value: unknown): value is ApiEvidence[] {
  return Array.isArray(value) && value.every(isEvidence);
}

export function isReplay(value: unknown): value is ApiReplay {
  return (
    isRecord(value) &&
    isNumber(value.events) &&
    Array.isArray(value.checkpoints) &&
    value.checkpoints.every(
      (checkpoint) => isRecord(checkpoint) && isString(checkpoint.id) && isNumber(checkpoint.last_event_id) && isString(checkpoint.created_at),
    )
  );
}

function isSystemIncident(value: unknown): value is NonNullable<SystemNode["incident"]> {
  return (
    isRecord(value) &&
    isString(value.title) &&
    isString(value.summary) &&
    isString(value.quarantine) &&
    isStringArray(value.drilldown)
  );
}

function isSystem(value: unknown): value is SystemNode {
  return (
    isRecord(value) &&
    isString(value.name) &&
    isString(value.role) &&
    isHealth(value.health) &&
    isString(value.jcp) &&
    isString(value.latency) &&
    (value.incident === undefined || value.incident === null || isSystemIncident(value.incident))
  );
}

export function isSystemArray(value: unknown): value is SystemNode[] {
  return Array.isArray(value) && value.every(isSystem);
}

function isAttentionPacket(value: unknown): value is ApiAttentionPacket {
  const options = isRecord(value) && Array.isArray(value.options) ? value.options : isRecord(value) && Array.isArray(value.alternatives) ? value.alternatives : undefined;
  return (
    isRecord(value) &&
    isString(value.attention_packet_id) &&
    isString(value.work_order_id) &&
    isAttentionLevel(value.attention_level) &&
    isString(value.modality) &&
    isString(value.user_visible_summary) &&
    (value.why_now === undefined || value.why_now === null || isString(value.why_now)) &&
    isString(value.recommendation) &&
    typeof value.decision_needed === "boolean" &&
    (options === undefined ||
      options.every(
        (alternative) =>
          isRecord(alternative) &&
          isString(alternative.option_id) &&
          isString(alternative.label) &&
          isString(alternative.effect) &&
          isRisk(alternative.risk),
      )) &&
    (value.risk_delta === undefined ||
      (isRecord(value.risk_delta) && isRisk(value.risk_delta.from) && isRisk(value.risk_delta.to) && isString(value.risk_delta.note))) &&
    (value.drilldown_refs === undefined ||
      (Array.isArray(value.drilldown_refs) &&
        value.drilldown_refs.every(
          (ref) => isRecord(ref) && isString(ref.label) && isString(ref.target) && (ref.kind === undefined || ref.kind === null || isString(ref.kind)),
        ))) &&
    isString(value.created_at) &&
    (value.expires_at === undefined || value.expires_at === null || isString(value.expires_at)) &&
    (value.incident === undefined ||
      (isRecord(value.incident) &&
        isString(value.incident.incident_id) &&
        isString(value.incident.title) &&
        isRisk(value.incident.severity) &&
        isString(value.incident.summary) &&
        isString(value.incident.quarantine) &&
        isStringArray(value.incident.drilldown)))
  );
}

export function isAttentionPacketArray(value: unknown): value is ApiAttentionPacket[] {
  return Array.isArray(value) && value.every(isAttentionPacket);
}

function isVoiceThread(value: unknown): value is ApiVoiceThread {
  const transcript = isRecord(value) ? value.transcript ?? value.message : undefined;
  return (
    isRecord(value) &&
    isString(value.interaction_id) &&
    isString(value.channel) &&
    isString(value.speaker_id) &&
    isString(value.title) &&
    (value.voice_state === undefined || value.voice_state === null || isVoiceState(value.voice_state) || value.voice_state === "draft") &&
    isString(transcript) &&
    isString(value.intent) &&
    (value.confidence === undefined || value.confidence === null || isNumber(value.confidence)) &&
    (value.confirmation_phrase === undefined || value.confirmation_phrase === null || isString(value.confirmation_phrase)) &&
    typeof value.requires_response === "boolean" &&
    (value.decision_options === undefined || isStringArray(value.decision_options)) &&
    isString(value.updated_at) &&
    isString(value.source_ref) &&
    (value.summary === undefined || value.summary === null || isString(value.summary))
  );
}

export function isVoiceThreadArray(value: unknown): value is ApiVoiceThread[] {
  return Array.isArray(value) && value.every(isVoiceThread);
}

function isMemoryIncident(value: unknown): value is NonNullable<ApiMemoryProposal["incident"]> {
  return (
    isRecord(value) &&
    isString(value.title) &&
    isString(value.summary) &&
    isString(value.quarantine) &&
    isStringArray(value.drilldown)
  );
}

function isMemoryProposal(value: unknown): value is ApiMemoryProposal {
  return (
    isRecord(value) &&
    isString(value.memory_id) &&
    isString(value.scope) &&
    isString(value.claim) &&
    isMemoryState(value.lesson_state) &&
    isNumber(value.confidence) &&
    isString(value.retention) &&
    isString(value.expiry) &&
    isRecord(value.promotion) &&
    isString(value.promotion.status) &&
    isString(value.promotion.gate) &&
    (value.promotion.reviewed_by === undefined || value.promotion.reviewed_by === null || isString(value.promotion.reviewed_by)) &&
    (value.promotion.promoted_at === undefined || value.promotion.promoted_at === null || isString(value.promotion.promoted_at)) &&
    isStringArray(value.counterexamples) &&
    isString(value.source) &&
    isString(value.rollback) &&
    (value.incident === undefined || value.incident === null || isMemoryIncident(value.incident))
  );
}

export function isMemoryProposalArray(value: unknown): value is ApiMemoryProposal[] {
  return Array.isArray(value) && value.every(isMemoryProposal);
}

function isApproval(value: unknown): value is ApiApproval {
  return (
    isRecord(value) &&
    isString(value.work_order_id) &&
    isString(value.approver) &&
    isString(value.expires_at) &&
    (value.decision === undefined || value.decision === null || isString(value.decision))
  );
}

export function isApprovalArray(value: unknown): value is ApiApproval[] {
  return Array.isArray(value) && value.every(isApproval);
}

function isApprovalChallenge(value: unknown): value is ApiApprovalChallenge {
  return (
    isRecord(value) &&
    isString(value.id) &&
    isString(value.work_order_id) &&
    isString(value.approver) &&
    isString(value.channel) &&
    isString(value.token_hash) &&
    (value.target_user_id === undefined || value.target_user_id === null || isNumber(value.target_user_id)) &&
    (value.target_chat_id === undefined || value.target_chat_id === null || isNumber(value.target_chat_id)) &&
    isString(value.expires_at) &&
    isString(value.state) &&
    (value.decision === undefined || value.decision === null || isString(value.decision))
  );
}

export function isApprovalChallengeArray(value: unknown): value is ApiApprovalChallenge[] {
  return Array.isArray(value) && value.every(isApprovalChallenge);
}

export function isAdapters(value: unknown): value is ApiAdapters {
  return (
    isRecord(value) &&
    Array.isArray(value.service_cards) &&
    value.service_cards.every((card) => isRecord(card) && isString(card.name) && isStringArray(card.capabilities) && isStringArray(card.subjects)) &&
    Array.isArray(value.health) &&
    value.health.every(
      (health) =>
        isRecord(health) &&
        isString(health.name) &&
        isHealth(health.health) &&
        (health.endpoint === undefined || health.endpoint === null || isString(health.endpoint)) &&
        isString(health.detail),
    )
  );
}

export function isEcosystem(value: unknown): value is ApiEcosystem {
  return (
    isRecord(value) &&
    Array.isArray(value.tools) &&
    value.tools.every((tool) => isRecord(tool) && isString(tool.name) && isString(tool.className)) &&
    typeof value.live === "boolean" &&
    (value.degradedReason === undefined || value.degradedReason === null || isString(value.degradedReason))
  );
}

export function isUniverse(value: unknown): value is ApiUniverse {
  return (
    isRecord(value) &&
    typeof value.live === "boolean" &&
    isRecord(value.bootstrapTui) &&
    typeof value.bootstrapTui.live === "boolean" &&
    isNumber(value.bootstrapTui.observedCoverage) &&
    Array.isArray(value.bootstrapTui.activeRepos) &&
    value.bootstrapTui.activeRepos.every(
      (repo) =>
        isRecord(repo) &&
        isString(repo.repo) &&
        isNumber(repo.toolCount) &&
        isNumber(repo.score) &&
        isHealth(repo.health),
    ) &&
    Array.isArray(value.bootstrapTui.repoScores) &&
    value.bootstrapTui.repoScores.every(
      (repo) =>
        isRecord(repo) &&
        isString(repo.repo) &&
        isNumber(repo.toolCount) &&
        isNumber(repo.score) &&
        isNumber(repo.coverage) &&
        isString(repo.currentTask) &&
        isString(repo.branch) &&
        isString(repo.pool) &&
        isString(repo.placement) &&
        isHealth(repo.health) &&
        (repo.degradedReason === undefined || repo.degradedReason === null || isString(repo.degradedReason)),
    ) &&
    Array.isArray(value.bootstrapTui.placements) &&
    value.bootstrapTui.placements.every(
      (placement) =>
        isRecord(placement) &&
        isString(placement.agent) &&
        isString(placement.repo) &&
        isString(placement.currentTask) &&
        isString(placement.branch) &&
        isString(placement.pool) &&
        isString(placement.placement) &&
        isNumber(placement.score) &&
        isHealth(placement.health) &&
        (placement.degradedReason === undefined || placement.degradedReason === null || isString(placement.degradedReason)),
    ) &&
    Array.isArray(value.bootstrapTui.degradedSlices) &&
    value.bootstrapTui.degradedSlices.every(
      (slice) =>
        isRecord(slice) &&
        isString(slice.name) &&
        typeof slice.live === "boolean" &&
        isNumber(slice.coverage) &&
        (slice.degradedReason === undefined || slice.degradedReason === null || isString(slice.degradedReason)),
    ) &&
    (value.bootstrapTui.degradedReason === undefined || value.bootstrapTui.degradedReason === null || isString(value.bootstrapTui.degradedReason)) &&
    isEcosystem(value.ecosystem)
  );
}

export function isEventBatch(value: unknown): value is EventBatch {
  return (
    Array.isArray(value) &&
    value.every(
      (event) =>
        isRecord(event) &&
        (event.id === undefined || isNumber(event.id)) &&
        (event.event_type === undefined || isString(event.event_type)),
    )
  );
}
