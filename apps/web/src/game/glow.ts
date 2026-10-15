// The two glows as DOM attributes (docs/polish/7-mobile-ux.md S5, S6), and Plague Chalice's warning
// beside them (R667).
//
// Green is `data-glow="ready"`: the element's testid is in `Highlight.glow`, which `highlightFor`
// derived from `legalActions` alone. Yellow is `data-condition-active="true"`: the engine put
// `conditionActive: true` on the card's view (R195, CLAUDE.md rule 7). Neither is decided here;
// these helpers only turn a set membership or a view flag into an attribute value, and return
// `undefined` so React omits the attribute rather than writing "false". `highlights.css` paints
// them. A third mark, `data-counted="true"`, rings the cards a hand card's preview names (#991, R583),
// read off the view like the yellow.

import type { CardView, PlayerView } from "@jackioh/shared";

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
 * #991, R583: the testids (`card-<id>`) of the cards a hand card's preview counts, every value's
 * `ids` (C+ #44, #45's Radiant "highlight targets"), for the viewer's hand card under the pointer
 * (`hovered`), else the one `highlight.selected` holds. The engine lists only cards the viewer may
 * read (R1447) and a back has no testid, so nothing hidden is marked. Nothing is decided here.
 */
export function countedTestids(view: PlayerView, highlight: Highlight, hovered: string | null): ReadonlySet<string> {
  const counted = new Set<string>();
  const hand = view.you.hand;
  if (!Array.isArray(hand)) return counted;
  const card =
    hand.find((entry) => entry.instanceId === hovered) ??
    hand.find((entry) => highlight.selected.has(testid.handCard(entry.instanceId)));
  for (const value of card?.preview ?? []) {
    for (const id of value.ids ?? []) counted.add(testid.card(id));
  }
  return counted;
}

/** #991: `data-counted`: "true" when `testId` is in `highlight.counted`; undefined otherwise. */
export function countedAttr(highlight: Highlight | undefined, testId: string | undefined): "true" | undefined {
  if (highlight === undefined || testId === undefined) return undefined;
  return highlight.counted?.has(testId) === true ? "true" : undefined;
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
