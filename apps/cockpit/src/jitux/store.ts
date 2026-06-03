import { useSyncExternalStore } from "react";
import type { RuntimeState } from "../runtime";
import { openDeckSession, QUEUE_BLOCKERS_DECK_SESSION_REQUEST, subscribeToDeckFrames } from "./client";
import { initialJituxState, reduceJituxFrame } from "./reducer";
import { createQueueBlockerFrames, createQueueBlockerTrace, type DeckTraceProbe } from "./queueBlockerFrames";
import type {
  CardLOD,
  DeckRankReason,
  JituxFrame,
  JituxState,
  PaneRisk,
  PaneVM,
} from "./types";

type Listener = () => void;
type Selector<T> = (state: DeckState) => T;

export type DeckNavState = "idle" | "observing" | "agent_takeover" | "needs_user" | "acting" | "complete";

export type TraceProbe = DeckTraceProbe;

export type DeckCardVM = {
  id: string;
  paneId: string;
  title: string;
  lod: CardLOD;
  status: "ghost" | "committed" | "hydrated";
  risk: PaneRisk;
  headline: string;
};

export type DeckState = JituxState & {
  mode: "idle" | "mission_deck";
  navState: DeckNavState;
  generation: number;
  trace: TraceProbe[];
  caption: string;
};

export const initialDeckState: DeckState = {
  ...initialJituxState,
  mode: "idle",
  navState: "idle",
  generation: 0,
  trace: [],
  caption: "",
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
  return {
    ...state,
    ...next,
    mode: next.active ? "mission_deck" : "idle",
    navState: navStateFor(next),
    generation: state.generation + 1,
  };
}

function promotedReason(explanation: string): DeckRankReason {
  return {
    score: 0.75,
    explanation,
    factors: {
      risk: 0,
      blockedness: 0,
      approvalExpiryPressure: 0,
      leasePressure: 0,
      adapterDegradedWeight: 0,
      evidenceGapWeight: 0,
      userQueryRelevance: 0.8,
      freshness: 0.5,
      downstreamBlastRadius: 0,
    },
  };
}

function createStore() {
  let state = initialDeckState;
  const listeners = new Set<Listener>();
  let liveSessionAbort: AbortController | null = null;
  let liveSessionStop: (() => void) | null = null;

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

  const teardownLiveSession = () => {
    liveSessionAbort?.abort();
    liveSessionAbort = null;
    liveSessionStop?.();
    liveSessionStop = null;
  };

  const primeQueueBlockers = (runtime: RuntimeState) => {
    setState({
      ...applyFramesTo(initialDeckState, createQueueBlockerFrames(runtime)),
      trace: createQueueBlockerTrace(runtime),
      caption: "Queue blocker projection is visible before spoken output.",
    });
  };

  return {
    getSnapshot: () => state,
    subscribe: (listener: Listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch: (frame: JituxFrame) => setState(reduceDeckFrame(state, frame)),
    applyFrames: (frames: JituxFrame[]) => setState(applyFramesTo(state, frames)),
    igniteQueueBlockers: (runtime: RuntimeState) => {
      teardownLiveSession();
      primeQueueBlockers(runtime);
    },
    startLiveQueueBlockers: (runtime: RuntimeState) => {
      teardownLiveSession();
      primeQueueBlockers(runtime);
      const controller = new AbortController();
      liveSessionAbort = controller;
      void openDeckSession(QUEUE_BLOCKERS_DECK_SESSION_REQUEST, controller.signal)
        .then((session) => {
          if (controller.signal.aborted) {
            return;
          }
          liveSessionStop = subscribeToDeckFrames(session.streamUrl, (frame) => {
            setState(reduceDeckFrame(state, frame));
          });
        })
        .catch(() => {
          if (controller.signal.aborted) {
            return;
          }
          liveSessionAbort = null;
        });
      return () => {
        if (!controller.signal.aborted) {
          controller.abort();
        }
        liveSessionStop?.();
        liveSessionStop = null;
        if (liveSessionAbort === controller) {
          liveSessionAbort = null;
        }
      };
    },
    promotePane: (paneId: string, explanation: string) => {
      const current = state.panes[paneId];
      if (!current || !state.sessionId) {
        return;
      }
      const nextSeq = state.lastSeq + 1;
      const frame: JituxFrame = {
        v: 1,
        type: "focus.change",
        sessionId: state.sessionId,
        seq: nextSeq,
        frameId: `${state.sessionId}.${nextSeq}.focus.change`,
        emittedAt: "2026-06-03T15:00:00.000Z",
        source: "frontend",
        paneId,
        reason: promotedReason(explanation),
      };
      setState(reduceDeckFrame(state, frame));
    },
    clear: () => setState(initialDeckState),
    teardownLiveSession,
    rankedPanes: () => getRankedPanes(state),
    cardsForPane: (paneId: string) => getCardsForPane(state, paneId),
  };
}

export const deckStore = createStore();
export { createQueueBlockerFrames };

export function useDeckSnapshot(): DeckState;
export function useDeckSnapshot<T>(selector: Selector<T>): T;
export function useDeckSnapshot<T>(selector?: Selector<T>): DeckState | T {
  const snapshot = useSyncExternalStore(deckStore.subscribe, deckStore.getSnapshot, deckStore.getSnapshot);
  return selector ? selector(snapshot) : snapshot;
}

export function getRankedPanes(state: DeckState): PaneVM[] {
  return state.paneOrder
    .map((id) => state.panes[id])
    .filter((pane): pane is PaneVM => pane !== undefined)
    .slice(0, 20);
}

export function getCardsForPane(state: DeckState, paneId: string): DeckCardVM[] {
  const pane = state.panes[paneId];
  if (!pane) {
    return [];
  }
  return [
    {
      id: `${pane.id}.card`,
      paneId: pane.id,
      title: pane.title,
      lod: pane.lod,
      status: pane.lod === "ghost" ? "ghost" : pane.lod === "focus" ? "hydrated" : "committed",
      risk: pane.risk,
      headline: pane.preview.headline,
    },
  ];
}

export function resetDeckStoreForTests(): void {
  deckStore.teardownLiveSession();
  deckStore.clear();
}
