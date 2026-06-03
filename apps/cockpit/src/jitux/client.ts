import { isJituxFrame } from "./guards";
import type { JituxFrame } from "./types";

const apiUrl = import.meta.env.VITE_JMCP_API_URL ?? "http://127.0.0.1:18877";

async function getJson<T>(path: string, guard: (value: unknown) => value is T, signal?: AbortSignal): Promise<T> {
  const response = await fetch(`${apiUrl}${path}`, { signal });
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

export function subscribeToDeckFrames(streamUrl: string, onFrame: (frame: JituxFrame) => void): () => void {
  if (typeof EventSource !== "function") {
    return () => undefined;
  }

  const events = new EventSource(streamUrl);
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
  return () => events.close();
}

export function subscribeToDeckGenerationBumps(onGenerationBump: () => void): () => void {
  if (typeof EventSource !== "function") {
    return () => undefined;
  }

  const events = new EventSource(`${apiUrl}/events`);
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
