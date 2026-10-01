// T-AI-3 Hallucination (SPEC §8.7 row T-AI-3, R57, R385; BUILD M9 row T-AI-3). (0) Spell, AI, Token.
//   Base:    "Add a copy of a random card in your opponent's deck to your hand. Give it Brittle 2."
//   Radiant: "Add copies of 2 different random cards in your opponent's deck to your hand. Give them Brittle 2."
//
// `addLibraryCopies` (engine): new cards you own carrying the picked cards' definition, radiant flag,
// `statsOverride` and `tuning` (R57), the originals staying in their deck; only you learn what they
// copied (R97, R177). Brittle 2 is given as each lands (R385), so a copy made on your turn t ticks to 1
// at the start of t + 2 and crumbles at the start of t + 4. An empty deck gives nothing (R129); the hand
// cap burns extras (§2.4). AI cards declare no numbers (B8), so the counts are this file's.

import type { Script } from "@jackioh/engine";
import { addLibraryCopies } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-03");

const COPIES = 1;
const RADIANT_COPIES = 2;
const BRITTLE = 2;

export const base: Script = {
  cry: () => [addLibraryCopies({ of: "enemy", count: COPIES, brittle: BRITTLE })],
};

export const radiant: Script = {
  cry: () => [addLibraryCopies({ of: "enemy", count: RADIANT_COPIES, brittle: BRITTLE })],
};
