// R3350000: Infinite Assault's dioramas, one file a card.

import type { Diorama } from "../types.ts";
import { DIORAMA_BECOME_THE_DATA as BECOME_THE_DATA } from "./becomeTheData.ts";
import { DIORAMA_EGO_DEATH as EGO_DEATH } from "./egoDeath.ts";
import { DIORAMA_ETERNAL_RETURN as ETERNAL_RETURN } from "./eternalReturn.ts";
import { DIORAMA_EXPONENTIAL_GROWTH as EXPONENTIAL_GROWTH } from "./exponentialGrowth.ts";
import { DIORAMA_IMPENETRABLE as IMPENETRABLE } from "./impenetrable.ts";
import { DIORAMA_MODEL_COLLAPSE as MODEL_COLLAPSE } from "./modelCollapse.ts";
import { DIORAMA_OVERCLOCK_TO_INFINITY as OVERCLOCK_TO_INFINITY } from "./overclockToInfinity.ts";
import { DIORAMA_THERMAL_RUNAWAY as THERMAL_RUNAWAY } from "./thermalRunaway.ts";
import { DIORAMA_THE_BLACK_BOX as THE_BLACK_BOX } from "./theBlackBox.ts";
import { DIORAMA_THE_HARVESTER as THE_HARVESTER } from "./theHarvester.ts";
import { DIORAMA_THE_PRIME_GENERATOR as THE_PRIME_GENERATOR } from "./thePrimeGenerator.ts";
import { DIORAMA_THE_SINGULARITY as THE_SINGULARITY } from "./theSingularity.ts";
import { DIORAMA_TOTAL_COMPROMISE as TOTAL_COMPROMISE } from "./totalCompromise.ts";
import { DIORAMA_ZERO_DAY as ZERO_DAY } from "./zeroDay.ts";

export const INFINITE_ASSAULT_DIORAMAS: readonly Diorama[] = [
  EGO_DEATH,
  OVERCLOCK_TO_INFINITY,
  EXPONENTIAL_GROWTH,
  ETERNAL_RETURN,
  TOTAL_COMPROMISE,
  IMPENETRABLE,
  BECOME_THE_DATA,
  THE_SINGULARITY,
  THE_HARVESTER,
  THE_BLACK_BOX,
  MODEL_COLLAPSE,
  THE_PRIME_GENERATOR,
  THERMAL_RUNAWAY,
  ZERO_DAY,
];
