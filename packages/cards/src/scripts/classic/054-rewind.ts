// C #54 Rewind (SPEC §8.6 row 54; §6.3 Trigger a Cry; R70, R81, R90, R386). Spell, cost 1, Common.
//   Base:    "Trigger a Cry of one of your Units on the field or in your graveyard." (once, not tunable, R749)
//   Radiant: "Trigger the Cry of any Unit on the field or in a graveyard, {repeats|time|times}." (2)
//   Engine:  the declared target (R81) is a Unit that has a Cry — the top of a pile on the field, or a
//            Unit card in a graveyard; its Cry runs with that unit as `self`, under your control, its
//            choices yours as prompts (R70); out of a graveyard "this" finds nothing. Radiant: either
//            side, two separate runs, each with its own choices.
//
// `triggerCry` is the engine's whole sequence; this card only declares the pick (a `check` on
// `hasTriggerableCry`) and runs it `param(ctx, "repeats")` times on the chosen Unit.

import { param, type Script } from "@jackioh/engine";
import { hasTriggerableCry, triggerCry } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-054");

function rewind(side: "ally" | "any"): Script {
  return {
    targets: [{ kind: "target", min: 1, max: 1, filter: { side, of: ["unit", "graveyard"], check: "hasCry" } }],
    targetChecks: { hasCry: ({ state, candidate }) => candidate !== null && hasTriggerableCry(state, candidate) },
    cry: (ctx) => Array.from({ length: param(ctx, "repeats") }, () => triggerCry()),
  };
}

export const base: Script = rewind("ally");

export const radiant: Script = rewind("any");
