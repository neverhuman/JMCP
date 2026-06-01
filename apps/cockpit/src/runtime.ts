import { approvalRequests, evidenceBundles, replayEvents, systems, toolAssets, workItems } from "./fixtures";
import type { ApprovalRequest, EvidenceBundle, Health, ReplayEvent, SystemNode, ToolAsset, WorkItem } from "./types";

const apiUrl = import.meta.env.VITE_JMCP_API_URL ?? "http://127.0.0.1:18877";

export type RuntimeState = {
  apiHealth: Health;
  workItems: WorkItem[];
  evidenceBundles: EvidenceBundle[];
  systems: SystemNode[];
  toolAssets: ToolAsset[];
  replayEvents: ReplayEvent[];
  approvalRequests: ApprovalRequest[];
  ecosystemLive: boolean;
  ecosystemDegradedReason: string;
  loadedAt: string;
  usingFixtures: boolean;
};

type ApiWorkOrder = {
  id: string;
  subject: string;
  status: string;
  task: { kind: string };
  evidence: unknown[];
  updated_at: string;
};

type ApiEvidence = {
  kind: string;
  uri: string;
  captured_at: string;
};

type ApiApproval = {
  work_order_id: string;
  approver: string;
  expires_at: string;
  decision?: string | null;
};

type ApiApprovalChallenge = {
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

type ApiReplay = {
  events: number;
  checkpoints: Array<{ id: string; last_event_id: number; created_at: string }>;
};

type ApiAdapters = {
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

type ApiEcosystem = {
  tools: ToolAsset[];
  live: boolean;
  degradedReason?: string;
};

type EventBatch = Array<{ id?: number; event_type?: string }>;

export function createFixtureRuntime(): RuntimeState {
  return {
    apiHealth: "degraded",
    workItems,
    evidenceBundles,
    systems,
    toolAssets,
    replayEvents,
    approvalRequests,
    ecosystemLive: false,
    ecosystemDegradedReason: "fixture data",
    loadedAt: "fixture",
    usingFixtures: true,
  };
}

export async function loadRuntime(): Promise<RuntimeState> {
  if (typeof fetch !== "function") {
    return createFixtureRuntime();
  }

  const [
    health,
    apiWork,
    apiEvidence,
    apiSystems,
    apiReplay,
    apiApprovals,
    apiApprovalChallenges,
    apiAdapters,
    apiEcosystem,
  ] = await Promise.allSettled([
    getJson<{ ok: boolean }>("/health", isHealthResponse),
    getJson<ApiWorkOrder[]>("/work-orders", isWorkOrderArray),
    getJson<ApiEvidence[]>("/evidence", isEvidenceArray),
    getJson<SystemNode[]>("/systems", isSystemArray),
    getJson<ApiReplay>("/replay", isReplay),
    getJson<ApiApproval[]>("/approvals", isApprovalArray),
    getJson<ApiApprovalChallenge[]>("/approval-challenges", isApprovalChallengeArray),
    getJson<ApiAdapters>("/adapters", isAdapters),
    getJson<ApiEcosystem>("/ecosystem", isEcosystem),
  ]);

  const allFailed = [
    health,
    apiWork,
    apiEvidence,
    apiSystems,
    apiReplay,
    apiApprovals,
    apiApprovalChallenges,
    apiAdapters,
    apiEcosystem,
  ].every((result) => result.status === "rejected");
  if (allFailed) {
    return createFixtureRuntime();
  }

  const liveWork = apiWork.status === "fulfilled" ? apiWork.value.map(mapWorkOrder) : workItems;
  const liveEvidence = apiEvidence.status === "fulfilled" ? apiEvidence.value.map(mapEvidence) : evidenceBundles;
  const liveSystems = apiSystems.status === "fulfilled" ? apiSystems.value : systems;
  const liveReplay = apiReplay.status === "fulfilled" ? mapReplay(apiReplay.value) : replayEvents;
  const liveApprovals =
    apiApprovalChallenges.status === "fulfilled"
      ? apiApprovalChallenges.value.map(mapApprovalChallenge)
      : apiApprovals.status === "fulfilled"
        ? apiApprovals.value.map(mapApproval)
        : approvalRequests;
  const adapterTools = apiAdapters.status === "fulfilled" ? mapAdapters(apiAdapters.value) : toolAssets;
  const liveTools = apiEcosystem.status === "fulfilled" ? apiEcosystem.value.tools : adapterTools;
  const ecosystemLive = apiEcosystem.status === "fulfilled" ? apiEcosystem.value.live : false;
  const ecosystemDegradedReason =
    apiEcosystem.status === "fulfilled"
      ? apiEcosystem.value.degradedReason ?? "Jeryu ecosystem unavailable"
      : "Jeryu ecosystem endpoint unavailable";
  const partialFailure = [
    health,
    apiWork,
    apiEvidence,
    apiSystems,
    apiReplay,
    apiApprovals,
    apiApprovalChallenges,
    apiAdapters,
    apiEcosystem,
  ].some((result) => result.status === "rejected");

  return {
    apiHealth: partialFailure || health.status !== "fulfilled" || !health.value.ok ? "watch" : "nominal",
    workItems: liveWork,
    evidenceBundles: liveEvidence,
    systems: liveSystems,
    toolAssets: liveTools,
    replayEvents: liveReplay,
    approvalRequests: liveApprovals,
    ecosystemLive,
    ecosystemDegradedReason,
    loadedAt: new Date().toISOString().slice(11, 19) + "Z",
    usingFixtures: partialFailure,
  };
}

export function hasValidEventBatch(data: string): boolean {
  try {
    const payload: unknown = JSON.parse(data);
    return isEventBatch(payload);
  } catch {
    return false;
  }
}

async function getJson<T>(path: string, validator: (value: unknown) => value is T): Promise<T> {
  const response = await fetch(`${apiUrl}${path}`);
  if (!response.ok) {
    throw new Error(`JMCP API ${path} returned ${response.status}`);
  }
  const payload: unknown = await response.json();
  if (!validator(payload)) {
    throw new Error(`JMCP API ${path} returned an unexpected payload`);
  }
  return payload;
}

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

function isHealthResponse(value: unknown): value is { ok: boolean } {
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

function isWorkOrderArray(value: unknown): value is ApiWorkOrder[] {
  return Array.isArray(value) && value.every(isWorkOrder);
}

function isEvidence(value: unknown): value is ApiEvidence {
  return isRecord(value) && isString(value.kind) && isString(value.uri) && isString(value.captured_at);
}

function isEvidenceArray(value: unknown): value is ApiEvidence[] {
  return Array.isArray(value) && value.every(isEvidence);
}

function isSystem(value: unknown): value is SystemNode {
  return isRecord(value) && isString(value.name) && isString(value.role) && isHealth(value.health) && isString(value.jcp) && isString(value.latency);
}

function isSystemArray(value: unknown): value is SystemNode[] {
  return Array.isArray(value) && value.every(isSystem);
}

function isReplay(value: unknown): value is ApiReplay {
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

function isApprovalArray(value: unknown): value is ApiApproval[] {
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

function isApprovalChallengeArray(value: unknown): value is ApiApprovalChallenge[] {
  return Array.isArray(value) && value.every(isApprovalChallenge);
}

function isAdapters(value: unknown): value is ApiAdapters {
  return (
    isRecord(value) &&
    Array.isArray(value.service_cards) &&
    value.service_cards.every(
      (card) => isRecord(card) && isString(card.name) && isStringArray(card.capabilities) && isStringArray(card.subjects),
    ) &&
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

function isEcosystem(value: unknown): value is ApiEcosystem {
  return (
    isRecord(value) &&
    Array.isArray(value.tools) &&
    value.tools.every((tool) => isRecord(tool) && isString(tool.name) && isString(tool.className)) &&
    typeof value.live === "boolean" &&
    (value.degradedReason === undefined || value.degradedReason === null || isString(value.degradedReason))
  );
}

function isEventBatch(value: unknown): value is EventBatch {
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

function mapWorkOrder(workOrder: ApiWorkOrder): WorkItem {
  const owner = workOrder.subject.split("/")[1] ?? "jmcp";
  const state = workOrder.status.toLowerCase();
  return {
    id: workOrder.id,
    title: workOrder.task.kind,
    owner,
    state,
    risk: state === "failed" ? "high" : state === "awaitingapproval" ? "medium" : "low",
    lease: state === "submitted" ? "lease required" : "lease active",
    updated: formatAge(workOrder.updated_at),
    evidence: workOrder.evidence.length,
  };
}

function mapEvidence(evidence: ApiEvidence): EvidenceBundle {
  return {
    id: evidence.uri.slice(0, 12),
    subject: evidence.kind,
    source: "jmcpd",
    status: "accepted",
    hash: evidence.uri,
    age: formatAge(evidence.captured_at),
  };
}

function mapReplay(replay: ApiReplay): ReplayEvent[] {
  if (replay.events === 0 && replay.checkpoints.length === 0) {
    return [];
  }
  const checkpoints =
    replay.checkpoints.length > 0
      ? replay.checkpoints
      : [
          {
            id: "live-events",
            last_event_id: replay.events,
            created_at: new Date().toISOString(),
          },
        ];
  return checkpoints.map((checkpoint) => ({
    sequence: checkpoint.last_event_id,
    subject: checkpoint.id,
    family: "ReplayCheckpoint",
    timestamp: new Date(checkpoint.created_at).toISOString().slice(11, 19) + "Z",
    producer: "jmcpd",
  }));
}

function mapApproval(approval: ApiApproval): ApprovalRequest {
  return {
    id: approval.work_order_id,
    workOrderId: approval.work_order_id,
    channel: "local",
    state: approval.decision ? approval.decision.toLowerCase() : "pending",
    decision: approval.decision ?? `Awaiting ${approval.approver}`,
    reason: "JMCP approval gate",
    risk: "medium",
    expires: formatUntil(approval.expires_at),
  };
}

function mapApprovalChallenge(challenge: ApiApprovalChallenge): ApprovalRequest {
  const state = challenge.state.toLowerCase();
  return {
    id: challenge.id,
    workOrderId: challenge.work_order_id,
    channel: challenge.channel,
    state,
    decision: challenge.decision ?? `Awaiting ${challenge.approver}`,
    reason: approvalChallengeReason(challenge),
    risk: riskForApprovalState(state),
    expires: formatUntil(challenge.expires_at),
  };
}

function approvalChallengeReason(challenge: ApiApprovalChallenge) {
  if (challenge.channel === "telegram") {
    return `Telegram user ${challenge.target_user_id ?? "unknown"} in chat ${challenge.target_chat_id ?? "unknown"}`;
  }
  return "Local approval challenge";
}

function riskForApprovalState(state: string): "low" | "medium" | "high" {
  if (state === "approved") {
    return "low";
  }
  if (state === "rejected" || state === "expired") {
    return "high";
  }
  return "medium";
}

function mapAdapters(adapters: ApiAdapters): ToolAsset[] {
  const healthByName = new Map(adapters.health.map((item) => [item.name, item]));
  return adapters.service_cards.flatMap((card) => {
    const health = healthByName.get(card.name);
    return card.capabilities.map((capability) => ({
      name: `${card.name}.${capability}`,
      className: card.subjects.join(", "),
      conformance: card.name === "jmcpd" ? "C2 native" : "C1 governed",
      sideEffects: capability.includes("health") || capability.includes("status") ? "none" : "lease gated",
      dataClasses: [card.name, capability],
      repo: card.name === "jmcpd" ? "JMCP" : titleCase(card.name),
      provider: card.name,
      health: health?.health ?? "degraded",
      dependsOn: card.name === "jeryu" ? ["jmcpd.work-orders", "jankurai.proof"] : ["jmcpd.leases"],
      queue: health?.health === "nominal" ? 0 : 1,
    }));
  });
}

function titleCase(value: string) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

function formatAge(value: string) {
  const time = new Date(value).getTime();
  if (Number.isNaN(time)) {
    return "live";
  }
  const seconds = Math.max(0, Math.round((Date.now() - time) / 1000));
  if (seconds < 60) {
    return `${seconds}s ago`;
  }
  return `${Math.round(seconds / 60)}m ago`;
}

function formatUntil(value: string) {
  const time = new Date(value).getTime();
  if (Number.isNaN(time)) {
    return "unknown";
  }
  const seconds = Math.round((time - Date.now()) / 1000);
  if (seconds <= 0) {
    return "expired";
  }
  if (seconds < 60) {
    return `${seconds}s`;
  }
  return `${Math.round(seconds / 60)}m`;
}
