import {
  Activity,
  CircleDot,
  GitBranch,
  History,
  Network,
  ShieldAlert,
  type LucideIcon,
} from "lucide-react";
import type {
  ApprovalRequest,
  AttentionPacket,
  Health,
  MemoryProposal,
  ReplayEvent,
  Risk,
  SystemNode,
  VoiceState,
  VoiceTextThread,
  WorkItem,
} from "./types";

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

export function ReplayView({ replayEvents }: { replayEvents: ReplayEvent[] }) {
  return (
    <section className="list-panel">
      <PanelHeader icon={History} title="Replay Ledger" meta="JPCM events reconstruct state" />
      <div className="timeline">
        {replayEvents.length === 0 && <EmptyRow label="No replay events" />}
        {replayEvents.map((event) => (
          <article className="timeline-item" key={event.sequence}>
            <span className="sequence">{event.sequence}</span>
            <div>
              <strong>{event.family}</strong>
              <span>{event.subject}</span>
            </div>
            <span>{event.producer}</span>
            <time>{event.timestamp}</time>
          </article>
        ))}
      </div>
    </section>
  );
}

export function ApprovalsView({ approvalRequests }: { approvalRequests: ApprovalRequest[] }) {
  const telegramChallenges = approvalRequests.filter((approval) => approval.channel === "telegram");
  const voiceBound = approvalRequests.filter((approval) => approval.voiceThreadId).length;

  return (
    <div className="approval-backplane">
      <section className="approval-hero">
        <div>
          <p className="eyebrow">Telegram backplane</p>
          <h3>
            {telegramChallenges.length} Telegram challenges, {voiceBound} voice/text confirmations, 1 approval lineage.
          </h3>
          <p>
            The approval surface exposes the token fingerprint, user/chat binding, voice or text confirmation, and work-order lineage instead of hiding it behind a single pending badge.
          </p>
        </div>
        <div className="approval-dial" aria-label={`${voiceBound} bound confirmations`}>
          <strong>{voiceBound}</strong>
          <span>bound confirmations</span>
        </div>
      </section>

      <section className="card-grid approval-grid">
        {approvalRequests.length === 0 && <EmptyCard label="No approvals pending" />}
        {approvalRequests.map((approval) => (
          <article className={`approval-card approval-card-${approval.channel}`} key={approval.id}>
            <div className="system-card-head">
              <strong>{approval.id}</strong>
              <span className={classForRisk(approval.risk)}>{approval.state}</span>
            </div>
            <h3>{approval.decision}</h3>
            <p>{approval.reason}</p>
            <div className="chip-list approval-binding">
              <span className="chip">
                <strong>Approver</strong>
                <small>{approval.approver}</small>
              </span>
              <span className="chip">
                <strong>Token</strong>
                <small>{approval.tokenHash}</small>
              </span>
              <span className="chip">
                <strong>User</strong>
                <small>{approval.targetUserId ?? "unbound"}</small>
              </span>
              <span className="chip">
                <strong>Chat</strong>
                <small>{approval.targetChatId ?? "unbound"}</small>
              </span>
            </div>
            <dl className="approval-meta">
              <div>
                <dt>Work</dt>
                <dd>{approval.workOrderId}</dd>
              </div>
              <div>
                <dt>Task</dt>
                <dd>{approval.currentTask}</dd>
              </div>
              <div>
                <dt>Branch</dt>
                <dd>{approval.branch}</dd>
              </div>
              <div>
                <dt>Pool</dt>
                <dd>{approval.pool}</dd>
              </div>
            </dl>
            <section className="approval-lineage">
              <span className="eyebrow">Lineage</span>
              <div className="chip-list">
                {approval.lineage.map((step) => (
                  <span className="chip" key={step}>
                    {step}
                  </span>
                ))}
              </div>
            </section>
            <section className="approval-evidence">
              <span className="eyebrow">Voice / text</span>
              <p>{approval.voiceTranscript ?? "No voice transcript observed."}</p>
              <div className="chip-list">
                <span className="chip chip-confirmation">
                  <strong>Confirmation</strong>
                  <small>{approval.voiceConfirmationPhrase ?? "none"}</small>
                </span>
                <span className="chip">
                  <strong>Placement</strong>
                  <small>{approval.placement}</small>
                </span>
                <span className="chip">
                  <strong>State</strong>
                  <small>{approval.voiceThreadState ?? "unobserved"}</small>
                </span>
              </div>
            </section>
            <span className="expires">{approval.expires === "expired" ? "Expired" : `Expires in ${approval.expires}`}</span>
          </article>
        ))}
      </section>
    </div>
  );
}

export function AttentionPacketCard({ packet }: { packet: AttentionPacket }) {
  return (
    <article className={`attention-card attention-card-${packet.attentionLevel}`}>
      <div className="attention-card-head">
        <div>
          <strong>{packet.id}</strong>
          <span>
            {packet.workOrderId} / {packet.modality}
          </span>
        </div>
        <div className="attention-flags">
          <span className={`pill attention-level attention-level-${packet.attentionLevel}`}>{packet.attentionLevel}</span>
          {packet.decisionNeeded && <span className="pill attention-decision">decision</span>}
        </div>
      </div>

      <h3>{packet.summary}</h3>
      <span className="eyebrow">Why now</span>
      <p className="why-now">{packet.whyNow}</p>

      <div className="attention-body">
        <div className="attention-recommendation">
          <span className="eyebrow">Recommended</span>
          <strong>{packet.recommendation}</strong>
        </div>

        <div className="attention-delta">
          <span className="delta-pill">
            {packet.riskDelta.from} to {packet.riskDelta.to}
          </span>
          <p>{packet.riskDelta.note}</p>
          <span className="expires">Expires in {packet.expires}</span>
        </div>
      </div>

      <section className="attention-alternatives">
        <span className="eyebrow">Alternatives</span>
        <div className="chip-list">
          {packet.alternatives.map((alternative) => (
            <span className={`chip chip-risk-${alternative.risk}`} key={alternative.id}>
              <strong>{alternative.label}</strong>
              <small>{alternative.effect}</small>
            </span>
          ))}
        </div>
      </section>

      <details className="packet-drilldown">
        <summary>Drill-down</summary>
        {packet.incident && (
          <div className="incident-callout">
            <span className={`pill risk-${packet.incident.severity}`}>{packet.incident.severity}</span>
            <strong>{packet.incident.title}</strong>
            <p>{packet.incident.summary}</p>
            <small>{packet.incident.quarantine}</small>
            <div className="drilldown-links">
              {packet.incident.drilldown.map((ref) => (
                <span className="drilldown-link" key={ref}>
                  {ref}
                </span>
              ))}
            </div>
          </div>
        )}
        <div className="drilldown-links">
          {packet.drilldown.map((ref) => (
            <span className="drilldown-link" key={`${ref.label}:${ref.target}`}>
              <strong>{ref.label}</strong>
              <small>{ref.target}</small>
            </span>
          ))}
        </div>
      </details>
    </article>
  );
}

export function SystemIncidentCard({ incident }: { incident: NonNullable<SystemNode["incident"]> }) {
  return (
    <details className="system-incident" open={incident.title.includes("Bridge")}>
      <summary>Incident drill-down</summary>
      <p>{incident.summary}</p>
      <strong>{incident.quarantine}</strong>
      <div className="drilldown-links">
        {incident.drilldown.map((ref) => (
          <span className="drilldown-link" key={ref}>
            {ref}
          </span>
        ))}
      </div>
    </details>
  );
}

export function MemoryIncidentCard({ incident }: { incident: NonNullable<MemoryProposal["incident"]> }) {
  return (
    <details className="memory-incident" open>
      <summary>Incident / quarantine</summary>
      <p>{incident.summary}</p>
      <strong>{incident.quarantine}</strong>
      <div className="drilldown-links">
        {incident.drilldown.map((ref) => (
          <span className="drilldown-link" key={ref}>
            {ref}
          </span>
        ))}
      </div>
    </details>
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

export function MetricCard({ label, value, tone, detail }: { label: string; value: string; tone: string; detail: string }) {
  return (
    <article className={`metric metric-${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
      <p>{detail}</p>
    </article>
  );
}

export function PanelHeader({ icon: Icon, title, meta }: { icon: LucideIcon; title: string; meta: string }) {
  return (
    <div className="panel-header">
      <div>
        <Icon size={18} aria-hidden="true" />
        <strong>{title}</strong>
      </div>
      <span>{meta}</span>
    </div>
  );
}

export function WorkRow({ item }: { item: WorkItem }) {
  return (
    <article className="row work-row">
      <div>
        <strong>{item.title}</strong>
        <span>
          {item.id} by {item.owner}
        </span>
      </div>
      <span className={classForRisk(item.risk)}>{item.risk}</span>
      <span>{item.state}</span>
      <code>{item.lease}</code>
      <span>{item.evidence} proofs</span>
      <span>{item.updated}</span>
    </article>
  );
}

export function EmptyRow({ label }: { label: string }) {
  return <article className="row">{label}</article>;
}

export function EmptyCard({ label }: { label: string }) {
  return (
    <article className="system-card">
      <strong>{label}</strong>
      <p>JMCP has no live records for this view.</p>
    </article>
  );
}

function classForRisk(risk: Risk) {
  return `pill risk-${risk}`;
}

export function classForHealth(health: Health) {
  return `status status-${health}`;
}

function normalizeVoiceState(state: VoiceState | "draft") {
  return state === "draft" ? "transcribed" : state;
}

export function riskRank(risk: Risk) {
  if (risk === "high") {
    return 3;
  }
  if (risk === "medium") {
    return 2;
  }
  return 1;
}
