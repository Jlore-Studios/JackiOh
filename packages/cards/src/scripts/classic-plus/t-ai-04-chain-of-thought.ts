// T-AI-4 Chain of Thought (SPEC §8.7 row T-AI-4, §7, B8). (1) Spell, AI, Token.
//   Base:    "Draw 1. If it costs (1) or less, repeat this, up to 4 more times."
//   Radiant: "Draw 1. If it costs (2) or less, repeat this, up to 4 more times."
//   Engine:  "'It' is the card the draw put in your hand, its cost read per R65 as it arrives (an X-cost
//            card as 0). A card cast on draw never gets there (R58), a burned one isn't there, and a
//            fatigue draw brings none, so each ends the chain, as does a draw the draw limit stops (§2.4,
//            `drawLimited`). At most five draws (`CHAIN_OF_THOUGHT_REPEATS`, 4 repeats after the first);
//            a prompt inside one parks the rest on `state.work` (R158). Tunes: none."
//
// The chain is one engine verb (`drawWhileCheap`, effects/datacenter.ts) over `drawOne` and the card
// that draw put in the hand (`cardThisDrawPutInHand`, R596): a cast-on-draw card ends it, the card its
// cast's own repeat brings included, so the only prompt a draw can open ends the chain where it stands.

import type { Script } from "@jackioh/engine";
import { CHAIN_OF_THOUGHT_REPEATS } from "@jackioh/engine";
import { drawWhileCheap } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-04");

/** §8.7: "If it costs (1) or less", Radiant "(2) or less". An AI card declares no params (B8). */
const MAX_COST = { base: 1, radiant: 2 } as const;

export const base: Script = { cry: () => [drawWhileCheap({ maxCost: MAX_COST.base, repeats: CHAIN_OF_THOUGHT_REPEATS })] };

export const radiant: Script = { cry: () => [drawWhileCheap({ maxCost: MAX_COST.radiant, repeats: CHAIN_OF_THOUGHT_REPEATS })] };
