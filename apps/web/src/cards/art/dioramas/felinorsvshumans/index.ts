// R3350000: Felinors vs Humans' dioramas, one file a card: its eight Mythics, its fourteen tokens printed Mythic (Yile
// Prime and the thirteen Jilliax modules) and its four Mythic reprints (R5264000).

import type { Diorama } from "../types.ts";
import { DIORAMA_BLOODFORGED_CRUSADER as BLOODFORGED_CRUSADER } from "./bloodforgedCrusader.ts";
import { DIORAMA_DEBUFF_MODULE as DEBUFF_MODULE } from "./debuffModule.ts";
import { DIORAMA_DURABILITY_MODULE as DURABILITY_MODULE } from "./durabilityModule.ts";
import { DIORAMA_HALF_TRUE_CRAFT_A_CARD as HALF_TRUE_CRAFT_A_CARD } from "./halfTrueCraftACard.ts";
import { DIORAMA_HAYWIRE_MODULE as HAYWIRE_MODULE } from "./haywireModule.ts";
import { DIORAMA_HEAVY_MODULE as HEAVY_MODULE } from "./heavyModule.ts";
import { DIORAMA_INDESTRUCTIBLE_MODULE as INDESTRUCTIBLE_MODULE } from "./indestructibleModule.ts";
import { DIORAMA_JILLIAX_DELUXE as JILLIAX_DELUXE } from "./jilliaxDeluxe.ts";
import { DIORAMA_KY_HIMSELF as KY_HIMSELF } from "./kyHimself.ts";
import { DIORAMA_KYS_PATENTED_LOG_2_UTILITY as KYS_PATENTED_LOG_2_UTILITY } from "./kysPatentedLog2Utility.ts";
import { DIORAMA_PERFECT_MODULE as PERFECT_MODULE } from "./perfectModule.ts";
import { DIORAMA_PLAGIARIZED_EGO_DEATH as PLAGIARIZED_EGO_DEATH } from "./plagiarizedEgoDeath.ts";
import { DIORAMA_PLAGIARIZED_ETERNAL_RETURN as PLAGIARIZED_ETERNAL_RETURN } from "./plagiarizedEternalReturn.ts";
import { DIORAMA_PLAGIARIZED_MODEL_COLLAPSE as PLAGIARIZED_MODEL_COLLAPSE } from "./plagiarizedModelCollapse.ts";
import { DIORAMA_PLAGIARIZED_SINGULARITY as PLAGIARIZED_SINGULARITY } from "./plagiarizedSingularity.ts";
import { DIORAMA_POST_HUMAN_AESTHETIST as POST_HUMAN_AESTHETIST } from "./postHumanAesthetist.ts";
import { DIORAMA_POWER_MODULE as POWER_MODULE } from "./powerModule.ts";
import { DIORAMA_PYLON_MODULE as PYLON_MODULE } from "./pylonModule.ts";
import { DIORAMA_RECURSION_MODULE as RECURSION_MODULE } from "./recursionModule.ts";
import { DIORAMA_STARPIECE_MODULE as STARPIECE_MODULE } from "./starpieceModule.ts";
import { DIORAMA_THE_LAST_MEOW as THE_LAST_MEOW } from "./theLastMeow.ts";
import { DIORAMA_TWIN_MODULE as TWIN_MODULE } from "./twinModule.ts";
import { DIORAMA_UTILITY_MODULE as UTILITY_MODULE } from "./utilityModule.ts";
import { DIORAMA_VIRAL_MODULE as VIRAL_MODULE } from "./viralModule.ts";
import { DIORAMA_YILE_ACCELERATION_SUPREMIST as YILE_ACCELERATION_SUPREMIST } from "./yileAccelerationSupremist.ts";
import { DIORAMA_YILE_ACCELERATION_SUPREMIST_PRIME as YILE_ACCELERATION_SUPREMIST_PRIME } from "./yileAccelerationSupremistPrime.ts";

export const FELINORS_VS_HUMANS_DIORAMAS: readonly Diorama[] = [
  HALF_TRUE_CRAFT_A_CARD,
  BLOODFORGED_CRUSADER,
  YILE_ACCELERATION_SUPREMIST,
  YILE_ACCELERATION_SUPREMIST_PRIME,
  POST_HUMAN_AESTHETIST,
  THE_LAST_MEOW,
  KYS_PATENTED_LOG_2_UTILITY,
  KY_HIMSELF,
  JILLIAX_DELUXE,
  RECURSION_MODULE,
  DURABILITY_MODULE,
  HAYWIRE_MODULE,
  STARPIECE_MODULE,
  PYLON_MODULE,
  POWER_MODULE,
  HEAVY_MODULE,
  UTILITY_MODULE,
  TWIN_MODULE,
  VIRAL_MODULE,
  PERFECT_MODULE,
  DEBUFF_MODULE,
  INDESTRUCTIBLE_MODULE,
  PLAGIARIZED_EGO_DEATH,
  PLAGIARIZED_ETERNAL_RETURN,
  PLAGIARIZED_MODEL_COLLAPSE,
  PLAGIARIZED_SINGULARITY,
];
