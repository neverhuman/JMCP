import type { RuntimeState } from "../runtime";
import {
  QUEUE_BLOCKERS_DECK_SESSION_REQUEST,
  openDeckSession,
  subscribeToDeckFrames,
  type DeckSessionDescriptor,
} from "./client";
import type { FrameSource, JituxFrame } from "./types";

export type DeckSessionChannelDescriptor = {
  sessionId: string;
  streamUrl: string;
};

export type DeckLiveStopReason = "deactivate" | "barge_in";

export type DeckSessionTraceProbe = {
  id: string;
  label: string;
  source: FrameSource;
  status: "queued" | "running" | "ready" | "degraded";
  latencyMs?: number;
};

export type DeckLiveSessionCallbacks = {
  onOpening: () => void;
  onOpen: (descriptor: DeckSessionDescriptor) => void;
  onFrame: (frame: JituxFrame, descriptor: DeckSessionDescriptor) => void;
  onSessionUnavailable: () => void;
  onStreamUnavailable: () => void;
};

type Listener = (descriptor: DeckSessionChannelDescriptor) => void;

let currentDescriptor: DeckSessionChannelDescriptor | null = null;
const listeners = new Set<Listener>();

function isAbortError(error: unknown): boolean {
  return (
    typeof DOMException !== "undefined" &&
    error instanceof DOMException &&
    error.name === "AbortError"
  );
}

export function createDeckTrace(runtime: RuntimeState, status: DeckSessionTraceProbe["status"], source: FrameSource): DeckSessionTraceProbe[] {
  return [
    { id: "session", label: "session", source, status, latencyMs: status === "ready" ? 12 : undefined },
    { id: "attention", label: "attention", source: "projection", status: runtime.attentionPackets.length > 0 ? "ready" : "degraded", latencyMs: 18 },
    { id: "work-orders", label: "work orders", source: "projection", status: runtime.workItems.length > 0 ? "ready" : "degraded", latencyMs: 24 },
    { id: "approvals", label: "approvals", source: "approval", status: runtime.approvalRequests.length > 0 ? "ready" : "queued", latencyMs: 31 },
    { id: "adapters", label: "adapters", source: "adapter", status: runtime.systems.some((system) => system.health === "degraded" || system.health === "blocked") ? "degraded" : "ready", latencyMs: 33 },
    { id: "replay", label: "replay", source: "replay", status: runtime.replayEvents.length > 0 ? "ready" : "queued", latencyMs: 39 },
  ];
}

export function createDeckLiveSession(callbacks: DeckLiveSessionCallbacks) {
  let abortController: AbortController | null = null;
  let closeStream: (() => void) | null = null;
  let retryTimer: ReturnType<typeof window.setTimeout> | null = null;
  let token = 0;

  const clearRetryTimer = () => {
    if (retryTimer === null) {
      return;
    }
    window.clearTimeout(retryTimer);
    retryTimer = null;
  };

  const scheduleRetry = (currentToken: number, controller: AbortController) => {
    if (retryTimer !== null || controller.signal.aborted || currentToken !== token) {
      return;
    }
    retryTimer = window.setTimeout(() => {
      retryTimer = null;
      if (controller.signal.aborted || currentToken !== token) {
        return;
      }
      void attemptOpen(currentToken, controller);
    }, 1500);
  };

  const attemptOpen = async (currentToken: number, controller: AbortController) => {
    closeStream?.();
    closeStream = null;
    callbacks.onOpening();

    try {
      const descriptor = await openDeckSession(QUEUE_BLOCKERS_DECK_SESSION_REQUEST, controller.signal);
      if (controller.signal.aborted || currentToken !== token) return;
      let receivedFrame = false;
      publishDeckSessionDescriptor({ sessionId: descriptor.sessionId, streamUrl: descriptor.streamUrl });
      callbacks.onOpen(descriptor);
      const close = subscribeToDeckFrames(
        descriptor.streamUrl,
        (frame) => {
          if (controller.signal.aborted || currentToken !== token) return;
          receivedFrame = true;
          clearRetryTimer();
          callbacks.onFrame(frame, descriptor);
        },
        () => {
          if (controller.signal.aborted || currentToken !== token || receivedFrame) return;
          closeStream?.();
          closeStream = null;
          callbacks.onStreamUnavailable();
          scheduleRetry(currentToken, controller);
        },
      );
      if (controller.signal.aborted || currentToken !== token) {
        close();
        return;
      }
      closeStream = close;
    } catch (error: unknown) {
      if (controller.signal.aborted || currentToken !== token || isAbortError(error)) return;
      callbacks.onSessionUnavailable();
      scheduleRetry(currentToken, controller);
    }
  };

  const stop = () => {
    token += 1;
    clearRetryTimer();
    abortController?.abort();
    abortController = null;
    closeStream?.();
    closeStream = null;
  };

  return {
    start: () => {
      stop();
      const currentToken = ++token;
      const controller = new AbortController();
      abortController = controller;
      void attemptOpen(currentToken, controller);

      return stop;
    },
    stop,
  };
}

export function publishDeckSessionDescriptor(descriptor: DeckSessionChannelDescriptor): void {
  currentDescriptor = descriptor;
  for (const listener of listeners) {
    listener(descriptor);
  }
}

export function getDeckSessionDescriptor(): DeckSessionChannelDescriptor | null {
  return currentDescriptor;
}

export function subscribeDeckSessionDescriptor(listener: Listener): () => void {
  listeners.add(listener);
  if (currentDescriptor !== null) {
    listener(currentDescriptor);
  }
  return () => listeners.delete(listener);
}

export function resetDeckSessionChannelForTests(): void {
  currentDescriptor = null;
  listeners.clear();
}
