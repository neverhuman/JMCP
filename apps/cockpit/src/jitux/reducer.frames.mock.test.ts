import { describe, expect, it } from "vitest";
import { isJituxFrame } from "./guards";
import { initialJituxState, reduceJituxFrame } from "./reducer";
import type { DeckRankReason, JituxFrame, PaneVM } from "./types";

const sessionId = "jitux_test";
const emittedAt = "2026-06-03T15:00:00.000Z";

function reason(score: number, explanation: string): DeckRankReason {
  return {
    score,
    explanation,
    factors: {
      risk: 0.8,
      blockedness: 0.7,
      approvalExpiryPressure: 0.6,
      leasePressure: 0.5,
      adapterDegradedWeight: 0.4,
      evidenceGapWeight: 0.3,
      userQueryRelevance: 0.9,
      freshness: 0.2,
      downstreamBlastRadius: 0.1,
    },
  };
}

function pane(id: string, title: string, rank: number): PaneVM {
  return {
    id,
    kind: "queue",
    title,
    rank,
    risk: "high",
    status: "predicted",
    lod: "ghost",
    confidence: 0.91,
    freshnessMs: 250,
    preview: {
      headline: `${title} is warming`,
      chips: ["queue", "blocker"],
      counters: [{ label: "blocked", value: 3 }],
    },
    preparedTabs: ["evidence", "actions"],
  };
}

function frame(seq: number, data: any): JituxFrame {
  return {
    v: 1,
    sessionId,
    seq,
    frameId: `frame_${seq.toString().padStart(4, "0")}`,
    emittedAt,
    source: "projection",
    ...data,
  } as JituxFrame;
}

describe("JITUX reducer frames", () => {
  it("reduces every canonical frame family and ignores stale frames", () => {
    const queuePane = pane("pane:queue", "Queue blocker", 0.98);
    const queuePaneUpdated = { ...queuePane, title: "Queue blocker v2", rank: 0.99 };
    const approvalPane = pane("pane:approval", "Approval gate", 0.72);
    const rankQueue = reason(8.4, "Queue blocker stays first.");
    const rankApproval = reason(5.1, "Approval gate is secondary.");

    const frames: JituxFrame[] = [
      frame(1, {
        type: "deck.patch",
        deck: { title: "Scanning queue blockers", active: true, mode: "mission_deck" },
      }),
      frame(2, { type: "pane.prepare", pane: queuePane, reason: "Queue blocker is relevant now." }),
      frame(3, { type: "pane.upsert", pane: queuePaneUpdated }),
      frame(4, { type: "card.ghost", pane: queuePaneUpdated }),
      frame(5, { type: "pane.prepare", pane: approvalPane, reason: "Approval gate is warming." }),
      frame(6, { type: "card.commit", paneId: approvalPane.id }),
      frame(7, { type: "pane.commit", paneId: queuePane.id }),
      frame(8, { type: "card.hydrated", paneId: queuePane.id, preparedTabs: ["evidence", "actions", "raw"] }),
      frame(9, {
        type: "deck.rank.changed",
        orderedPaneIds: [queuePane.id, approvalPane.id],
        reasons: [
          { paneId: queuePane.id, reason: rankQueue },
          { paneId: approvalPane.id, reason: rankApproval },
        ],
      }),
      frame(10, { type: "focus.change", paneId: queuePane.id, reason: rankQueue }),
      frame(11, {
        type: "evidence.attach",
        paneId: queuePane.id,
        evidence: [
          {
            id: "evidence:queue:0",
            label: "Queue projection",
            uri: "jmcp://evidence/queue",
            capturedAt: emittedAt,
          },
        ],
        freshnessMs: 100,
        confidence: 0.93,
      }),
      frame(12, {
        type: "action.ready",
        paneId: queuePane.id,
        action: {
          id: "show-evidence",
          label: "Show evidence",
          command: "jmcp.evidence.read queue",
          safety: "read_only",
          ready: true,
          requiresApproval: false,
          reason: "Read-only evidence preview.",
          previewRef: "jmcp://work-orders/queue/evidence",
        },
      }),
      frame(13, { type: "session.done", summary: "Mission Deck ignition complete." }),
      frame(14, {
        type: "session.error",
        error: { code: "stream_closed", message: "The session stopped.", paneId: queuePane.id },
      }),
    ];

    const state = frames.reduce(reduceJituxFrame, initialJituxState);
    const stale = reduceJituxFrame(
      state,
      frame(13, {
        type: "session.done",
        summary: "Older frame should be ignored.",
      }),
    );

    expect(state.sessionId).toBe(sessionId);
    expect(state.title).toBe("Scanning queue blockers");
    expect(state.lastSeq).toBe(14);
    expect(state.active).toBe(false);
    expect(state.complete).toBe(true);
    expect(state.focusPaneId).toBe(queuePane.id);
    expect(state.paneOrder).toEqual([queuePane.id, approvalPane.id]);
    expect(state.panes[queuePane.id].title).toBe("Queue blocker v2");
    expect(state.panes[queuePane.id].lod).toBe("focus");
    expect(state.panes[queuePane.id].status).toBe("active");
    expect(state.panes[approvalPane.id].status).toBe("warm");
    expect(state.panes[approvalPane.id].lod).toBe("preview");
    expect(state.rankReasons[queuePane.id]).toEqual(rankQueue);
    expect(state.evidenceByPane[queuePane.id]).toHaveLength(1);
    expect(state.actionsByPane[queuePane.id]).toHaveLength(1);
    expect(state.actionsByPane[queuePane.id][0].safety).toBe("read_only");
    expect(state.panes[queuePane.id].preparedTabs).toEqual(["evidence", "actions", "raw"]);
    expect(stale.lastSeq).toBe(14);
    expect(stale.error).toBe("The session stopped.");
  });

  it("rejects malformed frames through the canonical guards", () => {
    const valid = frame(1, {
      type: "deck.patch",
      deck: { title: "Scanning queue blockers", active: true, mode: "mission_deck" },
    });

    expect(isJituxFrame(valid)).toBe(true);
    expect(
      isJituxFrame({
        ...valid,
        deck: { title: "Scanning queue blockers", active: true, mode: "ghost_mode" },
      }),
    ).toBe(false);
    expect(
      isJituxFrame({
        ...valid,
        type: "action.ready",
        action: {
          id: "bad",
          label: "Bad",
          command: "jmcp.bad",
          safety: "secret",
          ready: true,
          requiresApproval: false,
          reason: "bad",
        },
      } as unknown as JituxFrame),
    ).toBe(false);
    expect(
      isJituxFrame({
        ...valid,
        type: "session.error",
        error: { code: "stream_closed" },
      } as unknown as JituxFrame),
    ).toBe(false);
  });
});
