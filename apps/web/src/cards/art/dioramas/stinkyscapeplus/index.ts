// R3350000: Stinkyscape+'s dioramas, one file a card.

import type { Diorama } from "../types.ts";
// The scene cards' (scene.ts, R3331000; stinkyscapePlusRecipes.ts):
import { DIORAMA as FORESHADOW } from "./foreshadow.ts";
import { DIORAMA as BLACK_HOLE } from "./blackHole.ts";
import { DIORAMA as SLASHED_DREAM_CRIES_IN_RED } from "./slashedDreamCriesInRed.ts";
import { DIORAMA as A_GLIMPSE_INTO_THE_ENDLESS_SEA } from "./aGlimpseIntoTheEndlessSea.ts";
import { DIORAMA as THE_LAST_VOYAGE } from "./theLastVoyage.ts";
import { DIORAMA as DOOMSAYER } from "./doomsayer.ts";
import { DIORAMA as ENDLESS_EXPEDITION } from "./endlessExpedition.ts";
import { DIORAMA as ANIMOSITY } from "./animosity.ts";
import { DIORAMA as TOXOPID } from "./toxopid.ts";
import { DIORAMA as GREAT_PAPERCLIP_THEORY_II } from "./greatPaperclipTheoryII.ts";
import { DIORAMA as GREAT_PAPERCLIP_THEORY_III } from "./greatPaperclipTheoryIII.ts";
import { DIORAMA as THE_FAR_VAGABOND } from "./theFarVagabond.ts";
// The painter's own under the Mindustrial overlay (stinkyscapeplus.ts `stinkyscapePlusSvg`):
import { DIORAMA as TOXOPID_LEG } from "./toxopidLeg.ts";
import { DIORAMA as TEARS_OF_THE_KWZKI_TREE } from "./tearsOfTheKwzkiTree.ts";
import { DIORAMA as SUPERNOVA } from "./supernova.ts";
import { DIORAMA as ALNILAM } from "./alnilam.ts";
import { DIORAMA as THE_ENIGMATA } from "./theEnigmata.ts";

export const STINKYSCAPEPLUS_DIORAMAS: readonly Diorama[] = [
  // The scene cards':
  FORESHADOW,
  BLACK_HOLE,
  SLASHED_DREAM_CRIES_IN_RED,
  A_GLIMPSE_INTO_THE_ENDLESS_SEA,
  THE_LAST_VOYAGE,
  DOOMSAYER,
  ENDLESS_EXPEDITION,
  ANIMOSITY,
  TOXOPID,
  GREAT_PAPERCLIP_THEORY_II,
  GREAT_PAPERCLIP_THEORY_III,
  THE_FAR_VAGABOND,
  // The painter's own:
  TOXOPID_LEG,
  TEARS_OF_THE_KWZKI_TREE,
  SUPERNOVA,
  ALNILAM,
  THE_ENIGMATA,
];
