// C #43 Plague Nuke (SPEC §8.6 row 43). (3) Spell, Epic.
//   Base:    "Destroy all Units. Gain {mana} mana for each Plague Counter that was on them." — 1
//   Radiant: "Destroy all Units. Gain {mana} mana for each Plague Counter that was on them. Then summon,
//            under your control, each of those Units that had a Plague Counter from its owner's
//            graveyard." — 1
//   Engine:  "Count the tokens on every unit first, destroy all (one state check, §4.5), then the
//            temporary mana. An Indestructible unit survives, but its tokens count ("on them" is every
//            Unit the Spell hit). Radiant: after that check, each non-token unit card that had a token
//            and is now in a graveyard is summoned to your side (control yours, owner unchanged, §3.2;
//            no Cry, R1), into your leftmost open zones in lane order (R64); a Reborn unit already
//            back on the field is not summoned again; tokens are gone (R11). Tunes: mana per token 1 ↑."
//
// "Them" is every Unit on the field as the Spell resolves — the top of each unit pile on both sides,
// never a card dormant under a Stack (R13) — read once, first: the Plague Counters on them all, and the
// ones that carry any. Then every Unit is destroyed, and the deaths happen in ONE §4.5 check
// (`afterStateCheck` runs it at this point of the list, R59), so an Indestructible unit survives with
// its tokens counted all the same. After that check comes the temporary mana (§2.3): {mana} per token.
//
// Radiant, after the same check: each of those Units that had a token, is not a token (R11: a token is
// gone) and now lies in a graveyard — not one Reborn already put back on the field, not one exiled
// instead of dying — is summoned for the caster (§6.3 Summon: no Cry, R1; its owner unchanged, §3.2),
// in the order they stood on the board (R68: the active side first, lane order), each into the
// caster's leftmost open zone (R64); a full row leaves the rest where they are. The rest of the text
// runs on a stay that begins after the check (R174), which is what lets it name a card in its
// graveyard.
//
// R280: the preview is the mana it would give now — the Plague Counters on the Units on the field,
// which are public (§10.8), times {mana} — read by the same count the resolution uses.
//
// The number is the declared `mana` (R386), read through `param`.

import {
  activeUnitsOf,
  defOf,
  findInstance,
  param,
  plagueOn,
  type CardInstance,
  type ConditionContext,
  type EffectContext,
  type Effect,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { afterStateCheck, destroyAll, gainMana, summon } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId, type PreviewValue } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-043");

/** R280: the words of the text the preview's value follows, on both faces. */
const MANA_LABEL = "for each Plague Counter that was on them";

/** Every Unit on the field, R68's order from `first`'s side: the tops of the unit piles (R13). */
function unitsOnField(state: GameState, first: PlayerId): CardInstance[] {
  return [...activeUnitsOf(state, first), ...activeUnitsOf(state, opponentOf(first))];
}

/** The Plague Counters on every Unit on the field now: what the mana counts. */
function tokensOnUnits(state: GameState, first: PlayerId): number {
  return unitsOnField(state, first).reduce((sum, unit) => sum + plagueOn(unit), 0);
}

function plagueNuke(resummons: boolean): Script {
  return {
    cry: (ctx: EffectContext): Effect[] => {
      // Read once, before anything dies: the tokens, and the non-token Units that carry any.
      const first = ctx.state.active;
      const mana = tokensOnUnits(ctx.state, first) * param(ctx, "mana");
      const plagued = unitsOnField(ctx.state, first)
        .filter((unit) => plagueOn(unit) > 0 && !defOf(ctx.state, unit.defId).token)
        .map((unit) => unit.id);
      return [
        destroyAll({ side: "any" }),
        afterStateCheck((after) => [
          gainMana({ amount: mana }),
          ...(resummons
            ? plagued
                .filter((id) => findInstance(after.state, id)?.zone.z === "graveyard")
                .map((instanceId) => summon({ instance: { of: "instance", instanceId }, player: "self" }))
            : []),
        ]),
      ];
    },
    // R280: the mana it would give now.
    preview: (ctx: ConditionContext): PreviewValue[] => [
      { label: MANA_LABEL, value: tokensOnUnits(ctx.state, ctx.controller) * param(ctx, "mana") },
    ],
  };
}

export const base: Script = plagueNuke(false);

export const radiant: Script = plagueNuke(true);
