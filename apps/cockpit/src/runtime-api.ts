import type { Health, SystemNode, ToolAsset } from "./types";

export type ApiWorkOrder = {
  id: string;
  subject: string;
  status: string;
  task: { kind: string };
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

function isSystem(value: unknown): value is SystemNode {
  return isRecord(value) && isString(value.name) && isString(value.role) && isHealth(value.health) && isString(value.jcp) && isString(value.latency);
}

export function isSystemArray(value: unknown): value is SystemNode[] {
  return Array.isArray(value) && value.every(isSystem);
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
