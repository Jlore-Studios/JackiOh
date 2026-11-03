// R2949000 (#2949): a look with a picture of its own (the Signature look) draws its card's own painter seeded
// apart from the card's normal picture, by a seed of the card's id and the picture's name (`artSeedOf`): a
// Stinkyscape or Training Grounds card keeps its subject, and a procedural one what its name, tags and type pick
// (theme, composition, motif), the rest grown from the seed (ART.md, Pictures). The normal picture keeps the
// card's id alone, so it and every golden hash are unchanged.
// Presentation only: nothing here reaches the engine, an action, `PlayerView` or the log (CLAUDE.md rule 7).

/** R2949000: which of a card's pictures a face draws: its normal one, or the Signature look's own. */
export type ArtPicture = "normal" | "signature";

/** What joins a card's id to a picture's name in its seed: no catalog id holds it. */
const PICTURE_SEED_SEPARATOR = "#";

/** R2949000: the id a card's painter seeds `picture` by: the card's own id for its normal picture. */
export function artSeedOf(defId: string, picture: ArtPicture): string {
  return picture === "normal" ? defId : `${defId}${PICTURE_SEED_SEPARATOR}${picture}`;
}
