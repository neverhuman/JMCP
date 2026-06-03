import type { CardLOD, PaneRisk, PaneVM } from "./types";

export type DeckCardVM = {
  id: string;
  paneId: string;
  title: string;
  lod: CardLOD;
  status: "ghost" | "committed" | "hydrated";
  risk: PaneRisk;
  headline: string;
};

type DeckSnapshot = {
  paneOrder: string[];
  panes: Record<string, PaneVM | undefined>;
};

export function getRankedPanes(state: DeckSnapshot): PaneVM[] {
  return state.paneOrder.map((id) => state.panes[id]).filter((pane): pane is PaneVM => pane !== undefined).slice(0, 20);
}

export function getCardsForPane(state: DeckSnapshot, paneId: string): DeckCardVM[] {
  const pane = state.panes[paneId];
  if (!pane) {
    return [];
  }
  return [
    {
      id: `${pane.id}.card`, paneId: pane.id, title: pane.title, lod: pane.lod,
      status: pane.lod === "ghost" ? "ghost" : pane.lod === "focus" ? "hydrated" : "committed",
      risk: pane.risk, headline: pane.preview.headline,
    },
  ];
}
