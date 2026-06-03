import { ArrowUpRight, Eye, Layers3 } from "lucide-react";
import { memo } from "react";
import type { DeckRankReason, PaneVM } from "../types";

type DeckCardViewProps = {
  pane: PaneVM;
  active: boolean;
  rankReason?: DeckRankReason;
  setElement: (id: string, element: HTMLElement | null) => void;
  onPromote: (paneId: string) => void;
};

function DeckCardViewImpl({ pane, active, rankReason, setElement, onPromote }: DeckCardViewProps) {
  const label = `${pane.rank}. ${pane.title}`;
  return (
    <li
      aria-label={label}
      className={`deck-card deck-card-${pane.lod} ${active ? "deck-card-active" : ""}`}
      data-lod={pane.lod}
      data-testid="deck-card"
      ref={(element) => setElement(pane.id, element)}
      style={{ opacity: active ? 1 : undefined }}
    >
      <div className="deck-card-head">
        <span className={`deck-risk deck-risk-${pane.risk}`}>{pane.risk}</span>
        <strong>{pane.title}</strong>
        <span className="deck-rank">#{pane.rank}</span>
      </div>
      <h3>{pane.preview.headline}</h3>
      <div className="deck-chip-row">
        {pane.preview.chips.slice(0, 4).map((chip) => (
          <span className="deck-chip" key={chip}>
            {chip}
          </span>
        ))}
      </div>
      <dl className="deck-counter-row">
        {pane.preview.counters.slice(0, 3).map((counter) => (
          <div key={counter.label}>
            <dt>{counter.label}</dt>
            <dd>{counter.value}</dd>
          </div>
        ))}
      </dl>
      {rankReason && <p className="deck-card-reason">{rankReason.explanation}</p>}
      <div className="deck-card-actions">
        <button aria-label={`Promote ${pane.title}`} onClick={() => onPromote(pane.id)} title="Promote pane" type="button">
          <ArrowUpRight size={16} aria-hidden="true" />
        </button>
        <span title={pane.lod}>
          {pane.lod === "focus" ? <Eye size={16} aria-hidden="true" /> : <Layers3 size={16} aria-hidden="true" />}
        </span>
      </div>
    </li>
  );
}

export const DeckCardView = memo(DeckCardViewImpl);
