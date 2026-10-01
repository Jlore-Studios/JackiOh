// T-AI-10 Fine-Tuning (SPEC §8.7 row T-AI-10, R386; BUILD M9 row T-AI-10). (2) Field Spell, AI, Token.
//   Base:    "End of turn: Upgrade a random card in your hand."
//   Radiant: "End of turn: Upgrade 2 random cards in your hand."
//
// At its controller's end of turn (R62): Upgrade (R386) of different random hand cards (R60); a card no
// change fits, or an Immutable one, is unchanged; an empty hand draws nothing (R129); the other player
// learns only that a card of that hand changed (R97, R440). AI cards declare no numbers (B8).

import type { Script } from "@jackioh/engine";
import { upgrade } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-10");

const CARDS = 1;
const RADIANT_CARDS = 2;

export const base: Script = {
  endOfTurn: () => [upgrade({ scope: { zones: ["hand"] }, random: CARDS })],
};

export const radiant: Script = {
  endOfTurn: () => [upgrade({ scope: { zones: ["hand"] }, random: RADIANT_CARDS })],
};
