import { describe, expect, it } from "vitest";
import { isJituxFrame } from "./guards";
import { initialJituxState, reduceJituxFrame } from "./reducer";
import type { DeckRankReason, JituxFrame, PaneVM } from "./types";

const reason: DeckRankReason = {
  score: 7.2,
  explanation: "Queue blocker ranks first because blockedness is highest.",
  factors: {
    risk: 0.8,
    blockedness: 1,
    approvalExpiryPressure: 0.8,
    leasePressure: 0.7,
    adapterDegradedWeight: 0.6,
    evidenceGapWeight: 0.8,
    userQueryRelevance: 1,
    freshness: 0.8,
    downstreamBlastRadius: 0.7,
  },
};

const pane: PaneVM = {
  id: "pane:queue",
  kind: "queue",
  title: "Queue blocker",
  rank: 1,
  risk: "medium",
  status: "predicted",
  lod: "ghost",
  confidence: 0.9,
  freshnessMs: 0,
  preview: {
    headline: "Finding blocked work.",
    chips: ["warming"],
    counters: [{ label: "prepared", value: "yes" }],
  },
  preparedTabs: ["evidence", "replay", "systems", "actions"],
};

type FramePatch = JituxFrame extends infer Frame
  ? Frame extends JituxFrame
    ? Omit<Frame, "v" | "sessionId" | "seq" | "frameId" | "emittedAt" | "source"> & { seq: number }
    : never
  : never;

function frame(patch: FramePatch): JituxFrame {
  return {
    v: 1,
    sessionId: "jitux_test",
    frameId: `frame_${patch.seq}`,
    emittedAt: "2026-06-03T15:00:00.000Z",
    source: "projection",
    ...patch,
  } as JituxFrame;
}

describe("JITUX guards and reducer", () => {
  it("validates committed backend frame shape", () => {
    expect(
      isJituxFrame(
        frame({
          type: "deck.patch",
          seq: 1,
          deck: { title: "Scanning queue blockers", active: true, mode: "mission_deck" },
        }),
      ),
    ).toBe(true);
    expect(isJituxFrame({ type: "focus.change", reason: { score: "bad" } })).toBe(false);
  });

  it("reduces ghost, rank, focus, evidence, action, and done frames", () => {
    const frames: JituxFrame[] = [
      frame({
        type: "deck.patch",
        seq: 1,
        deck: { title: "Scanning queue blockers", active: true, mode: "mission_deck" },
      }),
      frame({ type: "card.ghost", seq: 2, pane }),
      frame({ type: "deck.rank.changed", seq: 3, orderedPaneIds: [pane.id], reasons: [{ paneId: pane.id, reason }] }),
      frame({ type: "focus.change", seq: 4, paneId: pane.id, reason }),
      frame({
        type: "evidence.attach",
        seq: 5,
        paneId: pane.id,
        evidence: [{ id: "ev:queue", label: "Queue projection", uri: "jmcp://evidence/queue", capturedAt: "2026-06-03T15:00:00.000Z" }],
      }),
      frame({
        type: "action.ready",
        seq: 6,
        paneId: pane.id,
        action: {
          id: "show_evidence",
          label: "Show evidence",
          command: "jitux.evidence.preview",
          safety: "read_only",
          ready: true,
          requiresApproval: false,
          reason: "Read-only preview.",
        },
      }),
      frame({ type: "session.done", seq: 7, summary: "done" }),
    ];

    const state = frames.reduce(reduceJituxFrame, initialJituxState);

    expect(state.active).toBe(true);
    expect(state.title).toBe("Scanning queue blockers");
    expect(state.focusPaneId).toBe(pane.id);
    expect(state.panes[pane.id].lod).toBe("focus");
    expect(state.rankReasons[pane.id].explanation).toContain("blockedness");
    expect(state.evidenceByPane[pane.id]).toHaveLength(1);
    expect(state.actionsByPane[pane.id][0].safety).toBe("read_only");
    expect(state.complete).toBe(true);
  });

  it("ignores older frames for the active session", () => {
    const first = reduceJituxFrame(
      initialJituxState,
      frame({
        type: "deck.patch",
        seq: 3,
        deck: { title: "Fresh", active: true, mode: "mission_deck" },
      }),
    );
    const older = reduceJituxFrame(
      first,
      frame({
        type: "deck.patch",
        seq: 2,
        deck: { title: "Old", active: true, mode: "mission_deck" },
      }),
    );

    expect(older.title).toBe("Fresh");
    expect(older.lastSeq).toBe(3);
  });
});
