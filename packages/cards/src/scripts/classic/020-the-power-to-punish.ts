// C #20 The Power to Punish (SPEC §8.6 row 20). Field Spell, cost 2, Rare.
//   Base:    "Activate: Choose one: Deal {damage} damage; your opponent discards {discards|card|cards};
//             or choose a Unit, which is destroyed at the start of your next turn."
//   Radiant: "Activate: Choose one: Deal {damage} damage; your opponent discards {discards|card|cards};
//             or all enemy Units are destroyed at the start of your next turn."
//
// Activate (R384), once per turn: the mode and, for the modes that take one, the target are declared
// in the `activate` action (R81), each target bound to its mode (`forModes`, R90).
//   - "deal damage": one hit of {damage} from this card on a Unit or hero, either side.
//   - "opponent discards": the discard is random from their hand (R661: no "of your choice"), so no
//     prompt opens. With fewer cards than asked they discard all they have; an empty hand discards
//     nothing.
//   - the delayed destroy (a delayed effect, §10.1, `destroyAtNextTurnStart`): on the base face the
//     chosen Unit, on either side, keyed to that stay on the field — it fizzles if the Unit left the
//     field meanwhile, even if it came back (R174) — and on the Radiant face every enemy Unit on the
//     field when it resolves, not a list fixed at activation. It resolves with the start-of-turn
//     delayed effects (R62, R68) whether or not this card is still on the field (as R76), and it is a
//     destroy, so an Indestructible Unit survives it (R46).
// Activating is not a play (R384).

import type { ActivationDecl, Effect, EffectContext, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { damage, destroyAtNextTurnStart, discardRandom } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-020");

const DAMAGE_MODE = "deal damage";
const DISCARD_MODE = "opponent discards";
const DOOM_MODE = "destroy a Unit at the start of your next turn";
const DOOM_ALL_MODE = "destroy all enemy Units at the start of your next turn";

const DAMAGE_TARGET: TargetDecl = {
  kind: "target",
  min: 1,
  max: 1,
  filter: { of: ["unit", "hero"] },
  forModes: [DAMAGE_MODE],
};

const DOOM_TARGET: TargetDecl = { kind: "target", min: 1, max: 1, filter: { of: ["unit"] }, forModes: [DOOM_MODE] };

function punish(ctx: EffectContext, radiant: boolean): Effect[] {
  switch (ctx.modes[0]) {
    case DAMAGE_MODE:
      return [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })];
    case DISCARD_MODE:
      return [discardRandom({ count: param(ctx, "discards"), player: "enemy" })];
    case DOOM_MODE:
      // The unit marked for death wears #50 K-Pop Fanatic's aura, in red (R437).
      return radiant ? [] : [destroyAtNextTurnStart({ target: { of: "chosen" }, mark: { mark: "destroy", color: "red" } })];
    case DOOM_ALL_MODE:
      return radiant ? [destroyAtNextTurnStart({ scope: { side: "enemy" } })] : [];
    default:
      return [];
  }
}

function face(radiant: boolean): Script {
  const ability: ActivationDecl = {
    id: "punish",
    label: "Choose one",
    uses: 1,
    modes: [{ kind: "mode", options: [DAMAGE_MODE, DISCARD_MODE, radiant ? DOOM_ALL_MODE : DOOM_MODE] }],
    targets: radiant ? [DAMAGE_TARGET] : [DAMAGE_TARGET, DOOM_TARGET],
    run: (ctx) => punish(ctx, radiant),
  };
  return { activations: [ability] };
}

export const base: Script = face(false);

export const radiant: Script = face(true);
