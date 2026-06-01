import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  Archive,
  Boxes,
  CheckCircle2,
  CircleDot,
  Database,
  FileCheck2,
  Gauge,
  GitBranch,
  History,
  KeyRound,
  Layers3,
  Network,
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
import type {
  ApprovalRequest,
  EvidenceBundle,
  Health,
  ReplayEvent,
  Risk,
  SystemNode,
  ToolAsset,
  ViewId,
  WorkItem,
} from "./types";

const apiUrl = import.meta.env.VITE_JMCP_API_URL ?? "http://127.0.0.1:18877";

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
  const [runtime, setRuntime] = useState(() => ({
    apiHealth: "degraded" as Health,
    workItems,
    evidenceBundles,
    systems,
    toolAssets,
    replayEvents,
    approvalRequests,
    loadedAt: "fixture",
    usingFixtures: true,
  }));
  const currentView = useMemo(
    () => views.find((view) => view.id === activeView) ?? views[0],
    [activeView],
  );

  useEffect(() => {
    let cancelled = false;
    loadRuntime()
      .then((nextRuntime) => {
        if (!cancelled) {
          setRuntime(nextRuntime);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRuntime((current) => ({ ...current, apiHealth: "degraded" }));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

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
              <strong>JPCM stream {runtime.apiHealth === "nominal" ? "healthy" : "degraded"}</strong>
            </div>
          </div>
        </header>

        <section className="view-panel" aria-labelledby="view-heading">
          <div className="view-heading">
            <div>
              <p className="eyebrow">Current slice</p>
              <h2 id="view-heading">{currentView.description}</h2>
            </div>
            <span className="timestamp">Updated {runtime.loadedAt}</span>
          </div>
          {activeView === "now" && <NowView runtime={runtime} />}
          {activeView === "work" && <WorkView workItems={runtime.workItems} />}
          {activeView === "evidence" && <EvidenceView evidenceBundles={runtime.evidenceBundles} />}
          {activeView === "systems" && <SystemsView systems={runtime.systems} />}
          {activeView === "tools-data" && <ToolsDataView runtime={runtime} />}
          {activeView === "memory-lite" && <MemoryLiteView />}
          {activeView === "replay" && <ReplayView replayEvents={runtime.replayEvents} />}
          {activeView === "approvals" && <ApprovalsView approvalRequests={runtime.approvalRequests} />}
        </section>
      </main>
    </div>
  );
}

function NowView({ runtime }: { runtime: RuntimeState }) {
  const blocked = runtime.workItems.filter((item) => item.state === "blocked").length;
  const pendingEvidence = runtime.evidenceBundles.filter((bundle) => bundle.status === "pending").length;
  const leased = runtime.workItems.filter((item) => item.lease !== "lease required").length;

  return (
    <div className="dashboard-grid">
      <MetricCard label="Open work" value={runtime.workItems.length.toString()} tone="green" detail={`${leased} leased workers active`} />
      <MetricCard label="Evidence pending" value={pendingEvidence.toString()} tone="amber" detail={`${pendingEvidence} proof bundles need review`} />
      <MetricCard label="Blocked" value={blocked.toString()} tone="red" detail="adapter authority gap" />
      <MetricCard label="Approvals" value={runtime.approvalRequests.length.toString()} tone="ink" detail={runtime.usingFixtures ? "fixture fallback" : "live API"} />

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

function WorkView({ workItems }: { workItems: WorkItem[] }) {
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

function EvidenceView({ evidenceBundles }: { evidenceBundles: EvidenceBundle[] }) {
  return (
    <section className="list-panel">
      <PanelHeader icon={FileCheck2} title="Evidence Bundles" meta="Promotion requires accepted proof" />
      <div className="rows">
        {evidenceBundles.length === 0 && <EmptyRow label="No evidence bundles" />}
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

function SystemsView({ systems }: { systems: SystemNode[] }) {
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

function ToolsDataView({ runtime }: { runtime: RuntimeState }) {
  const repos = Array.from(new Set(runtime.toolAssets.map((tool) => tool.repo ?? "local")));
  const queueTotal =
    runtime.workItems.length + runtime.toolAssets.reduce((sum, tool) => sum + (tool.queue ?? 0), 0);
  const attentionSystems = runtime.systems.filter((system) => system.health !== "nominal");

  return (
    <div className="tool-portal">
      <section className="ecosystem-hero">
        <div>
          <p className="eyebrow">Jeryu Ecosystem</p>
          <h3>{runtime.toolAssets.length} tools across {repos.length} repos</h3>
          <p>Jeryu evidence, Jankurai audits, Jekko workers, and JMCP leases are tracked as one governed graph.</p>
        </div>
        <div className="queue-dial" aria-label={`${queueTotal} active queued items`}>
          <strong>{queueTotal}</strong>
          <span>active queue</span>
        </div>
      </section>

      <section className="ecosystem-grid">
        <div className="tool-map">
          {runtime.toolAssets.map((tool, index) => (
            <article className={`tool-node tool-node-${tool.health ?? "nominal"}`} key={tool.name}>
              <div>
                <Boxes size={18} aria-hidden="true" />
                <strong>{tool.name}</strong>
              </div>
              <span>{tool.repo ?? "local"} / {tool.provider ?? "jmcp"}</span>
              <small>{tool.dependsOn?.join(" -> ") ?? "direct"}</small>
              <i style={{ width: `${Math.max(18, 94 - index * 9)}%` }} />
            </article>
          ))}
        </div>

        <aside className="attention-panel">
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
        </aside>
      </section>

      <section className="list-panel">
        <PanelHeader icon={Wrench} title="Tools And Data Assets" meta="Conformance, side effects, data classes" />
        <div className="rows">
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

function ReplayView({ replayEvents }: { replayEvents: ReplayEvent[] }) {
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

function ApprovalsView({ approvalRequests }: { approvalRequests: ApprovalRequest[] }) {
  return (
    <section className="card-grid">
      {approvalRequests.length === 0 && <EmptyCard label="No approvals pending" />}
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

function WorkRow({ item }: { item: WorkItem }) {
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

type RuntimeState = {
  apiHealth: Health;
  workItems: WorkItem[];
  evidenceBundles: EvidenceBundle[];
  systems: SystemNode[];
  toolAssets: ToolAsset[];
  replayEvents: ReplayEvent[];
  approvalRequests: ApprovalRequest[];
  loadedAt: string;
  usingFixtures: boolean;
};

type ApiWorkOrder = {
  id: string;
  subject: string;
  status: string;
  task: { kind: string };
  evidence: unknown[];
  updated_at: string;
};

type ApiEvidence = {
  kind: string;
  uri: string;
  captured_at: string;
};

type ApiApproval = {
  work_order_id: string;
  approver: string;
  expires_at: string;
  decision?: string | null;
};

type ApiReplay = {
  events: number;
  checkpoints: Array<{ id: string; last_event_id: number; created_at: string }>;
};

type ApiAdapters = {
  service_cards: Array<{
    name: string;
    capabilities: string[];
    subjects: string[];
  }>;
  health: Array<{
    name: string;
    health: Health;
    endpoint?: string | null;
    detail: string;
  }>;
};

async function loadRuntime(): Promise<RuntimeState> {
  if (typeof fetch !== "function") {
    return {
      apiHealth: "degraded",
      workItems,
      evidenceBundles,
      systems,
      toolAssets,
      replayEvents,
      approvalRequests,
      loadedAt: "fixture",
      usingFixtures: true,
    };
  }

  const [health, apiWork, apiEvidence, apiSystems, apiReplay, apiApprovals, apiAdapters] = await Promise.allSettled([
    getJson<{ ok: boolean }>("/health"),
    getJson<ApiWorkOrder[]>("/work-orders"),
    getJson<ApiEvidence[]>("/evidence"),
    getJson<typeof systems>("/systems"),
    getJson<ApiReplay>("/replay"),
    getJson<ApiApproval[]>("/approvals"),
    getJson<ApiAdapters>("/adapters"),
  ]);

  const allFailed = [health, apiWork, apiEvidence, apiSystems, apiReplay, apiApprovals, apiAdapters].every(
    (result) => result.status === "rejected",
  );
  if (allFailed) {
    return {
      apiHealth: "degraded",
      workItems,
      evidenceBundles,
      systems,
      toolAssets,
      replayEvents,
      approvalRequests,
      loadedAt: "fixture",
      usingFixtures: true,
    };
  }

  const liveWork = apiWork.status === "fulfilled" ? apiWork.value.map(mapWorkOrder) : workItems;
  const liveEvidence = apiEvidence.status === "fulfilled" ? apiEvidence.value.map(mapEvidence) : evidenceBundles;
  const liveSystems = apiSystems.status === "fulfilled" ? apiSystems.value : systems;
  const liveReplay = apiReplay.status === "fulfilled" ? mapReplay(apiReplay.value) : replayEvents;
  const liveApprovals = apiApprovals.status === "fulfilled" ? apiApprovals.value.map(mapApproval) : approvalRequests;
  const liveTools = apiAdapters.status === "fulfilled" ? mapAdapters(apiAdapters.value) : toolAssets;
  const partialFailure = [health, apiWork, apiEvidence, apiSystems, apiReplay, apiApprovals, apiAdapters].some(
    (result) => result.status === "rejected",
  );

  return {
    apiHealth: partialFailure || health.status !== "fulfilled" || !health.value.ok ? "watch" : "nominal",
    workItems: liveWork,
    evidenceBundles: liveEvidence,
    systems: liveSystems,
    toolAssets: liveTools,
    replayEvents: liveReplay,
    approvalRequests: liveApprovals,
    loadedAt: new Date().toISOString().slice(11, 19) + "Z",
    usingFixtures: partialFailure,
  };
}

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${apiUrl}${path}`);
  if (!response.ok) {
    throw new Error(`JMCP API ${path} returned ${response.status}`);
  }
  return response.json() as Promise<T>;
}

function mapWorkOrder(workOrder: ApiWorkOrder) {
  const owner = workOrder.subject.split("/")[1] ?? "jmcp";
  return {
    id: workOrder.id,
    title: workOrder.task.kind,
    owner,
    state: workOrder.status.toLowerCase(),
    risk: workOrder.status === "Failed" ? "high" as Risk : "low" as Risk,
    lease: "lease required",
    updated: formatAge(workOrder.updated_at),
    evidence: workOrder.evidence.length,
  };
}

function mapEvidence(evidence: ApiEvidence) {
  return {
    id: evidence.uri.slice(0, 12),
    subject: evidence.kind,
    source: "jmcpd",
    status: "accepted" as const,
    hash: evidence.uri,
    age: formatAge(evidence.captured_at),
  };
}

function mapReplay(replay: ApiReplay) {
  if (replay.events === 0 && replay.checkpoints.length === 0) {
    return [];
  }
  const checkpoints = replay.checkpoints.length > 0 ? replay.checkpoints : [{
    id: "live-events",
    last_event_id: replay.events,
    created_at: new Date().toISOString(),
  }];
  return checkpoints.map((checkpoint) => ({
    sequence: checkpoint.last_event_id,
    subject: checkpoint.id,
    family: "ReplayCheckpoint",
    timestamp: new Date(checkpoint.created_at).toISOString().slice(11, 19) + "Z",
    producer: "jmcpd",
  }));
}

function mapApproval(approval: ApiApproval) {
  return {
    id: approval.work_order_id,
    decision: approval.decision ?? `Awaiting ${approval.approver}`,
    reason: "JMCP approval gate",
    risk: "medium" as Risk,
    expires: formatUntil(approval.expires_at),
  };
}

function mapAdapters(adapters: ApiAdapters): ToolAsset[] {
  const healthByName = new Map(adapters.health.map((item) => [item.name, item]));
  return adapters.service_cards.flatMap((card) => {
    const health = healthByName.get(card.name);
    return card.capabilities.map((capability) => ({
      name: `${card.name}.${capability}`,
      className: card.subjects.join(", "),
      conformance: card.name === "jmcpd" ? "C2 native" : "C1 governed",
      sideEffects: capability.includes("health") || capability.includes("status") ? "none" : "lease gated",
      dataClasses: [card.name, capability],
      repo: card.name === "jmcpd" ? "JMCP" : titleCase(card.name),
      provider: card.name,
      health: health?.health ?? "degraded",
      dependsOn: card.name === "jeryu" ? ["jmcpd.work-orders", "jankurai.proof"] : ["jmcpd.leases"],
      queue: health?.health === "nominal" ? 0 : 1,
    }));
  });
}

function titleCase(value: string) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

function formatAge(value: string) {
  const time = new Date(value).getTime();
  if (Number.isNaN(time)) {
    return "live";
  }
  const seconds = Math.max(0, Math.round((Date.now() - time) / 1000));
  if (seconds < 60) {
    return `${seconds}s ago`;
  }
  return `${Math.round(seconds / 60)}m ago`;
}

function formatUntil(value: string) {
  const time = new Date(value).getTime();
  if (Number.isNaN(time)) {
    return "unknown";
  }
  const seconds = Math.round((time - Date.now()) / 1000);
  if (seconds <= 0) {
    return "expired";
  }
  if (seconds < 60) {
    return `${seconds}s`;
  }
  return `${Math.round(seconds / 60)}m`;
}
