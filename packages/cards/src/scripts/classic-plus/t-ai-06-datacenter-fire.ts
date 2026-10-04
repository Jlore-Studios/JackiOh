// T-AI-6 Datacenter Fire (SPEC §8.7 row T-AI-6, §7, B8). (2) Spell, AI, Token.
//   Base:    "Destroy all Field Spells. Deal 1 damage to each hero for each one destroyed."
//   Radiant: "Destroy all enemy Field Spells. Deal 2 damage to the enemy hero for each one destroyed."
//   Engine:  "The Field Spells on both sides (Traps and Field Traps are not), an Ivory Tower included
//            whatever it has fused (R418, R588); Indestructible ones (#98) survive and don't count. The
//            damage is one instance per hero, 1 (Radiant 2) times the count, through §4.4, so Spell
//            Damage raises it once (§4.4 step 0); none destroyed, no damage. Its preview (R280) is the
//            damage each hero would take now. The base face burns your own Claude's Datacenter too
//            while it stands in its backrow zone; animated, it is a Unit and stays (R588). Tunes: none."
//
// The sweep and the count are one engine verb (`destroyFieldSpellsAndHit`, effects/datacenter.ts), and
// the preview reads the same count (`fieldSpellsDoomed`) off the public backrows, times the face's
// number — the hit before Spell Damage, as every Core preview reads its own number (R280). An Animated
// Field Spell standing in a unit zone is a Unit there (R383) and no Field Spell of the sweep's (R588).

import type { PreviewValue } from "@jackioh/shared";
import type { ConditionContext, Script } from "@jackioh/engine";
import { destroyFieldSpellsAndHit, fieldSpellsDoomed, type FieldSpellSide } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-06");

/** §8.7: 1 damage per Field Spell on the base face, 2 on the Radiant. An AI card declares no params (B8). */
const DAMAGE_PER = { base: 1, radiant: 2 } as const;

/** R280: the formula as each face prints it, read off the catalog text so it is always a substring. */
function formulaIn(text: string): string {
  const found = /Deal \d+ damage to [^.]+ for each one destroyed/.exec(text);
  if (found === null) throw new Error(`T-AI-6's text names no damage formula: ${text}`);
  return found[0];
}

function fire(side: FieldSpellSide, damagePer: number, label: string): Script {
  return {
    cry: () => [destroyFieldSpellsAndHit({ side, damagePer })],
    preview: (ctx: ConditionContext): PreviewValue[] => [{ label, value: fieldSpellsDoomed(ctx, side).length * damagePer }],
  };
}

export const base: Script = fire("any", DAMAGE_PER.base, formulaIn(def.base.text));

export const radiant: Script = fire("enemy", DAMAGE_PER.radiant, formulaIn(def.radiant.text));
