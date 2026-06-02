import type { AttentionPacket, MemoryProposal, SystemNode } from "./types";

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
