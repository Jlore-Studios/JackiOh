// T-AI-2 Scaling Law (SPEC §8.7 row T-AI-2; BUILD M9 row T-AI-2). (2) Unit, AI, Token, 2/2 → 4/4.
//   Base:    "Has +1/+1 for each AI generated card you've played this game."
//   Radiant: "Has +2/+2 for each AI generated card you've played this game."
//
// A §10.4 layer-2 `setStat` over its controller's per-game count of AI-tagged plays (B5 E4,
// `playedThisGameWithTag`): casts count (R70), itself once played (step 4 counts a play before its
// stats are read), a fused card carrying the tag once; the count outlives the cards. AI cards declare
// no numbers (B8), so the per-card bonus is this file's.

import { playedThisGameWithTag, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-02");

/** "+1/+1" (Radiant "+2/+2") for each AI generated card played. */
const PER_AI_CARD = 1;
const RADIANT_PER_AI_CARD = 2;

export const base: Script = {
  setStat: ({ state, self, radiant }) => {
    const bonus = (radiant ? RADIANT_PER_AI_CARD : PER_AI_CARD) * playedThisGameWithTag(state, self.controller, "AI");
    return { attack: bonus, maxHealth: bonus };
  },
};

// The same script: the hook reads the running face.
export const radiant: Script = base;
