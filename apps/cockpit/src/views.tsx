import {
  Boxes,
  FileCheck2,
  GitBranch,
  Network,
  ShieldAlert,
} from "lucide-react";
import type {
  EvidenceBundle,
  MemoryProposal,
  Risk,
  SystemNode,
  WorkItem,
} from "./types";
import { useEffect } from "react";
import type { RuntimeState } from "./runtime";
import { NowCommandDeck } from "./jitux/components/NowCommandDeck";
import { deckStore, useDeckSnapshot } from "./jitux/store";
import {
  AttentionPacketCard,
  EmptyCard,
  EmptyRow,
  MemoryIncidentCard,
  MetricCard,
  PanelHeader,
  SystemIncidentCard,
  WorkRow,
  classForHealth,
  riskRank,
} from "./views-extra";

export function NowView({ runtime }: { runtime: RuntimeState }) {
  const deckActive = useDeckSnapshot((state) => state.active);
  useEffect(() => {
    if (!deckActive) {
      deckStore.startLiveQueueBlockers(runtime);
    }
    return () => {};
  }, [deckActive, runtime]);
  const blocked = runtime.workItems.filter((item) => item.state === "blocked").length;
  const decisionPackets = runtime.attentionPackets.filter((packet) => packet.decisionNeeded).length;
  const urgentPackets = runtime.attentionPackets.filter(
    (packet) => packet.attentionLevel === "urgent" || packet.attentionLevel === "incident",
  ).length;
  const voiceConfirmations = runtime.voiceThreads.filter((thread) => thread.requiresResponse).length;
  const activePromotions = runtime.memoryLessons.filter((lesson) => lesson.state === "promoted").length;
  const topRisk = runtime.attentionPackets.reduce((highest, packet) => (riskRank(packet.riskDelta.to) > riskRank(highest) ? packet.riskDelta.to : highest), "low" as Risk);

  if (deckActive) {
    return <NowCommandDeck />;
  }

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

export { ApprovalsView, ReplayView, VoiceTextView } from "./views-extra";
