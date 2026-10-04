// T-AI-8 Rate Limit (SPEC §8.7 row T-AI-8, §7, B8). (1) Trap, AI, Token.
//   Base:    "Reveals when your opponent plays their 3rd card in a turn: After it resolves, their
//            turn ends."
//   Radiant: "… their 2nd card …"
//   Engine:  "Counts the opponent's plays that turn (`turnLog.cardsPlayed`, casts included, R70; a
//            countered card is never played) and fires as the 3rd (Radiant 2nd) is played
//            (`cardPlayed`, §10.5 step 4), and on that play only, going to the graveyard as it fires
//            (§3.2). The play that set it off resolves first (§10.5 step 7); then End the turn (§6.3,
//            `turnCutShort`): the rest of that effect list resolves and the turn ends as if they had
//            pressed End turn, every end-of-turn step running. Tunes: none."
//
// The condition is `when` (R99): any other play leaves it armed and face-down. §10.5 step 4 counts the
// play before it emits `cardPlayed`, so `cardsPlayedThisTurn` already counts the one that woke it, and
// "that play only" is the count being exactly N — on their own turn, the only turn of theirs there is to
// end (casts they make on yours leave it set). E10's `endTurn` on the opponent puts R456's rider on
// their turn, which ends once everything their action set off has resolved, the play's Cry included.

import type { Script, TrapTrigger } from "@jackioh/engine";
import { cardsPlayedThisTurn } from "@jackioh/engine";
import { endTurn } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-08");

/** §8.7: "their 3rd card", Radiant "their 2nd". An AI card declares no params (B8). */
const NTH_PLAY = { base: 3, radiant: 2 } as const;

function rateLimit(nth: number): TrapTrigger {
  return {
    id: "rate-limit",
    on: ["cardPlayed"],
    when: ({ event, state, controller }) =>
      event.type === "cardPlayed" &&
      event.player !== controller &&
      state.active === event.player &&
      cardsPlayedThisTurn(state, event.player) === nth,
    run: () => [endTurn({ player: "enemy" })],
  };
}

export const base: Script = { triggers: [rateLimit(NTH_PLAY.base)] };

export const radiant: Script = { triggers: [rateLimit(NTH_PLAY.radiant)] };
