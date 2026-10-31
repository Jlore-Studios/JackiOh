// Glows and card attribute helpers (R195, R583, R667, #991, CLAUDE.md rule 7).
// Translates Highlight and view flags to DOM data attributes without rules decisions.

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

/** R1121: data-condition-active from highlight.condition targets. */
export function conditionTargetAttr(
  highlight: Highlight | undefined,
  testId: string | undefined,
): "true" | undefined {
  if (highlight === undefined || testId === undefined) return undefined;
  return highlight.condition?.has(testId) === true ? "true" : undefined;
}

/** #991, R583, R1447: counted card testids for previewed hand card (#44, #45). */
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

/** R667: warning tooltip text for countered-on-play cards like Plague Chalice (#87). */
export const COUNTERED_NOTE = "Plague Chalice would counter this if you played it now";

/** R667: data-countered-on-play from view counteredOnPlay flag. */
export function counteredAttr(card: Pick<CardView, "counteredOnPlay"> | null | undefined): "true" | undefined {
  return card?.counteredOnPlay === true ? "true" : undefined;
}

/** R1844001: data-pareto-optimal from view paretoOptimal flag. */
export function paretoAttr(card: Pick<CardView, "paretoOptimal"> | null | undefined): "true" | undefined {
  return card?.paretoOptimal === true ? "true" : undefined;
}

/** Prefixes of the glowing testids that are a move: a play from the graveyard (B5 E11), an Activate control or a further Heroic Power (R384). */
const MOVE_PREFIXES: readonly string[] = [
  "hand-card-",
  "card-",
  testid.pilePlay(""),
  testid.activate(""),
  testid.powerOf(""),
];

/** True when glow includes an actionable move testid (play, attack, activate, power). */
export function hasMovesLeft(highlight: Highlight | undefined): boolean {
  const glow = highlight?.glow;
  if (glow === undefined) return false;
  for (const id of glow) {
    if (id === testid.power || MOVE_PREFIXES.some((prefix) => id.startsWith(prefix))) return true;
  }
  return false;
}
