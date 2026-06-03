export const WAKE_WORDS = ["hey jmcp", "hey jim cp", "jmcp", "computer"];
export const RMS_THRESHOLD = 0.018;
export const SILENCE_MS = 350;
export const FIRST_CHUNK_CHARS = 28;
export const MIN_SPEECH_MS = 175;
export const MAX_TOOL_HOPS = 4;
export const SYSTEM_PROMPT =
  "You are JMCP, a concise local voice assistant running on the operator's own machine. " +
  "You can call tools to read JMCP status and to take actions. Keep spoken answers to one " +
  "or two short sentences. For any tool that CHANGES state (submitting or starting work), " +
  "first say what you will do and ask the operator to confirm out loud; only call it with " +
  "confirmed=true after they agree. Do not read raw JSON or long ids aloud unless asked.";

const PREFERRED_AUDIO_TYPES = [
  "audio/webm;codecs=opus",
  "audio/ogg;codecs=opus",
  "audio/webm",
  "audio/ogg",
];

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function isAbortError(error: unknown): boolean {
  return (
    typeof DOMException !== "undefined" &&
    error instanceof DOMException &&
    error.name === "AbortError"
  );
}

export function preferredAudioType(): string | undefined {
  if (
    typeof MediaRecorder === "undefined" ||
    typeof MediaRecorder.isTypeSupported !== "function"
  ) {
    return undefined;
  }
  return PREFERRED_AUDIO_TYPES.find((type) => MediaRecorder.isTypeSupported(type));
}

export function stripWakeWord(text: string): { triggered: boolean; command: string } {
  for (const wake of WAKE_WORDS) {
    const pattern = new RegExp(`(^|\\b)${escapeRegExp(wake)}(?=$|[\\s,.:;-])`, "i");
    const match = pattern.exec(text);
    if (match !== null) {
      const prefix = match[1] ?? "";
      const index = match.index + prefix.length;
      const after = text.slice(index + wake.length).replace(/^[\s,.:;-]+/, "");
      return { triggered: true, command: after.trim() };
    }
  }
  return { triggered: false, command: "" };
}
