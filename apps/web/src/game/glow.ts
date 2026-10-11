// The two glows as DOM attributes (docs/polish/7-mobile-ux.md S5, S6), and Plague Chalice's warning
// beside them (R667).
//
// Green is `data-glow="ready"`: the element's testid is in `Highlight.glow`, which `highlightFor`
// derived from `legalActions` alone. Yellow is `data-condition-active="true"`: the engine put
// `conditionActive: true` on the card's view (R195, CLAUDE.md rule 7). Neither is decided here;
// these helpers only turn a set membership or a view flag into an attribute value, and return
// `undefined` so React omits the attribute rather than writing "false". `highlights.css` paints
// them.

import type { CardView } from "@jackioh/shared";

import { testid, type Highlight } from "./contract.ts";

export type GlowAttr = "ready";

/** `data-glow`: "ready" when `testId` is in `highlight.glow`; undefined (attribute omitted) otherwise. */
export function glowAttr(highlight: Highlight | undefined, testId: string | undefined): GlowAttr | undefined {
  if (highlight === undefined || testId === undefined) return undefined;
  return highlight.glow?.has(testId) === true ? "ready" : undefined;
}

/** `data-condition-active`: "true" when the view says so; undefined otherwise. */
export function conditionAttr(card: Pick<CardView, "conditionActive"> | null | undefined): "true" | undefined {
  return card?.conditionActive === true ? "true" : undefined;
}

/**
 * MD-D5, R1121: `data-condition-active` from the drag layer — "true" when `testId` is in
 * `highlight.condition`, which `highlightFor` filled from the dragged or selected attacker's
 * `conditionTargets`; undefined otherwise.
 */
export function conditionTargetAttr(
  highlight: Highlight | undefined,
  testId: string | undefined,
): "true" | undefined {
  if (highlight === undefined || testId === undefined) return undefined;
  return highlight.condition?.has(testId) === true ? "true" : undefined;
}

/**
 * R667: what the warning on a hand card says, in its tooltip, its inspect note and to a screen reader.
 * Classic #87 Plague Chalice is the one card whose counter the engine reads ahead (`wouldCounter`).
 */
export const COUNTERED_NOTE = "Plague Chalice would counter this if you played it now";

/**
 * `data-countered-on-play`: "true" when the engine put `counteredOnPlay: true` on the hand card's
 * view (R667); undefined otherwise. Like the yellow glow it is drawn, never decided, here.
 */
export function counteredAttr(card: Pick<CardView, "counteredOnPlay"> | null | undefined): "true" | undefined {
  return card?.counteredOnPlay === true ? "true" : undefined;
}

/** Prefixes of the glowing testids that are a move: a play from the graveyard (B5 E11), an Activate control or a further Heroic Power (R384). */
const MOVE_PREFIXES: readonly string[] = [
  "hand-card-",
  "card-",
  testid.pilePlay(""),
  testid.activate(""),
  testid.powerOf(""),
];

/**
 * True when `glow` holds a playable card (in the hand, or "Play" in the graveyard pile), an attacker,
 * an Activate control or a Heroic Power: any testid starting `hand-card-`, `card-`, `pile-play-`,
 * `activate-` or `power-`, or `power`.
 */
export function hasMovesLeft(highlight: Highlight | undefined): boolean {
  const glow = highlight?.glow;
  if (glow === undefined) return false;
  for (const id of glow) {
    if (id === testid.power || MOVE_PREFIXES.some((prefix) => id.startsWith(prefix))) return true;
  }
  return false;
}
