// Classic+ Fruit verbs (SPEC §8.7, the cards-plus-d workstream): the Grapes a Grape card rolls (C+ #65
// Two Grapes, #66 Vine of Grapes; `GRAPE_ODDS`, R382), a draw whose card takes a price (C+ #65.2 Normal
// Grape, #65.3 Large Grape), a hit on an enemy that is a heal on a friend (the same two Grapes), and a
// hand replaced card for card by random cards (C+ #65.5 Mythic Grape).
//
// Each is card-specific — no Core card and no generic B5 system asks for any of them — so they live
// here, beside the effects library they are written in, and nowhere else.

import type { PlayerId } from "@jackioh/shared";
import { excludingDefId, query, type CatalogQueryArgs } from "../catalog";
import { GRAPE_ODDS } from "../config";
import { drawOne, type DrawOutcome } from "../draw";
import { numberedKeywordsOn } from "../numbers";
import type { Rng } from "../rng";
import type { Effect, EffectContext } from "../script";
import { findInstance, type CardInstance } from "../state";
import { moveToZone, reportGraveyardLanding } from "../zones";
import { addToHand } from "./addToHand";
import { setCostMod, setCostOverride } from "./cost";
import { damage } from "./damage";
import { heal } from "./heal";
import { playerOf, type PlayerSpec } from "./targets";

// ---------------------------------------------------------------------------------------------
// Grapes (C+ #65, #66)
// ---------------------------------------------------------------------------------------------

/**
 * R382, BUILD §2: one Grape, rolled by `GRAPE_ODDS` — one draw of the match rng over the percents'
 * sum, walked in the table's order. `lucky` extra rolls (§6.1 Lucky X) keep the best, and the best is
 * the later entry, since the table runs from worst to best (Rotten < Normal < Large < Golden < Mythic).
 * Returns the Grape's def id.
 */
export function rollGrape(rng: Rng, lucky = 0): string {
  const roll = (): number => {
    const total = GRAPE_ODDS.reduce((sum, grape) => sum + grape.percent, 0);
    let at = rng.int(total);
    for (let i = 0; i < GRAPE_ODDS.length; i += 1) {
      const grape = GRAPE_ODDS[i];
      if (grape === undefined) break;
      if (at < grape.percent) return i;
      at -= grape.percent;
    }
    return GRAPE_ODDS.length - 1;
  };
  const extra = Math.max(0, Math.trunc(lucky));
  const index = extra === 0 ? roll() : rng.lucky(extra, roll, (a, b) => Math.max(a, b));
  const grape = GRAPE_ODDS[index] ?? GRAPE_ODDS[GRAPE_ODDS.length - 1];
  return grape?.defId ?? "";
}

/**
 * §6.1: the Lucky X the card running the script has now — its running face's printed Lucky, as a
 * Degrade or an Upgrade has moved it (B3.4's X change), 0 without one.
 */
function luckyOf(ctx: EffectContext): number {
  const self = ctx.self;
  if (self === null) return 0;
  return numberedKeywordsOn(ctx.state, self).find((keyword) => keyword.key === "Lucky")?.value ?? 0;
}

/**
 * C+ #65 Two Grapes, #66 Vine of Grapes: "Add N Grapes to your hand, each rolled" — N independent
 * rolls (R60), each Grape created in the hand through §6.3's Add to hand, so a full hand burns it (§2.4,
 * R4). `radiant` makes every Grape Radiant. The Lucky of the card running the script applies to every
 * roll (the Radiant faces print Lucky 1), unless `lucky` names a number.
 *
 * Each Grape is rolled and added before the next is rolled, so a fixed seed gives fixed Grapes in a
 * fixed order (§10.7).
 */
export function addRolledGrapes(args: { count: number; radiant?: boolean; lucky?: number; player?: PlayerSpec }): Effect {
  return {
    kind: "addRolledGrapes",
    apply(ctx): void {
      const count = Math.max(0, Math.trunc(args.count));
      const lucky = args.lucky ?? luckyOf(ctx);
      for (let i = 0; i < count; i += 1) {
        if (ctx.state.result !== null) return;
        const defId = rollGrape(ctx.rng, lucky);
        if (defId === "") return;
        addToHand({
          defId,
          ...(args.player === undefined ? {} : { player: args.player }),
          ...(args.radiant === true ? { radiant: true } : {}),
        }).apply(ctx);
      }
    },
  };
}

// ---------------------------------------------------------------------------------------------
// A hit on an enemy, a heal on a friend (C+ #65.2, #65.3)
// ---------------------------------------------------------------------------------------------

/**
 * C+ #65.2 Normal Grape, #65.3 Large Grape: "Choose a Unit or hero. If it's an enemy, deal N damage to
 * it; if it's yours, heal it N." The chosen card or hero (the play's declared target, R81) is judged
 * as the effect resolves: a hero by its seat, a Unit by its controller. An enemy takes one §4.4 hit
 * from the card running the script (a Spell's, so Spell Damage raises it); a friend is healed (§6.3
 * Heal, R19). A target gone by then is nothing.
 */
export function damageEnemyOrHealFriend(args: { amount: number }): Effect {
  return {
    kind: "damageEnemyOrHealFriend",
    apply(ctx): void {
      const picked = ctx.targets[0];
      if (picked === undefined) return;
      let side: PlayerId | null = null;
      if (picked.pick === "hero") side = picked.player;
      else if (picked.pick === "instance") side = findInstance(ctx.state, picked.instanceId)?.controller ?? null;
      if (side === null) return;
      const spec = { of: "chosen" as const };
      if (side === ctx.controller) heal({ target: spec, amount: args.amount }).apply(ctx);
      else damage({ to: spec, amount: args.amount }).apply(ctx);
    },
  };
}

// ---------------------------------------------------------------------------------------------
// A draw whose card takes a price (C+ #65.2, #65.3)
// ---------------------------------------------------------------------------------------------

/**
 * The card one draw put in a hand, by the `drawn` event that draw made: the first `drawn` of that
 * player after `from` — a draw's own event comes before anything its card's cast draws (R58) — and only
 * when the draw ended with the card in a hand (`drawn`, or Infinite Reserves' Rush Token). A card cast
 * on draw never reaches the hand, a burned one is in the graveyard, a fatigue draw brings none and a
 * limited draw none either (§2.4, R4, R58, R457), so each of those has no card.
 */
export function cardThisDrawPutInHand(
  ctx: Pick<EffectContext, "state" | "events">,
  player: PlayerId,
  from: number,
  outcome: DrawOutcome,
): CardInstance | null {
  if (outcome !== "drawn" && outcome !== "token") return null;
  for (let at = from; at < ctx.events.length; at += 1) {
    const event = ctx.events[at];
    if (event?.type !== "drawn" || event.player !== player) continue;
    const card = findInstance(ctx.state, event.instanceId);
    return card !== undefined && card.zone.z === "hand" ? card : null;
  }
  return null;
}

/**
 * C+ #65.2 "Draw N. Each costs (1) less." / #65.3 "Draw N. Each costs (0).": ONE draw (§2.4 — "draw N"
 * is N of these, one effect each, so a draw whose cast-on-draw card asks pauses the rest of the list
 * and the answer makes the rest, R113), and the card that draw put in the hand takes the price: a
 * `costMod` that stacks with every other modifier, or a `costOverride` (R65). A card cast on draw, a
 * burned card, a fatigue draw and a limited draw take nothing (`cardThisDrawPutInHand`).
 */
export function drawPriced(args: { costMod?: number; costOverride?: number; player?: PlayerSpec }): Effect {
  return {
    kind: "drawPriced",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      const from = ctx.events.length;
      const outcome = drawOne(ctx, player);
      const card = cardThisDrawPutInHand(ctx, player, from, outcome);
      if (card === null) return;
      const target = { of: "instance" as const, instanceId: card.id };
      if (args.costOverride !== undefined) setCostOverride({ target, cost: args.costOverride, inHandOnly: true }).apply(ctx);
      if (args.costMod !== undefined) setCostMod({ target, amount: args.costMod, inHandOnly: true }).apply(ctx);
    },
  };
}

// ---------------------------------------------------------------------------------------------
// A hand replaced card for card (C+ #65.5)
// ---------------------------------------------------------------------------------------------

/**
 * C+ #65.5 Mythic Grape: "Replace your hand with random Mythic cards. They cost (0)." Every card in the
 * player's hand as this resolves goes to its owner's graveyard — moved, NOT discarded: no `discarded`
 * event, so nothing that answers a discard sees it; `enteredGraveyard` reports it, and a unit-token
 * card ceases to exist instead (R11) — and as many random cards of `query` arrive, repeats allowed
 * (R60), each created through §6.3's Add to hand with the riders given (Core #76 Field of Dreams'
 * reading of "replace your hand"). The card running the script is never one of them (R387). An empty
 * hand moves nothing and draws nothing from the rng (R129).
 */
export function replaceHandWithRandom(args: {
  query: CatalogQueryArgs;
  radiant?: boolean;
  costOverride?: number;
  player?: PlayerSpec;
}): Effect {
  return {
    kind: "replaceHandWithRandom",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      // A snapshot: each move splices the hand.
      const replaced = [...ctx.state.players[player].hand];
      if (replaced.length === 0) return;
      for (const card of replaced) {
        if (card.zone.z !== "hand") continue;
        const moved = moveToZone(ctx.state, card, "graveyard");
        reportGraveyardLanding(ctx, card, moved);
      }
      const pool = query(excludingDefId(args.query, ctx.self?.defId ?? ctx.defId));
      if (pool.length === 0) return;
      for (let i = 0; i < replaced.length; i += 1) {
        if (ctx.state.result !== null) return;
        const def = ctx.rng.pick(pool);
        if (def === undefined) return;
        addToHand({
          defId: def.id,
          ...(args.player === undefined ? {} : { player: args.player }),
          ...(args.radiant === true ? { radiant: true } : {}),
          ...(args.costOverride === undefined ? {} : { costOverride: args.costOverride }),
        }).apply(ctx);
      }
    },
  };
}
