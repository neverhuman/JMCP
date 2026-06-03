import { afterEach, describe, expect, it, vi } from "vitest";
import { createFixtureRuntime } from "../runtime";
import { createQueueBlockerFrames } from "./store";
import { MockEventSource } from "./mock-event-source";
import { openDeckSession, subscribeToDeckFrames } from "./client";
import type { JituxFrame } from "./types";

afterEach(() => {
  MockEventSource.reset();
  vi.unstubAllGlobals();
});

function response(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("JITUX client session helpers", () => {
  it("opens a deck session and parses the descriptor", async () => {
    const fetch = vi.fn(() =>
      Promise.resolve(
        response({
          sessionId: "jitux_live",
          streamUrl: "/jitux/sessions/jitux_live/stream",
          wsUrl: "/jitux/sessions/jitux_live/ws",
        }),
      ),
    );
    vi.stubGlobal("fetch", fetch);

    await expect(
      openDeckSession({ prompt: "what is blocking the queue?", source: "deck" }),
    ).resolves.toEqual({
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

  it("rejects HTTP errors from the session endpoint", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(response({ error: "nope" }, 500))));

    await expect(openDeckSession({ prompt: "queue", source: "deck" })).rejects.toThrow(
      "JITUX session request failed: 500",
    );
  });

  it("rejects malformed JSON payloads", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => Promise.resolve(new Response("not json", { status: 200 }))),
    );

    await expect(openDeckSession({ prompt: "queue", source: "deck" })).rejects.toBeInstanceOf(
      SyntaxError,
    );
  });

  it("rejects when the request is aborted before completion", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        new Promise<Response>((_resolve, reject) => {
          init?.signal?.addEventListener("abort", () => {
            reject(Object.assign(new Error("Aborted"), { name: "AbortError" }));
          });
        }),
      ),
    );
    const controller = new AbortController();

    const promise = openDeckSession({ prompt: "queue", source: "deck" }, controller.signal);
    controller.abort();

    await expect(promise).rejects.toMatchObject({ name: "AbortError" });
  });

  it("subscribes to canonical frames in order and tears down cleanly", () => {
    vi.stubGlobal("EventSource", MockEventSource as unknown as typeof EventSource);

    const frames = createQueueBlockerFrames(createFixtureRuntime(), "jitux_live");
    const seen: string[] = [];
    const close = subscribeToDeckFrames("/jitux/sessions/jitux_live/stream", (frame: JituxFrame) => {
      seen.push(frame.type);
    });

    expect(MockEventSource.instances[0]?.url).toBe(
      "http://127.0.0.1:18877/jitux/sessions/jitux_live/stream",
    );
    MockEventSource.instances[0].emitFrame(frames[0]);
    MockEventSource.instances[0].emitFrame(frames[1]);
    MockEventSource.instances[0].emitFrame(frames[2]);

    expect(seen).toEqual(["deck.patch", "pane.prepare", "card.ghost"]);

    close();
    expect(MockEventSource.instances[0].closed).toBe(true);
    MockEventSource.instances[0].emitFrame(frames[3]);
    expect(seen).toEqual(["deck.patch", "pane.prepare", "card.ghost"]);
  });
});

