import { systems, toolAssets, workItems } from "./fixtures";
import type {
  Health,
  SystemNode,
  ToolAsset,
  UniverseBootstrapTui,
  UniverseRepoScore,
  UniverseSlice,
  UniverseSnapshot,
  WorkItem,
} from "./types";

const apiUrl = import.meta.env.VITE_JMCP_API_URL ?? "http://127.0.0.1:18877";

export function createFixtureUniverse(): UniverseSnapshot {
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

export function createDegradedEcosystem(degradedReason: string) {
  return {
    tools: [] as ToolAsset[],
    live: false,
    degradedReason,
  };
}

export function createRuntimeUniverse({
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

export async function getJson<T>(path: string, validator: (value: unknown) => value is T): Promise<T> {
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
