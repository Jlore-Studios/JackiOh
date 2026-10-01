// How a verb names cards anywhere a player keeps them — on the field, in a hand, in a deck — which
// patch v0.2.0's instance-data verbs need (docs/classic-sets.md B3.3, B3.4, B5 E38, E39): Brittle
// given to a hand, "Upgrade every card in your hand and deck twice", "your Units on the field, in your
// hand and in your deck get +2X Attack and Rush", "Degrade 4 random cards in your opponent's deck".
//
// `targets.BoardScope` names cards on the field only, so this is the wider vocabulary, and each verb
// that takes it walks `cardsInCardScope`. A card dormant under a Stack is not on the field (§3.2,
// R13) and is never in a scope.
//
// The walk is in R242's order: the cards everyone reads first (a unit, a face-up backrow card), then
// the cards only their owner reads (a hand, a face-down trap: §9.1, R33), then the cards nobody reads
// (a deck, §3) — each group side by side (the active player's first, R68), and within a side in the
// zones' own order (units by lane, backrow by lane, hand order, deck top down). A verb that reports
// its changes one card at a time reports them in this order, so a hidden card's place among the
// events says only which group it was in (R242), never which zone.

import type { CardType, PlayerId, Row, Tag } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { defOf } from "../catalog";
import { cardTypeOf } from "../faces";
import { isFaceDown } from "../preview";
import type { EffectContext } from "../script";
import type { CardInstance, GameState } from "../state";
import { cardAt, slotsOf } from "../zones";
import { sidesOf } from "./targets";

/** The piles a card scope reaches: the field (both rows unless `rows` narrows it), a hand, a deck. */
export type CardZone = "field" | "hand" | "library";

export type CardScope = {
  /** Sides, relative to `ctx.controller`. Default "self". */
  side?: "self" | "enemy" | "any";
  /** Which piles. A hand and a deck are their owner's; the field is its controller's. */
  zones: CardZone[];
  /** Field rows. Default both. */
  rows?: Row[];
  /** Card types to keep, by the type the card has now (B2.7, `faces.cardTypeOf`). */
  types?: CardType[];
  tags?: Tag[];
  notTags?: Tag[];
  /** Leave out the card running the script. */
  excludeSelf?: boolean;
};

/** R242: who may read a card where it sits. */
export type Readers = "everyone" | "owner" | "nobody";

/** R242's group order, public first. */
const READER_ORDER: readonly Readers[] = ["everyone", "owner", "nobody"];

/** One card of a scope: where it is, who reads it there, and whether it passes the scope's filters. */
export type ScopedCard = { card: CardInstance; zone: CardZone; readers: Readers; matches: boolean };

/** R97, R33, §9.1: who may read a card where it sits now. */
export function readersOf(state: GameState, card: CardInstance): Readers {
  const zone = card.zone;
  if (zone.z === "library") return "nobody";
  if (zone.z === "hand") return "owner";
  if (zone.z === "field" && zone.row === "backrow" && isFaceDown(state, card)) return "owner";
  return "everyone";
}

/**
 * R177: the players who may not read a card where it sits — both for a deck card, the other player for
 * a hand card or a face-down trap — which an event about a change made there records (`hiddenFrom`),
 * so a view keeps it unread for good.
 */
export function unreadableBy(state: GameState, card: CardInstance): PlayerId[] {
  const readers = readersOf(state, card);
  if (readers === "nobody") return ["p1", "p2"];
  if (readers === "everyone") return [];
  const reader = card.zone.z === "hand" ? card.zone.player : card.controller;
  return [opponentOf(reader)];
}

/** Whether a card passes a scope's filters (type, tags, self). Where it is is the walk's business. */
export function matchesCardScope(ctx: EffectContext, card: CardInstance, scope: CardScope): boolean {
  if (scope.excludeSelf === true && ctx.self !== null && card.id === ctx.self.id) return false;
  if (scope.types !== undefined && !scope.types.includes(cardTypeOf(ctx.state, card))) return false;
  const tags = defOf(ctx.state, card.defId).tags;
  if (scope.tags !== undefined && !scope.tags.some((tag) => tags.includes(tag))) return false;
  if (scope.notTags !== undefined && scope.notTags.some((tag) => tags.includes(tag))) return false;
  return true;
}

function fieldCards(state: GameState, player: PlayerId, rows: readonly Row[]): CardInstance[] {
  return rows.flatMap((row) =>
    slotsOf(player, row).flatMap((ref) => {
      const card = cardAt(state, ref);
      return card === null ? [] : [card];
    }),
  );
}

/**
 * Every card the scope reaches, in R242's order (this file's header). `wholeHiddenPiles` keeps every
 * card the scope reaches that someone may not read — a hand's, a deck's, a face-down trap — the ones
 * its filters reject marked `matches: false`: a
 * verb that reports each card of a hidden pile it changes must cue the whole pile, or the count of
 * its cues would tell the other player how many cards there passed the filter (R440). Otherwise the
 * walk keeps the matching cards only.
 */
export function cardsInCardScope(
  ctx: EffectContext,
  scope: CardScope,
  options: { wholeHiddenPiles?: boolean } = {},
): ScopedCard[] {
  const state = ctx.state;
  const rows = scope.rows ?? ["units", "backrow"];
  const out: ScopedCard[] = [];
  for (const player of sidesOf(ctx, scope.side ?? "self")) {
    const side = state.players[player];
    const piles: [CardZone, CardInstance[]][] = [
      ["field", scope.zones.includes("field") ? fieldCards(state, player, rows) : []],
      ["hand", scope.zones.includes("hand") ? [...side.hand] : []],
      ["library", scope.zones.includes("library") ? [...side.library] : []],
    ];
    for (const [zone, cards] of piles) {
      for (const card of cards) {
        const matches = matchesCardScope(ctx, card, scope);
        const readers = readersOf(state, card);
        if (!matches && !(options.wholeHiddenPiles === true && readers !== "everyone")) continue;
        out.push({ card, zone, readers, matches });
      }
    }
  }
  return READER_ORDER.flatMap((readers) => out.filter((entry) => entry.readers === readers));
}
