import { act, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createFixtureRuntime } from "../runtime";
import { createQueueBlockerFrames, deckStore, resetDeckStoreForTests } from "./store";
import { MockEventSource } from "./mock-event-source";

afterEach(() => {
  resetDeckStoreForTests();
  MockEventSource.reset();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

function descriptorResponse(sessionId = "jitux_live"): Response {
  return new Response(
    JSON.stringify({
      sessionId,
      streamUrl: `/jitux/sessions/${sessionId}/stream`,
      wsUrl: `/jitux/sessions/${sessionId}/ws`,
    }),
    { status: 200, headers: { "content-type": "application/json" } },
  );
}

describe("JITUX deck store session flow", () => {
  it("paints a labeled cached snapshot immediately", () => {
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    const snapshot = deckStore.getSnapshot();
    expect(snapshot.caption).toBe("Cached snapshot is visible while the broker session opens.");
    expect(snapshot.streamStatus).toBe("degraded");
    expect(deckStore.rankedPanes()).toHaveLength(5);
    expect(deckStore.rankedPanes()[0].title).toBe("Queue blocker");
    expect(deckStore.cardsForPane("queue_blockers")[0].headline).toContain("blocking");
  });

  it("flows live frames from the mock EventSource through reducer state", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(descriptorResponse("jitux_live"))));
    vi.stubGlobal("EventSource", MockEventSource as unknown as typeof EventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    const stop = deckStore.startLiveQueueBlockers();
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));

    const frames = createQueueBlockerFrames(createFixtureRuntime(), "jitux_live");
    for (const frame of frames) {
      act(() => MockEventSource.instances[0].emitFrame(frame));
    }

    const snapshot = deckStore.getSnapshot();
    expect(snapshot.streamStatus).toBe("live");
    expect(snapshot.caption).toBe("BROKER is driving the Mission Deck with live frames and ranked insights.");
    expect(snapshot.sessionId).toBe("jitux_live");
    expect(snapshot.focusPaneId).toBe("queue_blockers");
    expect(deckStore.rankedPanes()[0].id).toBe("queue_blockers");
    expect(snapshot.evidenceByPane.queue_blockers).toHaveLength(4);
    expect(snapshot.actionsByPane.queue_blockers.length).toBeGreaterThan(0);
    expect(deckStore.cardsForPane("queue_blockers")[0].status).toBe("hydrated");

    stop();
  });

  it("retains the cached snapshot and marks the caption degraded when the broker session cannot open", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("api unavailable"))));
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    deckStore.startLiveQueueBlockers();

    await waitFor(() =>
      expect(deckStore.getSnapshot().caption).toBe(
        "Broker session unavailable; retrying to keep the Mission Deck broker-driven.",
      ),
    );
    expect(deckStore.getSnapshot().streamStatus).toBe("degraded");
    expect(deckStore.rankedPanes()).toHaveLength(5);
  });

  it("retries a transient broker session failure and returns to live frames", async () => {
    vi.useFakeTimers();
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(
        new Response("", {
          status: 500,
        }),
      )
      .mockResolvedValueOnce(descriptorResponse("jitux_retry"));
    vi.stubGlobal("fetch", fetch);
    vi.stubGlobal("EventSource", MockEventSource as unknown as typeof EventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    const stop = deckStore.startLiveQueueBlockers();
    await act(async () => {
      await Promise.resolve();
    });
    expect(deckStore.getSnapshot().caption).toBe(
      "Broker session unavailable; retrying to keep the Mission Deck broker-driven.",
    );
    expect(fetch).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1500);
      await Promise.resolve();
    });
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(MockEventSource.instances).toHaveLength(1);

    const frames = createQueueBlockerFrames(createFixtureRuntime(), "jitux_retry");
    act(() => MockEventSource.instances[0].emitFrame(frames[0]));

    expect(deckStore.getSnapshot().caption).toBe(
      "BROKER is driving the Mission Deck with live frames and ranked insights.",
    );

    stop();
  });

  it("stops the mock stream on barge-in and ignores later frames", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(descriptorResponse("jitux_barge_in"))));
    vi.stubGlobal("EventSource", MockEventSource as unknown as typeof EventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    deckStore.startLiveQueueBlockers();
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));

    const frames = createQueueBlockerFrames(createFixtureRuntime(), "jitux_barge_in");
    act(() => MockEventSource.instances[0].emitFrame(frames[0]));
    const beforeStop = deckStore.getSnapshot();

    deckStore.stopLiveQueueBlockers("barge_in");
    expect(MockEventSource.instances[0].closed).toBe(true);

    act(() => MockEventSource.instances[0].emitFrame(frames[1]));
    const afterStop = deckStore.getSnapshot();

    expect(afterStop.lastSeq).toBe(beforeStop.lastSeq);
    expect(afterStop.caption).toBe("Live broker stream paused for barge-in; cached snapshot remains visible.");
  });

  it("stops the mock stream on deactivate and ignores later frames", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(descriptorResponse("jitux_deactivate"))));
    vi.stubGlobal("EventSource", MockEventSource as unknown as typeof EventSource);
    act(() => deckStore.igniteQueueBlockers(createFixtureRuntime()));

    deckStore.startLiveQueueBlockers();
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));

    const frames = createQueueBlockerFrames(createFixtureRuntime(), "jitux_deactivate");
    act(() => MockEventSource.instances[0].emitFrame(frames[0]));
    const beforeStop = deckStore.getSnapshot();

    deckStore.stopLiveQueueBlockers();
    expect(MockEventSource.instances[0].closed).toBe(true);

    act(() => MockEventSource.instances[0].emitFrame(frames[1]));
    const afterStop = deckStore.getSnapshot();

    expect(afterStop.lastSeq).toBe(beforeStop.lastSeq);
  });
});
