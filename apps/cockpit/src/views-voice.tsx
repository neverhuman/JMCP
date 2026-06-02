import { Activity, CircleDot, type LucideIcon } from "lucide-react";
import type { VoiceState, VoiceTextThread } from "./types";
import { EmptyRow, PanelHeader } from "./views-primitives";

export function VoiceTextView({ voiceThreads }: { voiceThreads: VoiceTextThread[] }) {
  const voiceCount = voiceThreads.filter((thread) => thread.channel === "voice").length;
  const textCount = voiceThreads.filter((thread) => thread.channel === "text").length;
  const confirmations = voiceThreads.filter((thread) => thread.requiresResponse).length;

  return (
    <div className="voice-text-board">
      <section className="voice-text-hero">
        <div>
          <p className="eyebrow">Voice/Text</p>
          <h3>
            {voiceCount} voice turns, {textCount} text threads, {confirmations} confirmations.
          </h3>
          <p>
            Text and voice share the same task semantics here. The cockpit shows transcripts, intent normalization, and confirmation prompts without turning the surface into a chat log.
          </p>
        </div>
        <div className="voice-text-dial" aria-label={`${confirmations} turns requiring response`}>
          <strong>{confirmations}</strong>
          <span>need response</span>
        </div>
      </section>

      <div className="voice-text-grid">
        <ChannelColumn title="Voice Turns" icon={Activity} voiceThreads={voiceThreads.filter((thread) => thread.channel === "voice")} />
        <ChannelColumn title="Text Threads" icon={CircleDot} voiceThreads={voiceThreads.filter((thread) => thread.channel === "text")} />
      </div>
    </div>
  );
}

function ChannelColumn({ title, icon: Icon, voiceThreads }: { title: string; icon: LucideIcon; voiceThreads: VoiceTextThread[] }) {
  return (
    <section className="channel-column">
      <PanelHeader icon={Icon} title={title} meta={`${voiceThreads.length} threads`} />
      <div className="channel-stack">
        {voiceThreads.length === 0 && <EmptyRow label="No threads" />}
        {voiceThreads.map((thread) => (
          <article className={`voice-card voice-card-${thread.channel}`} key={thread.id}>
            <div className="voice-card-head">
              <div>
                <strong>{thread.title}</strong>
                <span>
                  {thread.id} from {thread.speaker}
                </span>
              </div>
              <span className={`pill voice-state voice-state-${normalizeVoiceState(thread.state)}`}>{thread.state}</span>
            </div>
            <p className="voice-transcript">{thread.transcript}</p>
            <div className="voice-meta">
              <span className="eyebrow">Intent</span>
              <strong>{thread.intent}</strong>
              <span>{thread.updated}</span>
            </div>
            <dl className="voice-details">
              <div>
                <dt>Confidence</dt>
                <dd>{Math.round(thread.confidence * 100)}%</dd>
              </div>
              <div>
                <dt>Source</dt>
                <dd>{thread.sourceRef}</dd>
              </div>
            </dl>
            <div className="voice-actions">
              {thread.confirmationPhrase && (
                <span className="chip chip-confirmation">
                  <strong>Confirm</strong>
                  <small>{thread.confirmationPhrase}</small>
                </span>
              )}
              {thread.requiresResponse && <span className="pill attention-decision">response required</span>}
            </div>
            <div className="chip-list">
              {thread.decisionOptions.map((option) => (
                <span className="chip" key={option}>
                  {option}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

function normalizeVoiceState(state: VoiceState | "draft") {
  return state === "draft" ? "transcribed" : state;
}
