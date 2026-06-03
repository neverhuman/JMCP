import { isJituxFrame } from "./guards";
import type { JituxFrame } from "./types";

// Reach the broker through the same-origin "/jmcp" vite proxy (rewrites to the
// jmcpd API), exactly like the voice tools do. Hitting http://127.0.0.1:18877
// directly is cross-origin from the cockpit dev server and the browser blocks it,
// so the deck keeps retrying until live broker frames arrive.
const apiUrl = import.meta.env.VITE_JMCP_API_URL ?? "/jmcp";
const apiBase = apiUrl.replace(/\/+$/, "");

export type OpenDeckSessionRequest = {
  prompt?: string;
  source?: string;
};

export type DeckSessionDescriptor = {
  sessionId: string;
  streamUrl: string;
  wsUrl: string;
};

export const QUEUE_BLOCKERS_DECK_SESSION_REQUEST: OpenDeckSessionRequest = {
  prompt: "what is blocking the queue?",
  source: "deck",
};

function toApiUrl(pathOrUrl: string): string {
  try {
    return new URL(pathOrUrl).toString();
  } catch {
    const path = pathOrUrl.startsWith("/") ? pathOrUrl : `/${pathOrUrl}`;
    return `${apiBase}${path}`;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isDeckSessionDescriptor(value: unknown): value is DeckSessionDescriptor {
  return (
    isRecord(value) &&
    typeof value.sessionId === "string" &&
    value.sessionId.length > 0 &&
    typeof value.streamUrl === "string" &&
    value.streamUrl.length > 0 &&
    typeof value.wsUrl === "string" &&
    value.wsUrl.length > 0
  );
}

async function getJson<T>(path: string, guard: (value: unknown) => value is T, signal?: AbortSignal): Promise<T> {
  const response = await fetch(toApiUrl(path), { signal });
  if (!response.ok) {
    throw new Error(`JITUX request failed: ${response.status}`);
  }
  const payload: unknown = await response.json();
  if (!guard(payload)) {
    throw new Error(`JITUX response rejected for ${path}`);
  }
  return payload;
}

function isJituxFrameArray(value: unknown): value is JituxFrame[] {
  return Array.isArray(value) && value.every(isJituxFrame);
}

export function fetchJituxFrame(path: string, signal?: AbortSignal): Promise<JituxFrame> {
  return getJson(path, isJituxFrame, signal);
}

export function fetchJituxFrames(path: string, signal?: AbortSignal): Promise<JituxFrame[]> {
  return getJson(path, isJituxFrameArray, signal);
}

export async function openDeckSession(request: OpenDeckSessionRequest, signal?: AbortSignal): Promise<DeckSessionDescriptor> {
  const response = await fetch(toApiUrl("/jitux/sessions"), {
    method: "POST",
    headers: { "content-type": "application/json" },
    signal,
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error(`JITUX session request failed: ${response.status}`);
  }
  const payload: unknown = await response.json();
  if (!isDeckSessionDescriptor(payload)) {
    throw new Error("JITUX session response rejected");
  }
  return payload;
}

export function subscribeToDeckFrames(
  streamUrl: string,
  onFrame: (frame: JituxFrame) => void,
  onStreamError?: () => void,
): () => void {
  if (typeof EventSource !== "function") {
    onStreamError?.();
    return () => undefined;
  }

  let events: EventSource;
  try {
    events = new EventSource(toApiUrl(streamUrl));
  } catch {
    onStreamError?.();
    return () => undefined;
  }
  const handleMessage = (event: MessageEvent<string>) => {
    try {
      const payload: unknown = JSON.parse(event.data);
      if (isJituxFrame(payload)) {
        onFrame(payload);
      }
    } catch {
      return;
    }
  };

  events.addEventListener("message", handleMessage as EventListener);
  events.addEventListener("jitux.frame", handleMessage as EventListener);
  events.addEventListener("error", () => onStreamError?.());
  return () => events.close();
}

export function subscribeToDeckGenerationBumps(onGenerationBump: () => void): () => void {
  if (typeof EventSource !== "function") {
    return () => undefined;
  }

  const events = new EventSource(toApiUrl("/events"));
  const bump = () => onGenerationBump();
  events.addEventListener("jmcp.events", bump as EventListener);
  return () => events.close();
}

export type DeckInteractionEvent =
  | { type: "focus"; paneId: string }
  | { type: "hover"; paneId: string }
  | { type: "reveal"; paneId: string; tab?: string }
  | { type: "fan" }
  | { type: "collapse" }
  | { type: "tunnel"; paneId: string; target: string };

export class DeckInteractionSocket {
  private socket: WebSocket | null = null;

  constructor(private readonly wsUrl: string) {}

  connect(): void {
    if (typeof WebSocket !== "function" || this.socket) {
      return;
    }
    this.socket = new WebSocket(this.wsUrl);
  }

  send(event: DeckInteractionEvent): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return;
    }
    this.socket.send(JSON.stringify(event));
  }

  close(): void {
    this.socket?.close();
    this.socket = null;
  }
}
