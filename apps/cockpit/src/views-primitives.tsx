import type { LucideIcon } from "lucide-react";
import type { Health, Risk, WorkItem } from "./types";

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

export function classForRisk(risk: Risk) {
  return `pill risk-${risk}`;
}

export function classForHealth(health: Health) {
  return `status status-${health}`;
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
