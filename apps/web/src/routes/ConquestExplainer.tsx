// Conquest first-visit walkthrough explainer (SPEC §9.5, §9.10; R330–R334, R604).

import type { ReactElement } from "react";

import { conquestText } from "./conquestCopy.ts";

export const conquestExplainerTestid = {
  explainer: "conquest-explainer",
  dismiss: "conquest-explainer-dismiss",
  list: "conquest-explainer-list",
  item: (key: string): string => `conquest-explainer-item-${key}`,
} as const;

export type ConquestExplainerProps = {
  onDismiss: () => void;
};

export default function ConquestExplainer({ onDismiss }: ConquestExplainerProps): ReactElement {
  return (
    <section
      className="lobby-card play-panel series-panel conquest-explainer"
      data-testid={conquestExplainerTestid.explainer}
      aria-label={conquestText("explainerTitle")}
    >
      <div className="conquest-explainer__header">
        <h2 className="play-panel__heading">{conquestText("explainerTitle")}</h2>
        <button
          type="button"
          className="link-button conquest-explainer__dismiss"
          data-testid={conquestExplainerTestid.dismiss}
          onClick={onDismiss}
        >
          {conquestText("dismiss")}
        </button>
      </div>
      <ul className="conquest-explainer__list" data-testid={conquestExplainerTestid.list}>
        <li data-testid={conquestExplainerTestid.item("wins")}>{conquestText("winsNeeded")}</li>
        <li data-testid={conquestExplainerTestid.item("locks")}>{conquestText("lockOnWin")}</li>
        <li data-testid={conquestExplainerTestid.item("sealed")}>{conquestText("sealedPicks")}</li>
        <li data-testid={conquestExplainerTestid.item("draws")}>{conquestText("draws")}</li>
        <li data-testid={conquestExplainerTestid.item("concede")}>{conquestText("concedeVsForfeit")}</li>
        <li data-testid={conquestExplainerTestid.item("rating")}>{conquestText("ratingMovesOnce")}</li>
      </ul>
    </section>
  );
}
