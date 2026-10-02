// C #78 Mutate Spell (SPEC §8.6 row 78). (1) Field Spell (R402: the designer wrote Spell), Rare.
//   Base:    "Activate ♾️: Remove a Plague Token from a permanent. If it's an enemy permanent, exile it. If
//            it's your backrow card, draw {draw}. If it's your Unit, it attacks a random enemy
//            {attacks|time|times}." — draw 2, 1 attack
//   Radiant: "… If it's an enemy permanent, fuse it onto a card of yours of its type on your field, in your
//            hand or in your deck, or exile it if you have none. …" — draw 4, 2 attacks
//   Engine:  "Activate ♾️ (§6.2, R384) with a declared target (a permanent with a Plague Token, either
//            side), bounded by `ACTIVATE_UNLIMITED_CAP` and by the tokens on the field; removing the token
//            is the ability's cost. "Attacks a random enemy" is a forced attack (R53) on a random enemy,
//            hero or unit, drawn from the targets it may attack (§4.2); the Radiant's second attack is its
//            own combat and happens only if the unit is still on the field (R96). The Radiant's fuse is
//            Fuse (§6.3, R77, R102; the enemy card ceases to exist) onto the card you choose … an
//            Immutable card of yours is not offered (R23). Tunes: draw 2 ↑; attacks 1 ↑."
//
// R402: the target travels in the `activate` action (R81, R384); with no tokened permanent on the field
// the ability can't be activated. The branch is read off the target as it was activated: whose it is,
// and its row. The forced attacks are `forcedAttackRandom`; the Radiant fuse is `fuseOntoYourCard`, whose
// prompt offers your cards of its type (the deck's to you alone) and exiles the card when there is none.

import { param, permanentsOnField, plagueOn, type ActivationDecl, type Effect, type EffectContext, type Script } from "@jackioh/engine";
import { consumePlague, draw, exile, forcedAttackRandom, fuseOntoYourCard, instanceOf } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-078");

const CHOSEN = { of: "chosen" } as const;

function outcome(ctx: EffectContext, fuses: boolean): Effect[] {
  const card = instanceOf(ctx, CHOSEN);
  if (card === null || card.zone.z !== "field") return [];
  if (card.controller !== ctx.controller) return [fuses ? fuseOntoYourCard({ target: CHOSEN }) : exile({ target: CHOSEN })];
  if (card.zone.row === "backrow") return [draw({ count: param(ctx, "draw") })];
  return [forcedAttackRandom({ attacker: CHOSEN, times: param(ctx, "attacks") })];
}

function mutate(fuses: boolean): Script {
  const ability: ActivationDecl = {
    id: "mutate",
    label: "Remove a Plague Token",
    uses: "unlimited",
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"], plague: true } }],
    canActivate: ({ state, controller }) => permanentsOnField(state, controller).some((card) => plagueOn(card) > 0),
    run: (ctx) => [consumePlague({ target: CHOSEN }), ...outcome(ctx, fuses)],
  };
  return { activations: [ability] };
}

export const base: Script = mutate(false);

export const radiant: Script = mutate(true);
