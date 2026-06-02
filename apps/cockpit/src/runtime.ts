import { approvalRequests, evidenceBundles, replayEvents, systems, toolAssets, workItems } from "./fixtures";
import {
  isAdapters,
  isApprovalArray,
  isApprovalChallengeArray,
  isEcosystem,
  isEventBatch,
  isEvidenceArray,
  isHealthResponse,
  isReplay,
  isSystemArray,
  isWorkOrderArray,
  type ApiAdapters,
  type ApiApproval,
  type ApiApprovalChallenge,
  type ApiEcosystem,
  type ApiEvidence,
  type ApiReplay,
  type ApiWorkOrder,
} from "./runtime-api";
import { mapAdapters, mapApproval, mapApprovalChallenge, mapEvidence, mapReplay, mapWorkOrder } from "./runtime-mappers";
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
