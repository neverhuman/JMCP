import { ChevronsDownUp, ChevronsUpDown, Crosshair, ListFilter } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { deckStore, getCardsForPane, useDeckSnapshot } from "../store";
import { AnswerCaptionStream } from "./AnswerCaptionStream";
import { DataLoom } from "./DataLoom";
import { DeckViewport } from "./DeckViewport";
import { FocusPane } from "./FocusPane";
import { TraceRibbon } from "./TraceRibbon";

export function NowCommandDeck() {
  const state = useDeckSnapshot();
  const [viewMode, setViewMode] = useState<"stack" | "fan">("stack");
  const focusPane = state.focusPaneId ? state.panes[state.focusPaneId] : null;
  const focusCards = useMemo(() => (state.focusPaneId ? getCardsForPane(state, state.focusPaneId) : []), [state]);
  const focusEvidence = state.focusPaneId ? state.evidenceByPane[state.focusPaneId] ?? [] : [];
  const focusActions = state.focusPaneId ? state.actionsByPane[state.focusPaneId] ?? [] : [];
  const focusReason = state.focusPaneId ? state.rankReasons[state.focusPaneId] : undefined;

  useEffect(() => {
    if (!state.active) {
      return () => {};
    }
    return deckStore.startLiveQueueBlockers();
  }, [state.active]);

  return (
    <section className="command-deck" data-mobile-clearance="voice-bar" data-view-mode={viewMode} aria-label="AIUX Mission Deck">
      <DataLoom />
      <div className="command-deck-surface">
        <header className="command-deck-head">
          <div>
            <p className="eyebrow">AIUX Mission Deck</p>
            <h3>{state.title}</h3>
          </div>
          <div className="deck-toolbar" aria-label="Deck controls">
            <button aria-label="Fan panes" onClick={() => setViewMode("fan")} title="Fan panes" type="button">
              <ChevronsUpDown size={17} aria-hidden="true" />
            </button>
            <button aria-label="Collapse panes" onClick={() => setViewMode("stack")} title="Collapse panes" type="button">
              <ChevronsDownUp size={17} aria-hidden="true" />
            </button>
            <button aria-label="Tunnel focus" title="Tunnel focus" type="button">
              <Crosshair size={17} aria-hidden="true" />
            </button>
            <button aria-label="Rank filter" title="Rank filter" type="button">
              <ListFilter size={17} aria-hidden="true" />
            </button>
          </div>
        </header>
        <TraceRibbon trace={state.trace} />
        <div className="command-deck-grid">
          <DeckViewport state={state} />
          <FocusPane pane={focusPane} cards={focusCards} evidence={focusEvidence} actions={focusActions} reason={focusReason} />
        </div>
        <AnswerCaptionStream caption={state.error ?? state.caption} />
      </div>
    </section>
  );
}
