// T-AI-1 Helpful Assistant (SPEC §8.7 row T-AI-1, §10.8, R60, R177, R218). (1) Unit, AI, Token; 1/3
// Taunt → 2/6 Taunt, Divine Shield. Made by C+ #43 AI Slop and C+ #78 Claude's Datacenter.
//   Cry: Discover a card from your deck. Radiant: it costs (1) less.
//
// A Discover whose pool is your own deck, as Core #51 reveals library cards: up to 3 different cards,
// shown to you only (§10.8); the chosen card moves from deck to hand, which is not a draw (no Cast on
// draw, no fatigue, no draw limit), and the hand cap burns it; an empty deck offers nothing. The
// Radiant discount is a `costMod`, so it stacks with every other (R65). AI cards declare no params.

import type { Script } from "@jackioh/engine";
import { addToHand, discoverFromLibrary } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-01");

/** Radiant: "It costs (1) less." */
const DISCOUNT = 1;

function assistant(costMod: number): Script {
  return {
    cry: () => [discoverFromLibrary({ step: "picked", prompt: "Discover a card from your deck" })],
    resume: { picked: () => [addToHand({ instance: { of: "chosen" }, ...(costMod === 0 ? {} : { costMod }) })] },
  };
}

export const base: Script = assistant(0);

export const radiant: Script = assistant(-DISCOUNT);
