// #2267: the hand hints, one list in drawing order. A hint reads the card's `CardView`
// flags (`card.tradable`, `card.tribute`, SPEC §10.8), the `PlayerView` and the card's face the way
// `Card.tsx` reads it (`liveFace`: `card.keywords` where the view gives them, else the printed
// face's keywords, B5 E38, R243 — exactly what the card shows) and nothing else (CLAUDE.md rule 7,
// SPEC §10.8). Adding a hint is one entry here plus its icon in `cards/icons.tsx`. The list ships the
// Tradable pip (#2269, #2574, R2574000) and the Tribute pip (#2268).

import type { CardView, PlayerView } from "@jackioh/shared";

import type { FaceModel, IconName } from "../cards/index.ts";
import { t } from "../i18n/i18n.ts";

export type HintShown = {
  /** One short number the pip prints, when the hint has one. */
  n?: number;
  /** Lower opacity and no color (handHints.css). */
  dimmed?: boolean;
  /** The pip's name for a screen reader, already through `t()`: the hint calls `t()` itself, at render, never at module scope. */
  label: string;
};

export type HandHint = {
  /** Unique in the list: the pip's `data-hint` and React key. */
  id: string;
  icon: IconName;
  /** Null when the hint does not apply to this card. */
  applies: (card: CardView, view: PlayerView, face: FaceModel) => HintShown | null;
};

/**
 * #2269, #2574, R2574000: the Tradable pip (R1838000). A passive mark, no number and no motion,
 * so a player can see which cards in a big hand are Tradable without pointing at each one.
 * Reads `card.tradable` from the engine (§10.8), which includes printed, granted, and aura-lent
 * Tradable cards; the client works out no aura itself (CLAUDE.md rule 7). Dimmed once the
 * turn's Trade is spent (`PlayerView.trades.left === 0`): only an explicit `left === 0` dims,
 * so a view that simply carries no trade state never dims the pip. Never counts trades and never
 * reads a rule here; the Trade bar stays the way to Trade.
 */
const TRADABLE: HandHint = {
  id: "tradable",
  icon: "trade",
  applies: (card, view) => {
    if (!card.tradable) return null;
    return view.trades?.left === 0
      ? { dimmed: true, label: t("No Trades left this turn") }
      : { label: t("Tradable") };
  },
};

/** #2268, R2268000: how many Tributes a play of the card asks now, the engine's number (`CardView.tribute`, SPEC §6.3, §10.8), never worked out here (CLAUDE.md rule 7). Players read Sacrifice as Tribute (R373). */
export const TRIBUTE_HINT: HandHint = {
  id: "tribute",
  icon: "altar",
  applies: (card) => (card.tribute === undefined ? null : { n: card.tribute, label: t("Tribute") }),
};

/** The shipped hints, in drawing order. */
export const HAND_HINTS: readonly HandHint[] = [TRADABLE, TRIBUTE_HINT];

export function hintsFor(
  card: CardView,
  view: PlayerView,
  face: FaceModel,
  hints: readonly HandHint[] = HAND_HINTS,
): { hint: HandHint; shown: HintShown }[] {
  return hints.flatMap((hint) => {
    const shown = hint.applies(card, view, face);
    return shown === null ? [] : [{ hint, shown }];
  });
}
