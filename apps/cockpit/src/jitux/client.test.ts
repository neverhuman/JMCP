import { afterEach, describe, expect, it, vi } from "vitest";
import { openDeckSession, subscribeToDeckFrames } from "./client";
import { deckStore, resetDeckStoreForTests } from "./store";
import type { JituxFrame } from "./types";

class FakeEventSource {
  static instances: FakeEventSource[] = [];

  readonly listeners = new Map<string, EventListener[]>();
  closed = false;

  constructor(readonly url: string) {
    FakeEventSource.instances.push(this);
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

  close(): void {
    this.closed = true;
  }
}

const deckPatch: JituxFrame = {
  v: 1,
  type: "deck.patch",
  sessionId: "jitux_live",
  seq: 1,
  frameId: "frame_0001",
  emittedAt: "2026-06-03T15:00:00.000Z",
  source: "projection",
  deck: {
    title: "Scanning queue blockers",
    active: true,
    mode: "mission_deck",
  },
};

afterEach(() => {
  resetDeckStoreForTests();
  FakeEventSource.instances = [];
  vi.unstubAllGlobals();
});

describe("JITUX client", () => {
  it("opens a broker deck session with the canonical request body", async () => {
    const fetch = vi.fn(() =>
      Promise.resolve(
        new Response(
          JSON.stringify({
            sessionId: "jitux_live",
            streamUrl: "/jitux/sessions/jitux_live/stream",
            wsUrl: "/jitux/sessions/jitux_live/ws",
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        ),
      ),
    );
    vi.stubGlobal("fetch", fetch);

    await expect(openDeckSession({ prompt: "what is blocking the queue?", source: "deck" })).resolves.toEqual({
      sessionId: "jitux_live",
      streamUrl: "/jitux/sessions/jitux_live/stream",
      wsUrl: "/jitux/sessions/jitux_live/ws",
    });

    expect(fetch).toHaveBeenCalledWith(
      "http://127.0.0.1:18877/jitux/sessions",
      expect.objectContaining({
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ prompt: "what is blocking the queue?", source: "deck" }),
      }),
    );
  });

  it("feeds streamed broker frames through the reducer", () => {
    vi.stubGlobal("EventSource", FakeEventSource);

    const close = subscribeToDeckFrames("/jitux/sessions/jitux_live/stream", (frame) => deckStore.applyFrames([frame]));
    expect(FakeEventSource.instances[0]?.url).toBe("http://127.0.0.1:18877/jitux/sessions/jitux_live/stream");

    FakeEventSource.instances[0].emit("jitux.frame", deckPatch);

    expect(deckStore.getSnapshot().sessionId).toBe("jitux_live");
    expect(deckStore.getSnapshot().title).toBe("Scanning queue blockers");
    expect(deckStore.getSnapshot().active).toBe(true);

    close();
    expect(FakeEventSource.instances[0].closed).toBe(true);
  });
});
