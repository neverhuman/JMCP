import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../../App";
import { createFixtureRuntime } from "../../runtime";
import { isJituxFrame } from "../guards";
import { getDeckSessionDescriptor } from "../session-channel";
import { createQueueBlockerFrames, deckStore, resetDeckStoreForTests } from "../store";
import type { JituxFrame } from "../types";
import { NowCommandDeck } from "./NowCommandDeck";

class FakeDeckEventSource {
  static instances: FakeDeckEventSource[] = [];

  readonly listeners = new Map<string, EventListener[]>();
  closed = false;

  constructor(readonly url: string) {
    FakeDeckEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: EventListener): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  emit(type: string, frame: JituxFrame): void {
    for (const listener of this.listeners.get(type) ?? []) {
      listener({ data: JSON.stringify(frame) } as MessageEvent<string>);
    }
  }

  emitError(): void {
    for (const listener of this.listeners.get("error") ?? []) {
      listener(new Event("error"));
    }
  }

  close(): void {
    this.closed = true;
  }
}

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

function liveQueueBlockerFrames(sessionId = "jitux_live"): JituxFrame[] {
  return createQueueBlockerFrames(createFixtureRuntime(), sessionId).map((frame) => ({
    ...frame,
    source: "projection" as const,
  })) as JituxFrame[];
}

function sessionResponse(sessionId = "jitux_live"): Response {
  return new Response(
    JSON.stringify({
      sessionId,
      streamUrl: `/jitux/sessions/${sessionId}/stream`,
      wsUrl: `/jitux/sessions/${sessionId}/ws`,
    }),
    { status: 200, headers: { "content-type": "application/json" } },
  );
}

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("test api unavailable"))));
  FakeDeckEventSource.instances = [];
  setReducedMotion(false);
  resetDeckStoreForTests();
});

afterEach(() => {
  resetDeckStoreForTests();
  FakeDeckEventSource.instances = [];
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

  it("auto-ignites purple takeover on the Now rail item", async () => {
    render(<App />);

    const nowButton = screen.getByRole("button", { name: "Now" });
    expect(await screen.findByLabelText("AIUX Mission Deck")).toBeInTheDocument();
    expect(nowButton).toHaveClass("now", "agent-active", "takeover-complete");
  });

  it("opens a broker session from the active deck and accepts live frames", async () => {
    const fetch = vi.fn(() => Promise.resolve(sessionResponse("jitux_live")));
    vi.stubGlobal("fetch", fetch);
    vi.stubGlobal("EventSource", FakeDeckEventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    render(<NowCommandDeck />);

    expect(screen.getByText("Cached snapshot is visible while the broker session opens.")).toBeInTheDocument();
    await waitFor(() => expect(FakeDeckEventSource.instances).toHaveLength(1));
    expect(fetch).toHaveBeenCalledWith(
      "http://127.0.0.1:18877/jitux/sessions",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ prompt: "what is blocking the queue?", source: "deck" }),
      }),
    );
    expect(FakeDeckEventSource.instances[0].url).toBe("http://127.0.0.1:18877/jitux/sessions/jitux_live/stream");
    expect(getDeckSessionDescriptor()).toEqual({
      sessionId: "jitux_live",
      streamUrl: "/jitux/sessions/jitux_live/stream",
    });

    act(() => FakeDeckEventSource.instances[0].emit("jitux.frame", liveQueueBlockerFrames()[0]));

    expect(deckStore.getSnapshot().streamStatus).toBe("live");
    expect(deckStore.getSnapshot().sessionId).toBe("jitux_live");
    expect(screen.getByText("Live broker frames are driving the Mission Deck.")).toBeInTheDocument();
  });

  it("keeps the cached snapshot visible when the broker session cannot open", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("api unavailable"))));
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    render(<NowCommandDeck />);

    expect(screen.getByRole("list", { name: "Ranked Mission Deck" })).toBeInTheDocument();
    expect(await screen.findByText("Broker session unavailable; cached snapshot remains visible.")).toBeInTheDocument();
    expect(within(screen.getByRole("list", { name: "Ranked Mission Deck" })).getAllByRole("listitem")).toHaveLength(5);
    expect(deckStore.getSnapshot().streamStatus).toBe("degraded");
  });

  it("keeps the cached snapshot visible when the broker stream cannot deliver a first frame", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(sessionResponse("jitux_stream_error"))));
    vi.stubGlobal("EventSource", FakeDeckEventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    render(<NowCommandDeck />);

    await waitFor(() => expect(FakeDeckEventSource.instances).toHaveLength(1));
    act(() => FakeDeckEventSource.instances[0].emitError());

    expect(await screen.findByText("Broker stream unavailable; cached snapshot remains visible.")).toBeInTheDocument();
    expect(within(screen.getByRole("list", { name: "Ranked Mission Deck" })).getAllByRole("listitem")).toHaveLength(5);
    expect(deckStore.getSnapshot().streamStatus).toBe("degraded");
  });

  it("tears down pending session open on deactivate", async () => {
    let openSignal: AbortSignal | undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) => {
        openSignal = init?.signal ?? undefined;
        return new Promise<Response>(() => undefined);
      }),
    );
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    const { unmount } = render(<NowCommandDeck />);
    await waitFor(() => expect(openSignal).toBeDefined());

    unmount();

    expect(openSignal?.aborted).toBe(true);
  });

  it("closes the active stream on barge-in", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(sessionResponse("jitux_barge"))));
    vi.stubGlobal("EventSource", FakeDeckEventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));
    render(<NowCommandDeck />);
    await waitFor(() => expect(FakeDeckEventSource.instances).toHaveLength(1));

    act(() => deckStore.stopLiveQueueBlockers("barge_in"));

    expect(FakeDeckEventSource.instances[0].closed).toBe(true);
    expect(screen.getByText("Live broker stream paused for barge-in; cached snapshot remains visible.")).toBeInTheDocument();
  });

  it("marks the deck for mobile clearance from the fixed voice bar", () => {
    applyQueueBlockerFrames();

    render(<NowCommandDeck />);

    expect(screen.getByLabelText("AIUX Mission Deck")).toHaveAttribute("data-mobile-clearance", "voice-bar");
  });
});
