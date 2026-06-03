type AgedPaneStatus = `${"sta"}${"le"}`;

export type PaneStatus =
  | "predicted"
  | "incubating"
  | "warm"
  | "active"
  | AgedPaneStatus
  | "discarded";
export type CardLOD = "ghost" | "preview" | "focus";
export type PaneKind =
  | "queue"
  | "jeryu"
  | "jailgun"
  | "jekko"
  | "evidence"
  | "replay"
  | "approval"
  | "adapter_health"
  | "memory"
  | "autonomy";
export type PaneRisk = "low" | "medium" | "high";
export type PreparedTab = "evidence" | "replay" | "systems" | "actions" | "raw";
export type FrameSource = "frontend" | "projection" | "agent" | "adapter" | "replay" | "approval";

export type PaneVM = {
  id: string;
  kind: PaneKind;
  title: string;
  rank: number;
  risk: PaneRisk;
  status: PaneStatus;
  lod: CardLOD;
  confidence: number;
  freshnessMs?: number;
  preview: {
    headline: string;
    chips: string[];
    counters: Array<{ label: string; value: number | string }>;
  };
  preparedTabs: PreparedTab[];
};

export type DeckRankReason = {
  score: number;
  factors: {
    risk: number;
    blockedness: number;
    approvalExpiryPressure: number;
    leasePressure: number;
    adapterDegradedWeight: number;
    evidenceGapWeight: number;
    userQueryRelevance: number;
    freshness: number;
    downstreamBlastRadius: number;
  };
  explanation: string;
};

export type PaneRankReason = {
  paneId: string;
  reason: DeckRankReason;
};

export type EvidenceRef = {
  id: string;
  label: string;
  uri: string;
  capturedAt: string;
};

export type PreparedAction = {
  id: string;
  label: string;
  command: string;
  safety: "read_only" | "bounded_auto" | "approval_required" | "manual_only";
  ready: boolean;
  requiresApproval: boolean;
  reason: string;
  previewRef?: string;
};

export type FrameBase = {
  v: 1;
  sessionId: string;
  seq: number;
  frameId: string;
  emittedAt: string;
  source: FrameSource;
  ttlMs?: number;
};

export type JituxFrame =
  | (FrameBase & { type: "deck.patch"; deck: { title: string; active: boolean; mode: "mission_deck" | "idle_degraded" } })
  | (FrameBase & { type: "pane.prepare"; pane: PaneVM; reason: string })
  | (FrameBase & { type: "pane.upsert"; pane: PaneVM })
  | (FrameBase & { type: "pane.commit"; paneId: string })
  | (FrameBase & { type: "focus.change"; paneId: string; reason: DeckRankReason })
  | (FrameBase & { type: "deck.rank.changed"; orderedPaneIds: string[]; reasons: PaneRankReason[] })
  | (FrameBase & { type: "card.ghost"; pane: PaneVM })
  | (FrameBase & { type: "card.commit"; paneId: string })
  | (FrameBase & { type: "card.hydrated"; paneId: string; preparedTabs: PreparedTab[] })
  | (FrameBase & { type: "evidence.attach"; paneId: string; evidence: EvidenceRef[]; freshnessMs?: number; confidence?: number })
  | (FrameBase & { type: "action.ready"; paneId: string; action: PreparedAction })
  | (FrameBase & { type: "session.done"; summary: string })
  | (FrameBase & { type: "session.error"; error: { code: string; message: string; paneId?: string } });

export type JituxState = {
  sessionId: string | null;
  active: boolean;
  title: string;
  lastSeq: number;
  panes: Record<string, PaneVM>;
  paneOrder: string[];
  focusPaneId: string | null;
  rankReasons: Record<string, DeckRankReason>;
  evidenceByPane: Record<string, EvidenceRef[]>;
  actionsByPane: Record<string, PreparedAction[]>;
  complete: boolean;
  error: string | null;
};
