// C #31 Cookie Guild (SPEC §8.6 row 31). (2) Unit, Human, Common, 2/4 → 4/8.
//   Base:    "Cry: Recruit {units|Unit|Units} of ({costLimit}) Cost or less." — 1 Unit, (2)
//   Radiant: "Cry: Recruit {units|Unit|Units} of ({costLimit}) Cost or less." — 3 Units, (2)
//   Engine:  "Recruit (§6.3) filtered to Units whose deck cost (R65) is (2) or less, an X Unit counting
//            0 there (R396); three scans on the Radiant face, as #69 Call to Arms does, stopping when
//            the board is full. Tunes: cost limit 2 ↑; units 1 ↑."
//
// §6.3 Recruit is one top-down scan for the first permanent that matches, summoned per R64 into the
// leftmost open zone with no Cry (R1), the library otherwise keeping its order. "Recruit N" is N scans
// in order, as #69 Call to Arms writes it, and each scan takes the card it found, so the next finds the
// next match further down. A scan that finds a Unit but no open zone leaves it in the library, which is
// what "stopping when the board is full" is.
//
// The filter is `type: "Unit"` (the scan would otherwise take any permanent) and `costRange.max`: the
// scan reads each library card's cost out of play (R65: its own cost plus its `costMod`, no player
// discount; an X card 0, R396), which is `recruit`'s own reading.
//
// Both numbers are declared (`units`, `costLimit`, R386) and read through `param`; the faces differ
// only in `units` (1 or 3), so both run this one script.

import { param, type Effect, type Script } from "@jackioh/engine";
import { recruit } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-031");

export const base: Script = {
  cry: (ctx): Effect[] => {
    const filter = { type: "Unit" as const, costRange: { max: param(ctx, "costLimit") } };
    return Array.from({ length: param(ctx, "units") }, () => recruit({ filter, player: "self" }));
  },
};

// The same script: the Radiant face's three scans are its declared `units`, which `param` reads off the running face.
export const radiant: Script = base;
