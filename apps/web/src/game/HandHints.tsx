// #2267: the hand hints' row on one of the viewer's own hand cards. It draws the hints
// handHints.ts lists, in that order, and nothing at all when none applies. Each hint reads the
// card's face the way `Card.tsx` reads it (`liveFace`: the view plus the public catalog only,
// CLAUDE.md rule 7), so a hint sees the keywords the card shows.

import type { ReactElement } from "react";

import type { CardView, PlayerView } from "@jackioh/shared";

import { Icon } from "../cards/index.ts";
import { useLocale } from "../i18n/i18n.ts";
import { useCardInfo, useCopiedDef } from "./catalog.ts";
import { liveFace } from "./faces.ts";
import { HAND_HINTS, hintsFor, type HandHint } from "./handHints.ts";
import "./handHints.css";

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
  // The same face `Card.tsx` draws: the definition from the public catalog (or the match-made
  // defs, or nothing before the catalog loads), with whatever the view says about the card.
  const info = useCardInfo(card.defId, card.radiant);
  const copied = useCopiedDef(card);
  const face = liveFace(info, card, copied === undefined ? {} : { copied });
  const shown = hintsFor(card, view, face, hints);
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
