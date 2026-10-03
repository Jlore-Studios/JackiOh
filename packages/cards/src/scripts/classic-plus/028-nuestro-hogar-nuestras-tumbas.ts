// C+ #28 Nuestro hogar, nuestras tumbas (SPEC §8.7 row 28; §4.5, §6.1, R8, R19, R386). (2) Unit,
// Common, 3/4 → 6/8.
//   Base:    "Taunt, Reborn / Death: Heal your hero {heal}." — heal 3
//   Radiant: "Taunt, Reborn, Divine Shield / Death: Heal your hero {heal}." — heal 8
//   Engine:  "Death fires on both deaths of a Reborn unit (§4.5), so it heals twice. Tunes: heal 3 ↑."
//
// The keywords are the catalog's (Taunt, Reborn; Divine Shield on the Radiant face), which the layers
// read off the face. The Death is §4.5 step 3's hook, which runs on every death, the first one that
// Reborn answers included (R8), and never on an exile or a bounce, which are not deaths. A hero's heal
// has no cap (R19). The amount is the declared number `heal` (R386), read off the dying card's own
// face and tuning through `param`, so both faces run one script.

import { param, type Script } from "@jackioh/engine";
import { heal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-028");

export const base: Script = {
  death: (ctx) => [heal({ target: { of: "selfHero" }, amount: param(ctx, "heal") })],
};

// The same script: the Radiant face's 8 is its declared `heal`, and its Divine Shield is catalog data.
export const radiant: Script = base;
