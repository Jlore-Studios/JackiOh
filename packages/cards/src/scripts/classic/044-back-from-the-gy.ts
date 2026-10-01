// C #44 Back from the GY (SPEC §8.6 row 44, §6.3 Summon, §10.6; R1, R64, R65, R178, R396). Spell,
// cost 4, Legendary.
//   Base:    "Summon Units from your graveyard that cost ({budget}) or less in total. Exile this."
//   Radiant: "Summon every Unit from your graveyard. Exile this."
//   Engine:  "A `pick` prompt (§10.6) from your graveyard, budgeted by cost, costs per R65 out of play
//            (an X Unit counts 0, R396); then each is summoned (no Cry, R1) into your leftmost open
//            zones (R64); a full board leaves the rest. Radiant: every Unit, oldest first, until the
//            board is full. "Exile this" is the Spell's landing (§5.1, R178). Tunes: budget 5 ↑."
//
// THE BASE PICK is the engine's budgeted `choosePick` (B5 E18): every Unit in your graveyard is an
// option carrying its cost as R65 reads it there, out of play (its own cost, `costMod` and
// `costOverride` kept, R78; an X Unit 0), and the answer's costs may total no more than the card's
// budget (`param(ctx, "budget")`), any number of Units under it, none included. No Unit there asks
// nothing. Each pick is then summoned (§6.3 Summon: no Cry, R1) into your leftmost empty, unlocked,
// unreserved unit zone (R64); once the row is full a summon finds no zone and that card stays in the
// graveyard.
//
// THE RADIANT FACE asks nothing: every Unit in your graveyard, oldest first (the graveyard is
// chronological, §3), read once as the clause begins (`forEachCard`, R113), each summoned the same
// way until the row is full.
//
// "EXILE THIS" names where §10.5 step 7 sends the Spell (R178): it stays itself while it resolves and
// lands in exile rather than the graveyard, so it never summons or offers itself.

import type { CardInstance, Effect, EffectContext, Script } from "@jackioh/engine";
import { defOf, param, zoneCards, zoneCount } from "@jackioh/engine";
import { choosePick, exile, forEachCard, summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-044");

/** Your graveyard's Units, oldest first (§3: a graveyard is chronological). */
function graveyardUnits(ctx: EffectContext): readonly CardInstance[] {
  return zoneCards(ctx.state, ctx.controller, "graveyard").filter((card) => defOf(ctx.state, card.defId).type === "Unit");
}

function exileThis(): Effect {
  return exile({ target: { of: "self" } });
}

export const base: Script = {
  cry: (ctx) => [
    choosePick({
      step: "picked",
      from: [{ zone: "graveyard" }],
      filter: { type: "Unit" },
      // Any number of Units, the budget the only bound.
      max: zoneCount(ctx.state, ctx.controller, "graveyard"),
      budget: param(ctx, "budget"),
      prompt: "Summon Units from your graveyard within the budget",
    }),
    exileThis(),
  ],
  resume: {
    picked: (ctx) => ctx.targets.map((_, index) => summon({ instance: { of: "chosen", index } })),
  },
};

export const radiant: Script = {
  cry: () => [
    forEachCard({
      cards: graveyardUnits,
      each: (instanceId) => summon({ instance: { of: "instance", instanceId } }),
    }),
    exileThis(),
  ],
};
