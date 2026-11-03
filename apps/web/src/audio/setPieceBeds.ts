// R2800000: the sound bed of each board clear's set piece, #2785's sound hook: a short sequence of the
// existing `sfx.ts` recipes laid across its hold, sent as the held entry starts (director.ts). The
// card's own cast (Twisting Nether's "alien" voice and "rumble") sounds as ever with its play, and the
// removals the wipe makes sound once, condensed, after the hold. It reads only the entry's set piece,
// which keys on a definition the seat reads (R202); an entry with no hold (Big moments "off", Reduce
// motion, the effects off) has no bed.

import type { AnimationEntry } from "../game/animations.ts";
import { isSetPieceKey, type SetPieceKey } from "../fx/setPieceKeys.ts";
import type { SfxId, SfxParams, SoundCue } from "./types.ts";

/** One sound of a bed: its recipe, its params and when it plays, as a fraction of the hold. */
export type SetPieceBedStep = { id: SfxId; at: number; params?: SfxParams };

/** R2800000: every set piece's bed; a missing one fails `typecheck`. */
export const SET_PIECE_BEDS: { readonly [K in SetPieceKey]: readonly SetPieceBedStep[] } = {
  // The Nether opens with a rush of air, drains the board into the vortex, and swallows it.
  nether: [
    { id: "whoosh", at: 0 },
    { id: "drain", at: 0.4 },
    { id: "death", at: 1 },
  ],
  // The Void's fold drains the light, folds in, and the Void stands alone in a Mythic's sting.
  voidFold: [
    { id: "drain", at: 0 },
    { id: "whoosh", at: 0.5 },
    { id: "entrance", at: 1, params: { mythic: true } },
  ],
  // Every other board clear: the rush, the drain and a puff as the board empties.
  vortex: [
    { id: "whoosh", at: 0 },
    { id: "drain", at: 0.5 },
    { id: "poof", at: 1 },
  ],
};

/** R2800000: the bed of an entry's held set piece, each sound at its share of the hold; none for any other entry. */
export function setPieceBed(entry: Pick<AnimationEntry, "setPiece">): SoundCue[] {
  const setPiece = entry.setPiece;
  if (setPiece === undefined || !isSetPieceKey(setPiece.key)) return [];
  return SET_PIECE_BEDS[setPiece.key].map((step) => ({
    kind: "sfx",
    id: step.id,
    ...(step.params === undefined ? {} : { params: { ...step.params } }),
    delayMs: Math.round(step.at * setPiece.holdMs),
  }));
}
