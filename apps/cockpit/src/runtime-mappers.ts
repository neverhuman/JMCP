import type { ApiAdapters, ApiApproval, ApiApprovalChallenge, ApiEvidence, ApiReplay, ApiWorkOrder } from "./runtime-api";
import type { ApprovalRequest, EvidenceBundle, ReplayEvent, ToolAsset, WorkItem } from "./types";

export function mapWorkOrder(workOrder: ApiWorkOrder): WorkItem {
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

export function mapEvidence(evidence: ApiEvidence): EvidenceBundle {
  return {
    id: evidence.uri.slice(0, 12),
    subject: evidence.kind,
    source: "jmcpd",
    status: "accepted",
    hash: evidence.uri,
    age: formatAge(evidence.captured_at),
  };
}

export function mapReplay(replay: ApiReplay): ReplayEvent[] {
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

export function mapApproval(approval: ApiApproval): ApprovalRequest {
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

export function mapApprovalChallenge(challenge: ApiApprovalChallenge): ApprovalRequest {
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

export function mapAdapters(adapters: ApiAdapters): ToolAsset[] {
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
