// Browser client for the local, on-box voice stack: ASR (:18878), TTS (:18901),
// and the reasoning LLM (vLLM OpenAI API, :18902). All three are reached through
// the Vite dev proxy (/asr, /tts, /llm) so the browser stays same-origin (no CORS)
// and no audio ever leaves the machine. Responses are narrowed from `unknown`
// with explicit guards (matching runtime-api-guards.ts) — never `as`-cast.

export interface Transcription {
  text: string;
  confidence: number | null;
}

export interface ToolCallFunction {
  id: string;
  type: "function";
  function: { name: string; arguments: string };
}

export interface ChatMessage {
  role: "system" | "user" | "assistant" | "tool";
  content: string;
  // Present on an assistant turn that asked to call tools, and on the tool-result
  // turn that answers one. Optional so plain chat turns stay `{ role, content }`.
  tool_calls?: ToolCallFunction[];
  tool_call_id?: string;
}

/** One tool the model asked to call, reassembled from the streamed deltas. */
export interface ToolCall {
  id: string;
  name: string;
  arguments: string; // raw JSON string the model emitted
}

/** A streaming reasoning turn's result: spoken text plus any tool calls. */
export interface ReasonResult {
  text: string;
  toolCalls: ToolCall[];
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
// Served-model name the vLLM sidecar registers under. Override with VITE_LLM_MODEL
// when the underlying model is swapped.
export const VOICE_MODEL = envBase("VITE_LLM_MODEL", "local/qwen3-30b-a3b");

/** Transcribe recorded audio (webm/opus/ogg/wav) via the faster-whisper sidecar. */
export async function transcribe(
  audio: Blob,
  language = "en",
  beamSize = 1,
): Promise<Transcription> {
  const params = new URLSearchParams({
    language,
    beam_size: String(beamSize),
  });
  const response = await fetch(`${ASR}/transcribe?${params.toString()}`, {
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
export async function synthesize(text: string, signal?: AbortSignal): Promise<Blob> {
  const response = await fetch(`${TTS}/synthesize?format=ogg`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    signal,
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

interface ToolCallDelta {
  index: number;
  id: string;
  name: string;
  argChunk: string;
}

interface StreamChoice {
  content: string;
  toolCalls: ToolCallDelta[];
}

/** One streaming SSE chunk -> its incremental content + tool-call deltas (no `as`). */
function parseStreamChoice(payload: string): StreamChoice {
  const empty: StreamChoice = { content: "", toolCalls: [] };
  let parsed: unknown;
  try {
    parsed = JSON.parse(payload);
  } catch {
    return empty;
  }
  if (!isRecord(parsed) || !Array.isArray(parsed.choices)) {
    return empty;
  }
  const choice: unknown = parsed.choices[0];
  if (!isRecord(choice) || !isRecord(choice.delta)) {
    return empty;
  }
  const delta = choice.delta;
  const toolCalls: ToolCallDelta[] = [];
  if (Array.isArray(delta.tool_calls)) {
    for (const raw of delta.tool_calls) {
      if (!isRecord(raw)) {
        continue;
      }
      const index = typeof raw.index === "number" ? raw.index : 0;
      let name = "";
      let argChunk = "";
      if (isRecord(raw.function)) {
        name = readString(raw.function.name);
        argChunk = readString(raw.function.arguments);
      }
      toolCalls.push({ index, id: readString(raw.id), name, argChunk });
    }
  }
  return { content: readString(delta.content), toolCalls };
}

/**
 * Streaming reasoning turn: `onDelta` fires for each incremental content chunk so
 * the caller can start speaking the first sentence before the rest generates.
 * When `tools` are supplied the model may instead (or also) emit tool calls, which
 * are reassembled from the stream and returned alongside the text for the voice
 * agent's tool loop. Lets the loop hit first-audio in well under a second.
 */
export async function reasonStream(
  messages: ChatMessage[],
  onDelta: (delta: string) => void,
  signal?: AbortSignal,
  model: string = VOICE_MODEL,
  tools?: unknown[],
): Promise<ReasonResult> {
  const payload: Record<string, unknown> = {
    model,
    messages,
    temperature: 0.4,
    max_tokens: 200,
    stream: true,
  };
  if (Array.isArray(tools) && tools.length > 0) {
    payload.tools = tools;
    payload.tool_choice = "auto";
  }
  const response = await fetch(`${LLM}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    signal,
    body: JSON.stringify(payload),
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
  const callsByIndex = new Map<number, ToolCall>();
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
      const choice = parseStreamChoice(payload);
      if (choice.content.length > 0) {
        full += choice.content;
        onDelta(choice.content);
      }
      for (const tc of choice.toolCalls) {
        const existing = callsByIndex.get(tc.index);
        if (existing === undefined) {
          callsByIndex.set(tc.index, { id: tc.id, name: tc.name, arguments: tc.argChunk });
        } else {
          if (existing.id.length === 0 && tc.id.length > 0) existing.id = tc.id;
          if (existing.name.length === 0 && tc.name.length > 0) existing.name = tc.name;
          existing.arguments += tc.argChunk;
        }
      }
    }
  }
  const ordered = [...callsByIndex.entries()].sort((a, b) => a[0] - b[0]);
  return { text: full.trim(), toolCalls: ordered.map((entry) => entry[1]) };
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
