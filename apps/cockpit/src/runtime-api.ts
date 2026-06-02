import type { AttentionLevel, Health, MemoryState, Risk, SystemNode, ToolAsset, VoiceState } from "./types";

export type ApiWorkOrder = {
  id: string;
  subject: string;
  status: string;
  task: { kind: string; payload?: unknown };
  evidence: unknown[];
  updated_at: string;
};

export type ApiEvidence = {
  kind: string;
  uri: string;
  captured_at: string;
};

export type ApiApproval = {
  work_order_id: string;
  approver: string;
  expires_at: string;
  decision?: string | null;
};

export type ApiApprovalChallenge = {
  id: string;
  work_order_id: string;
  approver: string;
  channel: string;
  token_hash: string;
  target_user_id?: number | null;
  target_chat_id?: number | null;
  expires_at: string;
  state: string;
  decision?: string | null;
};

export type ApiReplay = {
  events: number;
  checkpoints: Array<{ id: string; last_event_id: number; created_at: string }>;
};

export type ApiAttentionPacket = {
  attention_packet_id: string;
  work_order_id: string;
  attention_level: AttentionLevel;
  modality: "text" | "voice" | "ui-card" | "notification" | "api";
  user_visible_summary: string;
  why_now?: string | null;
  recommendation: string;
  decision_needed: boolean;
  options?: Array<{
    option_id: string;
    label: string;
    effect: string;
    risk: Risk;
  }>;
  alternatives?: Array<{
    option_id: string;
    label: string;
    effect: string;
    risk: Risk;
  }>;
  risk_delta?: {
    from: Risk;
    to: Risk;
    note: string;
  };
  drilldown_refs?: Array<{
    label: string;
    target: string;
    kind?: string;
  }>;
  created_at: string;
  expires_at?: string | null;
  incident?: {
    incident_id: string;
    title: string;
    severity: Risk;
    summary: string;
    quarantine: string;
    drilldown: string[];
  };
};

export type ApiVoiceThread = {
  interaction_id: string;
  channel: "voice" | "text";
  speaker_id: string;
  title: string;
  voice_state?: VoiceState | "draft" | null;
  transcript?: string;
  message?: string;
  intent: string;
  confidence?: number | null;
  confirmation_phrase?: string | null;
  requires_response: boolean;
  decision_options?: string[];
  updated_at: string;
  source_ref: string;
  summary?: string | null;
};

export type ApiMemoryProposal = {
  memory_id: string;
  scope: string;
  claim: string;
  lesson_state: MemoryState;
  confidence: number;
  retention: string;
  expiry: string;
  promotion: {
    status: string;
    gate: string;
    reviewed_by?: string | null;
    promoted_at?: string | null;
  };
  counterexamples: string[];
  source: string;
  rollback: string;
  incident?: {
    title: string;
    summary: string;
    quarantine: string;
    drilldown: string[];
  };
};

export type ApiAdapters = {
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

export type ApiEcosystem = {
  tools: ToolAsset[];
  live: boolean;
  degradedReason?: string;
};

export type ApiUniverse = {
  live: boolean;
  bootstrapTui: {
    live: boolean;
    observedCoverage: number;
    activeRepos: Array<{
      repo: string;
      toolCount: number;
      score: number;
      health: Health;
    }>;
    repoScores: Array<{
      repo: string;
      toolCount: number;
      score: number;
      coverage: number;
      currentTask: string;
      branch: string;
      pool: string;
      placement: string;
      health: Health;
      degradedReason?: string;
    }>;
    placements: Array<{
      agent: string;
      repo: string;
      currentTask: string;
      branch: string;
      pool: string;
      placement: string;
      score: number;
      health: Health;
      degradedReason?: string;
    }>;
    degradedSlices: Array<{
      name: string;
      live: boolean;
      coverage: number;
      degradedReason?: string;
    }>;
    degradedReason?: string;
  };
  ecosystem: ApiEcosystem;
};

export type EventBatch = Array<{ id?: number; event_type?: string }>;

export { isAdapters, isApprovalArray, isApprovalChallengeArray, isAttentionPacketArray, isEcosystem, isEventBatch, isEvidenceArray, isHealthResponse, isMemoryProposalArray, isReplay, isSystemArray, isUniverse, isVoiceThreadArray, isWorkOrderArray } from "./runtime-api-guards";
