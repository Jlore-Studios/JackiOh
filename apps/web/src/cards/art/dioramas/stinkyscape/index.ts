// R3350000: Stinkyscape's dioramas, one file a card.

import type { Diorama } from "../types.ts";
// The scene cards' (scene.ts, R3331000):
import { DIORAMA as TYLANNT } from "./tylannt.ts";
import { DIORAMA as ROD_OF_AGES } from "./rodOfAges.ts";
import { DIORAMA as DRONE_CORE_X } from "./droneCoreX.ts";
import { DIORAMA as GREAT_PAPERCLIP_THEORY } from "./greatPaperclipTheory.ts";
import { DIORAMA as PARAGON_X } from "./paragonX.ts";
import { DIORAMA as PROTOTYPE_X2 } from "./prototypeX2.ts";
import { DIORAMA as XX_FRESH_JACK } from "./xxFreshJack.ts";
import { DIORAMA as I_KWZKI } from "./iKwzki.ts";
import { DIORAMA as SHOCKWAVE447 } from "./shockwave447.ts";
import { DIORAMA as ALTERAN_REACH } from "./alteranReach.ts";
import { DIORAMA as EDGE_OF_THE_UNIVERSE } from "./edgeOfTheUniverse.ts";
import { DIORAMA as RED_NARCOR } from "./redNarcor.ts";
// The painter's own (stinkyscape.ts `stinkyParts`):
import { DIORAMA as OLIVIAN_EXPANSE } from "./olivianExpanse.ts";
import { DIORAMA as FAREWELL } from "./farewell.ts";
import { DIORAMA as KYS_LAST_EXAM } from "./kysLastExam.ts";
import { DIORAMA as EVENT_HORIZON } from "./eventHorizon.ts";
import { DIORAMA as LAST_WORD } from "./lastWord.ts";
import { DIORAMA as FINAL_DESTINATION } from "./1v1FinalDestination.ts";
import { DIORAMA as THE_BIG_CRUNCH } from "./theBigCrunch.ts";
import { DIORAMA as DICE_OF_DOOM } from "./diceOfDoom.ts";
import { DIORAMA as ANOMALY_FOUND } from "./anomalyFound.ts";
import { DIORAMA as DRONE_BATTLESHIP_MELTDOWN_LAYER } from "./droneBattleshipMeltdownLayer.ts";
import { DIORAMA as DRONE_BATTLESHIP_PROPAGATION_LAYER } from "./droneBattleshipPropagationLayer.ts";
import { DIORAMA as DRONE_BATTLESHIP_SHIELD_LAYER } from "./droneBattleshipShieldLayer.ts";

export const STINKYSCAPE_DIORAMAS: readonly Diorama[] = [
  // The scene cards':
  TYLANNT,
  ROD_OF_AGES,
  DRONE_CORE_X,
  GREAT_PAPERCLIP_THEORY,
  PARAGON_X,
  PROTOTYPE_X2,
  XX_FRESH_JACK,
  I_KWZKI,
  SHOCKWAVE447,
  ALTERAN_REACH,
  EDGE_OF_THE_UNIVERSE,
  RED_NARCOR,
  // The painter's own:
  OLIVIAN_EXPANSE,
  FAREWELL,
  KYS_LAST_EXAM,
  EVENT_HORIZON,
  LAST_WORD,
  FINAL_DESTINATION,
  THE_BIG_CRUNCH,
  DICE_OF_DOOM,
  ANOMALY_FOUND,
  DRONE_BATTLESHIP_MELTDOWN_LAYER,
  DRONE_BATTLESHIP_PROPAGATION_LAYER,
  DRONE_BATTLESHIP_SHIELD_LAYER,
];
