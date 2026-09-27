// #8 Mr. Vanilla (SPEC §8.1): a 1-cost 4/4 → 12/12 Human Unit with no text on either face (patch
// v0.1.1 took away its Immutable and the Radiant face's Divine Shield, and made it a true vanilla:
// its Radiant face is its stats alone, tripled). A card with no text has no hooks, so both Scripts
// are empty — an empty Script is the answer, not a placeholder.
//
// Nothing about it is special any more: Transform, Vanilla and Fuse-onto all reach it like any unit,
// and making it Radiant on the field swaps its stat layer in place (§5.2), keeping its damage.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-008");

export const base: Script = {};

export const radiant: Script = {};
