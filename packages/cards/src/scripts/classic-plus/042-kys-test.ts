// C+ #42 KY's Test (SPEC §8.7 row 42, R420, R465, R580). (1) Spell, KY, Legendary.
//   Offer an Easy, a Medium and a Hard problem, each showing a random reward from its list; choose one
//   and answer it; if you're right, gain its reward. Radiant: the same, and the reward cards are Radiant.
//
// The whole card is the engine's question-bank subsystem (`subsystems/kyTest.ts`, E31): the reward
// rolls, the two prompts, the Easy generator (R580) and the grade. This file hands it the bank of
// Medium and Hard problems, which is card data (`../../kyTestBank.ts`). Both faces run the same script:
// the grade step reads the face that asked, so the Radiant face's reward cards are Radiant.

import { subsystems, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";
import { KY_TEST_BANK } from "../../kyTestBank";

export const def = cardDef("classicplus-042");

export const base: Script = subsystems.kyTestScript(KY_TEST_BANK);

export const radiant: Script = base;
