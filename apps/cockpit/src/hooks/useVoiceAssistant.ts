import { useCallback, useEffect, useRef, useState } from "react";
import {
  micSupported,
  reason,
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
const SILENCE_MS = 800; // trailing silence that ends an utterance
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
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = "";
      audioRef.current = null;
    }
  }, []);

  const speak = useCallback(async (text: string) => {
    try {
      const ogg = await synthesize(text);
      const url = URL.createObjectURL(ogg);
      const audio = new Audio(url);
      audioRef.current = audio;
      setBoth("speaking");
      audio.onended = () => {
        URL.revokeObjectURL(url);
        if (audioRef.current === audio) audioRef.current = null;
        if (stateRef.current === "speaking") setBoth("listening");
      };
      await audio.play();
    } catch {
      setBoth("listening");
    }
  }, [setBoth]);

  const runCommand = useCallback(async (command: string) => {
    setBoth("thinking");
    historyRef.current.push({ role: "user", content: command });
    try {
      const answer = await reason(historyRef.current);
      historyRef.current.push({ role: "assistant", content: answer });
      // keep history bounded (system + last ~6 turns)
      if (historyRef.current.length > 13) {
        historyRef.current = [
          historyRef.current[0],
          ...historyRef.current.slice(historyRef.current.length - 12),
        ];
      }
      setReply(answer);
      await speak(answer || "Sorry, I did not catch that.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "reasoning failed");
      setBoth("listening");
    }
  }, [setBoth, speak]);

  const handleUtterance = useCallback(async (blob: Blob) => {
    if (stateRef.current === "off") return;
    const wasArmed = stateRef.current === "armed";
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
    if (!heard) {
      setBoth(wasArmed ? "armed" : "listening");
      return;
    }
    setTranscript(heard);

    if (wasArmed) {
      await runCommand(heard);
      return;
    }
    const { triggered, command } = stripWakeWord(heard);
    if (!triggered) {
      setBoth("listening");
      return;
    }
    if (command.length > 0) {
      await runCommand(command);
    } else {
      setBoth("armed"); // wake word alone -> the next utterance is the command
    }
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
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
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
          if (!capturingRef.current) {
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
