import { isJituxFrame } from "../jitux/guards";
import type { JituxFrame } from "../jitux/types";

export const JITUX_FIRST_FRAME_TIMEOUT_MS = 120;

export interface VoiceJituxSession {
  sessionId: string;
  streamUrl: string;
  wsUrl: string;
}

export type VoiceJituxSessionStart =
  | { kind: "ready"; session: VoiceJituxSession }
  | { kind: "unavailable"; reason: string };

export type VoiceJituxDeckReadiness =
  | { kind: "frame"; sessionId: string; frameType: JituxFrame["type"] }
  | { kind: "timeout" }
  | { kind: "unavailable"; reason: string };

type ParsedFrame =
  | { kind: "frame"; sessionId: string; frameType: JituxFrame["type"] }
  | { kind: "ignore" };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function jmcpBase(): string {
  const value = import.meta.env.VITE_JMCP_BASE;
  return typeof value === "string" && value.length > 0 ? value : "/jmcp";
}

function isAbortError(error: unknown): boolean {
  return (
    typeof DOMException !== "undefined" &&
    error instanceof DOMException &&
    error.name === "AbortError"
  );
}

function isVoiceJituxSession(value: unknown): value is VoiceJituxSession {
  if (!isRecord(value)) {
    return false;
  }
  return (
    readString(value.sessionId).length > 0 &&
    readString(value.streamUrl).length > 0 &&
    readString(value.wsUrl).length > 0
  );
}

export async function openVoiceJituxSession(
  prompt: string,
  signal?: AbortSignal,
): Promise<VoiceJituxSessionStart> {
  try {
    const response = await fetch(`${jmcpBase()}/jitux/sessions`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      signal,
      body: JSON.stringify({ prompt, source: "voice" }),
    });
    if (!response.ok) {
      return { kind: "unavailable", reason: `jitux_session_${response.status}` };
    }
    const body: unknown = await response.json();
    if (!isVoiceJituxSession(body)) {
      return { kind: "unavailable", reason: "jitux_session_shape" };
    }
    return { kind: "ready", session: body };
  } catch (error) {
    if (isAbortError(error)) {
      return { kind: "unavailable", reason: "aborted" };
    }
    return {
      kind: "unavailable",
      reason: error instanceof Error ? error.message : "jitux_session_error",
    };
  }
}

function isUsefulDeckFrame(frame: JituxFrame): boolean {
  switch (frame.type) {
    case "deck.patch":
    case "card.ghost":
    case "pane.prepare":
    case "pane.upsert":
    case "focus.change":
    case "deck.rank.changed":
      return true;
    default:
      return false;
  }
}

function parseSseLine(line: string): ParsedFrame {
  if (!line.startsWith("data:")) {
    return { kind: "ignore" };
  }
  const payload = line.slice(5).trim();
  if (payload.length === 0) {
    return { kind: "ignore" };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(payload);
  } catch {
    return { kind: "ignore" };
  }
  if (!isJituxFrame(parsed) || !isUsefulDeckFrame(parsed)) {
    return { kind: "ignore" };
  }
  return { kind: "frame", sessionId: parsed.sessionId, frameType: parsed.type };
}

async function readUsefulDeckFrame(
  start: Promise<VoiceJituxSessionStart>,
  signal: AbortSignal,
): Promise<VoiceJituxDeckReadiness> {
  const sessionStart = await start;
  if (sessionStart.kind === "unavailable") {
    return sessionStart;
  }
  const response = await fetch(`${jmcpBase()}${sessionStart.session.streamUrl}`, {
    headers: { accept: "text/event-stream" },
    signal,
  });
  if (!response.ok) {
    return { kind: "unavailable", reason: `jitux_stream_${response.status}` };
  }
  const stream = response.body;
  if (stream === null) {
    return { kind: "unavailable", reason: "jitux_stream_body_missing" };
  }
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  for (;;) {
    const chunk = await reader.read();
    if (chunk.done) {
      break;
    }
    buffer += decoder.decode(chunk.value, { stream: true });
    let newlineAt = buffer.indexOf("\n");
    while (newlineAt >= 0) {
      const line = buffer.slice(0, newlineAt).trim();
      buffer = buffer.slice(newlineAt + 1);
      newlineAt = buffer.indexOf("\n");
      const parsed = parseSseLine(line);
      if (parsed.kind === "frame") {
        return parsed;
      }
    }
  }
  return { kind: "unavailable", reason: "jitux_stream_ended" };
}

function timeoutReadiness(
  timeoutMs: number,
  controller: AbortController,
): { promise: Promise<VoiceJituxDeckReadiness>; cancel: () => void } {
  let timeoutCancel = () => {};
  const promise = new Promise<VoiceJituxDeckReadiness>((resolve) => {
    const timer = setTimeout(() => {
      controller.abort();
      resolve({ kind: "timeout" });
    }, timeoutMs);
    timeoutCancel = () => clearTimeout(timer);
  });
  return {
    promise,
    cancel: () => timeoutCancel(),
  };
}

export async function waitForUsefulVoiceDeckFrame(
  start: Promise<VoiceJituxSessionStart>,
  signal: AbortSignal,
  timeoutMs = JITUX_FIRST_FRAME_TIMEOUT_MS,
): Promise<VoiceJituxDeckReadiness> {
  if (signal.aborted) {
    return { kind: "unavailable", reason: "aborted" };
  }
  const controller = new AbortController();
  const abortStream = () => controller.abort();
  signal.addEventListener("abort", abortStream, { once: true });
  const timeout = timeoutReadiness(timeoutMs, controller);
  try {
    return await Promise.race([readUsefulDeckFrame(start, controller.signal), timeout.promise]);
  } catch (error) {
    if (isAbortError(error)) {
      return signal.aborted
        ? { kind: "unavailable", reason: "aborted" }
        : { kind: "timeout" };
    }
    return {
      kind: "unavailable",
      reason: error instanceof Error ? error.message : "jitux_stream_error",
    };
  } finally {
    timeout.cancel();
    signal.removeEventListener("abort", abortStream);
    controller.abort();
  }
}
