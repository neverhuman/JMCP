import { useCallback, useEffect, useRef, useState } from "react";
import {
  VOICE_MODEL,
  micSupported,
  reasonStream,
  synthesize,
  transcribe,
  type ChatMessage,
  type ToolCallFunction,
} from "../lib/speechClient";
import { VOICE_TOOL_SPECS, executeVoiceTool } from "../lib/voiceTools";

// Always-listening, privacy-first voice assistant. The mic runs continuously in
// the browser; a lightweight energy VAD segments speech; each utterance is
// transcribed on the LOCAL ASR sidecar and handled as a command while the widget
// is active. No audio or text leaves the machine. Barge-in cancels stale
// reasoning, queued TTS, and current playback the moment you start talking again.

export type VoiceState =
  | "off"
  | "listening"
  | "armed" // retained for compatibility with older wake-word UI states
  | "transcribing"
  | "thinking"
  | "speaking"
  | "error";

export interface VoiceAssistantApi {
  state: VoiceState;
  supported: boolean;
  transcript: string;
  reply: string;
  error: string | null;
  wakeWords: string[];
  start: () => Promise<void>;
  stop: () => void;
}

const WAKE_WORDS = ["hey jmcp", "hey jim cp", "jmcp", "computer"];
const RMS_THRESHOLD = 0.018; // speech vs silence
const SILENCE_MS = 350; // trailing silence that ends an utterance (snappy turn-taking)
const FIRST_CHUNK_CHARS = 28; // flush the opening phrase fast for low time-to-first-word
const MIN_SPEECH_MS = 175; // ignore blips
const SYSTEM_PROMPT =
  "You are JMCP, a concise local voice assistant running on the operator's own machine. " +
  "You can call tools to read JMCP status and to take actions. Keep spoken answers to one " +
  "or two short sentences. For any tool that CHANGES state (submitting or starting work), " +
  "first say what you will do and ask the operator to confirm out loud; only call it with " +
  "confirmed=true after they agree. Do not read raw JSON or long ids aloud unless asked.";
const MAX_TOOL_HOPS = 4; // cap tool round-trips per turn so a loop can't run away
const PREFERRED_AUDIO_TYPES = [
  "audio/webm;codecs=opus",
  "audio/ogg;codecs=opus",
  "audio/webm",
  "audio/ogg",
];

type QueuedSpeech = {
  audio: Promise<Blob | null>;
  signal?: AbortSignal;
};

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isAbortError(error: unknown): boolean {
  return (
    typeof DOMException !== "undefined" &&
    error instanceof DOMException &&
    error.name === "AbortError"
  );
}

function preferredAudioType(): string | undefined {
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

export function useVoiceAssistant(): VoiceAssistantApi {
  const supported = micSupported();
  const [state, setState] = useState<VoiceState>("off");
  const [transcript, setTranscript] = useState("");
  const [reply, setReply] = useState("");
  const [error, setError] = useState<string | null>(null);

  const stateRef = useRef<VoiceState>("off");
  const streamRef = useRef<MediaStream | null>(null);
  const ctxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const timerRef = useRef<number | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const speechQueueRef = useRef<QueuedSpeech[]>([]);
  const drainingRef = useRef<boolean>(false);
  const historyRef = useRef<ChatMessage[]>([{ role: "system", content: SYSTEM_PROMPT }]);
  const turnAbortRef = useRef<AbortController | null>(null);
  // VAD bookkeeping
  const speakingSinceRef = useRef<number>(0);
  const silenceSinceRef = useRef<number>(0);
  const capturingRef = useRef<boolean>(false);

  const setBoth = useCallback((next: VoiceState) => {
    stateRef.current = next;
    setState(next);
  }, []);

  const abortActiveWork = useCallback(() => {
    turnAbortRef.current?.abort();
    turnAbortRef.current = null;
  }, []);

  const cancelPlayback = useCallback(() => {
    abortActiveWork();
    speechQueueRef.current = [];
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = "";
      audioRef.current = null;
    }
  }, [abortActiveWork]);

  // Synthesize + play one sentence, resolving when it finishes.
  const playOne = useCallback(async (queued: QueuedSpeech): Promise<void> => {
    const ogg = await queued.audio;
    if (ogg === null || queued.signal?.aborted || stateRef.current === "off") {
      return;
    }
    const url = URL.createObjectURL(ogg);
    const audio = new Audio(url);
    audioRef.current = audio;
    await new Promise<void>((resolve) => {
      const finish = () => {
        queued.signal?.removeEventListener("abort", finish);
        resolve();
      };
      if (queued.signal?.aborted) {
        finish();
        return;
      }
      queued.signal?.addEventListener("abort", finish, { once: true });
      audio.onended = () => finish();
      audio.onerror = () => finish();
      void audio.play().catch(() => finish());
    });
    URL.revokeObjectURL(url);
    if (audioRef.current === audio) audioRef.current = null;
  }, []);

  // Drain the sentence queue sequentially so speech plays in order while later
  // sentences are still being synthesized.
  const drainQueue = useCallback(async () => {
    if (drainingRef.current) return;
    drainingRef.current = true;
    setBoth("speaking");
    while (speechQueueRef.current.length > 0 && stateRef.current !== "off") {
      const next = speechQueueRef.current.shift();
      if (next === undefined) continue;
      try {
        await playOne(next);
      } catch (err) {
        if (!isAbortError(err)) {
          /* ignore a single failed sentence */
        }
      }
    }
    drainingRef.current = false;
    if (stateRef.current === "speaking") {
      setBoth("listening");
    }
  }, [playOne, setBoth]);

  const enqueueSpeech = useCallback((text: string, signal?: AbortSignal) => {
    const clean = text.trim();
    if (clean.length === 0) return;
    const audio = synthesize(clean, signal).then(
      (blob) => blob,
      () => null,
    );
    speechQueueRef.current.push({ audio, signal });
    void drainQueue();
  }, [drainQueue]);

  // Stream the reply and speak each complete sentence as soon as it lands, so
  // first audio plays within a second instead of after the whole reply.
  const runCommand = useCallback(async (command: string) => {
    abortActiveWork();
    const turnAbort = new AbortController();
    turnAbortRef.current = turnAbort;
    setBoth("thinking");
    historyRef.current.push({ role: "user", content: command });
    speechQueueRef.current = [];
    let pending = "";
    let firstChunk = true;
    // Speak in clause-sized chunks (split on , . ! ? : ;) so the opening phrase
    // is voiced the instant it streams in — minimal time-to-first-word. When a
    // clause runs long with no punctuation, break at the last word boundary.
    const flushChunks = (force: boolean) => {
      const chunks = pending.match(/[^,.!?:;]*[,.!?:;]+\s*/g);
      if (chunks !== null) {
        let consumed = 0;
        for (const chunk of chunks) {
          enqueueSpeech(chunk, turnAbort.signal);
          consumed += chunk.length;
        }
        pending = pending.slice(consumed);
        firstChunk = false;
      }
      if (force) {
        const lastSpace = pending.lastIndexOf(" ");
        if (lastSpace > 12) {
          enqueueSpeech(pending.slice(0, lastSpace), turnAbort.signal);
          pending = pending.slice(lastSpace + 1);
          firstChunk = false;
        }
      }
    };
    const onDelta = (delta: string) => {
      pending += delta;
      if (/[,.!?:;]/.test(pending)) {
        flushChunks(false);
      } else if (pending.length > (firstChunk ? FIRST_CHUNK_CHARS : 120)) {
        flushChunks(true);
      }
    };
    try {
      let lastText = "";
      // Tool loop: the model may call JMCP tools before it speaks the final answer.
      // Each hop streams content (spoken live) and/or tool calls; we run the tools,
      // feed the results back, and continue until it answers with no further calls.
      for (let hop = 0; hop < MAX_TOOL_HOPS; hop++) {
        pending = "";
        firstChunk = true;
        const result = await reasonStream(
          historyRef.current,
          onDelta,
          turnAbort.signal,
          VOICE_MODEL,
          VOICE_TOOL_SPECS,
        );
        enqueueSpeech(pending, turnAbort.signal);
        pending = "";
        lastText = result.text;
        if (result.toolCalls.length === 0) {
          historyRef.current.push({ role: "assistant", content: result.text });
          break;
        }
        // Record the assistant's tool-call turn, run each tool, feed results back.
        const toolCalls: ToolCallFunction[] = result.toolCalls.map((call) => ({
          id: call.id,
          type: "function",
          function: { name: call.name, arguments: call.arguments },
        }));
        historyRef.current.push({ role: "assistant", content: result.text, tool_calls: toolCalls });
        setBoth("thinking");
        for (const call of result.toolCalls) {
          const output = await executeVoiceTool(call.name, call.arguments, turnAbort.signal);
          historyRef.current.push({ role: "tool", tool_call_id: call.id, content: output });
        }
      }
      if (historyRef.current.length > 13) {
        historyRef.current = [
          historyRef.current[0],
          ...historyRef.current.slice(historyRef.current.length - 12),
        ];
      }
      setReply(lastText);
      if (lastText.length === 0) enqueueSpeech("Sorry, I did not catch that.", turnAbort.signal);
    } catch (err) {
      if (turnAbort.signal.aborted || isAbortError(err)) {
        return;
      }
      setError(err instanceof Error ? err.message : "reasoning failed");
      setBoth("listening");
    }
  }, [abortActiveWork, enqueueSpeech, setBoth]);

  const handleUtterance = useCallback(async (blob: Blob) => {
    if (stateRef.current === "off") return;
    setBoth("transcribing");
    let heard = "";
    try {
      const result = await transcribe(blob);
      heard = result.text;
    } catch (err) {
      setError(err instanceof Error ? err.message : "transcription failed");
      setBoth("listening");
      return;
    }
    // Continuous conversation: no wake word — every spoken turn is acted on.
    if (!heard) {
      setBoth("listening");
      return;
    }
    setTranscript(heard);
    await runCommand(heard);
  }, [runCommand, setBoth]);

  const beginCapture = useCallback(() => {
    const stream = streamRef.current;
    if (!stream) return;
    // barge-in: talking over the assistant cancels playback and stale LLM/TTS work
    if (stateRef.current === "speaking" || stateRef.current === "thinking") {
      cancelPlayback();
      setBoth("listening");
    }
    chunksRef.current = [];
    const audioType = preferredAudioType();
    const recorder =
      audioType === undefined ? new MediaRecorder(stream) : new MediaRecorder(stream, { mimeType: audioType });
    recorderRef.current = recorder;
    recorder.ondataavailable = (event) => {
      if (event.data.size > 0) chunksRef.current.push(event.data);
    };
    recorder.onstop = () => {
      const blob = new Blob(chunksRef.current, { type: recorder.mimeType || "audio/webm" });
      chunksRef.current = [];
      void handleUtterance(blob);
    };
    recorder.start();
    capturingRef.current = true;
  }, [cancelPlayback, handleUtterance, setBoth]);

  const endCapture = useCallback((durationMs: number) => {
    const recorder = recorderRef.current;
    capturingRef.current = false;
    if (!recorder) return;
    if (durationMs < MIN_SPEECH_MS) {
      recorder.onstop = null;
      try {
        recorder.stop();
      } catch {
        /* ignore */
      }
      recorderRef.current = null;
      return;
    }
    try {
      recorder.stop();
    } catch {
      /* ignore */
    }
    recorderRef.current = null;
  }, []);

  const stop = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
    cancelPlayback();
    if (recorderRef.current) {
      recorderRef.current.onstop = null;
      try {
        recorderRef.current.stop();
      } catch {
        /* ignore */
      }
      recorderRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }
    if (ctxRef.current) {
      void ctxRef.current.close();
      ctxRef.current = null;
    }
    analyserRef.current = null;
    capturingRef.current = false;
    setBoth("off");
  }, [cancelPlayback, setBoth]);

  const start = useCallback(async () => {
    if (!supported || stateRef.current !== "off") return;
    setError(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
      });
      streamRef.current = stream;
      const ctx = new AudioContext();
      ctxRef.current = ctx;
      const source = ctx.createMediaStreamSource(stream);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 1024;
      source.connect(analyser);
      analyserRef.current = analyser;
      const buffer = new Float32Array(analyser.fftSize);
      setBoth("listening");

      timerRef.current = window.setInterval(() => {
        const node = analyserRef.current;
        if (!node) return;
        node.getFloatTimeDomainData(buffer);
        let sum = 0;
        for (let i = 0; i < buffer.length; i += 1) sum += buffer[i] * buffer[i];
        const rms = Math.sqrt(sum / buffer.length);
        const now = Date.now();

        if (rms > RMS_THRESHOLD) {
          // Echo cancellation handles assistant playback; live speech here is a
          // barge-in and cancels stale reasoning/playback work.
          if (
            !capturingRef.current &&
            (stateRef.current === "listening" ||
              stateRef.current === "speaking" ||
              stateRef.current === "thinking")
          ) {
            speakingSinceRef.current = now;
            beginCapture();
          }
          silenceSinceRef.current = now;
        } else if (capturingRef.current) {
          if (now - silenceSinceRef.current > SILENCE_MS) {
            endCapture(now - speakingSinceRef.current);
          }
        }
      }, 50);
    } catch (err) {
      setError(err instanceof Error ? err.message : "microphone permission denied");
      setBoth("error");
    }
  }, [beginCapture, endCapture, setBoth, supported]);

  useEffect(() => stop, [stop]);

  return {
    state,
    supported,
    transcript,
    reply,
    error,
    wakeWords: WAKE_WORDS,
    start,
    stop,
  };
}
