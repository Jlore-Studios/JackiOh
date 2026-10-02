// Fixture cards for a kill credited to another unit (R42, R412: `effects/killCredit.ts`) and for
// `isCastOnDraw` (R58: `castOnDrawNow.ts`), shaped like Classic+ #19.2 Jungle Loser, #19.5 Bot Loser
// and #26 Tommy Tempo. The engine never imports `packages/cards` (CLAUDE.md). Ids `kc-…`, indexed from
// 5800 so they collide with no other fixture file.

import type { CardDef } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { isCastOnDraw } from "../../src/castOnDrawNow";
import { buff, forcedAttackRandom, goBerserk, remember, withKillCredit } from "../../src/effects";
import type { KillCredit } from "../../src/killCredit";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import type { GameState } from "../../src/state";
import { cardAt, slotsOf } from "../../src/zones";

let nextIndex = 5800;

function unit(name: string, attack: number, health: number): CardDef {
  nextIndex += 1;
  const face = { attack, health, keywords: [], text: name };
  return {
    id: `kc-${name}`,
    index: String(nextIndex),
    name: `${name} (kill credit)`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: face,
    radiant: face,
  };
}

/** Classic+ #19.5's shape: "Whenever this destroys a Unit, it gets +5 Attack." */
export const bot = unit("bot", 5, 5);
/** Classic+ #19.2's shape with no roll: at end of turn it attacks a random enemy Unit; base sends the bot across Berserk, Radiant credits it the kill. */
export const jungle = unit("jungle", 5, 5);
/** Classic+ #26's shape: it casts itself on draw and remembers whether its Cry ran as that cast. */
export const tempo = unit("tempo", 1, 1);

/** Each `bot` of the controller's with the enemy Unit across from it. */
function acrossFromBots(state: GameState, controller: "p1" | "p2"): KillCredit[] {
  return slotsOf(controller, "units").flatMap((ref) => {
    const mine = cardAt(state, ref);
    if (mine === null || mine.defId !== bot.id) return [];
    const victim = cardAt(state, { player: opponentOf(controller), row: "units", lane: ref.lane });
    return victim === null ? [] : [{ victimId: victim.id, toId: mine.id }];
  });
}

function jungleFace(transfer: boolean): Script {
  return {
    endOfTurn: () => [
      withKillCredit({
        killer: { of: "self" },
        pairs: (ctx) => acrossFromBots(ctx.state, ctx.controller),
        transfer,
        during: forcedAttackRandom({ attacker: { of: "self" }, among: "enemyUnits" }),
        ...(transfer ? {} : { then: (pair: KillCredit) => [goBerserk({ target: { of: "instance", instanceId: pair.toId } })] }),
      }),
    ],
  };
}

const botScript: Script = {
  triggers: [
    {
      id: "kc-bot-kill",
      on: ["destroyed"],
      run: (ctx) =>
        ctx.event.type === "destroyed" && ctx.self !== null && ctx.event.killerId === ctx.self.id
          ? [buff({ target: { of: "self" }, attack: 5 })]
          : [],
    },
  ],
};

const tempoScript: Script = {
  staticFlags: { castOnDraw: true },
  cry: (ctx) => [remember({ key: "castOnDraw", value: ctx.self !== null && isCastOnDraw(ctx.state, ctx.self) })],
};

export const KC_SCRIPTS: Record<string, CardScripts> = {
  [bot.id]: { base: botScript, radiant: botScript },
  [jungle.id]: { base: jungleFace(false), radiant: jungleFace(true) },
  [tempo.id]: { base: tempoScript, radiant: tempoScript },
};

/** Adds these cards to whatever catalog and scripts are registered (after the base fixture's setup). */
export function registerKillCredit(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries([bot, jungle, tempo].map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...KC_SCRIPTS });
}
