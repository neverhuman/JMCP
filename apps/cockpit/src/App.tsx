import { useMemo, useState } from "react";
import {
  Activity,
  Archive,
  CheckCircle2,
  CircleDot,
  Database,
  FileCheck2,
  Gauge,
  GitBranch,
  History,
  KeyRound,
  Layers3,
  ShieldAlert,
  Wrench,
} from "lucide-react";
import {
  approvalRequests,
  evidenceBundles,
  memoryProposals,
  replayEvents,
  systems,
  toolAssets,
  views,
  workItems,
} from "./fixtures";
import type { Health, Risk, ViewId } from "./types";

const icons = {
  now: Gauge,
  work: GitBranch,
  evidence: FileCheck2,
  systems: Layers3,
  "tools-data": Database,
  "memory-lite": Archive,
  replay: History,
  approvals: ShieldAlert,
};

function classForRisk(risk: Risk) {
  return `pill risk-${risk}`;
}

function classForHealth(health: Health) {
  return `status status-${health}`;
}

function App() {
  const [activeView, setActiveView] = useState<ViewId>("now");
  const currentView = useMemo(
    () => views.find((view) => view.id === activeView) ?? views[0],
    [activeView],
  );

  return (
    <div className="shell">
      <aside className="rail" aria-label="JMCP views">
        <div className="brand">
          <div className="brand-mark">J</div>
          <div>
            <strong>JMCP</strong>
            <span>JCP/1.0.0 via JPCM</span>
          </div>
        </div>
        <nav className="nav-list">
          {views.map((view) => {
            const Icon = icons[view.id];
            return (
              <button
                key={view.id}
                className={view.id === activeView ? "nav-item active" : "nav-item"}
                type="button"
                onClick={() => setActiveView(view.id)}
                title={view.description}
                aria-pressed={view.id === activeView}
              >
                <Icon size={18} aria-hidden="true" />
                <span>{view.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Joint Master Control Plane</p>
            <h1>{currentView.label}</h1>
          </div>
          <div className="protocol-card">
            <Activity size={18} aria-hidden="true" />
            <div>
              <span>Backbone</span>
              <strong>JPCM stream healthy</strong>
            </div>
          </div>
        </header>

        <section className="view-panel" aria-labelledby="view-heading">
          <div className="view-heading">
            <div>
              <p className="eyebrow">Current slice</p>
              <h2 id="view-heading">{currentView.description}</h2>
            </div>
            <span className="timestamp">Updated 12:09 UTC</span>
          </div>
          {activeView === "now" && <NowView />}
          {activeView === "work" && <WorkView />}
          {activeView === "evidence" && <EvidenceView />}
          {activeView === "systems" && <SystemsView />}
          {activeView === "tools-data" && <ToolsDataView />}
          {activeView === "memory-lite" && <MemoryLiteView />}
          {activeView === "replay" && <ReplayView />}
          {activeView === "approvals" && <ApprovalsView />}
        </section>
      </main>
    </div>
  );
}

function NowView() {
  const blocked = workItems.filter((item) => item.state === "blocked").length;
  const pendingEvidence = evidenceBundles.filter((bundle) => bundle.status === "pending").length;

  return (
    <div className="dashboard-grid">
      <MetricCard label="Open work" value={workItems.length.toString()} tone="green" detail="2 leased workers active" />
      <MetricCard label="Evidence pending" value={pendingEvidence.toString()} tone="amber" detail="1 proof bundle needs review" />
      <MetricCard label="Blocked" value={blocked.toString()} tone="red" detail="adapter authority gap" />
      <MetricCard label="Approvals" value={approvalRequests.length.toString()} tone="ink" detail="oldest expires in 6m" />

      <section className="attention-strip">
        <div>
          <p className="eyebrow">Minimum safe signal</p>
          <h3>Quarantine remains active for the MCP bridge.</h3>
          <p>
            JMCP has enough evidence to keep work running, but not enough to grant write authority to the adapter.
          </p>
        </div>
        <ShieldAlert size={32} aria-hidden="true" />
      </section>

      <section className="list-panel wide">
        <PanelHeader icon={CircleDot} title="Live Work" meta={`${workItems.length} work orders`} />
        <div className="rows">
          {workItems.map((item) => (
            <WorkRow key={item.id} item={item} />
          ))}
        </div>
      </section>
    </div>
  );
}

function WorkView() {
  return (
    <section className="list-panel">
      <PanelHeader icon={GitBranch} title="Work Orders" meta="Task state, risk, lease, proof count" />
      <div className="rows">
        {workItems.map((item) => (
          <WorkRow key={item.id} item={item} />
        ))}
      </div>
    </section>
  );
}

function EvidenceView() {
  return (
    <section className="list-panel">
      <PanelHeader icon={FileCheck2} title="Evidence Bundles" meta="Promotion requires accepted proof" />
      <div className="rows">
        {evidenceBundles.map((bundle) => (
          <article className="row" key={bundle.id}>
            <div>
              <strong>{bundle.subject}</strong>
              <span>{bundle.id} from {bundle.source}</span>
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

function SystemsView() {
  return (
    <section className="card-grid">
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

function ToolsDataView() {
  return (
    <section className="list-panel">
      <PanelHeader icon={Wrench} title="Tools And Data Assets" meta="Conformance, side effects, data classes" />
      <div className="rows">
        {toolAssets.map((tool) => (
          <article className="row tool-row" key={tool.name}>
            <div>
              <strong>{tool.name}</strong>
              <span>{tool.className}</span>
            </div>
            <span className="pill neutral">{tool.conformance}</span>
            <span>{tool.sideEffects}</span>
            <span>{tool.dataClasses.join(", ")}</span>
          </article>
        ))}
      </div>
    </section>
  );
}

function MemoryLiteView() {
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

function ReplayView() {
  return (
    <section className="list-panel">
      <PanelHeader icon={History} title="Replay Ledger" meta="JPCM events reconstruct state" />
      <div className="timeline">
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

function ApprovalsView() {
  return (
    <section className="card-grid">
      {approvalRequests.map((approval) => (
        <article className="approval-card" key={approval.id}>
          <div className="system-card-head">
            <strong>{approval.id}</strong>
            <span className={classForRisk(approval.risk)}>{approval.risk}</span>
          </div>
          <h3>{approval.decision}</h3>
          <p>{approval.reason}</p>
          <div className="approval-actions">
            <button type="button">
              <CheckCircle2 size={17} aria-hidden="true" />
              Approve
            </button>
            <button type="button" className="secondary">
              <KeyRound size={17} aria-hidden="true" />
              Deny
            </button>
          </div>
          <span className="expires">Expires in {approval.expires}</span>
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

function PanelHeader({ icon: Icon, title, meta }: { icon: typeof CircleDot; title: string; meta: string }) {
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

function WorkRow({ item }: { item: (typeof workItems)[number] }) {
  return (
    <article className="row work-row">
      <div>
        <strong>{item.title}</strong>
        <span>{item.id} by {item.owner}</span>
      </div>
      <span className={classForRisk(item.risk)}>{item.risk}</span>
      <span>{item.state}</span>
      <code>{item.lease}</code>
      <span>{item.evidence} proofs</span>
      <span>{item.updated}</span>
    </article>
  );
}

export default App;
