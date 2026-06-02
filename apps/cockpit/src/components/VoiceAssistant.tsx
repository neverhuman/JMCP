import { Mic, MicOff, Loader2, Volume2, Brain, Ear } from "lucide-react";
import { useVoiceAssistant, type VoiceState } from "../hooks/useVoiceAssistant";
import "../voice-assistant.css";

// A floating, always-listening voice widget for the local on-box assistant.
// Mounted only on the standalone cockpit (main.tsx), so it never affects the
// shared <App/> that the web proof-host screenshots.

const LABELS: Record<VoiceState, string> = {
  off: "Voice off",
  listening: "Listening for “hey JMCP”…",
  armed: "I’m listening — say your command",
  transcribing: "Transcribing…",
  thinking: "Thinking…",
  speaking: "Speaking…",
  error: "Microphone error",
};

function StateIcon({ state }: { state: VoiceState }) {
  if (state === "listening") return <Ear size={16} aria-hidden />;
  if (state === "armed") return <Mic size={16} aria-hidden />;
  if (state === "transcribing") return <Loader2 size={16} className="spin" aria-hidden />;
  if (state === "thinking") return <Brain size={16} aria-hidden />;
  if (state === "speaking") return <Volume2 size={16} aria-hidden />;
  return <MicOff size={16} aria-hidden />;
}

export default function VoiceAssistant() {
  const voice = useVoiceAssistant();
  const active = voice.state !== "off" && voice.state !== "error";

  if (!voice.supported) {
    // No mic (SSR / test / insecure origin): render a quiet disabled affordance.
    return (
      <aside className="voice-assistant" data-voice-state="off">
        <span className="voice-toggle" aria-disabled="true">
          <MicOff size={16} aria-hidden />
          <span>Voice needs a microphone</span>
        </span>
      </aside>
    );
  }

  return (
    <aside className="voice-assistant" data-voice-state={voice.state} aria-live="polite">
      <button
        type="button"
        className={active ? "voice-toggle active" : "voice-toggle"}
        aria-pressed={active}
        aria-label={active ? "Stop voice assistant" : "Start voice assistant"}
        onClick={() => (active ? voice.stop() : void voice.start())}
      >
        <StateIcon state={voice.state} />
        <span>{LABELS[voice.state]}</span>
      </button>

      {(voice.transcript || voice.reply || voice.error) && (
        <div className="voice-log">
          {voice.transcript && (
            <p className="voice-heard">
              <span className="voice-tag">heard</span> {voice.transcript}
            </p>
          )}
          {voice.reply && (
            <p className="voice-reply">
              <span className="voice-tag">JMCP</span> {voice.reply}
            </p>
          )}
          {voice.error && <p className="voice-error">{voice.error}</p>}
        </div>
      )}
    </aside>
  );
}
