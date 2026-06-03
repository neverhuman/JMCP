import type { JituxFrame, JituxState, PaneVM } from "./types";

export const initialJituxState: JituxState = {
  sessionId: null,
  active: false,
  title: "Mission Deck",
  lastSeq: 0,
  panes: {},
  paneOrder: [],
  focusPaneId: null,
  rankReasons: {},
  evidenceByPane: {},
  actionsByPane: {},
  complete: false,
  error: null,
};

function shouldIgnore(state: JituxState, frame: JituxFrame): boolean {
  return state.sessionId === frame.sessionId && frame.seq <= state.lastSeq;
}

function withBase(state: JituxState, frame: JituxFrame): JituxState {
  return { ...state, sessionId: frame.sessionId, lastSeq: frame.seq, error: null };
}

function upsertPane(state: JituxState, pane: PaneVM): JituxState {
  const panes = { ...state.panes, [pane.id]: pane };
  const paneOrder = state.paneOrder.includes(pane.id) ? state.paneOrder : [...state.paneOrder, pane.id];
  return { ...state, panes, paneOrder };
}

export function reduceJituxFrame(state: JituxState, frame: JituxFrame): JituxState {
  if (shouldIgnore(state, frame)) return state;
  const base = withBase(state, frame);
  switch (frame.type) {
    case "deck.patch":
      return { ...base, active: frame.deck.active, title: frame.deck.title, complete: false };
    case "pane.prepare":
    case "pane.upsert":
    case "card.ghost":
      return upsertPane(base, frame.pane);
    case "pane.commit":
    case "card.commit": {
      const pane = base.panes[frame.paneId];
      return pane ? upsertPane(base, { ...pane, status: "active", lod: "preview" }) : base;
    }
    case "card.hydrated": {
      const pane = base.panes[frame.paneId];
      return pane ? upsertPane(base, { ...pane, lod: "focus", preparedTabs: frame.preparedTabs }) : base;
    }
    case "focus.change": {
      const panes = Object.fromEntries(
        Object.entries(base.panes).map(([id, pane]) => [
          id,
          id === frame.paneId
            ? { ...pane, status: "active" as const, lod: "focus" as const }
            : pane.status === "active"
              ? { ...pane, status: "warm" as const, lod: pane.lod === "focus" ? ("preview" as const) : pane.lod }
              : pane,
        ]),
      );
      return {
        ...base,
        panes,
        focusPaneId: frame.paneId,
        rankReasons: { ...base.rankReasons, [frame.paneId]: frame.reason },
      };
    }
    case "deck.rank.changed":
      return {
        ...base,
        paneOrder: [...frame.orderedPaneIds, ...base.paneOrder.filter((id) => !frame.orderedPaneIds.includes(id))],
        rankReasons: Object.fromEntries(frame.reasons.map((reason) => [reason.paneId, reason.reason])),
      };
    case "evidence.attach":
      return {
        ...base,
        evidenceByPane: {
          ...base.evidenceByPane,
          [frame.paneId]: [...(base.evidenceByPane[frame.paneId] ?? []), ...frame.evidence],
        },
      };
    case "action.ready":
      return {
        ...base,
        actionsByPane: {
          ...base.actionsByPane,
          [frame.paneId]: [...(base.actionsByPane[frame.paneId] ?? []), frame.action],
        },
      };
    case "session.done":
      return { ...base, complete: true };
    case "session.error":
      return { ...base, active: false, error: frame.error.message };
  }
}
