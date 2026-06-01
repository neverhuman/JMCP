import type {
  ApprovalRequest,
  EvidenceBundle,
  MemoryProposal,
  ReplayEvent,
  SystemNode,
  ToolAsset,
  ViewDefinition,
  WorkItem,
} from "./types";

export const views: ViewDefinition[] = [
  { id: "now", label: "Now", description: "Attention packet, live risk, and throughput." },
  { id: "work", label: "Work", description: "Governed tasks, leases, and task state." },
  { id: "evidence", label: "Evidence", description: "Proof bundles accepted before promotion." },
  { id: "systems", label: "Systems", description: "JCP-speaking services and adapters." },
  { id: "tools-data", label: "Tools/Data", description: "Capability graph and governed assets." },
  { id: "memory-lite", label: "Memory-lite", description: "Scoped lessons awaiting promotion." },
  { id: "replay", label: "Replay", description: "JPCM stream reconstruction and audit trail." },
  { id: "approvals", label: "Approvals", description: "Decisions that require user authority." },
];

export const workItems: WorkItem[] = [
  {
    id: "WO-1042",
    title: "Promote JCP schema bindings into sdk crate",
    owner: "jmcp-core",
    state: "evidence-gate",
    risk: "medium",
    lease: "write:crates/jcp-core",
    updated: "2m ago",
    evidence: 5,
  },
  {
    id: "WO-1043",
    title: "Quarantine legacy MCP bridge until service card is complete",
    owner: "adapter/mcp",
    state: "blocked",
    risk: "high",
    lease: "read-only",
    updated: "9m ago",
    evidence: 2,
  },
  {
    id: "WO-1044",
    title: "Compress replay incident into reusable policy test",
    owner: "jankurai",
    state: "running",
    risk: "low",
    lease: "lesson-proposal",
    updated: "14m ago",
    evidence: 3,
  },
];

export const evidenceBundles: EvidenceBundle[] = [
  {
    id: "EV-731",
    subject: "schema-bindings-build",
    source: "jeryu/checks",
    status: "accepted",
    hash: "sha256:b91f...a204",
    age: "4m",
  },
  {
    id: "EV-732",
    subject: "adapter-service-card",
    source: "jcp-conformance",
    status: "pending",
    hash: "sha256:8d3a...19fe",
    age: "7m",
  },
  {
    id: "EV-733",
    subject: "direct-write-audit",
    source: "policy-kernel",
    status: "rejected",
    hash: "sha256:f044...77be",
    age: "11m",
  },
];

export const systems: SystemNode[] = [
  { name: "jmcpd", role: "authority kernel", health: "nominal", jcp: "1.0.0", latency: "18ms" },
  { name: "jeryu", role: "evidence runner", health: "watch", jcp: "1.0.0", latency: "42ms" },
  { name: "jankurai", role: "standards memory", health: "nominal", jcp: "1.0.0", latency: "25ms" },
  { name: "mcp-bridge", role: "quarantined adapter", health: "blocked", jcp: "adapter", latency: "n/a" },
];

export const toolAssets: ToolAsset[] = [
  {
    name: "repo.apply_patch",
    className: "code mutation",
    conformance: "C2 native",
    sideEffects: "workspace write",
    dataClasses: ["source", "diff"],
    repo: "JMCP",
    provider: "codex",
    health: "nominal",
    dependsOn: ["jeryu.repo.adopt", "jankurai.diff-audit"],
    queue: 1,
  },
  {
    name: "ci.run_lane",
    className: "evidence",
    conformance: "C1 constrained",
    sideEffects: "compute spend",
    dataClasses: ["logs", "artifacts"],
    repo: "JMCP",
    provider: "jeryu",
    health: "watch",
    dependsOn: ["jeryu.checks.status", "jankurai.proof"],
    queue: 2,
  },
  {
    name: "store.query_lessons",
    className: "memory read",
    conformance: "C2 native",
    sideEffects: "none",
    dataClasses: ["lessons", "policy"],
    repo: "jankurai",
    provider: "jankurai",
    health: "nominal",
    dependsOn: ["jeryu.evidence.bundle"],
    queue: 0,
  },
  {
    name: "jeryu.repo.adopt",
    className: "repository governance",
    conformance: "C1 constrained",
    sideEffects: "local git remote",
    dataClasses: ["repo", "policy"],
    repo: "Jeryu",
    provider: "jeryu",
    health: "watch",
    dependsOn: ["git.remote", "jeryu.api.health"],
    queue: 1,
  },
  {
    name: "jeryu.evidence.bundle",
    className: "evidence",
    conformance: "C2 native",
    sideEffects: "artifact write",
    dataClasses: ["logs", "hashes", "checks"],
    repo: "Jeryu",
    provider: "jeryu",
    health: "nominal",
    dependsOn: ["ci.run_lane"],
    queue: 0,
  },
  {
    name: "jekko.run_headless",
    className: "worker execution",
    conformance: "C1 leased",
    sideEffects: "tool calls",
    dataClasses: ["prompt", "diff", "logs"],
    repo: "Jekko",
    provider: "jekko",
    health: "degraded",
    dependsOn: ["lease.validate", "jeryu.evidence.bundle"],
    queue: 0,
  },
];

export const memoryProposals: MemoryProposal[] = [
  {
    id: "ML-219",
    lesson: "Adapters that emit raw webhooks must be quarantined until wrapped in JCP envelopes.",
    scope: "adapter conformance",
    status: "proposed",
    confidence: 92,
  },
  {
    id: "ML-220",
    lesson: "Evidence gates need independent replay checks for schema promotion tasks.",
    scope: "release policy",
    status: "shadow",
    confidence: 81,
  },
  {
    id: "ML-221",
    lesson: "Direct credential access inside workers is a policy violation even when tests pass.",
    scope: "authority kernel",
    status: "promoted",
    confidence: 98,
  },
];

export const replayEvents: ReplayEvent[] = [
  {
    sequence: 18421,
    subject: "work.WO-1042.task.schema",
    family: "TaskStateChanged",
    timestamp: "12:08:14Z",
    producer: "jmcpd",
  },
  {
    sequence: 18422,
    subject: "evidence.EV-731",
    family: "EvidenceAttached",
    timestamp: "12:08:21Z",
    producer: "jeryu",
  },
  {
    sequence: 18423,
    subject: "attention.approval.AP-88",
    family: "ApprovalRequested",
    timestamp: "12:09:02Z",
    producer: "policy-kernel",
  },
];

export const approvalRequests: ApprovalRequest[] = [
  {
    id: "AP-88",
    decision: "Allow bridge to request a temporary write lease",
    reason: "Legacy MCP bridge lacks C1 service-card evidence.",
    risk: "high",
    expires: "6m",
  },
  {
    id: "AP-89",
    decision: "Promote memory lesson ML-221 globally",
    reason: "Lesson affects authority policy across all workers.",
    risk: "medium",
    expires: "22m",
  },
];
