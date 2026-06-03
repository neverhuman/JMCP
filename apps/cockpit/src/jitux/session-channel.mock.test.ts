import { afterEach, describe, expect, it } from "vitest";
import {
  getDeckSessionDescriptor,
  publishDeckSessionDescriptor,
  resetDeckSessionChannelForTests,
  subscribeDeckSessionDescriptor,
} from "./session-channel";

afterEach(() => {
  resetDeckSessionChannelForTests();
});

describe("JITUX session channel", () => {
  it("publishes descriptors to multiple subscribers and supports unsubscribe", () => {
    const first: Array<{ sessionId: string; streamUrl: string }> = [];
    const second: Array<{ sessionId: string; streamUrl: string }> = [];

    const unsubscribeFirst = subscribeDeckSessionDescriptor((descriptor) => {
      first.push(descriptor);
    });

    publishDeckSessionDescriptor({
      sessionId: "jitux_a",
      streamUrl: "/jitux/sessions/jitux_a/stream",
    });

    const unsubscribeSecond = subscribeDeckSessionDescriptor((descriptor) => {
      second.push(descriptor);
    });

    publishDeckSessionDescriptor({
      sessionId: "jitux_b",
      streamUrl: "/jitux/sessions/jitux_b/stream",
    });

    unsubscribeFirst();
    publishDeckSessionDescriptor({
      sessionId: "jitux_c",
      streamUrl: "/jitux/sessions/jitux_c/stream",
    });
    unsubscribeSecond();

    expect(first).toEqual([
      { sessionId: "jitux_a", streamUrl: "/jitux/sessions/jitux_a/stream" },
      { sessionId: "jitux_b", streamUrl: "/jitux/sessions/jitux_b/stream" },
    ]);
    expect(second).toEqual([
      { sessionId: "jitux_a", streamUrl: "/jitux/sessions/jitux_a/stream" },
      { sessionId: "jitux_b", streamUrl: "/jitux/sessions/jitux_b/stream" },
      { sessionId: "jitux_c", streamUrl: "/jitux/sessions/jitux_c/stream" },
    ]);
    expect(getDeckSessionDescriptor()).toEqual({
      sessionId: "jitux_c",
      streamUrl: "/jitux/sessions/jitux_c/stream",
    });
    unsubscribeFirst();
    unsubscribeSecond();
  });
});

