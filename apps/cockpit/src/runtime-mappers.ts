import type {
  ApiAdapters,
  ApiApproval,
  ApiApprovalChallenge,
  ApiAttentionPacket,
  ApiEvidence,
  ApiMemoryProposal,
  ApiReplay,
  ApiUniverse,
  ApiVoiceThread,
  ApiWorkOrder,
} from "./runtime-api";
import type {
  AttentionAlternative,
  AttentionIncident,
  AttentionPacket,
  ApprovalRequest,
  EvidenceBundle,
  DrilldownRef,
  MemoryIncident,
  MemoryProposal,
  MemoryPromotion,
  ReplayEvent,
  UniverseSnapshot,
  ToolAsset,
  VoiceTextThread,
  WorkItem,
} from "./types";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

const BRANCH_PAYLOAD_KEYS = ["branch", "repo_branch", "repoBranch", "git_branch", "gitBranch"] as const;
const REPO_PAYLOAD_KEYS = ["repo", "repository", "provider"] as const;

type RuntimeStringCandidate =
  | { state: "present"; value: string }
  | { state: "absent"; reason: "payload_not_record" | "missing_string" | "subject_unrecognized" };

export function mapWorkOrder(workOrder: ApiWorkOrder): WorkItem {
  const owner = workOrder.subject.split("/")[1] ?? "jmcp";
  const state = workOrder.status.toLowerCase();
  const repo = repoCandidateForWorkOrder(workOrder);
  const branch = stringCandidateFromPayload(workOrder.task.payload, BRANCH_PAYLOAD_KEYS);
  return {
    id: workOrder.id,
    title: workOrder.task.kind,
    owner,
    state,
    risk: state === "failed" ? "high" : state === "awaitingapproval" ? "medium" : "low",
    lease: state === "submitted" ? "lease required" : "lease active",
    updated: formatAge(workOrder.updated_at),
    evidence: workOrder.evidence.length,
    ...optionalWorkField("repo", repo),
    ...optionalWorkField("branch", branch),
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

export function mapAttentionPacket(packet: ApiAttentionPacket): AttentionPacket {
  return {
    id: packet.attention_packet_id,
    workOrderId: packet.work_order_id,
    attentionLevel: packet.attention_level,
    modality: packet.modality,
    summary: packet.user_visible_summary,
    whyNow: packet.why_now ?? packet.user_visible_summary,
    recommendation: packet.recommendation,
    decisionNeeded: packet.decision_needed,
    alternatives: (packet.options ?? packet.alternatives ?? []).map(mapAttentionAlternative),
    riskDelta: packet.risk_delta ?? {
      from: "medium",
      to: "medium",
      note: "No risk delta supplied.",
    },
    drilldown: (packet.drilldown_refs ?? []).map(mapDrilldownRef),
    expires: packet.expires_at ? formatUntil(packet.expires_at) : "open",
    incident: packet.incident ? mapAttentionIncident(packet.incident) : undefined,
  };
}

export function mapVoiceThread(thread: ApiVoiceThread): VoiceTextThread {
  return {
    id: thread.interaction_id,
    channel: thread.channel,
    speaker: thread.speaker_id,
    title: thread.title,
    state: thread.voice_state ?? "draft",
    confidence: thread.confidence ?? 0,
    transcript: thread.transcript ?? thread.message ?? "",
    intent: thread.intent,
    confirmationPhrase: thread.confirmation_phrase ?? undefined,
    requiresResponse: thread.requires_response,
    decisionOptions: thread.decision_options ?? [],
    updated: formatAgeOrLiteral(thread.updated_at),
    sourceRef: thread.source_ref,
  };
}

export function mapMemoryProposal(proposal: ApiMemoryProposal): MemoryProposal {
  return {
    id: proposal.memory_id,
    scope: proposal.scope,
    claim: proposal.claim,
    state: proposal.lesson_state,
    confidence: proposal.confidence,
    retention: proposal.retention,
    expiry: formatUntilOrLiteral(proposal.expiry),
    promotion: mapMemoryPromotion(proposal.promotion),
    counterexamples: proposal.counterexamples,
    source: proposal.source,
    rollback: proposal.rollback,
    incident: proposal.incident ? mapMemoryIncident(proposal.incident) : undefined,
  };
}

export function mapApproval(approval: ApiApproval): ApprovalRequest {
  return {
    id: approval.work_order_id,
    challengeId: approval.work_order_id,
    workOrderId: approval.work_order_id,
    channel: "local",
    state: approval.decision ? approval.decision.toLowerCase() : "pending",
    decision: approval.decision ?? `Awaiting ${approval.approver}`,
    reason: "JMCP approval gate",
    risk: "medium",
    expires: formatUntil(approval.expires_at),
    approver: approval.approver,
    tokenHash: `sha256:${approval.work_order_id.slice(0, 12)}`,
    workOrderTitle: "Local approval",
    workOrderState: "pending",
    workOrderOwner: approval.approver,
    currentTask: "local approval",
    branch: "unobserved",
    pool: "local",
    placement: "jmcpd",
    lineage: [`approval.${approval.work_order_id}`],
  };
}

export function mapApprovalChallenge(
  challenge: ApiApprovalChallenge,
  context?: { workOrder?: WorkItem; approval?: ApiApproval; voiceThread?: VoiceTextThread },
): ApprovalRequest {
  const state = challenge.state.toLowerCase();
  const workOrder = context?.workOrder;
  const voiceThread = context?.voiceThread;
  return {
    id: challenge.id,
    challengeId: challenge.id,
    workOrderId: challenge.work_order_id,
    channel: challenge.channel,
    state,
    decision: challenge.decision ?? `Awaiting ${challenge.approver}`,
    reason: approvalChallengeReason(challenge),
    risk: riskForApprovalState(state),
    expires: formatUntil(challenge.expires_at),
    approver: challenge.approver,
    tokenHash: challenge.token_hash,
    targetUserId: challenge.target_user_id ?? undefined,
    targetChatId: challenge.target_chat_id ?? undefined,
    workOrderTitle: workOrder?.title ?? "unobserved",
    workOrderState: workOrder?.state ?? "unobserved",
    workOrderOwner: workOrder?.owner ?? "unobserved",
    currentTask: workOrder?.title ?? "unobserved",
    branch: workOrder?.branch ?? "unobserved",
    pool: workOrder?.owner ?? "unassigned",
    placement: workOrder?.owner ?? "unplaced",
    voiceThreadId: voiceThread?.id,
    voiceThreadState: voiceThread?.state,
    voiceTranscript: voiceThread?.transcript,
    voiceConfirmationPhrase: voiceThread?.confirmationPhrase,
    lineage: buildApprovalLineage(challenge, workOrder, voiceThread, context?.approval),
  };
}

export function mapUniverse(universe: ApiUniverse): UniverseSnapshot {
  return {
    live: universe.live,
    bootstrapTui: {
      live: universe.bootstrapTui.live,
      observedCoverage: universe.bootstrapTui.observedCoverage,
      activeRepos: universe.bootstrapTui.activeRepos,
      repoScores: universe.bootstrapTui.repoScores,
      placements: universe.bootstrapTui.placements,
      degradedSlices: universe.bootstrapTui.degradedSlices,
      degradedReason: universe.bootstrapTui.degradedReason,
    },
    ecosystem: universe.ecosystem,
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

function mapAttentionAlternative(alternative: NonNullable<ApiAttentionPacket["alternatives"]>[number]): AttentionAlternative {
  return {
    id: alternative.option_id,
    label: alternative.label,
    effect: alternative.effect,
    risk: alternative.risk,
  };
}

function mapDrilldownRef(ref: NonNullable<ApiAttentionPacket["drilldown_refs"]>[number]): DrilldownRef {
  return {
    label: ref.label,
    target: ref.target,
    kind: ref.kind ?? undefined,
  };
}

function mapAttentionIncident(incident: NonNullable<ApiAttentionPacket["incident"]>): AttentionIncident {
  return {
    id: incident.incident_id,
    title: incident.title,
    severity: incident.severity,
    summary: incident.summary,
    quarantine: incident.quarantine,
    drilldown: incident.drilldown,
  };
}

function mapMemoryPromotion(promotion: ApiMemoryProposal["promotion"]): MemoryPromotion {
  return {
    status: promotion.status,
    gate: promotion.gate,
    reviewedBy: promotion.reviewed_by ?? undefined,
    promotedAt: promotion.promoted_at ? formatAgeOrLiteral(promotion.promoted_at) : undefined,
  };
}

function mapMemoryIncident(incident: NonNullable<ApiMemoryProposal["incident"]>): MemoryIncident {
  return {
    title: incident.title,
    summary: incident.summary,
    quarantine: incident.quarantine,
    drilldown: incident.drilldown,
  };
}

function approvalChallengeReason(challenge: ApiApprovalChallenge) {
  if (challenge.channel === "telegram") {
    return `Telegram user ${challenge.target_user_id ?? "unknown"} in chat ${challenge.target_chat_id ?? "unknown"}`;
  }
  return "Local approval challenge";
}

function buildApprovalLineage(
  challenge: ApiApprovalChallenge,
  workOrder?: WorkItem,
  voiceThread?: VoiceTextThread,
  approval?: ApiApproval,
) {
  return [
    `challenge.${challenge.id}`,
    `work.${challenge.work_order_id}`,
    workOrder ? `task.${workOrder.title}` : "task.unobserved",
    approval?.decision ? `decision.${approval.decision.toLowerCase()}` : "decision.pending",
    voiceThread ? `voice.${voiceThread.id}` : "voice.unobserved",
  ];
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

function repoCandidateForWorkOrder(workOrder: ApiWorkOrder): RuntimeStringCandidate {
  const fromPayload = stringCandidateFromPayload(workOrder.task.payload, REPO_PAYLOAD_KEYS);
  if (fromPayload.state === "present") {
    return fromPayload;
  }
  return repoCandidateFromSubject(workOrder.subject);
}

function stringCandidateFromPayload(
  payload: unknown,
  keys: readonly string[],
): RuntimeStringCandidate {
  if (!isRecord(payload)) {
    return { state: "absent", reason: "payload_not_record" };
  }
  for (const key of keys) {
    const value = payload[key];
    if (isString(value) && value.trim().length > 0) {
      return { state: "present", value };
    }
  }
  return { state: "absent", reason: "missing_string" };
}

function repoCandidateFromSubject(subject: string): RuntimeStringCandidate {
  const parts = subject.split("/");
  if (parts.length > 1 && parts[1]) {
    return { state: "present", value: titleCase(parts[1]) };
  }
  return { state: "absent", reason: "subject_unrecognized" };
}

function optionalWorkField(
  key: "repo" | "branch",
  candidate: RuntimeStringCandidate,
): Partial<Pick<WorkItem, "repo" | "branch">> {
  if (candidate.state === "present") {
    return { [key]: candidate.value };
  }
  return {};
}

function formatAge(value: string) {
  const time = new Date(value).getTime();
  if (Number.isNaN(time)) {
    return value;
  }
  const seconds = Math.max(0, Math.round((Date.now() - time) / 1000));
  if (seconds < 60) {
    return `${seconds}s ago`;
  }
  return `${Math.round(seconds / 60)}m ago`;
}

function formatAgeOrLiteral(value: string) {
  const formatted = formatAge(value);
  return formatted === value ? value : formatted;
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

function formatUntilOrLiteral(value: string) {
  const formatted = formatUntil(value);
  return formatted === "unknown" ? value : formatted;
}
