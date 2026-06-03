import { useSyncExternalStore } from "react";
import type { RuntimeState } from "../runtime";
import type { ApprovalRequest, WorkItem } from "../types";
import { initialJituxState, reduceJituxFrame } from "./reducer";
import {
  createDeckLiveSession,
  createDeckTrace,
  resetDeckSessionChannelForTests,
  type DeckLiveStopReason,
  type DeckSessionTraceProbe,
} from "./session-channel";
import type { CardLOD, DeckRankReason, EvidenceRef, JituxFrame, JituxState, PaneKind, PaneRisk, PaneVM, PreparedAction } from "./types";

type Listener = () => void;
type Selector<T> = (state: DeckState) => T;

export type DeckNavState = "idle" | "observing" | "agent_takeover" | "needs_user" | "acting" | "complete";
export type DeckStreamStatus = "idle" | "opening" | "live" | "degraded";
export type TraceProbe = DeckSessionTraceProbe;

export type DeckCardVM = { id: string; paneId: string; title: string; lod: CardLOD; status: "ghost" | "committed" | "hydrated"; risk: PaneRisk; headline: string };

export type DeckState = JituxState & {
  mode: "idle" | "mission_deck";
  navState: DeckNavState;
  generation: number;
  trace: TraceProbe[];
  caption: string;
  streamStatus: DeckStreamStatus;
  streamUrl: string | null;
  wsUrl: string | null;
};

const emittedAt = "2026-06-03T15:00:00.000Z";

export const initialDeckState: DeckState = {
  ...initialJituxState, mode: "idle", navState: "idle", generation: 0, trace: [], caption: "", streamStatus: "idle", streamUrl: null, wsUrl: null,
};

function emptyFactors() {
  return { risk: 0, blockedness: 0, approvalExpiryPressure: 0, leasePressure: 0, adapterDegradedWeight: 0, evidenceGapWeight: 0, userQueryRelevance: 0, freshness: 0, downstreamBlastRadius: 0 };
}

function reason(score: number, explanation: string, factors: Partial<ReturnType<typeof emptyFactors>>): DeckRankReason {
  return {
    score,
    explanation,
    factors: { ...emptyFactors(), ...factors },
  };
}

type FrameBaseKey = "type" | "v" | "sessionId" | "seq" | "frameId" | "emittedAt" | "source";

function seqFrame<T extends JituxFrame["type"]>(
  type: T,
  sessionId: string,
  seq: number,
  data: Omit<Extract<JituxFrame, { type: T }>, FrameBaseKey>,
): Extract<JituxFrame, { type: T }> {
  return { v: 1, type, sessionId, seq, frameId: `${sessionId}.${seq}.${type}`, emittedAt, source: "frontend", ...data } as Extract<JituxFrame, { type: T }>;
}

function rankRisk(work: WorkItem | undefined): PaneRisk {
  return work?.risk ?? "medium";
}

function pane(
  id: string,
  kind: PaneKind,
  title: string,
  rank: number,
  risk: PaneRisk,
  status: PaneVM["status"],
  lod: PaneVM["lod"],
  headline: string,
  chips: string[],
  counters: PaneVM["preview"]["counters"],
): PaneVM {
  return {
    id, kind, title, rank, risk, status, lod,
    confidence: Math.max(0.55, 0.96 - rank * 0.08),
    freshnessMs: rank * 15000,
    preview: { headline, chips, counters },
    preparedTabs: ["evidence", "replay", "systems", "actions", "raw"],
  };
}

function actionsFor(blockedWork: WorkItem | undefined, approval: ApprovalRequest | undefined): PreparedAction[] {
  return [
    {
      id: "show-evidence",
      label: "Show evidence",
      command: blockedWork ? `jmcp.evidence.read ${blockedWork.id}` : "jmcp.evidence.read queue",
      safety: "read_only",
      ready: true,
      requiresApproval: false,
      reason: "Evidence read is safe and already scoped to the active work order.",
      previewRef: blockedWork ? `evidence.${blockedWork.id}` : "evidence.queue",
    },
    {
      id: "open-replay-window",
      label: "Replay window",
      command: blockedWork ? `jmcp.replay.inspect ${blockedWork.id}` : "jmcp.replay.inspect queue",
      safety: "read_only",
      ready: true,
      requiresApproval: false,
      reason: "Replay inspection does not mutate durable state.",
      previewRef: blockedWork ? `replay.${blockedWork.id}` : "replay.queue",
    },
    {
      id: "prepare-approval",
      label: "Prepare approval packet",
      command: approval ? `jmcp.approval.prepare ${approval.id}` : "jmcp.approval.prepare queue",
      safety: "approval_required",
      ready: false,
      requiresApproval: true,
      reason: approval ? "Approval exists but mutation still requires explicit authority." : "No active approval gate is green.",
      previewRef: approval ? `approval.${approval.id}` : "approval.queue",
    },
  ];
}

function evidenceRef(id: string, label: string, uri: string): EvidenceRef {
  return {
    id,
    label,
    uri,
    capturedAt: emittedAt,
  };
}

export function createQueueBlockerFrames(runtime: RuntimeState, sessionId = "frontend.queue-blockers"): JituxFrame[] {
  const blockedWork = runtime.workItems.find((item) => item.state === "blocked") ?? runtime.workItems[0];
  const approval = runtime.approvalRequests.find((request) => request.workOrderId === blockedWork?.id) ?? runtime.approvalRequests[0];
  const urgent = runtime.attentionPackets.filter((packet) => packet.attentionLevel === "urgent" || packet.attentionLevel === "incident").length;
  const blockedCount = runtime.workItems.filter((item) => item.state === "blocked").length;
  const degradedSystems = runtime.systems.filter((system) => system.health === "degraded" || system.health === "blocked").length;
  const queueHeadline = blockedWork
    ? `${blockedWork.id} is blocking queue flow: ${blockedWork.title}`
    : "Queue blocker scan is incubating with no work orders visible yet.";

  const panes = [
    pane(
      "queue_blockers",
      "queue",
      "Queue blocker",
      1,
      rankRisk(blockedWork),
      "active",
      "focus",
      queueHeadline,
      ["blocked work", blockedWork?.lease ?? "lease scan", `${blockedWork?.evidence ?? 0} evidence refs`],
      [
        { label: "blocked", value: blockedCount },
        { label: "urgent", value: urgent },
        { label: "evidence", value: blockedWork?.evidence ?? 0 },
      ],
    ),
    pane(
      "approval_gate",
      "approval",
      "Approval gate",
      2,
      approval?.risk ?? "medium",
      "warm",
      "preview",
      approval ? `${approval.id} expires in ${approval.expires} for ${approval.workOrderId}` : "No active approval challenge was attached to the queue scan.",
      ["authority", approval?.channel ?? "local", approval?.state ?? "clear"],
      [
        { label: "pending", value: runtime.approvalRequests.filter((request) => request.state === "pending").length },
        { label: "expires", value: approval?.expires ?? "n/a" },
      ],
    ),
    pane(
      "adapter_health",
      "jeryu",
      "Jeryu adapter context",
      3,
      degradedSystems > 0 ? "high" : "medium",
      "warm",
      "preview",
      degradedSystems > 0 ? `${degradedSystems} adapter surfaces need attention.` : "Adapter surfaces are visible for queue context.",
      ["adapter health", "service card", runtime.ecosystemLive ? "live graph" : "cached snapshot"],
      [
        { label: "degraded", value: degradedSystems },
        { label: "tools", value: runtime.toolAssets.length },
      ],
    ),
    pane(
      "replay_lens",
      "replay",
      "Replay lens",
      4,
      "medium",
      "incubating",
      "preview",
      `${runtime.replayEvents.length} recent replay events are ready for tunnel inspection.`,
      ["event stream", "checkpoint", "rank proof"],
      [
        { label: "events", value: runtime.replayEvents.length },
        { label: "freshness", value: "current" },
      ],
    ),
    pane(
      "jailgun_runs",
      "jailgun",
      "Jailgun run lane",
      5,
      "low",
      "predicted",
      "ghost",
      "Run capture is warming behind the blocker scan.",
      ["run capture", "bounded", "ghost"],
      [
        { label: "captures", value: 0 },
        { label: "risk", value: "low" },
      ],
    ),
  ];

  const reasons: Record<string, DeckRankReason> = {
    queue_blockers: reason(0.94, "Blocked work, visible lease pressure, and user query relevance make the queue blocker the active pane.", {
      risk: 0.9,
      blockedness: 1,
      leasePressure: 0.75,
      evidenceGapWeight: blockedWork && blockedWork.evidence < 3 ? 0.8 : 0.35,
      userQueryRelevance: 1,
      downstreamBlastRadius: 0.72,
    }),
    approval_gate: reason(0.78, "Approval expiry pressure is attached to the blocked work order.", {
      approvalExpiryPressure: 0.85,
      userQueryRelevance: 0.72,
      blockedness: 0.5,
    }),
    adapter_health: reason(0.7, "Adapter health is relevant because the blocked work depends on governed adapter authority.", {
      adapterDegradedWeight: degradedSystems > 0 ? 0.9 : 0.35,
      userQueryRelevance: 0.62,
    }),
    replay_lens: reason(0.58, "Replay context is warm because the answer needs evidence and recent checkpoints.", {
      freshness: 0.72,
      evidenceGapWeight: 0.45,
      userQueryRelevance: 0.52,
    }),
    jailgun_runs: reason(0.41, "Run capture is predicted as a likely drilldown after evidence review.", {
      userQueryRelevance: 0.38,
      freshness: 0.4,
    }),
  };

  let seq = 1;
  const frames: JituxFrame[] = [
    seqFrame("deck.patch", sessionId, seq++, {
      deck: {
        mode: "mission_deck",
        title: "Queue Blockers Mission Deck",
        active: true,
      },
    }),
  ];

  for (const currentPane of panes) {
    frames.push(seqFrame("pane.prepare", sessionId, seq++, { pane: currentPane, reason: reasons[currentPane.id].explanation }));
    frames.push(seqFrame("card.ghost", sessionId, seq++, { pane: currentPane }));
  }

  frames.push(
    seqFrame("deck.rank.changed", sessionId, seq++, {
      orderedPaneIds: panes.map((item) => item.id),
      reasons: panes.map((item) => ({ paneId: item.id, reason: reasons[item.id] })),
    }),
  );
  frames.push(seqFrame("pane.commit", sessionId, seq++, { paneId: "queue_blockers" }));
  frames.push(seqFrame("card.hydrated", sessionId, seq++, { paneId: "queue_blockers", preparedTabs: panes[0].preparedTabs }));
  frames.push(seqFrame("focus.change", sessionId, seq++, { paneId: "queue_blockers", reason: reasons.queue_blockers }));
  frames.push(
    seqFrame("evidence.attach", sessionId, seq++, {
      paneId: "queue_blockers",
      evidence: [
        evidenceRef("evidence.work-order", blockedWork ? blockedWork.id : "queue scan", blockedWork ? `jmcp://evidence/work/${blockedWork.id}` : "jmcp://evidence/queue"),
        evidenceRef(
          "evidence.approval",
          approval ? approval.challengeId : "approval scan",
          approval ? `jmcp://approval/challenge/${approval.challengeId}` : "jmcp://approval/queue",
        ),
      ],
      freshnessMs: 9000,
      confidence: approval ? 0.86 : 0.62,
    }),
  );
  for (const action of actionsFor(blockedWork, approval)) {
    frames.push(seqFrame("action.ready", sessionId, seq++, { paneId: "queue_blockers", action }));
  }
  frames.push(seqFrame("session.done", sessionId, seq++, { summary: "Queue blocker deck prepared." }));
  return frames;
}

function navStateFor(state: JituxState): DeckNavState {
  if (state.error) {
    return "needs_user";
  }
  if (state.complete) {
    return "complete";
  }
  if (state.active) {
    return "agent_takeover";
  }
  return "idle";
}

function reduceDeckFrame(state: DeckState, frame: JituxFrame): DeckState {
  const next = reduceJituxFrame(state, frame);
  if (next === state) {
    return state;
  }
  return { ...state, ...next, mode: next.active ? "mission_deck" : "idle", navState: navStateFor(next), generation: state.generation + 1 };
}

function createStore() {
  let state = initialDeckState;
  const listeners = new Set<Listener>();
  let latestRuntime: RuntimeState | null = null;

  const emit = () => {
    for (const listener of listeners) {
      listener();
    }
  };

  const setState = (nextState: DeckState) => {
    if (nextState !== state) {
      state = nextState;
      emit();
    }
  };

  const applyFramesTo = (base: DeckState, frames: JituxFrame[]): DeckState => {
    let nextState = base;
    for (const frame of frames) {
      nextState = reduceDeckFrame(nextState, frame);
    }
    return nextState;
  };

  const applyFrames = (frames: JituxFrame[]) => setState(applyFramesTo(state, frames));

  const primeQueueBlockers = (runtime: RuntimeState) => {
    const frames = createQueueBlockerFrames(runtime);
    setState({
      ...applyFramesTo(initialDeckState, frames),
      trace: createDeckTrace(runtime, "degraded", "frontend"),
      caption: "Cached snapshot is visible while the broker session opens.",
      streamStatus: "degraded",
      streamUrl: null,
      wsUrl: null,
    });
  };

  const markStreamDegraded = (caption: string) => {
    setState({
      ...state,
      streamStatus: "degraded",
      trace: latestRuntime ? createDeckTrace(latestRuntime, "degraded", "frontend") : state.trace,
      caption,
    });
  };

  const liveSession = createDeckLiveSession({
    onOpening: () => {
      const runtime = latestRuntime;
      if (!runtime) return;
      setState({
        ...state,
        streamStatus: "opening",
        streamUrl: null,
        wsUrl: null,
        trace: createDeckTrace(runtime, "degraded", "frontend"),
        caption: "Cached snapshot is visible while the broker session opens.",
      });
    },
    onOpen: (descriptor) => {
      setState({
        ...state,
        streamStatus: "opening",
        streamUrl: descriptor.streamUrl,
        wsUrl: descriptor.wsUrl,
        caption: `Broker session ${descriptor.sessionId} opened; cached snapshot remains visible until live frames arrive.`,
      });
    },
    onFrame: (frame, descriptor) => {
      const runtime = latestRuntime;
      applyFrames([frame]);
      setState({
        ...state,
        streamStatus: "live",
        streamUrl: descriptor.streamUrl,
        wsUrl: descriptor.wsUrl,
        trace: runtime ? createDeckTrace(runtime, "ready", "projection") : state.trace,
        caption: "Live broker frames are driving the Mission Deck.",
      });
    },
    onSessionUnavailable: () => markStreamDegraded("Broker session unavailable; cached snapshot remains visible."),
    onStreamUnavailable: () => markStreamDegraded("Broker stream unavailable; cached snapshot remains visible."),
  });

  return {
    getSnapshot: () => state,
    subscribe: (listener: Listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch: (frame: JituxFrame) => setState(reduceDeckFrame(state, frame)),
    applyFrames,
    igniteQueueBlockers: (runtime: RuntimeState) => {
      latestRuntime = runtime;
      liveSession.stop();
      primeQueueBlockers(runtime);
    },
    startLiveQueueBlockers: (runtime?: RuntimeState) => {
      if (runtime) {
        latestRuntime = runtime;
      }
      const currentRuntime = latestRuntime;
      if (!currentRuntime) {
        return () => undefined;
      }
      if (!state.active) {
        primeQueueBlockers(currentRuntime);
      }
      return liveSession.start();
    },
    stopLiveQueueBlockers: (reason: DeckLiveStopReason = "deactivate") => {
      liveSession.stop();
      if (reason === "barge_in" && state.active) {
        markStreamDegraded("Live broker stream paused for barge-in; cached snapshot remains visible.");
      }
    },
    // Driven by the realtime reasoning agent: re-ignite the deck for this turn so
    // panes fly in and the Now rail goes purple the instant a question is asked,
    // before any speech. No-op until a runtime snapshot is known (Now view mounted).
    beginAgentTurn: (label?: string) => {
      const runtime = latestRuntime;
      if (!runtime) {
        return;
      }
      liveSession.stop();
      primeQueueBlockers(runtime);
      const trimmed = (label ?? "").trim();
      if (trimmed.length > 0) {
        setState({ ...state, caption: `Agent investigating: ${trimmed}` });
      }
      liveSession.start();
    },
    // One reshuffle per reasoning step / tool call: advance focus to the next ranked
    // pane so the deck visibly moves at the speed of the agent's reasoning.
    pulseAgentStep: (label: string) => {
      const ranked = getRankedPanes(state);
      if (ranked.length === 0 || !state.sessionId) {
        return;
      }
      const currentIndex = ranked.findIndex((pane) => pane.id === state.focusPaneId);
      const nextPane = ranked[(currentIndex + 1) % ranked.length];
      const frame = seqFrame("focus.change", state.sessionId, state.lastSeq + 1, {
        paneId: nextPane.id,
        reason: reason(0.8, label, { userQueryRelevance: 0.85, freshness: 0.6 }),
      });
      setState(reduceDeckFrame(state, frame));
    },
    promotePane: (paneId: string, explanation: string) => {
      const current = state.panes[paneId];
      if (!current || !state.sessionId) {
        return;
      }
      const frame = seqFrame("focus.change", state.sessionId, state.lastSeq + 1, {
        paneId,
        reason: reason(0.75, explanation, { userQueryRelevance: 0.8, freshness: 0.5 }),
      });
      setState(reduceDeckFrame(state, frame));
    },
    clear: () => {
      liveSession.stop();
      latestRuntime = null;
      resetDeckSessionChannelForTests();
      setState(initialDeckState);
    },
    rankedPanes: () => getRankedPanes(state),
    cardsForPane: (paneId: string) => getCardsForPane(state, paneId),
  };
}

export const deckStore = createStore();

export function useDeckSnapshot(): DeckState;
export function useDeckSnapshot<T>(selector: Selector<T>): T;
export function useDeckSnapshot<T>(selector?: Selector<T>): DeckState | T {
  const snapshot = useSyncExternalStore(deckStore.subscribe, deckStore.getSnapshot, deckStore.getSnapshot);
  return selector ? selector(snapshot) : snapshot;
}

export function getRankedPanes(state: DeckState): PaneVM[] {
  return state.paneOrder.map((id) => state.panes[id]).filter((pane): pane is PaneVM => pane !== undefined).slice(0, 20);
}

export function getCardsForPane(state: DeckState, paneId: string): DeckCardVM[] {
  const pane = state.panes[paneId];
  if (!pane) {
    return [];
  }
  return [
    {
      id: `${pane.id}.card`, paneId: pane.id, title: pane.title, lod: pane.lod,
      status: pane.lod === "ghost" ? "ghost" : pane.lod === "focus" ? "hydrated" : "committed",
      risk: pane.risk, headline: pane.preview.headline,
    },
  ];
}

export function resetDeckStoreForTests(): void {
  deckStore.clear();
}
