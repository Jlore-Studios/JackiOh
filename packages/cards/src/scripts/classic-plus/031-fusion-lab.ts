// C+ #31 Fusion Lab (SPEC §8.7 row 31; §6.3 Fuse, R23, R65, R77, R81, R102, R380, R387, R561).
// (2) Field Spell, Epic.
//   Base:    "Activate: Choose a card in your hand. Fuse a random card into it. Its cost doesn't
//            change." (balance patch 1: the Cry and the end-of-turn trigger became one Activate)
//   Radiant: "… Fuse a random Radiant card into it …"
//
// "A card in your hand" has no "random", so its controller chooses: the pick is declared with the
// activation and travels in the `activate` action (R81). An Immutable card can't be chosen, since no
// Fuse keeps one (R23), and an empty hand offers no activation.
//
// The Fuse is B5 E23's "fuse a random card into a card in your hand" (`fuseRandomInto`): one random
// non-token card of every set (R380) but Fusion Lab — every ingredient's id, on a fused Lab (R387) — is
// fused per R77 and R102 into the chosen card, which is the kept instance and stays in the hand, its
// type the result's, with a `costOverride` of the cost it had (R65), so its cost doesn't change. On
// the Radiant face the random card goes in on its Radiant face and lends it to both of the fusion's
// forms, so its text and stats show whether or not the hand card is Radiant (R561). The opponent's
// view names neither the hand card, the ingredient nor the fused id (R97, R179).

import { unitHas, type ActivationDecl, type CardInstance, type GameState, type Script } from "@jackioh/engine";
import { fuseRandomInto } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-031");

/** The `targetChecks` predicate the ability's declared pick names: a card a Fuse may keep (R23). */
const FUSABLE = "fusable";

/** R23: a card a Fuse may keep is not Immutable. */
const fusable = (state: GameState, card: CardInstance): boolean => !unitHas(state, card, "Immutable");

/** R81: one card of the controller's own hand, declared with the activation. */
const HAND_PICK: TargetDecl[] = [{ kind: "hand", min: 1, max: 1, filter: { check: FUSABLE } }];

function fusionLab(radiant: boolean): Script {
  // B5 E23: a random card of every set but this one into the chosen hand card, keeping its cost.
  const fuse: ActivationDecl = {
    id: "fuse",
    label: "Choose a card in your hand. Fuse a random card into it",
    uses: 1,
    targets: HAND_PICK,
    run: () => fuseRandomInto({ into: { target: { of: "chosen" } }, radiant, keepCost: true }),
  };
  return {
    targetChecks: {
      [FUSABLE]: ({ state, candidate }) => candidate !== null && fusable(state, candidate),
    },
    activations: [fuse],
  };
}

export const base: Script = fusionLab(false);

export const radiant: Script = fusionLab(true);
