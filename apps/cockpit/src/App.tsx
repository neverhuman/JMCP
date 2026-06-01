import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  Archive,
  Database,
  FileCheck2,
  Gauge,
  GitBranch,
  History,
  Layers3,
  ShieldAlert,
} from "lucide-react";
import { views } from "./fixtures";
import { createFixtureRuntime, hasValidEventBatch, loadRuntime, type RuntimeState } from "./runtime";
import {
  ApprovalsView,
  EvidenceView,
  MemoryLiteView,
  NowView,
  ReplayView,
  SystemsView,
  ToolsDataView,
  WorkView,
} from "./views";
import type { ViewId } from "./types";

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

function App() {
  const [activeView, setActiveView] = useState<ViewId>("now");
  const [runtime, setRuntime] = useState<RuntimeState>(() => createFixtureRuntime());
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

  useEffect(() => {
    if (typeof EventSource !== "function") {
      return;
    }

    let cancelled = false;
    const events = new EventSource(`${apiUrl}/events`);
    const refresh = (event: MessageEvent<string>) => {
      if (!hasValidEventBatch(event.data)) {
        return;
      }

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
    };

    events.addEventListener("jmcp.events", refresh as EventListener);
    return () => {
      cancelled = true;
      events.close();
    };
  }, []);

  return (
    <div className="shell">
      <nav className="rail" aria-label="JMCP views">
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
      </nav>

      <div className="workspace">
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
      </div>
    </div>
  );
}

export default App;
