import type {
  EvidenceBundle,
  ViewDefinition,
  WorkItem,
} from "./types";

export const views: ViewDefinition[] = [
  { id: "now", label: "Now", description: "Attention inbox, live risk, and throughput." },
  { id: "voice-text", label: "Voice/Text", description: "Conversation turns, transcripts, and confirmations." },
  { id: "work", label: "Work", description: "Governed tasks, leases, and task state." },
  { id: "evidence", label: "Evidence", description: "Proof bundles accepted before promotion." },
  { id: "systems", label: "Systems", description: "JCP-speaking services and adapters." },
  { id: "universe", label: "Universe", description: "Jeryu coverage, repo scores, and placement rows." },
  { id: "memory", label: "Memory", description: "Scoped lessons, promotion gates, and incidents." },
  { id: "replay", label: "Replay", description: "JPCM stream reconstruction and audit trail." },
  { id: "approvals", label: "Approvals", description: "Decisions that require user authority." },
];

export const workItems: WorkItem[] = [
  {
    id: "WO-1042",
    title: "Promote JCP schema bindings into the SDK crate",
    owner: "jmcp-core",
    state: "evidence-gate",
    risk: "medium",
    lease: "write:crates/jcp-core",
    updated: "2m ago",
    evidence: 5,
    repo: "Jeryu",
    branch: "main",
  },
  {
    id: "WO-1043",
    title: "Quarantine the MCP bridge until the service card is complete",
    owner: "adapter/mcp",
    state: "blocked",
    risk: "high",
    lease: "read-only",
    updated: "9m ago",
    evidence: 2,
    repo: "Jekko",
    branch: "jmcp/bridge-quarantine",
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
    repo: "Jankurai",
    branch: "policy/replay-ratchet",
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
