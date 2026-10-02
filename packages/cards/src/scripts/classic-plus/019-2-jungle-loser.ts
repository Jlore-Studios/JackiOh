// C+ #19.2 Jungle Loser (SPEC §8.7 row 19.2): end of turn, {chance}%: a forced attack on a random enemy
// Unit; a kill across from your Bot Loser sends it Berserk (Radiant: credits it the kill, R412).

import { cardAt, param, randomAttackTargets, slotsOf, type EffectContext, type GameState, type KillCredit, type Script } from "@jackioh/engine";
import { forEachCard, forcedAttackRandom, goBerserk, withKillCredit } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-019-2");

const BOT_LOSER = "classicplus-019-5";

/** The declared `chance` is a percentage. */
const PERCENT = 100;

/** §3.1: lane N faces lane N. Each Bot Loser of yours with the enemy Unit across from it. */
function acrossFromBotLosers(state: GameState, controller: PlayerId): KillCredit[] {
  const enemy = opponentOf(controller);
  return slotsOf(controller, "units").flatMap((ref) => {
    const bot = cardAt(state, ref);
    if (bot === null || bot.defId !== BOT_LOSER) return [];
    const victim = cardAt(state, { player: enemy, row: "units", lane: ref.lane });
    return victim === null ? [] : [{ victimId: victim.id, toId: bot.id }];
  });
}

/** "{chance}% chance": rolled only when an enemy Unit it may attack stands (R129). */
function attacks(ctx: EffectContext): string[] {
  const self = ctx.self;
  if (self === null || self.zone.z !== "field") return [];
  if (randomAttackTargets(ctx.state, self, "enemyUnits").length === 0) return [];
  return ctx.rng.chance(param(ctx, "chance") / PERCENT) ? [self.id] : [];
}

function jungle(transfer: boolean): Script {
  return {
    endOfTurn: () => [
      forEachCard({
        cards: attacks,
        each: (instanceId) =>
          withKillCredit({
            killer: { of: "instance", instanceId },
            pairs: (ctx) => acrossFromBotLosers(ctx.state, ctx.controller),
            transfer,
            during: forcedAttackRandom({ attacker: { of: "instance", instanceId }, among: "enemyUnits" }),
            // Base: the Bot Loser across from the kill goes Berserk. Radiant: the credit is the whole of it.
            ...(transfer ? {} : { then: (pair: KillCredit) => [goBerserk({ target: { of: "instance", instanceId: pair.toId } })] }),
          }),
      }),
    ],
  };
}

export const base: Script = jungle(false);

export const radiant: Script = jungle(true);
