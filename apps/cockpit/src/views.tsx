import { Boxes, CircleDot, FileCheck2, GitBranch, History, Network, ShieldAlert, Wrench, type LucideIcon } from "lucide-react";
import { memoryProposals } from "./fixtures";
import type { ApprovalRequest, EvidenceBundle, Health, ReplayEvent, Risk, SystemNode, ToolAsset, WorkItem } from "./types";
import type { RuntimeState } from "./runtime";

export function NowView({ runtime }: { runtime: RuntimeState }) {
  const blocked = runtime.workItems.filter((item) => item.state === "blocked").length;
  const pendingEvidence = runtime.evidenceBundles.filter((bundle) => bundle.status === "pending").length;
  const leased = runtime.workItems.filter((item) => item.lease !== "lease required").length;

  return (
    <div className="dashboard-grid">
      <MetricCard label="Open work" value={runtime.workItems.length.toString()} tone="green" detail={`${leased} leased workers active`} />
      <MetricCard label="Evidence pending" value={pendingEvidence.toString()} tone="amber" detail={`${pendingEvidence} proof bundles need review`} />
      <MetricCard label="Blocked" value={blocked.toString()} tone="red" detail="adapter authority gap" />
      <MetricCard label="Approvals" value={runtime.approvalRequests.length.toString()} tone="ink" detail={runtime.usingFixtures ? "fixture data" : "live API"} />

      <section className="attention-strip">
        <div>
          <p className="eyebrow">Minimum safe signal</p>
          <h3>Quarantine remains active for the MCP bridge.</h3>
          <p>JMCP has enough evidence to keep work running, but not enough to grant write authority to the adapter.</p>
        </div>
        <ShieldAlert size={32} aria-hidden="true" />
      </section>

      <section className="list-panel wide">
        <PanelHeader icon={CircleDot} title="Live Work" meta={`${runtime.workItems.length} work orders`} />
        <div className="rows">
          {runtime.workItems.length === 0 && <EmptyRow label="No work orders" />}
          {runtime.workItems.map((item) => (
            <WorkRow key={item.id} item={item} />
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
    <section className="card-grid">
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
        </article>
      ))}
    </section>
  );
}

export function ToolsDataView({ runtime }: { runtime: RuntimeState }) {
  const repos = Array.from(new Set(runtime.toolAssets.map((tool) => tool.repo ?? "local")));
  const queueTotal = runtime.workItems.length + runtime.toolAssets.reduce((sum, tool) => sum + (tool.queue ?? 0), 0);
  const attentionSystems = runtime.systems.filter((system) => system.health !== "nominal");

  return (
    <div className="tool-portal">
      <section className="ecosystem-hero">
        <div>
          <p className="eyebrow">Jeryu Ecosystem</p>
          <h3>
            {runtime.toolAssets.length} tools across {repos.length} repos
          </h3>
          <p>
            {runtime.ecosystemLive
              ? "Jeryu evidence, Jankurai audits, Jekko workers, and JMCP leases are tracked as one governed graph."
              : runtime.ecosystemDegradedReason}
          </p>
        </div>
        <div className="queue-dial" aria-label={`${queueTotal} active queued items`}>
          <strong>{queueTotal}</strong>
          <span>active queue</span>
        </div>
      </section>

      <section className="ecosystem-grid">
        <div className="tool-map">
          {runtime.toolAssets.length === 0 && <EmptyCard label="No ecosystem tools reported" />}
          {runtime.toolAssets.map((tool, index) => (
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

        <section className="attention-panel">
          <PanelHeader icon={ShieldAlert} title="Needs Attention" meta={`${attentionSystems.length} systems`} />
          <div className="attention-list">
            {attentionSystems.length === 0 && <EmptyRow label="No systems need attention" />}
            {attentionSystems.map((system) => (
              <article className="attention-row" key={system.name}>
                <Network size={16} aria-hidden="true" />
                <div>
                  <strong>{system.name}</strong>
                  <span>{system.role}</span>
                </div>
                <span className={classForHealth(system.health)}>{system.health}</span>
              </article>
            ))}
          </div>
        </section>
      </section>

      <section className="list-panel">
        <PanelHeader icon={Wrench} title="Tools And Data Assets" meta="Conformance, side effects, data classes" />
        <div className="rows">
          {runtime.toolAssets.length === 0 && <EmptyRow label="No tools or data assets reported" />}
          {runtime.toolAssets.map((tool) => (
            <article className="row tool-row" key={tool.name}>
              <div>
                <strong>{tool.name}</strong>
                <span>{tool.className}</span>
              </div>
              <span className={classForHealth(tool.health ?? "nominal")}>{tool.health ?? "nominal"}</span>
              <span>{tool.sideEffects}</span>
              <span>{tool.dataClasses.join(", ")}</span>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

export function MemoryLiteView() {
  return (
    <section className="card-grid">
      {memoryProposals.map((proposal) => (
        <article className="memory-card" key={proposal.id}>
          <div className="system-card-head">
            <strong>{proposal.id}</strong>
            <span className={`pill memory-${proposal.status}`}>{proposal.status}</span>
          </div>
          <p>{proposal.lesson}</p>
          <div className="meter" aria-label={`${proposal.confidence}% confidence`}>
            <span style={{ width: `${proposal.confidence}%` }} />
          </div>
          <span className="scope">{proposal.scope}</span>
        </article>
      ))}
    </section>
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
  return (
    <section className="card-grid">
      {approvalRequests.length === 0 && <EmptyCard label="No approvals pending" />}
      {approvalRequests.map((approval) => (
        <article className="approval-card" key={approval.id}>
          <div className="system-card-head">
            <strong>{approval.id}</strong>
            <span className={classForRisk(approval.risk)}>{approval.state}</span>
          </div>
          <h3>{approval.decision}</h3>
          <p>{approval.reason}</p>
          <dl>
            <div>
              <dt>Work</dt>
              <dd>{approval.workOrderId}</dd>
            </div>
            <div>
              <dt>Channel</dt>
              <dd>{approval.channel}</dd>
            </div>
          </dl>
          <span className="expires">{approval.expires === "expired" ? "Expired" : `Expires in ${approval.expires}`}</span>
        </article>
      ))}
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
