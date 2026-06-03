import { Code2, FileCheck2, History, Server, Zap } from "lucide-react";
import type { DeckCardVM } from "../deck-queries";
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

const warmingPane = {
  title: "Focus warming",
  risk: "low" as const,
  preview: {
    headline: "A focus pane will appear as soon as the deck has a resolved target.",
    chips: ["warming", "stand by", "deck-first"],
    counters: [
      { label: "prepared tabs", value: 0 },
      { label: "evidence", value: 0 },
      { label: "actions", value: 0 },
    ],
  },
  preparedTabs: ["evidence", "replay", "systems", "actions", "raw"] as const,
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
  const activePane = pane ?? warmingPane;
  const showPreparedData = pane !== null;

  return (
    <section className="focus-pane" aria-label="Focus pane">
      <div className="focus-pane-head">
        <div>
          <p className="eyebrow">Focus pane</p>
          <h3>{activePane.title}</h3>
        </div>
        <span className={`deck-risk deck-risk-${activePane.risk}`}>{activePane.risk}</span>
      </div>
      <p className="focus-headline">{activePane.preview.headline}</p>
      <div className="focus-tabs" role="tablist" aria-label="Prepared drilldowns">
        {activePane.preparedTabs.map((tab) => {
          const Icon = tabIcons[tab];
          return (
            <button aria-selected={tab === "evidence"} key={tab} role="tab" title={tab} type="button">
              <Icon size={16} aria-hidden="true" />
              <span>{tab}</span>
            </button>
          );
        })}
      </div>
      {showPreparedData ? (
        <>
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
        </>
      ) : (
        <div className="focus-card-list" aria-live="polite">
          <article className="focus-card focus-card-degraded">
            <strong>Focus is warming</strong>
            <span>incubating</span>
            <p>The deck is preparing a resolved target, evidence ribbon, and action rail.</p>
          </article>
        </div>
      )}
    </section>
  );
}
