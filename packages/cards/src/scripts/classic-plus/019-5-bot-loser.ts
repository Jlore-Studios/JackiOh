// C+ #19.5 Bot Loser (SPEC §8.7 row 19.5): +{attackGain} Attack whenever it is R42's killer; while
// Berserk it attacks its own hero at your start and end of turn; the Radiant face can't go Berserk (R412).

import { isBerserk, param, type EffectContext, type Effect, type Script, type TriggerDef } from "@jackioh/engine";
import { buff, forcedAttackOwnHero } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-019-5");

/** "Whenever this destroys a Unit, it gets +{attackGain} Attack." (`when` is read for traps only, so the test is in `run`.) */
const onKill: TriggerDef = {
  id: "bot-loser-kill",
  on: ["destroyed"],
  run: (ctx) =>
    ctx.event.type === "destroyed" && ctx.self !== null && ctx.event.killerId === ctx.self.id
      ? [buff({ target: { of: "self" }, attack: param(ctx, "attackGain") })]
      : [],
};

/** "While Berserk: At the start and end of your turn, this attacks your hero." */
function berserkAttack(ctx: EffectContext): Effect[] {
  return ctx.self !== null && isBerserk(ctx.self) ? [forcedAttackOwnHero({ attacker: { of: "self" } })] : [];
}

export const base: Script = {
  triggers: [onKill],
  startOfTurn: berserkAttack,
  endOfTurn: berserkAttack,
  conditionMet: (ctx) => ctx.zone === "field" && isBerserk(ctx.self),
};

/** "This can't go Berserk": no flag, and no Berserk attacks printed (R412). */
export const radiant: Script = {
  triggers: [onKill],
  staticFlags: { neverBerserk: true },
};
