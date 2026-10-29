// #2267: the hand hints, one list in drawing order. A hint reads the card's `CardView`
// and the `PlayerView` and nothing else (CLAUDE.md rule 7, SPEC §10.8). Adding a hint is
// one entry here plus its icon in `cards/icons.tsx`. The list ships empty.

import type { CardView, PlayerView } from "@jackioh/shared";

import type { IconName } from "../cards/index.ts";

export type HintShown = {
  /** One short number the pip prints, when the hint has one. */
  n?: number;
  /** Lower opacity and no colour (handHints.css). */
  dimmed?: boolean;
  /** The pip's name for a screen reader, already through `t()`: the hint calls `t()` itself, at render, never at module scope. */
  label: string;
};

export type HandHint = {
  /** Unique in the list: the pip's `data-hint` and React key. */
  id: string;
  icon: IconName;
  /** Null when the hint does not apply to this card. */
  applies: (card: CardView, view: PlayerView) => HintShown | null;
};

/** The shipped hints, in drawing order. */
export const HAND_HINTS: readonly HandHint[] = [];

/** The hints that apply to `card`, in the list's order, each with what it shows. */
export function hintsFor(
  card: CardView,
  view: PlayerView,
  hints: readonly HandHint[] = HAND_HINTS,
): { hint: HandHint; shown: HintShown }[] {
  return hints.flatMap((hint) => {
    const shown = hint.applies(card, view);
    return shown === null ? [] : [{ hint, shown }];
  });
}
