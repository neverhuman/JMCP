import type { AttentionLevel, Health, MemoryState, Risk, SystemNode, VoiceState } from "./types";
import type {
  ApiAdapters,
  ApiApproval,
  ApiApprovalChallenge,
  ApiAttentionPacket,
  ApiEcosystem,
  ApiEvidence,
  ApiMemoryProposal,
  ApiReplay,
  ApiUniverse,
  ApiVoiceThread,
  ApiWorkOrder,
  EventBatch,
} from "./runtime-api";

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

function isWorkOrderValue(value: unknown): value is ApiWorkOrder {
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

function isEvidenceValue(value: unknown): value is ApiEvidence {
  return isRecord(value) && isString(value.kind) && isString(value.uri) && isString(value.captured_at);
}

function isSystemIncidentValue(value: unknown): value is NonNullable<SystemNode["incident"]> {
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
    (value.incident === undefined || value.incident === null || isSystemIncidentValue(value.incident))
  );
}

function isAttentionPacketValue(value: unknown): value is ApiAttentionPacket {
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

function isVoiceThreadValue(value: unknown): value is ApiVoiceThread {
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

function isMemoryIncidentValue(value: unknown): value is NonNullable<ApiMemoryProposal["incident"]> {
  return (
    isRecord(value) &&
    isString(value.title) &&
    isString(value.summary) &&
    isString(value.quarantine) &&
    isStringArray(value.drilldown)
  );
}

function isMemoryProposalValue(value: unknown): value is ApiMemoryProposal {
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
    (value.incident === undefined || value.incident === null || isMemoryIncidentValue(value.incident))
  );
}

function isApprovalValue(value: unknown): value is ApiApproval {
  return (
    isRecord(value) &&
    isString(value.work_order_id) &&
    isString(value.approver) &&
    isString(value.expires_at) &&
    (value.decision === undefined || value.decision === null || isString(value.decision))
  );
}

function isApprovalChallengeValue(value: unknown): value is ApiApprovalChallenge {
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

function isAdaptersValue(value: unknown): value is ApiAdapters {
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

function isEcosystemValue(value: unknown): value is ApiEcosystem {
  return (
    isRecord(value) &&
    Array.isArray(value.tools) &&
    value.tools.every((tool) => isRecord(tool) && isString(tool.name) && isString(tool.className)) &&
    typeof value.live === "boolean" &&
    (value.degradedReason === undefined || value.degradedReason === null || isString(value.degradedReason))
  );
}

function isUniverseValue(value: unknown): value is ApiUniverse {
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

export function isHealthResponse(value: unknown): value is { ok: boolean } {
  return isRecord(value) && typeof value.ok === "boolean";
}

export function isWorkOrderArray(value: unknown): value is ApiWorkOrder[] {
  return Array.isArray(value) && value.every(isWorkOrderValue);
}

export function isEvidenceArray(value: unknown): value is ApiEvidence[] {
  return Array.isArray(value) && value.every(isEvidenceValue);
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

export function isSystemArray(value: unknown): value is SystemNode[] {
  return Array.isArray(value) && value.every(isSystem);
}

export function isAttentionPacketArray(value: unknown): value is ApiAttentionPacket[] {
  return Array.isArray(value) && value.every(isAttentionPacketValue);
}

export function isVoiceThreadArray(value: unknown): value is ApiVoiceThread[] {
  return Array.isArray(value) && value.every(isVoiceThreadValue);
}

export function isMemoryProposalArray(value: unknown): value is ApiMemoryProposal[] {
  return Array.isArray(value) && value.every(isMemoryProposalValue);
}

export function isApprovalArray(value: unknown): value is ApiApproval[] {
  return Array.isArray(value) && value.every(isApprovalValue);
}

export function isApprovalChallengeArray(value: unknown): value is ApiApprovalChallenge[] {
  return Array.isArray(value) && value.every(isApprovalChallengeValue);
}

export function isAdapters(value: unknown): value is ApiAdapters {
  return isAdaptersValue(value);
}

export function isEcosystem(value: unknown): value is ApiEcosystem {
  return isEcosystemValue(value);
}

export function isUniverse(value: unknown): value is ApiUniverse {
  return isUniverseValue(value);
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
