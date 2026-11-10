// R3350000: Core's dioramas, one file a card.

import type { Diorama } from "../types.ts";
import { DIORAMA as MY_PAWN } from "./myPawn.ts";
import { DIORAMA as ZEPHRYS } from "./zephrys.ts";
import { DIORAMA as HEROIC_POWER } from "./heroicPower.ts";
import { DIORAMA as CRAFT_A_CARD } from "./craftACard.ts";
import { DIORAMA as CEASELESS_VOID } from "./ceaselessVoid.ts";

export const CORE_DIORAMAS: readonly Diorama[] = [MY_PAWN, ZEPHRYS, HEROIC_POWER, CRAFT_A_CARD, CEASELESS_VOID];
