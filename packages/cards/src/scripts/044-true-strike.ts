// #44 True Strike (SPEC §8.2, R346). Spell, cost 1, Common.
//   Base:    "Pierce / Deal 4 damage. Exile this."
//   Radiant: "Pierce / Deal 9 damage. Exile this." (patch v0.1.1, issue #27, gave the card Pierce in
//            place of "ignoring Armor"; the faces differ only in the number.)
//   Engine:  "Pierce: skips pipeline step 2; Divine Shield still applies."
//
// PIERCE ON A SPELL (R346): its damage ignores Armor. Pierce is printed on both catalog faces, and
// §4.4 step 2 reads it off the source (`damage.pierces`): the spell is the source of its own damage
// while it resolves, so the printed keyword is what skips step 2. The effect also states it
// (`ignoreArmor`, which R346 reads as the effect's own Pierce, as R85's `lifesteal` is Lifesteal),
// so the hit pierces on every run of the script — an Echo repeat included, whose source R98 may
// have left empty once "exile this" took the card from the resolving zone. Pierce skips step 2 and
// NOTHING else: step 1 still negates the whole hit on a Divine Shield target and spends the shield,
// step 3 still clamps a hero with Anti-oneshot Armor, and step 4 still makes an Indestructible
// target take nothing. R63's zero rule cannot bite here: 4 and 9 are both above 0 before step 1.
//
// "Deal 4 damage" takes a target (§8 Conventions: the player picks at play time from all legal
// units and heroes on either side unless narrowed, and nothing narrows it here) — so the
// declaration below is `side: "any"`, `of: ["unit", "hero"]`. It is a DECLARED play-time choice
// (R81), travelling in the `play` action's `targets` and arriving as `ctx.targets[0]`, which
// `{ of: "chosen" }` reads; it never opens a prompt. R90 validates it against this declaration.
//
// "Exile this" (§5.1, §6.3): the spell is mid-resolution in the `resolving` zone while its script
// runs (§10.1, §10.5 step 4), and `exile` moves it from there, so §10.5 step 7 finds it already in
// exile and must not send it to the graveyard. `exile` also bumps the game exile counter (R55),
// which #40 Echoes of the Forgotten and #100 Ceaseless Void read.
//
// Order: damage first, then the exile. Both are in one list, so the state check runs after the pair
// (R59) — the target dies, if it dies, with the spell already in exile.

import type { Script } from "@jackioh/engine";
import { damage, exile } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-044");

/** The faces differ only in how much damage they deal. */
function trueStrike(amount: number): Script {
  return {
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }],
    // A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9). R346: the hit pierces.
    cry: () => [
      damage({ to: { of: "chosen" }, amount, ignoreArmor: true }),
      exile({ target: { of: "self" } }),
    ],
  };
}

export const base: Script = trueStrike(4);

export const radiant: Script = trueStrike(9);
