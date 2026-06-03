import { act, cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../../App";
import { createFixtureRuntime } from "../../runtime";
import { isJituxFrame } from "../guards";
import { createQueueBlockerFrames, deckStore, resetDeckStoreForTests } from "../store";
import type { JituxFrame } from "../types";
import { NowCommandDeck } from "./NowCommandDeck";

function setReducedMotion(matches: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

function applyQueueBlockerFrames() {
  const frames = createQueueBlockerFrames(createFixtureRuntime());
  expect(frames.every(isJituxFrame)).toBe(true);
  act(() => deckStore.applyFrames(frames));
  return frames;
}

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("test api unavailable"))));
  setReducedMotion(false);
  resetDeckStoreForTests();
});

afterEach(() => {
  resetDeckStoreForTests();
  vi.unstubAllGlobals();
  cleanup();
});

describe("NowCommandDeck", () => {
  it("accepts canonical queue-blocker frames and rejects malformed frames", () => {
    const frames = createQueueBlockerFrames(createFixtureRuntime());
    const actionFrame = frames.find((frame): frame is Extract<JituxFrame, { type: "action.ready" }> => frame.type === "action.ready");
    const firstFrame = frames[0];

    expect(frames.every(isJituxFrame)).toBe(true);
    expect(actionFrame).toBeDefined();
    expect(firstFrame).toBeDefined();
    expect(isJituxFrame({ ...actionFrame!, action: { ...actionFrame!.action, safety: "secret" } })).toBe(false);
    expect(isJituxFrame({ ...firstFrame!, type: "focus.change", paneId: "queue_blockers", reason: { score: "bad" } })).toBe(false);
  });

  it("renders ranked order and LOD states from canonical reducer state", () => {
    applyQueueBlockerFrames();

    render(<NowCommandDeck />);

    expect(deckStore.getSnapshot().focusPaneId).toBe("queue_blockers");
    expect(deckStore.getSnapshot().actionsByPane.queue_blockers).toHaveLength(3);

    const list = screen.getByRole("list", { name: "Ranked Mission Deck" });
    const cards = within(list).getAllByRole("listitem");

    expect(cards).toHaveLength(5);
    expect(cards[0]).toHaveAttribute("data-lod", "focus");
    expect(cards[1]).toHaveAttribute("data-lod", "preview");
    expect(cards[4]).toHaveAttribute("data-lod", "ghost");
    expect(cards.map((card) => card.getAttribute("aria-label"))).toEqual([
      "1. Queue blocker",
      "2. Approval gate",
      "3. Jeryu adapter context",
      "4. Replay lens",
      "5. Jailgun run lane",
    ]);
  });

  it("uses reduced-motion list mode", () => {
    setReducedMotion(true);
    applyQueueBlockerFrames();

    render(<NowCommandDeck />);

    expect(screen.getByLabelText("Mission Deck viewport")).toHaveAttribute("data-motion", "reduced");
    expect(screen.getByRole("list", { name: "Ranked Mission Deck" })).toBeInTheDocument();
  });

  it("renders the sparse active-deck warming focus state without panes or trace probes", () => {
    act(() =>
      deckStore.dispatch({
        v: 1,
        sessionId: "jitux_sparse",
        seq: 1,
        frameId: "jitux_sparse.1",
        emittedAt: "2026-06-03T15:00:00.000Z",
        source: "projection",
        type: "deck.patch",
        deck: { title: "Sparse Mission Deck", active: true, mode: "mission_deck" },
      }),
    );

    render(<NowCommandDeck />);

    expect(screen.getByLabelText("AIUX Mission Deck")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Sparse Mission Deck" })).toBeInTheDocument();
    expect(within(screen.getByLabelText("Mission trace")).getByText("running")).toBeInTheDocument();
    expect(within(screen.getByRole("list", { name: "Ranked Mission Deck" })).queryAllByRole("listitem")).toEqual([]);
    expect(screen.getByLabelText("Focus pane")).toBeInTheDocument();
    expect(screen.getByText("Focus is warming")).toBeInTheDocument();
  });

  it("fans, collapses, and promotes a pane through deck controls", async () => {
    const user = userEvent.setup();
    applyQueueBlockerFrames();

    render(<NowCommandDeck />);

    const deck = screen.getByLabelText("AIUX Mission Deck");
    expect(deck).toHaveAttribute("data-view-mode", "stack");

    await user.click(screen.getByRole("button", { name: "Fan panes" }));
    expect(deck).toHaveAttribute("data-view-mode", "fan");

    await user.click(screen.getByRole("button", { name: "Collapse panes" }));
    expect(deck).toHaveAttribute("data-view-mode", "stack");

    await user.click(screen.getByRole("button", { name: "Promote Approval gate" }));

    expect(deckStore.getSnapshot().focusPaneId).toBe("approval_gate");
    expect(screen.getByLabelText("2. Approval gate")).toHaveAttribute("data-lod", "focus");
    expect(screen.getByLabelText("1. Queue blocker")).toHaveAttribute("data-lod", "preview");
    expect(screen.getAllByText("Approval gate was promoted by direct user focus.")).toHaveLength(2);
  });

  it("auto-ignites purple takeover on the Now rail item", async () => {
    render(<App />);

    const nowButton = screen.getByRole("button", { name: "Now" });
    expect(await screen.findByLabelText("AIUX Mission Deck")).toBeInTheDocument();
    expect(nowButton).toHaveClass("now", "agent-active", "takeover-complete");
  });
});
