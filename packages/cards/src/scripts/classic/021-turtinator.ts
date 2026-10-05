// C #21 Turtinator (SPEC §8.6 row 21). Unit 5/4 → 10/8, cost 2, Common.
//   Both faces: "Activate ♾️: Tribute a Unit. Deal damage equal to {multiplier}× its Attack to any
//   target." — the multiplier is 1 on the base face and 2 on the Radiant face (a declared number).
//
// Activate ♾️ (R384): any number of uses a turn, bounded by `ACTIVATE_UNLIMITED_CAP` and, in
// practice, by the units there are to Tribute. The cost is "sacrifice one of your units", a pick
// carried in the `activate` action (`tributes`) — never Turtinator itself (R662,
// `cost.tributeExcludesSelf`), paid as the ability is activated (`cost.tribute`); the Tribute is a
// Sacrifice, so it is a death (Death fires, §6.3) and it bypasses Indestructible. One unit is one
// Tribute here: a Sheep Token's "worth 2" counts only toward a play's Tribute X (§6.3), and an
// activation needs no zone (R391 is about plays).
//
// The hit's amount is the tributed unit's Attack as it stood when it was paid — its last-known
// state (R78), which the activation records (`subsystems.activationPaid`), since the unit is in a
// graveyard, reset, by the time the effect runs — times the multiplier. The hit comes from Turtinator
// (the source is the card as it last stood, §4.4). An amount of 0 is no hit at all (R63), so nothing
// is dealt and nothing is reported.

import type { ActivationDecl, Script } from "@jackioh/engine";
import { param, subsystems } from "@jackioh/engine";
import { damage } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-021");

const eat: ActivationDecl = {
  id: "eat",
  label: "Tribute a Unit. Deal damage equal to its Attack times the multiplier to any target",
  uses: "unlimited",
  cost: { tribute: 1, tributeExcludesSelf: true },
  targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero"] } }],
  run: (ctx) => {
    const tributed = subsystems.activationPaid(ctx).tributed[0];
    const amount = (tributed?.attack ?? 0) * param(ctx, "multiplier");
    return amount > 0 ? [damage({ to: { of: "chosen" }, amount })] : [];
  },
};

export const base: Script = { activations: [eat] };

// The same script: the Radiant face's "twice its Attack" is its declared multiplier (2), which
// `param` reads off the running face.
export const radiant: Script = base;
