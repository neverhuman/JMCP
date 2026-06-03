import { FileCheck2, History, Radar } from "lucide-react";
import type { DeckRankReason, EvidenceRef, PaneVM } from "../types";

export function EvidenceRibbon({
  evidence,
  pane,
  reason,
}: {
  evidence: EvidenceRef[];
  pane: PaneVM;
  reason?: DeckRankReason;
}) {
  return (
    <section className="evidence-ribbon" aria-label="Evidence">
      <div className="evidence-summary">
        <Radar size={16} aria-hidden="true" />
        <span>{Math.round(pane.confidence * 100)}% confidence</span>
        {pane.freshnessMs !== undefined && <span>{Math.round(pane.freshnessMs / 1000)}s freshness</span>}
      </div>
      {reason && <p>{reason.explanation}</p>}
      <div className="evidence-items">
        {evidence.map((item) => (
          <span className="evidence-item" key={item.id}>
            <FileCheck2 size={15} aria-hidden="true" />
            <strong>{item.label}</strong>
            <small>{item.uri}</small>
            {item.capturedAt && (
              <em>
                <History size={13} aria-hidden="true" />
                {item.capturedAt}
              </em>
            )}
          </span>
        ))}
      </div>
    </section>
  );
}
