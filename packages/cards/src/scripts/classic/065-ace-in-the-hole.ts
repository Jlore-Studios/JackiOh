// C #65 Ace in the Hole (SPEC §8.6 row 65; §2.2, §6.3 Recruit; R33, R62, R78, R99, R386). Trap, cost 2,
// Common.
//   Base:    "End of your turn: Flip a coin. On heads, this activates: Recruit {recruits|card|cards}." (1)
//   Radiant: "Revealed. End of your turn: Flip a coin. On tails, Recruit a card. On heads, this
//            activates: Recruit {recruits|card|cards}." (3)
//   Engine:  one seeded coin at each end of its controller's turn. Heads fires it in the end-of-turn trap
//            window (R62; consumed); Recruit scans the deck top down for a permanent, "3 cards" three
//            scans. The Radiant's tails recruits one without firing: the trap stays set, face-down.
//
// The coin is the `endOfTurn` hook's: a backrow card answers it at its controller's end of turn alone,
// in the end-of-turn triggers, which R62 runs before the trap window. The hook flips once, remembers a
// heads on the instance under the turn's number, and on the Radiant's tails recruits there and then —
// no `trapFired`, so the trap stays face-down. The window's check (R99) only reads that memory, so no
// predicate draws from the rng, and a memory from another turn or another stay (R78) arms nothing.

import { param, recalled, type Script, type TriggerDef } from "@jackioh/engine";
import { recruit, remember, reveal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-065");

const HEADS = "headsOnTurn";

const fire: TriggerDef = {
  id: "ace-in-the-hole",
  on: ["turnEnded"],
  when: (ctx) => ctx.event.type === "turnEnded" && ctx.event.player === ctx.controller && recalled(ctx, HEADS) === ctx.state.turn,
  run: (ctx) => [recruit({ count: param(ctx, "recruits") })],
};

function flip(onTails: boolean): Script["endOfTurn"] {
  return (ctx) => {
    if (ctx.rng.coin()) return [remember({ key: HEADS, value: ctx.state.turn })];
    return onTails ? [recruit()] : [];
  };
}

export const base: Script = { endOfTurn: flip(false), triggers: [fire] };

/**
 * The Radiant face is Revealed regardless of the coin flip (balance patch 1, R658): the end of turn
 * shows its face to both players first, then flips as usual. Revealed is not face-up, so a tails
 * that recruits without firing leaves the trap armed and still answering.
 */
export const radiant: Script = {
  endOfTurn: (ctx) => {
    const tails = flip(true)?.(ctx) ?? [];
    return [reveal(), ...tails];
  },
  triggers: [fire],
};
