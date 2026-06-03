import { Code2, FileCheck2, History, Server, Zap } from "lucide-react";
import type { DeckCardVM } from "../store";
import type { DeckRankReason, EvidenceRef, PaneVM, PreparedAction } from "../types";
import { EvidenceRibbon } from "./EvidenceRibbon";
import { PreparedActionRail } from "./PreparedActionRail";

const tabIcons = {
  evidence: FileCheck2,
  replay: History,
  systems: Server,
  actions: Zap,
  raw: Code2,
};

export function FocusPane({
  pane,
  cards,
  evidence,
  actions,
  reason,
}: {
  pane: PaneVM | null;
  cards: DeckCardVM[];
  evidence: EvidenceRef[];
  actions: PreparedAction[];
  reason?: DeckRankReason;
}) {
  if (!pane) {
    return null;
  }

  return (
    <section className="focus-pane" aria-label="Focus pane">
      <div className="focus-pane-head">
        <div>
          <p className="eyebrow">Focus pane</p>
          <h3>{pane.title}</h3>
        </div>
        <span className={`deck-risk deck-risk-${pane.risk}`}>{pane.risk}</span>
      </div>
      <p className="focus-headline">{pane.preview.headline}</p>
      <div className="focus-tabs" role="tablist" aria-label="Prepared drilldowns">
        {pane.preparedTabs.map((tab) => {
          const Icon = tabIcons[tab];
          return (
            <button aria-selected={tab === "evidence"} key={tab} role="tab" title={tab} type="button">
              <Icon size={16} aria-hidden="true" />
              <span>{tab}</span>
            </button>
          );
        })}
      </div>
      <div className="focus-card-list">
        {cards.map((card) => (
          <article className={`focus-card focus-card-${card.status}`} key={card.id}>
            <strong>{card.title}</strong>
            <span>{card.status}</span>
            <p>{card.headline}</p>
          </article>
        ))}
      </div>
      <EvidenceRibbon evidence={evidence} pane={pane} reason={reason} />
      <PreparedActionRail actions={actions} />
    </section>
  );
}
