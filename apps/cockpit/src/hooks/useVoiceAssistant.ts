import { useCallback, useEffect, useRef, useState } from "react";
import { synthesize, transcribe, type ChatMessage } from "../lib/speechClient";
import { micSupported, requestMicrophoneStream } from "../lib/microphone";
import {
  MIN_SPEECH_MS,
  RMS_THRESHOLD,
  SILENCE_MS,
  SYSTEM_PROMPT,
  WAKE_WORDS,
  isAbortError,
  preferredAudioType,
  stripWakeWord,
} from "../lib/voiceAssistantConfig";
import { runVoiceTurn } from "../lib/voiceAssistantTurn";
import type { VoiceAssistantApi, VoiceState } from "../lib/voiceAssistantTypes";

export { stripWakeWord };
export type { VoiceAssistantApi, VoiceState };

// Always-listening, privacy-first voice assistant. The mic runs continuously in
// the browser; a lightweight energy VAD segments speech; each utterance is
// transcribed on the LOCAL ASR sidecar and handled as a command while the widget
// is active. No audio or text leaves the machine. Barge-in cancels stale
// reasoning, queued TTS, and current playback the moment you start talking again.

type QueuedSpeech = {
  audio: Promise<Blob | null>;
  signal?: AbortSignal;
};

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
      // Settle back to the mic loop if it's running, else to off (text-test path).
      setBoth(streamRef.current === null ? "off" : "listening");
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
    speechQueueRef.current = [];
    try {
      const lastText = await runVoiceTurn({
        command,
        history: historyRef.current,
        signal: turnAbort.signal,
        enqueueSpeech,
        setThinking: () => setBoth("thinking"),
      });
      setReply(lastText);
      if (lastText.length === 0) enqueueSpeech("Sorry, I did not catch that.", turnAbort.signal);
    } catch (err) {
      if (turnAbort.signal.aborted || isAbortError(err)) {
        return;
      }
      setError(err instanceof Error ? err.message : "reasoning failed");
      setBoth(streamRef.current === null ? "off" : "listening");
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

  // Run a typed command through the same reason -> tools -> speak pipeline as a
  // spoken turn. Lets the agent be tested and used without a microphone (the
  // reply is still synthesized and played through the browser's speakers).
  const sendText = useCallback(async (text: string) => {
    const clean = text.trim();
    if (clean.length === 0) return;
    if (stateRef.current === "transcribing" || stateRef.current === "thinking") return;
    setError(null);
    setTranscript(clean);
    await runCommand(clean);
  }, [runCommand]);

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
      audioType.kind === "browser_default"
        ? new MediaRecorder(stream)
        : new MediaRecorder(stream, { mimeType: audioType.mimeType });
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
    let stream: MediaStream | null = null;
    try {
      stream = await requestMicrophoneStream();
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
      if (stream !== null) {
        stream.getTracks().forEach((track) => track.stop());
      }
      streamRef.current = null;
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
    sendText,
  };
}
