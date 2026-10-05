// #80 Zao Gao (SPEC §8.3, §5.2, §7; R11, R16, R21, R64, R215, R275, R276, R354). Spell, CN, cost 2.
//   Base:    "Discard 2 random cards. Summon 2 Rush Tokens, each with 2 random keywords."
//   Radiant: "Discard 2 random cards. Summon 2 Radiant Rush Tokens, each with 3 random keywords."
//
// Patch v0.1.1 (issue #27) changed three things, and R354 records how they are read: the discard is
// random ("not of your choice", so R662's random default, never a prompt), the Radiant face's tokens
// gain a third keyword on top of being Radiant ("Modify": the change is added to the face the
// Radiant pass gave it, R276), and the card is tagged CN.
//
// The discard is `discardRandom`: each of the two cards is one uniform pick from the match rng over
// the hand as it then stands, so the two are different cards, and fewer than 2 in hand discards what
// there is (§8's Engine cell). A discarded card is the printed card again, keeping only its
// `costMod`, `costOverride` and radiant flag (R215), and a unit-token card among the discards ceases
// to exist instead of reaching the graveyard (R11) — `discard`'s rules. Nothing asks the player
// anything, so an Echo repeat discards at random again, and an empty hand simply summons.
//
// R21: each token rolls DISTINCT keywords from the pool (thirteen with R346's Pierce and R636's Windfury), and the two
// tokens roll independently. `grantRandomKeywords` is that rule already — it recomputes the pool per
// draw off the unit's §10.4 keywords, so it never repeats inside one grant and never offers a keyword
// the unit already has: a Rush Token (printed Rush, §7) draws its two from the other eleven, and a
// Radiant one (printed Rush and Cleave) its three from the other ten. §8's Engine cell puts the order
// in words: "each token is summoned Radiant and then rolls its keywords" — `summon` sets the flag as
// it creates the card and rolls only once it has landed, so the roll reads the Radiant face.
//
// R64: a summon with no named zone takes the leftmost empty, unlocked, unreserved unit zone and
// fizzles silently when the row has none, so a board with one free zone gets one token.
//
// The keywords are rolled by `summon` itself (`randomKeywords`), since `summon` returns nothing a
// script can reference and `TargetSpec` has no "last summoned" case: rolling inside the summon keeps
// the rng draws adjacent to the summon they belong to (replay parity, §9.3) and has no fizzle
// hazard. The discards come first, so their draws precede the tokens' in the rng stream.

import type { Effect, Script } from "@jackioh/engine";
import { discardRandom, summon } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-080");

/** §7: the token this card makes, taken from the catalog rather than restated. */
const RUSH_TOKEN = cardDef("core-t-rush").id;

const DISCARD_COUNT = 2;
const TOKEN_COUNT = 2;

/** What a face summons: whether the tokens are Radiant, and how many keywords each rolls (R21). */
type Tokens = { radiant: boolean; keywords: number };

/** Base: plain Rush Tokens, two keywords each. */
const BASE_TOKENS: Tokens = { radiant: false, keywords: 2 };
/** Radiant (R276, R354): Radiant Rush Tokens, three keywords each. */
const RADIANT_TOKENS: Tokens = { radiant: true, keywords: 3 };

/** One Rush Token with its rolled keywords (R21, R64). */
function rushToken(tokens: Tokens): Effect {
  return summon({ defId: RUSH_TOKEN, radiant: tokens.radiant, randomKeywords: tokens.keywords });
}

/** The faces differ only in the tokens they summon. */
function zaoGao(tokens: Tokens): Script {
  return {
    cry: () => [
      // R16, R354: "random" is stated, so the match rng picks and nobody is asked.
      discardRandom({ count: DISCARD_COUNT }),
      ...Array.from({ length: TOKEN_COUNT }, () => rushToken(tokens)),
    ],
  };
}

export const base: Script = zaoGao(BASE_TOKENS);

export const radiant: Script = zaoGao(RADIANT_TOKENS);
