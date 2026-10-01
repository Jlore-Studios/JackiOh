// C #86 Genn (SPEC §8.6 row 86). (4) Unit, Common, 14/14 → 42/42.
//   Base:    no text
//   Radiant: no text; the Radiant face is its tripled stats
//   Engine:  "A vanilla Unit: no keywords and no script, as #8 Mr. Vanilla, whose Radiant face triples
//            its stats too. The designer printed 14/14 on both faces; R276 refuses identical faces, so
//            the Radiant face is 42/42. Tunes: none."
//
// Nothing to script: both faces' stats are the catalog's (§10.4 layer 1), and a card made Radiant on
// the field takes its Radiant face at once (§5.2). Its proof: `test/classic/086-genn.test.ts`.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-086");

export const base: Script = {};

// The same empty script: the Radiant face's 42/42 is catalog data (R276).
export const radiant: Script = base;
