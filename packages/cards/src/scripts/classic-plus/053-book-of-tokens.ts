// C+ #53 Book of Tokens (SPEC §8.7 row 53). (1) Spell, Book, Epic.
//   Base:    "Summon {tokens|Rush Token|Rush Tokens}." — tokens 2
//   Radiant: "Summon {tokens|Radiant Rush Token|Radiant Rush Tokens}." — tokens 2
//   Engine:  "Rush Tokens (§7) placed per R64; a full board takes fewer. Tunes: tokens 2 ↑."
//
// One laneless `summon` per token: each takes the leftmost empty, unlocked, unreserved unit zone (R64)
// and fizzles silently on a full row (§3.2), as Core #15's do. The token's stats are its own (§7).

import { param, type Script } from "@jackioh/engine";
import { summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-053");

const RUSH_TOKEN = cardDef("core-t-rush").id;

function bookOfTokens(radiant: boolean): Script {
  return { cry: (ctx) => Array.from({ length: param(ctx, "tokens") }, () => summon({ defId: RUSH_TOKEN, radiant })) };
}

export const base: Script = bookOfTokens(false);

export const radiant: Script = bookOfTokens(true);
