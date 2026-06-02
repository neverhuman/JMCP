import {
  Activity,
  Boxes,
  CircleDot,
  FileCheck2,
  GitBranch,
  History,
  Network,
  ShieldAlert,
  type LucideIcon,
} from "lucide-react";
import type {
  ApprovalRequest,
  AttentionPacket,
  EvidenceBundle,
  Health,
  MemoryProposal,
  ReplayEvent,
  Risk,
  SystemNode,
  ToolAsset,
  VoiceState,
  VoiceTextThread,
  WorkItem,
} from "./types";
import type { RuntimeState } from "./runtime";

export function NowView({ runtime }: { runtime: RuntimeState }) {
  const blocked = runtime.workItems.filter((item) => item.state === "blocked").length;
  const decisionPackets = runtime.attentionPackets.filter((packet) => packet.decisionNeeded).length;
  const urgentPackets = runtime.attentionPackets.filter(
    (packet) => packet.attentionLevel === "urgent" || packet.attentionLevel === "incident",
  ).length;
  const voiceConfirmations = runtime.voiceThreads.filter((thread) => thread.requiresResponse).length;
  const activePromotions = runtime.memoryLessons.filter((lesson) => lesson.state === "promoted").length;
  const topRisk = runtime.attentionPackets.reduce((highest, packet) => (riskRank(packet.riskDelta.to) > riskRank(highest) ? packet.riskDelta.to : highest), "low" as Risk);

  return (
    <div className="dashboard-grid now-layout">
      <MetricCard label="Decision packets" value={runtime.attentionPackets.length.toString()} tone="green" detail={`${decisionPackets} packets need explicit decisions`} />
      <MetricCard label="Urgent" value={urgentPackets.toString()} tone="amber" detail={`${voiceConfirmations} voice/text confirmations are waiting`} />
      <MetricCard label="Blocked" value={blocked.toString()} tone="red" detail="adapter authority gap" />
      <MetricCard label="Memory promotions" value={activePromotions.toString()} tone="ink" detail={runtime.usingFixtures ? "fixture data" : "live API"} />

      <section className="attention-inbox wide">
        <div className="attention-inbox-hero">
          <div>
            <p className="eyebrow">Attention inbox</p>
            <h3>
              {runtime.attentionPackets.length} decision packets, {decisionPackets} explicit choices, {urgentPackets} urgent items.
            </h3>
            <p>
              Only decision-worthy packets surface here. Each card carries why-now, alternatives, risk delta, and drill-down links instead of a warning strip.
            </p>
          </div>
          <div className="inbox-dial" aria-label={`Highest risk ${topRisk}`}>
            <strong>{topRisk}</strong>
            <span>highest risk</span>
          </div>
        </div>

        <div className="attention-packet-grid">
          {runtime.attentionPackets.length === 0 && <EmptyCard label="No attention packets" />}
          {runtime.attentionPackets.map((packet) => (
            <AttentionPacketCard key={packet.id} packet={packet} />
          ))}
        </div>
      </section>
    </div>
  );
}

export function WorkView({ workItems }: { workItems: WorkItem[] }) {
  return (
    <section className="list-panel">
      <PanelHeader icon={GitBranch} title="Work Orders" meta="Task state, risk, lease, proof count" />
      <div className="rows">
        {workItems.length === 0 && <EmptyRow label="No work orders" />}
        {workItems.map((item) => (
          <WorkRow key={item.id} item={item} />
        ))}
      </div>
    </section>
  );
}

export function EvidenceView({ evidenceBundles }: { evidenceBundles: EvidenceBundle[] }) {
  return (
    <section className="list-panel">
      <PanelHeader icon={FileCheck2} title="Evidence Bundles" meta="Proof bundles accepted before promotion" />
      <div className="rows">
        {evidenceBundles.length === 0 && <EmptyRow label="No evidence bundles" />}
        {evidenceBundles.map((bundle) => (
          <article className="row" key={bundle.id}>
            <div>
              <strong>{bundle.subject}</strong>
              <span>
                {bundle.id} from {bundle.source}
              </span>
            </div>
            <span className={`pill evidence-${bundle.status}`}>{bundle.status}</span>
            <code>{bundle.hash}</code>
            <span>{bundle.age}</span>
          </article>
        ))}
      </div>
    </section>
  );
}

export function SystemsView({ systems }: { systems: SystemNode[] }) {
  return (
    <section className="card-grid system-grid">
      {systems.length === 0 && <EmptyCard label="No systems reported" />}
      {systems.map((system) => (
        <article className="system-card" key={system.name}>
          <div className="system-card-head">
            <strong>{system.name}</strong>
            <span className={classForHealth(system.health)}>{system.health}</span>
          </div>
          <p>{system.role}</p>
          <dl>
            <div>
              <dt>JCP</dt>
              <dd>{system.jcp}</dd>
            </div>
            <div>
              <dt>Latency</dt>
              <dd>{system.latency}</dd>
            </div>
          </dl>
          {system.incident && <SystemIncidentCard incident={system.incident} />}
        </article>
      ))}
    </section>
  );
}

export function UniverseView({ runtime }: { runtime: RuntimeState }) {
  const canonicalRepos = ["Jeryu", "Jekko", "Jankurai"];
  const bootstrap = runtime.universe.bootstrapTui;
  const repoOrder = new Map(canonicalRepos.map((repo, index) => [repo, index]));
  const repoScores = bootstrap.repoScores
    .filter((repo) => canonicalRepos.includes(repo.repo))
    .sort((left, right) => (repoOrder.get(left.repo) ?? 99) - (repoOrder.get(right.repo) ?? 99));
  const placements = bootstrap.placements
    .filter((placement) => canonicalRepos.includes(placement.repo))
    .sort((left, right) => (repoOrder.get(left.repo) ?? 99) - (repoOrder.get(right.repo) ?? 99));
  const activeRepos = bootstrap.activeRepos.filter((repo) => canonicalRepos.includes(repo.repo));
  const liveTools = runtime.universe.ecosystem.tools;
  const observedCoverage = bootstrap.observedCoverage;
  const ecosystemCoverage = runtime.universe.ecosystem.live ? 100 : 0;
  const degradedReason =
    bootstrap.degradedReason ??
    runtime.universe.ecosystem.degradedReason ??
    "All observed slices are live.";

  return (
    <div className="universe-board">
      <section className="universe-hero">
        <div>
          <p className="eyebrow">Universe</p>
          <h3>
            {observedCoverage}% observed coverage, {activeRepos.length} active repos, {liveTools.length} graph nodes.
          </h3>
          <p>{degradedReason}</p>
          <div className="chip-list">
            {activeRepos.length === 0 && <span className="chip">No active repos observed</span>}
            {activeRepos.map((repo) => (
              <span className="chip" key={repo.repo}>
                <strong>{repo.repo}</strong>
                <small>
                  {repo.score}% observed score, {repo.toolCount} tools
                </small>
              </span>
            ))}
          </div>
        </div>
        <div className="universe-dial" aria-label={`${observedCoverage}% observed coverage and ${ecosystemCoverage}% ecosystem coverage`}>
          <strong>{observedCoverage}</strong>
          <span>observed</span>
        </div>
      </section>

      <section className="card-grid universe-scorecards">
        {repoScores.length === 0 && <EmptyCard label="No repo scores observed" />}
        {repoScores.map((repo) => (
          <article className={`universe-card universe-card-${repo.health}`} key={repo.repo}>
            <div className="system-card-head">
              <strong>{repo.repo}</strong>
              <span className={classForHealth(repo.health)}>{repo.health}</span>
            </div>
            <h3>{repo.score}% observed score</h3>
            <p>{repo.degradedReason ?? "All bootstrap fields observed."}</p>
            <dl className="universe-meta">
              <div>
                <dt>Coverage</dt>
                <dd>{repo.coverage}%</dd>
              </div>
              <div>
                <dt>Tools</dt>
                <dd>{repo.toolCount}</dd>
              </div>
              <div>
                <dt>Task</dt>
                <dd>{repo.currentTask}</dd>
              </div>
              <div>
                <dt>Branch</dt>
                <dd>{repo.branch}</dd>
              </div>
            </dl>
            <div className="meter" aria-label={`${repo.coverage}% coverage`}>
              <span style={{ width: `${repo.coverage}%` }} />
            </div>
          </article>
        ))}
      </section>

      <section className="universe-grid">
        <section className="tool-map universe-graph">
          <PanelHeader icon={Network} title="Live Graph" meta={`${liveTools.length} tools across ${new Set(liveTools.map((tool) => tool.repo ?? "local")).size} repos`} />
          <div className="tool-stack">
            {liveTools.length === 0 && <EmptyCard label="No ecosystem tools reported" />}
            {liveTools.map((tool, index) => (
              <article className={`tool-node tool-node-${tool.health ?? "nominal"}`} key={tool.name}>
                <div>
                  <Boxes size={18} aria-hidden="true" />
                  <strong>{tool.name}</strong>
                </div>
                <span>
                  {tool.repo ?? "local"} / {tool.provider ?? "jmcp"}
                </span>
                <small>{tool.dependsOn?.join(" -> ") ?? "direct"}</small>
                <i style={{ width: `${Math.max(18, 94 - index * 9)}%` }} />
              </article>
            ))}
          </div>
        </section>

        <section className="attention-panel universe-slices">
          <PanelHeader icon={ShieldAlert} title="Degraded Slices" meta={`${bootstrap.degradedSlices.filter((slice) => !slice.live).length} degraded`} />
          <div className="attention-list">
            {bootstrap.degradedSlices.length === 0 && <EmptyRow label="No degraded slices" />}
            {bootstrap.degradedSlices.map((slice) => (
              <article className="attention-row universe-slice-row" key={slice.name}>
                <Network size={16} aria-hidden="true" />
                <div>
                  <strong>{slice.name}</strong>
                  <span>{slice.degradedReason ?? "slice live"}</span>
                </div>
                <span className={classForHealth(slice.live ? "nominal" : "degraded")}>{slice.coverage}%</span>
              </article>
            ))}
          </div>
        </section>
      </section>

      <section className="list-panel">
        <PanelHeader icon={GitBranch} title="Placement Rows" meta="current task, branch, pool, and score" />
        <div className="rows universe-placement-rows">
          {placements.length === 0 && <EmptyRow label="No placements observed" />}
          {placements.map((placement) => (
            <article className="row universe-placement-row" key={placement.repo}>
              <div>
                <strong>{placement.agent}</strong>
                <span>{placement.placement}</span>
              </div>
              <span className={classForHealth(placement.health)}>{placement.health}</span>
              <span>{placement.currentTask}</span>
              <span>{placement.branch}</span>
              <span>{placement.pool}</span>
              <span>{placement.score}%</span>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

export const ToolsDataView = UniverseView;

export function MemoryView({ memoryLessons }: { memoryLessons: MemoryProposal[] }) {
  const promoted = memoryLessons.filter((lesson) => lesson.state === "promoted").length;
  const quarantined = memoryLessons.filter((lesson) => lesson.state === "quarantined").length;
  const shadowed = memoryLessons.filter((lesson) => lesson.state === "shadow" || lesson.state === "proposed").length;

  return (
    <div className="memory-board">
      <section className="memory-hero">
        <div>
          <p className="eyebrow">Memory</p>
          <h3>
            {memoryLessons.length} lessons, {promoted} promoted, {quarantined} quarantined.
          </h3>
          <p>
            Promotion stays visible with expiry, counterexamples, and rollback context attached to each lesson so the cockpit can keep the learning surface honest.
          </p>
        </div>
        <div className="memory-dial" aria-label={`${shadowed} lessons still in review`}>
          <strong>{shadowed}</strong>
          <span>in review</span>
        </div>
      </section>

      <section className="card-grid memory-grid">
        {memoryLessons.length === 0 && <EmptyCard label="No memory lessons" />}
        {memoryLessons.map((lesson) => (
          <article className={`memory-card memory-${lesson.state}`} key={lesson.id}>
            <div className="system-card-head">
              <strong>{lesson.id}</strong>
              <span className={`pill memory-${lesson.state}`}>{lesson.state}</span>
            </div>
            <h3>{lesson.claim}</h3>
            <p>{lesson.scope}</p>
            <dl className="memory-meta">
              <div>
                <dt>Retention</dt>
                <dd>{lesson.retention}</dd>
              </div>
              <div>
                <dt>Expiry</dt>
                <dd>{lesson.expiry}</dd>
              </div>
              <div>
                <dt>Confidence</dt>
                <dd>{lesson.confidence}%</dd>
              </div>
              <div>
                <dt>Source</dt>
                <dd>{lesson.source}</dd>
              </div>
            </dl>
            <div className="meter" aria-label={`${lesson.confidence}% confidence`}>
              <span style={{ width: `${lesson.confidence}%` }} />
            </div>
            <section className="memory-promotion">
              <span className="eyebrow">Promotion</span>
              <strong>{lesson.promotion.status}</strong>
              <p>{lesson.promotion.gate}</p>
              <span>{lesson.promotion.reviewedBy ? `Reviewed by ${lesson.promotion.reviewedBy}` : "Awaiting reviewer"}</span>
              {lesson.promotion.promotedAt && <span>{lesson.promotion.promotedAt}</span>}
            </section>
            <section className="memory-counterexamples">
              <span className="eyebrow">Counterexamples</span>
              <div className="chip-list">
                {lesson.counterexamples.map((counterexample) => (
                  <span className="chip" key={counterexample}>
                    {counterexample}
                  </span>
                ))}
              </div>
            </section>
            <section className="memory-rollback">
              <span className="eyebrow">Rollback</span>
              <p>{lesson.rollback}</p>
            </section>
            {lesson.incident && <MemoryIncidentCard incident={lesson.incident} />}
          </article>
        ))}
      </section>
    </div>
  );
}

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

function AttentionPacketCard({ packet }: { packet: AttentionPacket }) {
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

function SystemIncidentCard({ incident }: { incident: NonNullable<SystemNode["incident"]> }) {
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

function MemoryIncidentCard({ incident }: { incident: NonNullable<MemoryProposal["incident"]> }) {
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

function MetricCard({ label, value, tone, detail }: { label: string; value: string; tone: string; detail: string }) {
  return (
    <article className={`metric metric-${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
      <p>{detail}</p>
    </article>
  );
}

function PanelHeader({ icon: Icon, title, meta }: { icon: LucideIcon; title: string; meta: string }) {
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

function WorkRow({ item }: { item: WorkItem }) {
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

function EmptyRow({ label }: { label: string }) {
  return <article className="row">{label}</article>;
}

function EmptyCard({ label }: { label: string }) {
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

function classForHealth(health: Health) {
  return `status status-${health}`;
}

function normalizeVoiceState(state: VoiceState | "draft") {
  return state === "draft" ? "transcribed" : state;
}

function riskRank(risk: Risk) {
  if (risk === "high") {
    return 3;
  }
  if (risk === "medium") {
    return 2;
  }
  return 1;
}
