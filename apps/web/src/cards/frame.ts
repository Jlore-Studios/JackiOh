// What a face's frame is drawn with: the rarity its gem and crest show and the foil it wears. Art only
// (docs/polish/6-cards.md, Surface B): the words on the frame, its tags and type line, are the face
// model's (model.ts, jackioh-shared), and nothing here is a rule (CLAUDE.md rule 7).

import type { Rarity } from "@jackioh/shared";

import type { FaceModel } from "./model.ts";

/** R1382: the tag chip the frame prints in place of the five tribal tags on a card with them all (`FaceModel.tagChips`). */
export const ALL_TRIBES = "All Tribes";

/** R503: the rarity a face's frame is drawn in. Display only; nothing reads it as the card's rarity. */
export function frameRarity(face: Pick<FaceModel, "rarity" | "printedRarity">): Rarity | null {
  return face.printedRarity ?? face.rarity;
}

/**
 * `data-foil` on `.cf`: a Radiant face carries foil, and a Mythic one while the viewer's Rarity
 * foil setting is on (R2721000), animated while the player's `animatedFoil` setting is on and
 * still while it is off. The CSS never animates it under `prefers-reduced-motion: reduce`
 * whatever this says.
 */
export function foilFor(
  face: Pick<FaceModel, "radiant" | "rarity">,
  animatedFoil: boolean,
  rarityFoil: boolean,
): "animated" | "static" | "none" {
  if (!face.radiant && !(rarityFoil && face.rarity === "Mythic")) return "none";
  return animatedFoil ? "animated" : "static";
}
