import {
  attentionPackets,
  approvalRequests,
  evidenceBundles,
  memoryLessons,
  replayEvents,
  systems,
  toolAssets,
  voiceTextThreads,
  workItems,
} from "./fixtures";
import {
  isAdapters,
  isApprovalArray,
  isApprovalChallengeArray,
  isAttentionPacketArray,
  isEcosystem,
  isEventBatch,
  isEvidenceArray,
  isHealthResponse,
  isMemoryProposalArray,
  isReplay,
  isSystemArray,
  isUniverse,
  isVoiceThreadArray,
  isWorkOrderArray,
  type ApiAdapters,
  type ApiApproval,
  type ApiApprovalChallenge,
  type ApiAttentionPacket,
  type ApiEcosystem,
  type ApiEvidence,
  type ApiMemoryProposal,
  type ApiReplay,
  type ApiUniverse,
  type ApiVoiceThread,
  type ApiWorkOrder,
} from "./runtime-api";
import {
  mapAdapters,
  mapApproval,
  mapApprovalChallenge,
  mapAttentionPacket,
  mapEvidence,
  mapMemoryProposal,
  mapReplay,
  mapUniverse,
  mapVoiceThread,
  mapWorkOrder,
} from "./runtime-mappers";
import type {
  ApprovalRequest,
  AttentionPacket,
  EvidenceBundle,
  Health,
  MemoryProposal,
  ReplayEvent,
  SystemNode,
  UniverseActiveRepo,
  UniverseBootstrapTui,
  UniversePlacement,
  UniverseRepoScore,
  UniverseSlice,
  UniverseSnapshot,
  ToolAsset,
  VoiceTextThread,
  WorkItem,
} from "./types";

import { createDegradedEcosystem, createFixtureUniverse, createRuntimeUniverse, getJson } from "./runtime-helpers";
const apiUrl = import.meta.env.VITE_JMCP_API_URL ?? "http://127.0.0.1:18877";

export type RuntimeState = {
  apiHealth: Health;
  workItems: WorkItem[];
  evidenceBundles: EvidenceBundle[];
  systems: SystemNode[];
  toolAssets: ToolAsset[];
  universe: UniverseSnapshot;
  attentionPackets: AttentionPacket[];
  voiceThreads: VoiceTextThread[];
  memoryLessons: MemoryProposal[];
  replayEvents: ReplayEvent[];
  approvalRequests: ApprovalRequest[];
  ecosystemLive: boolean;
  ecosystemDegradedReason: string;
  loadedAt: string;
  usingFixtures: boolean;
};

export function createFixtureRuntime(): RuntimeState {
  return {
    apiHealth: "degraded",
    workItems,
    evidenceBundles,
    systems,
    toolAssets,
    universe: createFixtureUniverse(),
    attentionPackets,
    voiceThreads: voiceTextThreads,
    memoryLessons,
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
    apiAttention,
    apiVoiceText,
    apiMemory,
    apiReplay,
    apiApprovals,
    apiApprovalChallenges,
    apiAdapters,
    apiEcosystem,
    apiUniverse,
  ] = await Promise.allSettled([
    getJson<{ ok: boolean }>("/health", isHealthResponse),
    getJson<ApiWorkOrder[]>("/work-orders", isWorkOrderArray),
    getJson<ApiEvidence[]>("/evidence", isEvidenceArray),
    getJson<SystemNode[]>("/systems", isSystemArray),
    getJson<ApiAttentionPacket[]>("/attention", isAttentionPacketArray),
    getJson<ApiVoiceThread[]>("/voice-text", isVoiceThreadArray),
    getJson<ApiMemoryProposal[]>("/memory", isMemoryProposalArray),
    getJson<ApiReplay>("/replay", isReplay),
    getJson<ApiApproval[]>("/approvals", isApprovalArray),
    getJson<ApiApprovalChallenge[]>("/approval-challenges", isApprovalChallengeArray),
    getJson<ApiAdapters>("/adapters", isAdapters),
    getJson<ApiEcosystem>("/ecosystem", isEcosystem),
    getJson<ApiUniverse>("/universe", isUniverse),
  ]);

  const allFailed = [
    health,
    apiWork,
    apiEvidence,
    apiSystems,
    apiAttention,
    apiVoiceText,
    apiMemory,
    apiReplay,
    apiApprovals,
    apiApprovalChallenges,
    apiAdapters,
    apiEcosystem,
    apiUniverse,
  ].every((result) => result.status === "rejected");
  if (allFailed) {
    return createFixtureRuntime();
  }

  const liveWork = apiWork.status === "fulfilled" ? apiWork.value.map(mapWorkOrder) : workItems;
  const liveEvidence = apiEvidence.status === "fulfilled" ? apiEvidence.value.map(mapEvidence) : evidenceBundles;
  const liveSystems = apiSystems.status === "fulfilled" ? apiSystems.value : systems;
  const liveAttention = apiAttention.status === "fulfilled" ? apiAttention.value.map(mapAttentionPacket) : attentionPackets;
  const liveVoiceText = apiVoiceText.status === "fulfilled" ? apiVoiceText.value.map(mapVoiceThread) : voiceTextThreads;
  const liveMemory = apiMemory.status === "fulfilled" ? apiMemory.value.map(mapMemoryProposal) : memoryLessons;
  const liveReplay = apiReplay.status === "fulfilled" ? mapReplay(apiReplay.value) : replayEvents;
  const liveUniverse =
    apiUniverse.status === "fulfilled"
      ? mapUniverse(apiUniverse.value)
      : createRuntimeUniverse({
          systems: liveSystems,
          workItems: liveWork,
          toolAssets: apiEcosystem.status === "fulfilled" ? apiEcosystem.value.tools : apiAdapters.status === "fulfilled" ? mapAdapters(apiAdapters.value) : toolAssets,
          ecosystem: apiEcosystem.status === "fulfilled" ? apiEcosystem.value : createDegradedEcosystem(apiAdapters.status === "rejected" ? "Jeryu ecosystem endpoint unavailable" : "Jeryu ecosystem unavailable"),
        });
  const workOrderById = new Map(liveWork.map((item) => [item.id, item]));
  const approvalByWorkOrderId = new Map(
    apiApprovals.status === "fulfilled" ? apiApprovals.value.map((approval) => [approval.work_order_id, approval]) : [],
  );
  const voiceByApprover = new Map(liveVoiceText.map((thread) => [thread.speaker, thread]));
  const liveApprovals =
    apiApprovalChallenges.status === "fulfilled"
      ? apiApprovalChallenges.value.map((challenge) =>
          mapApprovalChallenge(challenge, {
            workOrder: workOrderById.get(challenge.work_order_id),
            approval: approvalByWorkOrderId.get(challenge.work_order_id),
            voiceThread: voiceByApprover.get(challenge.approver),
          }),
        )
      : apiApprovals.status === "fulfilled"
        ? apiApprovals.value.map(mapApproval)
        : approvalRequests;
  const liveTools = liveUniverse.ecosystem.tools.length > 0 ? liveUniverse.ecosystem.tools : apiAdapters.status === "fulfilled" ? mapAdapters(apiAdapters.value) : toolAssets;
  const ecosystemLive = liveUniverse.ecosystem.live;
  const ecosystemDegradedReason = liveUniverse.ecosystem.degradedReason ?? "Jeryu ecosystem unavailable";
  const partialFailure = [
    health,
    apiWork,
    apiEvidence,
    apiSystems,
    apiAttention,
    apiVoiceText,
    apiMemory,
    apiReplay,
    apiApprovals,
    apiApprovalChallenges,
    apiAdapters,
    apiEcosystem,
    apiUniverse,
  ].some((result) => result.status === "rejected");

  return {
    apiHealth: partialFailure || health.status !== "fulfilled" || !health.value.ok ? "watch" : "nominal",
    workItems: liveWork,
    evidenceBundles: liveEvidence,
    systems: liveSystems,
    toolAssets: liveTools,
    universe: liveUniverse,
    attentionPackets: liveAttention,
    voiceThreads: liveVoiceText,
    memoryLessons: liveMemory,
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

