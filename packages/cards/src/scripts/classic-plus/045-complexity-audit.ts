// C+ #45 Complexity Audit (SPEC §8.7 row 45, E36, R280, R583). (2) Spell, Rare.
//   Exile every permanent whose card has more lines of code than this one. Radiant: choose all
//   permanents or only your opponent's (a mode declared at play, R81) — and "highlight targets".
//
// Each permanent's `loc` (§5: its script file's non-blank, non-comment lines, imports excluded; a fused
// card's is its ingredients' sum) against this card's own, read off the running definition, so this
// file's own length is the card's balance (R388). The permanents are the tops of piles and the backrow
// cards, face-down ones included (`subsystems.auditTargets`); an equal `loc` stays. Exile fires no Death
// and a token ceases to exist (§6.3, R11). The Radiant face's preview (R280, R583) is, for each choice,
// the permanents it would exile now — never one its controller may not read (an enemy face-down card,
// R177), which the exile still takes; the base face marks nothing.

import { subsystems, type EffectContext, type GameState, type Script } from "@jackioh/engine";
import { opponentOf, type PlayerId } from "@jackioh/shared";
import { chosenOptions, exile, forEachCard, unreadableBy } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-045");

const ALL = "All permanents";
const THEIRS = "Only your opponent's";

/** The permanents whose card has more lines of code than `defId`'s. */
function targets(state: GameState, controller: PlayerId, active: PlayerId, defId: string, enemyOnly: boolean) {
  const loc = subsystems.linesOfCode(state, defId);
  return subsystems.auditTargets(state, { controller, active, loc, more: true, enemyOnly });
}

function audit(enemyOnly: (ctx: EffectContext) => boolean): Script["cry"] {
  return () => [
    forEachCard({
      cards: (run) => targets(run.state, run.controller, run.state.active, run.self?.defId ?? run.defId ?? def.id, enemyOnly(run)),
      each: (instanceId) => exile({ target: { of: "instance", instanceId } }),
    }),
  ];
}

export const base: Script = { cry: audit(() => false) };

export const radiant: Script = {
  modes: [{ kind: "mode", options: [ALL, THEIRS] }],
  cry: audit((ctx) => chosenOptions(ctx)[0] === THEIRS),
  preview: (ctx) =>
    ([["all permanents", false], ["only your opponent's", true]] as const).map(([label, enemyOnly]) => {
      const active = ctx.yourTurn ? ctx.controller : opponentOf(ctx.controller);
      const seen = targets(ctx.state, ctx.controller, active, ctx.self.defId, enemyOnly);
      const ids = seen.filter((card) => !unreadableBy(ctx.state, card).includes(ctx.controller)).map((card) => card.id);
      return { label, value: ids.length, ids };
    }),
};
