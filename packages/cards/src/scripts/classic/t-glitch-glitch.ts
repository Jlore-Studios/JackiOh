// T-Glitch Glitch (SPEC §7; R661–R664). (0) Spell, Token, hidden: the Easter egg of C #18 Glitch in
// the System and C #25 Lag in the System.
//   Base:    "Always playable on your turn. Something happens at random."
//   Radiant: "Always playable on your turn. Draw 1. Something happens at random." — R275's draw rider,
//            the standard's way up for a Spell whose effect has no number to double.
//
// No card generates it by name. R661's replacement is the only way a game makes one: once a System
// card has been played, a card an effect makes from a random pool may be Glitch instead
// (`catalog.pickGenerated`), and `hidden` keeps it out of every pool (R662). That is the engine's, not
// this file's: a card file owns what the card does when it is played and nothing about how it got
// into a hand.
//
// "Always playable on your turn" is a static flag (R663): the engine prices it at (0) whatever would
// change that and lets no ban refuse it. "Something happens at random" is the outcome roll (R664),
// one draw of the match rng over four outcomes, none of them built yet.

import type { Script } from "@jackioh/engine";
import { draw, glitchOutcome } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-t-glitch");

/** §7: the Radiant face's draw rider (R275). */
const RADIANT_DRAW = 1;

export const base: Script = {
  staticFlags: { alwaysPlayable: true },
  cry: () => [glitchOutcome()],
};

export const radiant: Script = {
  staticFlags: { alwaysPlayable: true },
  cry: () => [draw({ count: RADIANT_DRAW }), glitchOutcome()],
};
