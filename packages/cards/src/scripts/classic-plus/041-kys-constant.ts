// C+ #41 KY's Constant (SPEC §8.7 row 41, B3.4, R60, R81, R129, R386). (1) Spell, KY, Rare.
//   Choose a card in your hand. Change a random number on it to 3.
//   Radiant: Choose a card in your hand. Discover a number on it and change that number to 3.
//
// "A number on a card" is Degrade and Upgrade's (R386, `numbersOn`): its own cost (never an X), its
// attack and health, a numbered keyword's value, a declared number; an Immutable card has none. The
// card is a declared hand pick (R81) among your other hand cards with such a number not already 3;
// with none the Spell fizzles and still counts as played. Base: one of them at random (R60; one
// choice draws nothing, R129). Radiant: a Discover of up to 3 of them, shown to you only. The change
// is the engine's `setNumber`: `tuning` (the cost through `costMod`), kept in every zone and by a copy,
// reported by a `numberChanged` hidden as the card is.

import { numbersOn, type Script } from "@jackioh/engine";
import { chosenTuningNumber, discoverNumber, setNumber } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-041");

/** "change … to 3". */
const CONSTANT = 3;
/** §6.3 Discover: up to 3 options. */
const OFFERED = 3;

const targets: TargetDecl[] = [{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"], excludeSelf: true, check: "number" } }];
const targetChecks: Script["targetChecks"] = {
  number: ({ state, candidate }) => candidate !== null && numbersOn(state, candidate).some((entry) => entry.value !== CONSTANT),
};

export const base: Script = {
  targets,
  targetChecks,
  cry: () => [setNumber({ target: { of: "chosen" }, which: "random", value: CONSTANT })],
};

export const radiant: Script = {
  targets,
  targetChecks,
  cry: () => [discoverNumber({ target: { of: "chosen" }, value: CONSTANT, count: OFFERED, step: "picked" })],
  resume: {
    picked: (ctx) => {
      const pick = chosenTuningNumber(ctx);
      return pick === null ? [] : [setNumber({ instanceId: pick.instanceId, which: pick.which, value: CONSTANT })];
    },
  },
};
