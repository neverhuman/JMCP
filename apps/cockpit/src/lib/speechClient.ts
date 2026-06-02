// Browser client for the local, on-box voice stack: ASR (:18878), TTS (:18901),
// and the reasoning LLM (vLLM OpenAI API, :18902). All three are reached through
// the Vite dev proxy (/asr, /tts, /llm) so the browser stays same-origin (no CORS)
// and no audio ever leaves the machine. Responses are narrowed from `unknown`
// with explicit guards (matching runtime-api-guards.ts) — never `as`-cast.

export interface Transcription {
  text: string;
  confidence: number | null;
}

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function envBase(key: string, defaultBase: string): string {
  const value = import.meta.env[key];
  return typeof value === "string" && value.length > 0 ? value : defaultBase;
}

const ASR = envBase("VITE_ASR_BASE", "/asr");
const TTS = envBase("VITE_TTS_BASE", "/tts");
const LLM = envBase("VITE_LLM_BASE", "/llm");
export const VOICE_MODEL = envBase("VITE_LLM_MODEL", "local/qwen3-30b-a3b");

/** Transcribe recorded audio (webm/opus/ogg/wav) via the faster-whisper sidecar. */
export async function transcribe(audio: Blob, language = "en"): Promise<Transcription> {
  const response = await fetch(`${ASR}/transcribe?language=${encodeURIComponent(language)}`, {
    method: "POST",
    headers: { "content-type": audio.type || "audio/webm" },
    body: audio,
  });
  if (!response.ok) {
    throw new Error(`ASR ${response.status}`);
  }
  const body: unknown = await response.json();
  const text = isRecord(body) ? readString(body.text).trim() : "";
  const confidence =
    isRecord(body) && typeof body.confidence === "number" ? body.confidence : null;
  return { text, confidence };
}

/** Synthesize speech via the Kokoro sidecar; returns an OGG/Opus blob to play. */
export async function synthesize(text: string): Promise<Blob> {
  const response = await fetch(`${TTS}/synthesize?format=ogg`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ text }),
  });
  if (!response.ok) {
    throw new Error(`TTS ${response.status}`);
  }
  return response.blob();
}

/** Extract the assistant message content from an OpenAI chat-completion body. */
function firstChoiceContent(body: unknown): string {
  if (!isRecord(body) || !Array.isArray(body.choices)) {
    return "";
  }
  const choice: unknown = body.choices[0];
  if (!isRecord(choice) || !isRecord(choice.message)) {
    return "";
  }
  return readString(choice.message.content);
}

/** One non-streaming reasoning turn against the local vLLM OpenAI endpoint. */
export async function reason(
  messages: ChatMessage[],
  signal?: AbortSignal,
  model: string = VOICE_MODEL,
): Promise<string> {
  const response = await fetch(`${LLM}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    signal,
    body: JSON.stringify({ model, messages, temperature: 0.4, max_tokens: 200, stream: false }),
  });
  if (!response.ok) {
    throw new Error(`LLM ${response.status}`);
  }
  const body: unknown = await response.json();
  return firstChoiceContent(body).trim();
}

/** One streaming SSE chunk -> its incremental delta text (narrowed, no `as`). */
function streamDelta(payload: string): string {
  let parsed: unknown;
  try {
    parsed = JSON.parse(payload);
  } catch {
    return "";
  }
  if (!isRecord(parsed) || !Array.isArray(parsed.choices)) {
    return "";
  }
  const choice: unknown = parsed.choices[0];
  if (!isRecord(choice) || !isRecord(choice.delta)) {
    return "";
  }
  return readString(choice.delta.content);
}

/**
 * Streaming reasoning turn: `onDelta` fires for each incremental token chunk so
 * the caller can start speaking the first sentence before the rest generates.
 * Returns the full text. Lets the voice loop hit first-audio in well under a second.
 */
export async function reasonStream(
  messages: ChatMessage[],
  onDelta: (delta: string) => void,
  signal?: AbortSignal,
  model: string = VOICE_MODEL,
): Promise<string> {
  const response = await fetch(`${LLM}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    signal,
    body: JSON.stringify({ model, messages, temperature: 0.4, max_tokens: 200, stream: true }),
  });
  if (!response.ok) {
    throw new Error(`LLM ${response.status}`);
  }
  const stream = response.body;
  if (stream === null) {
    throw new Error("LLM stream unavailable");
  }
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let full = "";
  for (;;) {
    const chunk = await reader.read();
    if (chunk.done) {
      break;
    }
    buffer += decoder.decode(chunk.value, { stream: true });
    // Consume whole lines, keeping any trailing partial in `buffer` (index-based
    // so there are no defensive coalescing operators).
    let newlineAt = buffer.indexOf("\n");
    while (newlineAt >= 0) {
      const line = buffer.slice(0, newlineAt).trim();
      buffer = buffer.slice(newlineAt + 1);
      newlineAt = buffer.indexOf("\n");
      if (!line.startsWith("data:")) {
        continue;
      }
      const payload = line.slice(5).trim();
      if (payload === "[DONE]") {
        continue;
      }
      if (payload.length === 0) {
        continue;
      }
      const delta = streamDelta(payload);
      if (delta.length > 0) {
        full += delta;
        onDelta(delta);
      }
    }
  }
  return full.trim();
}

/** True when this browser can capture a microphone (guards SSR / test / insecure-origin). */
export function micSupported(): boolean {
  return (
    typeof navigator !== "undefined" &&
    typeof navigator.mediaDevices?.getUserMedia === "function" &&
    typeof window !== "undefined" &&
    typeof window.MediaRecorder === "function"
  );
}
