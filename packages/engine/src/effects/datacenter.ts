// The AI generated cards' verbs (SPEC §8.7 rows T-AI-4 and T-AI-6, the cards-plus-d workstream): a draw
// that repeats while the card it brought is cheap (T-AI-4 Chain of Thought), and a sweep of Field Spells
// that hits the heroes once per Field Spell it dooms (T-AI-6 Datacenter Fire).
//
// Each is card-specific — no Core card and no generic B5 system asks for either — so they live here,
// beside the effects library they are written in.

import { opponentOf, PLAYER_IDS, type PlayerId } from "@jackioh/shared";
import { drawOne } from "../draw";
import { cardTypeOf } from "../faces";
import { unitHas } from "../layers";
import { effectiveCost } from "../mana";
import { unaffectedBy } from "../restrictions";
import type { Effect, EffectContext } from "../script";
import type { CardInstance } from "../state";
import { cardAt, slotsOf } from "../zones";
import { damage } from "./damage";
import { destroyAll } from "./destroy";
import { cardThisDrawPutInHand } from "./fruit";
import { sidesOf } from "./targets";

// ---------------------------------------------------------------------------------------------
// T-AI-4 Chain of Thought
// ---------------------------------------------------------------------------------------------

/**
 * T-AI-4: "Draw 1. If it costs (`maxCost`) or less, repeat this, up to `repeats` more times." Each
 * round is one §2.4 draw; "it" is the card THAT draw put in the hand (`cardThisDrawPutInHand`, R596),
 * priced as it arrives (R65's current cost: an X-cost card reads 0). A card cast on draw never gets
 * there (R58) — not even the card the cast's own repeat then brings (R596) — a burned one isn't
 * there, and a fatigue or limited draw brings none, so each ends the chain; so does the end of the game
 * (R216). A draw can pause only on a cast-on-draw card that asks (R158), whose draw has then ended the
 * chain already: its cast parks its own remainder (R113), and nothing of this chain is left to owe.
 */
export function drawWhileCheap(args: { maxCost: number; repeats: number; player?: PlayerId }): Effect {
  return {
    kind: "drawWhileCheap",
    apply(ctx): void {
      const player = args.player ?? ctx.controller;
      for (let round = 0; round <= args.repeats; round += 1) {
        if (ctx.state.result !== null) return;
        const before = ctx.state.pending;
        const from = ctx.events.length;
        const card = cardThisDrawPutInHand(ctx, player, from, drawOne(ctx, player));
        if (card === null || ctx.state.pending !== before) return;
        if (effectiveCost(ctx.state, card) > args.maxCost) return;
      }
    },
  };
}

// ---------------------------------------------------------------------------------------------
// T-AI-6 Datacenter Fire
// ---------------------------------------------------------------------------------------------

/** Whose Field Spells a sweep takes, relative to the card running it. */
export type FieldSpellSide = "any" | "enemy";

/** The reader a doomed count needs: the card running it (a Spell's effects pass a Spell-immune card by). */
type SweepReader = Pick<EffectContext, "state" | "self" | "defId" | "radiant" | "controller">;

/**
 * T-AI-6: the Field Spells "destroy all (enemy) Field Spells" would destroy now — every Field Spell in a
 * backrow zone of those sides (the acting card of each zone, so an Ivory Tower beneath the Unit it
 * holds counts, R418, and a card dormant under a backrow pile does not, §3.2), except an Indestructible
 * one (R46) and one unaffected by the running Spell (B5 E35). Traps and Field Traps are no Field
 * Spells, and an animated one standing in a unit zone is a Unit there (R383), so not one (R588). A pure read, so a card's
 * `preview` (R280) and its resolution count the same cards.
 */
export function fieldSpellsDoomed(reader: SweepReader, side: FieldSpellSide): CardInstance[] {
  const sides: readonly PlayerId[] = side === "enemy" ? [opponentOf(reader.controller)] : PLAYER_IDS;
  return sides.flatMap((player) =>
    slotsOf(player, "backrow").flatMap((ref) => {
      const card = cardAt(reader.state, ref);
      if (card === null || cardTypeOf(reader.state, card) !== "Field Spell") return [];
      return unitHas(reader.state, card, "Indestructible") || unaffectedBy(reader, card) ? [] : [card];
    }),
  );
}

/**
 * T-AI-6 Datacenter Fire: "Destroy all Field Spells. Deal `damagePer` damage to each hero for each one
 * destroyed" (Radiant: the enemy's Field Spells, and the enemy hero). Every Field Spell of those sides is
 * marked destroyed (§6.3 Destroy over a backrow scope, so §4.5's check collects them together, R59, and
 * fires their Death), and the ones the mark will take (`fieldSpellsDoomed`, read as the sweep lands, as
 * C+ #12.6's "each one destroyed" is, R408) set the hit: one §4.4 instance per hero of `damagePer` times
 * that count, from the running Spell (so Spell Damage raises it once and a per-hit cap caps it once), in
 * R68's order. None doomed, no damage.
 */
export function destroyFieldSpellsAndHit(args: { side: FieldSpellSide; damagePer: number }): Effect {
  return {
    kind: "destroyFieldSpellsAndHit",
    apply(ctx): void {
      const doomed = fieldSpellsDoomed(ctx, args.side).length;
      destroyAll({ side: args.side, rows: ["backrow"], types: ["Field Spell"] }).apply(ctx);
      if (doomed === 0) return;
      for (const player of args.side === "enemy" ? [opponentOf(ctx.controller)] : sidesOf(ctx, "any")) {
        if (ctx.state.result !== null) return;
        damage({ to: { of: player === ctx.controller ? "selfHero" : "enemyHero" }, amount: doomed * args.damagePer }).apply(ctx);
      }
    },
  };
}
