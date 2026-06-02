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

function createFixtureUniverse(): UniverseSnapshot {
  return createRuntimeUniverse({
    systems,
    workItems,
    toolAssets,
    ecosystem: {
      tools: toolAssets,
      live: false,
      degradedReason: "fixture data",
    },
  });
}

function createDegradedEcosystem(degradedReason: string) {
  return {
    tools: [] as ToolAsset[],
    live: false,
    degradedReason,
  };
}

function createRuntimeUniverse({
  systems,
  workItems,
  toolAssets,
  ecosystem,
}: {
  systems: SystemNode[];
  workItems: WorkItem[];
  toolAssets: ToolAsset[];
  ecosystem: UniverseSnapshot["ecosystem"];
}): UniverseSnapshot {
  const repoNames = repoNamesFromTools(toolAssets);
  const repoScores = repoNames.map((repo) => buildRepoScore(repo, systems, workItems, toolAssets, ecosystem));
  const activeRepos = repoScores.map((repo) => ({
    repo: repo.repo,
    toolCount: repo.toolCount,
    score: repo.score,
    health: repo.health,
  }));
  const placements = repoScores.map((repo) => ({
    agent: repo.repo,
    repo: repo.repo,
    currentTask: repo.currentTask,
    branch: repo.branch,
    pool: repo.pool,
    placement: repo.placement,
    score: repo.score,
    health: repo.health,
    degradedReason: repo.degradedReason,
  }));
  const bootstrapSlice: UniverseSlice = {
    name: "bootstrap.tui",
    live: repoScores.every((repo) => repo.coverage === 100),
    coverage: averageCoverage(repoScores.map((repo) => repo.coverage)),
    degradedReason: joinReasons(repoScores.map((repo) => repo.degradedReason).filter(Boolean)),
  };
  const ecosystemSlice: UniverseSlice = {
    name: "ecosystem",
    live: ecosystem.live,
    coverage: ecosystem.live ? 100 : 0,
    degradedReason: ecosystem.degradedReason ?? undefined,
  };
  const bootstrapTui: UniverseBootstrapTui = {
    live: bootstrapSlice.live,
    observedCoverage: averageCoverage([bootstrapSlice.coverage, ecosystemSlice.coverage]),
    activeRepos,
    repoScores,
    placements,
    degradedSlices: [bootstrapSlice, ecosystemSlice],
    degradedReason: joinReasons([bootstrapSlice.degradedReason, ecosystemSlice.degradedReason]),
  };

  return {
    live: bootstrapTui.live && ecosystem.live,
    bootstrapTui,
    ecosystem,
  };
}

function buildRepoScore(
  repo: string,
  systems: SystemNode[],
  workItems: WorkItem[],
  toolAssets: ToolAsset[],
  ecosystem: UniverseSnapshot["ecosystem"],
): UniverseRepoScore {
  const tools = toolAssets.filter((tool) => (tool.repo ?? "local").toLowerCase() === repo.toLowerCase());
  const system = systems.find((item) => item.name.toLowerCase() === repo.toLowerCase());
  const workItem = workItems.find((item) => repoMatchesWorkItem(repo, item));
  const currentTask = workItem?.title ?? "unobserved";
  const branch = workItem?.branch ?? "unobserved";
  const pool = workItem?.owner ?? system?.role ?? "unassigned";
  const placement = system?.name ?? repo.toLowerCase();
  const coverage = repoCoverage(currentTask, branch, pool);
  const score = repoScore(coverage, tools);
  return {
    repo,
    toolCount: tools.length,
    score,
    coverage,
    currentTask,
    branch,
    pool,
    placement,
    health: scoreHealth(score),
    degradedReason: repoDegradedReason(repo, currentTask, branch, pool, tools.length, ecosystem),
  };
}

function repoNamesFromTools(_toolAssets: ToolAsset[]) {
  return ["Jeryu", "Jekko", "Jankurai"];
}

function repoMatchesWorkItem(repo: string, workItem: WorkItem) {
  const repoLower = repo.toLowerCase();
  return (
    workItem.repo?.toLowerCase() === repoLower ||
    workItem.owner.toLowerCase().includes(repoLower) ||
    workItem.title.toLowerCase().includes(repoLower) ||
    workItem.branch?.toLowerCase().includes(repoLower) === true
  );
}

function repoCoverage(currentTask: string, branch: string, pool: string) {
  let observed = 0;
  if (currentTask !== "unobserved") observed += 1;
  if (branch !== "unobserved") observed += 1;
  if (pool !== "unassigned") observed += 1;
  return Math.round((observed / 3) * 100);
}

function repoScore(coverage: number, tools: ToolAsset[]) {
  const penalties = tools.reduce((sum, tool) => {
    if (tool.health === "degraded") return sum + 18;
    if (tool.health === "blocked") return sum + 22;
    if (tool.health === "watch") return sum + 8;
    return sum;
  }, 0);
  return clamp(46 + Math.round(coverage / 2) + tools.length * 4 - penalties);
}

function scoreHealth(score: number): Health {
  if (score >= 85) {
    return "nominal";
  }
  if (score >= 65) {
    return "watch";
  }
  if (score >= 35) {
    return "degraded";
  }
  return "blocked";
}

function repoDegradedReason(
  repo: string,
  currentTask: string,
  branch: string,
  pool: string,
  toolCount: number,
  ecosystem: UniverseSnapshot["ecosystem"],
) {
  const reasons: string[] = [];
  if (toolCount === 0) {
    reasons.push(`${repo} has no observed ecosystem tools`);
  }
  if (currentTask === "unobserved") {
    reasons.push(`${repo} current task not observed`);
  }
  if (branch === "unobserved") {
    reasons.push(`${repo} branch not observed`);
  }
  if (pool === "unassigned") {
    reasons.push(`${repo} pool not observed`);
  }
  if (!ecosystem.live && repo.toLowerCase() === "jeryu" && ecosystem.degradedReason) {
    reasons.push(ecosystem.degradedReason);
  }
  return joinReasons(reasons);
}

function averageCoverage(values: number[]) {
  if (values.length === 0) {
    return 0;
  }
  return Math.round(values.reduce((sum, value) => sum + value, 0) / values.length);
}

function joinReasons(values: Array<string | undefined | null>) {
  const reasons = values.filter((value): value is string => Boolean(value && value.trim()));
  return reasons.length === 0 ? undefined : reasons.join("; ");
}

function clamp(value: number) {
  return Math.max(0, Math.min(100, value));
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
