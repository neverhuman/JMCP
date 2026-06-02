import { useState } from "react";
import { AlertTriangle, CheckCircle2, LoaderCircle, Lock, ScreenShare, type LucideIcon } from "lucide-react";
import CockpitApp from "../../cockpit/src/App";
import { geometryRuntimeReceipt, requiredProofStates } from "./geometry-runtime";

const proofStates = [
  { id: "loading", label: "Loading", description: "JMCP is synchronizing the cockpit state." },
  { id: "empty", label: "Empty", description: "No local work orders are currently queued." },
  { id: "error", label: "Error", description: "The proof host can show recovery guidance." },
  { id: "permission-denied", label: "Permission denied", description: "Unauthorized access is explicitly blocked." },
  { id: "success", label: "Success", description: "The live cockpit UI is rendered in place." },
] as const;

type ProofState = (typeof proofStates)[number]["id"];

function App() {
  const [activeState, setActiveState] = useState<ProofState>("success");

  return (
    <main className="proof-shell" data-proof-state={activeState} data-geometry-runtime={geometryRuntimeReceipt}>
      <section className="proof-hero" aria-labelledby="proof-title">
        <div className="proof-kicker">Rendered UX proof host</div>
        <h1 id="proof-title">JMCP cockpit states, verified visually.</h1>
        <p>
          This host reuses the live cockpit UI for the success path and exposes the required loading, empty,
          error, permission-denied, and success states to Playwright.
        </p>

        <nav className="state-switcher" aria-label="Proof states">
          {proofStates.map((state) => {
            const active = state.id === activeState;
            return (
              <button
              key={state.id}
              type="button"
              className={active ? "state-pill active" : "state-pill"}
              aria-pressed={active}
              aria-label={state.label}
              aria-describedby={`state-${state.id}-description`}
              onClick={() => setActiveState(state.id)}
            >
              <span>{state.label}</span>
              <small id={`state-${state.id}-description`}>{state.description}</small>
            </button>
          );
        })}
        </nav>
      </section>

      <section className="proof-stage" aria-label="Rendered state">
        {activeState === "loading" && (
          <StateCard
            id="loading"
            icon={LoaderCircle}
            tone="loading"
            title="Loading"
            body="JMCP is fetching the current operator view and proof bundle."
          />
        )}
        {activeState === "empty" && (
          <StateCard
            id="empty"
            icon={ScreenShare}
            tone="empty"
            title="Empty"
            body="No work orders are currently visible in this test fixture."
          />
        )}
        {activeState === "error" && (
          <StateCard
            id="error"
            icon={AlertTriangle}
            tone="error"
            title="Error"
            body="The host can present recovery guidance without losing structure."
          />
        )}
        {activeState === "permission-denied" && (
          <StateCard
            id="permission-denied"
            icon={Lock}
            tone="locked"
            title="Permission denied"
            body="Access is blocked until the backend grants the required authority."
          />
        )}
        {activeState === "success" && (
          <div className="success-stage" data-state-panel="success">
            <article className="state-card state-success" data-state-card="success">
              <div className="state-card-head">
                <CheckCircle2 size={28} aria-hidden="true" />
                <div>
                  <p className="proof-kicker">State proof</p>
                  <h2>Success</h2>
                </div>
              </div>
              <p>The live cockpit UI is rendered in place and can be captured by Playwright.</p>
            </article>
            <div className="cockpit-frame">
              <CockpitApp />
            </div>
          </div>
        )}
      </section>

      <footer className="proof-footer">
        <CheckCircle2 size={16} aria-hidden="true" />
        <span>Playwright exercises every required state and captures screenshots under target/jankurai/ux-qa.</span>
      </footer>
    </main>
  );
}

function StateCard({
  id,
  icon: Icon,
  tone,
  title,
  body,
}: {
  id: (typeof requiredProofStates)[number]["id"];
  icon: LucideIcon;
  tone: "loading" | "empty" | "error" | "locked";
  title: string;
  body: string;
}) {
  return (
    <article className={`state-card state-${tone}`} data-state-card={id}>
      <div className="state-card-head">
        <Icon size={28} aria-hidden="true" />
        <div>
          <p className="proof-kicker">State proof</p>
          <h2>{title}</h2>
        </div>
      </div>
      <p>{body}</p>
    </article>
  );
}

export default App;
