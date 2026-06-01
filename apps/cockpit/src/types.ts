export type ViewId =
  | "now"
  | "work"
  | "evidence"
  | "systems"
  | "tools-data"
  | "memory-lite"
  | "replay"
  | "approvals";

export type Health = "nominal" | "watch" | "blocked" | "degraded";
export type Risk = "low" | "medium" | "high";

export interface ViewDefinition {
  id: ViewId;
  label: string;
  description: string;
}

export interface WorkItem {
  id: string;
  title: string;
  owner: string;
  state: string;
  risk: Risk;
  lease: string;
  updated: string;
  evidence: number;
}

export interface EvidenceBundle {
  id: string;
  subject: string;
  source: string;
  status: "accepted" | "pending" | "rejected";
  hash: string;
  age: string;
}

export interface SystemNode {
  name: string;
  role: string;
  health: Health;
  jcp: string;
  latency: string;
}

export interface ToolAsset {
  name: string;
  className: string;
  conformance: string;
  sideEffects: string;
  dataClasses: string[];
  repo?: string;
  provider?: string;
  health?: Health;
  dependsOn?: string[];
  queue?: number;
}

export interface MemoryProposal {
  id: string;
  lesson: string;
  scope: string;
  status: "shadow" | "proposed" | "promoted";
  confidence: number;
}

export interface ReplayEvent {
  sequence: number;
  subject: string;
  family: string;
  timestamp: string;
  producer: string;
}

export interface ApprovalRequest {
  id: string;
  workOrderId: string;
  channel: string;
  state: string;
  decision: string;
  reason: string;
  risk: Risk;
  expires: string;
}
