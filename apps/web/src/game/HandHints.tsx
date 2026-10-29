// #2267: the hand hints' row on one of the viewer's own hand cards. It draws the hints
// handHints.ts lists, in that order, and nothing at all when none applies.

import type { ReactElement } from "react";

import type { CardView, PlayerView } from "@jackioh/shared";

import { Icon } from "../cards/index.ts";
import { useLocale } from "../i18n/i18n.ts";
import { HAND_HINTS, hintsFor, type HandHint } from "./handHints.ts";
import "./handHints.css";

/** The testid of a hand card's hint row. */
export function handHintsTestid(instanceId: string): string {
  return `hand-hints-${instanceId}`;
}

export default function HandHints({
  card,
  view,
  hints = HAND_HINTS,
}: {
  card: CardView;
  view: PlayerView;
  hints?: readonly HandHint[] | undefined;
}): ReactElement | null {
  // A language change redraws the labels, as CardFace's does.
  useLocale();
  const shown = hintsFor(card, view, hints);
  if (shown.length === 0) return null;
  return (
    <span className="hand-hints" data-testid={handHintsTestid(card.instanceId)}>
      {shown.map(({ hint, shown: pip }) => (
        <span
          key={hint.id}
          className="hand-hint"
          data-hint={hint.id}
          data-dimmed={pip.dimmed === true ? "true" : undefined}
          role="img"
          aria-label={pip.label}
        >
          <Icon name={hint.icon} />
          {pip.n === undefined ? null : (
            <span className="hand-hint-n" aria-hidden="true">
              {pip.n}
            </span>
          )}
        </span>
      ))}
    </span>
  );
}
