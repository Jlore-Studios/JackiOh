// R3350000: Meditative+'s dioramas, one file a card.

import type { Diorama } from "../types.ts";
import { DIORAMA as BOBS_PLAN } from "./bobsPlan.ts";
import { DIORAMA as SCRATCH_PAD } from "./scratchPad.ts";

export const MEDITATIVEPLUS_DIORAMAS: readonly Diorama[] = [BOBS_PLAN, SCRATCH_PAD];
