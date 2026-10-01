// C+ #35 Rollback (SPEC §8.7 row 35; R419, R562, R563, R566): N — 1, 2 or 3, declared with the play
// (R81) — and the board goes back to how it was at the start of the player-turn N before this one; the
// Radiant face also declares the part: your side, your opponent's side or both. The history and the
// restore are E29's (`subsystems/boardHistory.ts`).
import { ROLLBACK_MAX_TURNS, type EffectContext, type Script } from "@jackioh/engine";
import { chosenNumber, rollBack } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-035");

const TURNS = { kind: "number" as const, options: Array.from({ length: ROLLBACK_MAX_TURNS }, (_, n) => String(n + 1)) };

/** The Radiant face's parts of the board, as its play names them. */
const SIDES = { "your side": "self", "your opponent's side": "enemy", "both sides": "both" } as const;

function back(ctx: EffectContext, sides: "self" | "enemy" | "both" | undefined) {
  const turnsAgo = chosenNumber(ctx);
  return turnsAgo === undefined || sides === undefined ? [] : [rollBack({ turnsAgo, sides })];
}

export const base: Script = {
  modes: [TURNS],
  cry: (ctx) => back(ctx, "both"),
};

export const radiant: Script = {
  modes: [TURNS, { kind: "mode", options: Object.keys(SIDES) }],
  cry: (ctx) => back(ctx, SIDES[ctx.modes[1] as keyof typeof SIDES]),
};
