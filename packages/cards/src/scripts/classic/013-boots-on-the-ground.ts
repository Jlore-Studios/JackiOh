// C #13 Boots on the Ground (SPEC §8.6 row 13). (1) Unit, Human, Common, 2/1 → 4/2.
//   Base:    "Charge\nAfter this attacks, draw {draw}." — draw 1
//   Radiant: "Charge\nAfter this attacks, Recruit {recruits|card|cards}." — recruits 1
//   Engine:  "A trigger after each combat it attacked in, forced attacks included, whether or not it
//            survived (it reads its last-known state, R78). Radiant: Recruit (§6.3), the first
//            permanent from the top of your deck. Tunes: draw 1 ↑; Radiant recruits 1 ↑."
//
// Charge is printed on both catalog faces, so §10.4 layer 1 grants it and nothing here does.
//
// "After this attacks", forced attacks included, whether or not it survived: the card's
// `afterAttack` hook, which the engine runs after the state check that closes each combat this card
// attacked in — declared by its controller or forced (R53, #9 Moths to the Flame) — on the attacker's
// last-known snapshot, as a Death hook runs on one (R78, R89): `ctx.self` is that snapshot, so its
// face and its tuned numbers are the ones it attacked with, and the controller is the attack's. An
// attack on this card is not one it attacked in, so defending does nothing.
//
// The engine does not run `afterAttack` yet (it is requested of the combat module: a trigger on
// `attackDeclared` could not serve, since an entry a card queued on the field is dropped when it
// leaves the field, R174, and a forced attack's event reaches no card that died in its combat, R212).
// `Script` does not declare the key yet either, so the faces are typed with it added below; once the
// engine declares it, that local type is `Script` itself. Until then its test waits.
//
// Radiant: §6.3 Recruit, one top-down scan per recruit for the first permanent, summoned into its
// row per R64 with no Cry (R1), a Trap face-down (R33); nothing when there is none or its row is full.
//
// The numbers are the declared `draw` and `recruits` (R386), read through `param`.

import { param, type Hook, type Script } from "@jackioh/engine";
import { draw, recruit } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-013");

/** `Script` with the requested hook: run after each combat this card attacked in, on its last-known state. */
type AfterAttackScript = Script & { afterAttack?: Hook };

/** "After this attacks, draw {draw}." */
export const base: AfterAttackScript = {
  afterAttack: (ctx) => [draw({ count: param(ctx, "draw") })],
};

/** "After this attacks, Recruit {recruits|card|cards}.": one top-down scan per recruit. */
export const radiant: AfterAttackScript = {
  afterAttack: (ctx) => Array.from({ length: param(ctx, "recruits") }, () => recruit({ player: "self" })),
};
