import { useCallback, useEffect, useMemo, useState } from "react";
import { deckStore, getRankedPanes, type DeckState } from "../store";
import { deckTransformScheduler } from "../scheduler";
import { getDeckTransform } from "../layout/deck";
import { DeckCardView } from "./DeckCardView";

function prefersReducedMotion(): boolean {
  if (typeof matchMedia !== "function") {
    return false;
  }
  return matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function useReducedMotion() {
  const [reduced, setReduced] = useState(prefersReducedMotion);

  useEffect(() => {
    if (typeof matchMedia !== "function") {
      return;
    }
    const query = matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setReduced(query.matches);
    update();
    query.addEventListener?.("change", update);
    return () => query.removeEventListener?.("change", update);
  }, []);

  return reduced;
}

export function DeckViewport({ state }: { state: DeckState }) {
  const reducedMotion = useReducedMotion();
  const panes = useMemo(() => getRankedPanes(state), [state]);
  const setElement = useCallback((id: string, element: HTMLElement | null) => {
    deckTransformScheduler.register(id, element);
  }, []);

  useEffect(() => {
    if (reducedMotion) {
      return;
    }
    const transforms = Object.fromEntries(
      panes.map((pane, index) => [pane.id, getDeckTransform(index, pane.id === state.focusPaneId)]),
    );
    deckTransformScheduler.schedule(transforms);
  }, [panes, reducedMotion, state.focusPaneId]);

  useEffect(() => () => deckTransformScheduler.clear(), []);

  return (
    <section
      aria-label="Mission Deck viewport"
      className="deck-viewport"
      data-motion={reducedMotion ? "reduced" : "depth"}
    >
      <div className="deck-stage" role="list" aria-label="Ranked Mission Deck">
        {panes.map((pane) => (
          <DeckCardView
            active={pane.id === state.focusPaneId}
            key={pane.id}
            onPromote={(paneId) => deckStore.promotePane(paneId, `${pane.title} was promoted by direct user focus.`)}
            pane={pane}
            rankReason={state.rankReasons[pane.id]}
            setElement={setElement}
          />
        ))}
      </div>
    </section>
  );
}
