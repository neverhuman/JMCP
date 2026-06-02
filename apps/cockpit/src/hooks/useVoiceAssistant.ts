import { useCallback, useEffect, useRef, useState } from "react";
import {
  micSupported,
  reasonStream,
  synthesize,
  transcribe,
  type ChatMessage,
} from "../lib/speechClient";

// Always-listening, privacy-first voice assistant. The mic runs continuously in
// the browser; a lightweight energy VAD segments speech; each utterance is
// transcribed on the LOCAL ASR sidecar. While idle we only act when the
// transcript contains the WAKE WORD ("hey jmcp"); the words after it (or the next
// utterance) become the command, which is reasoned by the local LLM and spoken
// back by the local TTS. No audio or text leaves the machine. Barge-in: speaking
// is cancelled the moment you start talking again.

export type VoiceState =
  | "off"
  | "listening" // idle, waiting for the wake word
  | "armed" // heard the wake word, capturing the command
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
const SILENCE_MS = 400; // trailing silence that ends an utterance (snappy turn-taking)
const FIRST_CHUNK_CHARS = 40; // flush the opening phrase fast for low time-to-first-word
const MIN_SPEECH_MS = 250; // ignore blips
const SYSTEM_PROMPT =
  "You are JMCP, a concise local voice assistant running on the operator's own machine. " +
  "Answer in one or two short spoken sentences. Be direct and helpful.";

export function stripWakeWord(text: string): { triggered: boolean; command: string } {
  const lower = text.toLowerCase();
  for (const wake of WAKE_WORDS) {
    const index = lower.indexOf(wake);
    if (index >= 0) {
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
  const speechQueueRef = useRef<string[]>([]);
  const drainingRef = useRef<boolean>(false);
  const historyRef = useRef<ChatMessage[]>([{ role: "system", content: SYSTEM_PROMPT }]);
  // VAD bookkeeping
  const speakingSinceRef = useRef<number>(0);
  const silenceSinceRef = useRef<number>(0);
  const capturingRef = useRef<boolean>(false);

  const setBoth = useCallback((next: VoiceState) => {
    stateRef.current = next;
    setState(next);
  }, []);

  const cancelPlayback = useCallback(() => {
    speechQueueRef.current = [];
    drainingRef.current = false;
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = "";
      audioRef.current = null;
    }
  }, []);

  // Synthesize + play one sentence, resolving when it finishes.
  const playOne = useCallback(async (text: string): Promise<void> => {
    const ogg = await synthesize(text);
    const url = URL.createObjectURL(ogg);
    const audio = new Audio(url);
    audioRef.current = audio;
    await new Promise<void>((resolve) => {
      audio.onended = () => resolve();
      audio.onerror = () => resolve();
      void audio.play().catch(() => resolve());
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
      } catch {
        /* ignore a single failed sentence */
      }
    }
    drainingRef.current = false;
    if (stateRef.current === "speaking") setBoth("listening");
  }, [playOne, setBoth]);

  const enqueueSpeech = useCallback((text: string) => {
    const clean = text.trim();
    if (clean.length === 0) return;
    speechQueueRef.current.push(clean);
    void drainQueue();
  }, [drainQueue]);

  // Stream the reply and speak each complete sentence as soon as it lands, so
  // first audio plays within a second instead of after the whole reply.
  const runCommand = useCallback(async (command: string) => {
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
          enqueueSpeech(chunk);
          consumed += chunk.length;
        }
        pending = pending.slice(consumed);
        firstChunk = false;
      }
      if (force) {
        const lastSpace = pending.lastIndexOf(" ");
        if (lastSpace > 12) {
          enqueueSpeech(pending.slice(0, lastSpace));
          pending = pending.slice(lastSpace + 1);
          firstChunk = false;
        }
      }
    };
    try {
      const answer = await reasonStream(historyRef.current, (delta) => {
        pending += delta;
        if (/[,.!?:;]\s/.test(pending)) {
          flushChunks(false);
        } else if (pending.length > (firstChunk ? FIRST_CHUNK_CHARS : 120)) {
          flushChunks(true);
        }
      });
      enqueueSpeech(pending);
      pending = "";
      historyRef.current.push({ role: "assistant", content: answer });
      if (historyRef.current.length > 13) {
        historyRef.current = [
          historyRef.current[0],
          ...historyRef.current.slice(historyRef.current.length - 12),
        ];
      }
      setReply(answer);
      if (answer.length === 0) enqueueSpeech("Sorry, I did not catch that.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "reasoning failed");
      setBoth("listening");
    }
  }, [enqueueSpeech, setBoth]);

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
    // barge-in: talking over the assistant cancels its playback
    if (stateRef.current === "speaking") cancelPlayback();
    chunksRef.current = [];
    const recorder = new MediaRecorder(stream);
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
  }, [cancelPlayback, handleUtterance]);

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
          // Only capture while idle: don't transcribe the assistant's own TTS
          // (half-duplex turn-taking; echo cancellation guards the rest).
          if (!capturingRef.current && stateRef.current === "listening") {
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
