// C #41 State of the Game (SPEC §8.6 row 41). Unit 3/3 → 6/6, cost 1, Common.
//   Base:    "Indestructible"
//   Radiant: "Indestructible, Lifesteal" — the adopted Radiant face: a keyword-only Unit's Radiant
//            owes one more keyword or a stronger one (R275), and Taunt is out (R347).
//   Engine:  "Keywords only; under R347 it never has Taunt."
//
// There is nothing to script. The stats and both faces' keywords are printed on the catalog entry
// (`base.keywords = [Indestructible]`, `radiant.keywords = [Indestructible, Lifesteal]`), and §10.4
// layer 1 reads them from there; granting them here would only say the same thing twice. Where the
// behaviour lives instead:
//   Indestructible — §4.4 step 4 (it takes no damage), §4.5 step 1 and R46 (a destroy mark is
//                    ignored and knocks it into Attack Position), R69 (it still dies if its max
//                    health falls to 0), §6.1 (exile, bounce and a Tribute still remove it), and
//                    R347 (§10.4's keyword set drops Taunt whenever it holds Indestructible, so it has
//                    no Taunt even in Defense Position).
//   Lifesteal      — §4.4 step 8 heals its controller's hero the amount it actually deals.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-041");

export const base: Script = {};

// The same empty script: the Radiant face differs only in what the engine reads off the catalog
// (its doubled stats and the added Lifesteal).
export const radiant: Script = base;
