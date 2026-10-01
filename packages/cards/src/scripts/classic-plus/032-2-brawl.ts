// C+ #32.2 Brawl (SPEC §8.7 row 32.2; §6.1, §6.3 Destroy, R46, R59, R60, R81, R129). (2) Spell token,
// printed Epic.
//   Base:    "Destroy all Units but one chosen at random."
//   Radiant: "Destroy all Units but one of your choice."
//
// The survivor is one of the Units on the field a Spell may affect (either side, the tops of piles; an
// Immune to Spells Unit is not among them and stays anyway, §6.1): drawn by the match rng on the base
// face (R60), and on the Radiant face a Unit target declared with the play (R81). Every other such Unit
// is marked at once, so one state check collects them all (R59); an Indestructible one survives
// anyway (R46) and a Reborn one comes back. With no Unit there is nothing to destroy and nothing is
// drawn (R129).

import { recalled, type EffectContext, type Script } from "@jackioh/engine";
import { cardsInScope, destroy, forEachCard, instanceOf, rememberRandom } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-032-2");

/** Where the base face keeps the survivor it drew, until the destroy reads it. */
const SURVIVOR = "survivor";

/** R81: the Radiant face's survivor, one Unit of either side. */
const SURVIVOR_PICK: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }];

/** "All Units": both sides' Units a Spell may affect, the tops of piles (§3.2), in R68's order. */
function unitIds(ctx: EffectContext): string[] {
  return cardsInScope(ctx, { side: "any" }).map((unit) => unit.id);
}

/** Every such Unit but `survivor`, each marked destroyed; read as the list reaches it. */
function destroyAllBut(survivor: (ctx: EffectContext) => string | null) {
  return forEachCard({
    cards: (ctx) => {
      const spared = survivor(ctx);
      return unitIds(ctx).filter((id) => id !== spared);
    },
    each: (instanceId) => destroy({ target: { of: "instance", instanceId } }),
  });
}

export const base: Script = {
  cry: (ctx) => [
    // R60: one draw of the match rng over the Units it would destroy; none, no draw (R129).
    rememberRandom({ key: SURVIVOR, options: unitIds(ctx) }),
    destroyAllBut((at) => {
      const drawn = recalled(at, SURVIVOR);
      return typeof drawn === "string" ? drawn : null;
    }),
  ],
};

export const radiant: Script = {
  targets: SURVIVOR_PICK,
  cry: () => [destroyAllBut((at) => instanceOf(at, { of: "chosen" })?.id ?? null)],
};
