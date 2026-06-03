import { useSyncExternalStore } from "react";
import type { RuntimeState } from "../runtime";
import { initialJituxState, reduceJituxFrame } from "./reducer";
import {
  createDeckLiveSession,
  createDeckTrace,
  resetDeckSessionChannelForTests,
  type DeckLiveStopReason,
  type DeckSessionTraceProbe,
} from "./session-channel";
import { createQueueBlockerFrames, reason, seqFrame } from "./queue-blocker-frames";
export { createQueueBlockerFrames } from "./queue-blocker-frames";
import { getCardsForPane, getRankedPanes } from "./deck-queries";
import type { JituxFrame, JituxState } from "./types";

type Listener = () => void;
type Selector<T> = (state: DeckState) => T;

export type DeckNavState = "idle" | "observing" | "agent_takeover" | "needs_user" | "acting" | "complete";
export type DeckStreamStatus = "idle" | "opening" | "live" | "degraded";
export type TraceProbe = DeckSessionTraceProbe;

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

export const initialDeckState: DeckState = {
  ...initialJituxState, mode: "idle", navState: "idle", generation: 0, trace: [], caption: "", streamStatus: "idle", streamUrl: null, wsUrl: null,
};

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
        caption: "Broker session opening; waiting for live frames to drive the Mission Deck.",
      });
    },
    onOpen: (descriptor) => {
      setState({
        ...state,
        streamStatus: "opening",
        streamUrl: descriptor.streamUrl,
        wsUrl: descriptor.wsUrl,
        caption: `Broker session ${descriptor.sessionId} opened; live broker frames are about to drive the Mission Deck.`,
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
        caption: "BROKER is driving the Mission Deck with live frames and ranked insights.",
      });
    },
    onSessionUnavailable: () =>
      markStreamDegraded("Broker session unavailable; retrying to keep the Mission Deck broker-driven."),
    onStreamUnavailable: () =>
      markStreamDegraded("Broker stream unavailable; retrying to keep the Mission Deck broker-driven."),
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

export function resetDeckStoreForTests(): void {
  deckStore.clear();
}
