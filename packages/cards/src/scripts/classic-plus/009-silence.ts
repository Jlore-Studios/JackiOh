// C+ #9 Silence (SPEC §8.7 row 9, R407). (0) Spell, Common.
// Vanilla a target Unit on either side (§6.3; Immutable refuses it, R23). Radiant: any permanent, a
// backrow card face-down or not included (R407: an aura stops, a trap never fires, Animated is lost).

import type { Script } from "@jackioh/engine";
import { vanilla } from "@jackioh/engine/effects";
import type { TargetFilter } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-009");

function silence(of: TargetFilter["of"]): Script {
  return { targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of } }], cry: () => [vanilla()] };
}

export const base: Script = silence(["unit"]);

export const radiant: Script = silence(["unit", "backrow"]);
