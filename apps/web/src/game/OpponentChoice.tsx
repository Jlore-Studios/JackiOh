// R2739000: the other seat's card picker as the viewer sees it, when the view says how many options it
// offers (`PendingElsewhereView.optionCount`): one card back per option, the back the opponent's hand wears
// (R1834004), laid out as the chooser's bare picker (#2738) over a light dim, with no panel, words or
// control. That count is all it says: the client draws it only when the view carries it, and never asks
// whether it may (CLAUDE.md rule 7). R2739001: the option the opponent's pointer or focus is on, as last
// relayed, is outlined in the opponent's color, as R738 marks the card they aim with. Nothing here takes a
// hit, so the board under it stays live (prompt.css, which Prompt.tsx loads).

import type { ReactElement } from "react";

import type { PlayerId, PlayerView } from "@jackioh/shared";

import { CardBack } from "../cards/index.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { useSeatDeckCosmetics } from "./wearer.ts";

export type OpponentChoiceProps = {
  view: PlayerView;
  /** The seat choosing, always the viewer's opponent (a prompt for the viewer is `forYou`). */
  pendingFor: PlayerId;
  /** R2739000: the view's `optionCount`. */
  count: number;
  /** R2739001: the option the opponent hovers as last relayed, or null. */
  hovered: number | null;
};

export default function OpponentChoice({ view, pendingFor, count, hovered }: OpponentChoiceProps): ReactElement {
  useLocale();
  const { cardBack } = useSeatDeckCosmetics(view.opponent.player);
  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      {/* Prompt.tsx's `Waiting` contract (`prompt-modal`, `data-prompt-waiting`, no `data-prompt-kind`), so
          nothing that waits on a prompt of the viewer's takes it for one. */}
      <div
        className="prompt prompt-opponent-choice"
        data-testid="prompt-modal"
        data-prompt-waiting="waiting"
        data-pending-for={pendingFor}
        data-prompt-layout="cards"
        data-bare="true"
        role="dialog"
        aria-modal="true"
        aria-label={t("Your opponent is choosing…")}
      >
        <div className="prompt-cards" data-testid="opponent-choice">
          {Array.from({ length: count }, (_, at) => (
            <span
              key={at}
              className="prompt-card prompt-card--back"
              data-testid={`opponent-choice-${at}`}
              data-hovered={hovered === at ? "true" : undefined}
            >
              <CardBack back={cardBack} />
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}
