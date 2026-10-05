// T-glitch Glitch (SPEC §7, issue #170; R673–R679). Spell, Token, cost 0. Hidden: in no pool, not in
// the Almanac or the Deck Builder (R674).
//   Base:    "" — the card is blank; its client face draws corrupted text over glitch art.
//   Radiant: the same blank face (§7: Glitch prints no Radiant form, and its text has no number).
//
// How it gets into a hand or a deck is not this file's (R673): once a "… in the System" card has been
// played in the match, `catalog.pickGenerated` may hand one out in place of a generated card. It is
// always playable on its owner's turn — it costs (0) whatever modifies costs and no ban refuses it
// (R675), which is the engine's (`mana.priceOf`, `costRules.whyPlayBanned`), not a hook here.
//
// When it resolves it does one of four things, drawn by the match rng (`glitch`, R676–R679): reset
// the match, swap the seats, lay two other games' boards on the field, or void the match.

import type { Script } from "@jackioh/engine";
import { glitch } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-t-glitch");

export const base: Script = { cry: () => [glitch()] };

export const radiant: Script = base;
