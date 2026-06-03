import { Activity, CheckCircle2, CircleDashed, RadioTower, TriangleAlert } from "lucide-react";
import type { TraceProbe } from "../store";

function iconFor(status: TraceProbe["status"]) {
  if (status === "ready") {
    return CheckCircle2;
  }
  if (status === "degraded") {
    return TriangleAlert;
  }
  if (status === "running") {
    return RadioTower;
  }
  return CircleDashed;
}

export function TraceRibbon({ trace }: { trace: TraceProbe[] }) {
  return (
    <section className="trace-ribbon" aria-label="Mission trace">
      {trace.map((probe) => {
        const Icon = iconFor(probe.status);
        return (
          <span className={`trace-chip trace-chip-${probe.status}`} key={probe.id}>
            <Icon size={15} aria-hidden="true" />
            <strong>{probe.label}</strong>
            <small>{probe.latencyMs === undefined ? probe.status : `${probe.latencyMs}ms`}</small>
          </span>
        );
      })}
      {trace.length === 0 && (
        <span className="trace-chip trace-chip-running">
          <Activity size={15} aria-hidden="true" />
          <strong>session</strong>
          <small>running</small>
        </span>
      )}
    </section>
  );
}
