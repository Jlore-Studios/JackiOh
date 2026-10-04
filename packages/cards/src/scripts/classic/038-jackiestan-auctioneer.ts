// C #38 Jackiestan Auctioneer (SPEC §8.6 row 38). Field Trap, Human, cost 2, Rare, 4/4 → 8/8 (its unit
// face).
//   Both faces: "Animated
//                Reveals when the cards a player has played in a turn reach {plays}: Summon this as a
//                Unit.
//                Once this has revealed: Whenever a player plays a card, draw {draw} and deal {damage}
//                damage to the enemy hero." — plays 3 on the base face and 2 on the Radiant, damage 2 and 4.
//
// R395: while it is face-down only the reveal condition is live. It answers the `cardPlayed` that
// takes any player's plays this turn to {plays} (the per-player per-turn count, which already counts
// the play under way; a cast counts, R70; a countered card was never played, R448, so it never
// reaches here), and then animates (Animated, B3.1, R383) in Attack Position into the unit zone in its
// own lane, else the leftmost open, unlocked, unreserved one (R64), summoning sick; with no open unit
// zone it stays face-up in its backrow zone. It remembers that it has revealed (`memory.activated`).
//
// From the next play on — never the play that set it off, as a permanent never answers its own arrival
// (R119) — every card either player plays makes its controller draw {draw} and deals one hit of
// {damage} from it to the enemy hero ("each enemy hero" is the multiplayer phrasing, R45), whether it is
// animated or stuck face-up in the backrow. A later firing, while it is still in the
// backrow and a unit zone has opened, animates it then — every firing of an Animated trap ends with
// its animating (B3.1 rule 4) — and one already a Unit stays put (R383).
//
// A Field Trap is never consumed. One trigger carries both texts, so one event can never be answered
// by both the activation and the "whenever" (R395). The conditions live in `when` (R99).

import type { GameEvent } from "@jackioh/shared";
import type { Effect, EffectContext, Script, TrapTrigger } from "@jackioh/engine";
import { cardsPlayedThisTurn, param, recalled } from "@jackioh/engine";
import { animate, damage, draw, remember } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-038");

/** What the Auctioneer keeps once it has revealed. */
const ACTIVATED = "activated";

type Played = Extract<GameEvent, { type: "cardPlayed" }>;

function hasActivated(ctx: EffectContext): boolean {
  return recalled(ctx, ACTIVATED) === true;
}

/** The answer this play gets: "activate" (the {plays}th play of a turn), "sale" (once activated), or none. */
function answer(ctx: EffectContext & { event: GameEvent }): "activate" | "sale" | null {
  const event = ctx.event;
  if (event.type !== "cardPlayed") return null;
  const played: Played = event;
  const reached = cardsPlayedThisTurn(ctx.state, played.player) === param(ctx, "plays");
  if (hasActivated(ctx)) return "sale";
  return reached ? "activate" : null;
}

function run(ctx: EffectContext & { event: GameEvent }): Effect[] {
  const kind = answer(ctx);
  if (kind === "activate") return [remember({ key: ACTIVATED, value: true }), animate()];
  if (kind === "sale") {
    // B3.1 rule 4: every firing of an Animated trap ends with its animating, so one stuck in the
    // backrow for want of a zone steps into a unit zone as soon as one is open; a Unit stays put.
    return [draw({ count: param(ctx, "draw") }), damage({ to: { of: "enemyHero" }, amount: param(ctx, "damage") }), animate()];
  }
  return [];
}

const auction: TrapTrigger = {
  id: "auctioneer",
  on: ["cardPlayed"],
  when: (ctx) => answer(ctx) !== null,
  run,
};

export const base: Script = { triggers: [auction] };

// The same script: the Radiant face's 2nd play and 4 damage are its declared numbers, which `param`
// reads off the face; its 8/8 body is printed on the catalog face.
export const radiant: Script = base;
