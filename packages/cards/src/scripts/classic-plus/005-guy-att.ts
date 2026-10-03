// C+ #5 Guy Att (SPEC §8.7 row 5). (2) Unit, Human, Common, 6/8 → 12/16.
// Cry: destroy every backrow card you control (Radiant: every backrow card), face-down ones included,
// tops of backrow piles only; one state check (R59), Indestructible ones staying (R46). An Animated
// card standing in a unit zone is a Unit and is not hit (R383); an Ivory Tower is, whatever it has fused
// (R418), and a Unit standing on it while its play resolves is not (R446, R635).

import type { Script } from "@jackioh/engine";
import { destroyAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-005");

export const base: Script = { cry: () => [destroyAll({ side: "self", rows: ["backrow"] })] };

export const radiant: Script = { cry: () => [destroyAll({ side: "any", rows: ["backrow"] })] };
