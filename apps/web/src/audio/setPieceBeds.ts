// R2800000: the sound bed of each board clear's set piece (R2801000: and of a Starship's launch), #2785's
// sound hook: a short sequence of the existing `sfx.ts` recipes laid across its hold, sent as the held
// entry starts (director.ts). The
// card's own cast (Twisting Nether's "alien" voice and "rumble") sounds as ever with its play, and the
// removals the wipe makes sound once, condensed, after the hold. It reads only the entry's set piece,
// which keys on a definition the seat reads (R202); an entry with no hold (Big moments "off", Reduce
// motion, the effects off) has no bed.

import { STARSHIP_SET_PIECE, type AnimationEntry } from "../game/animations.ts";
import { FX_STARSHIP_COUNT_AT, FX_STARSHIP_IGNITE_AT, FX_STARSHIP_LAND_AT } from "../fx/constants.ts";
import { isSetPieceKey, type SetPieceKey } from "../fx/setPieceKeys.ts";
import type { SfxId, SfxParams, SoundCue } from "./types.ts";

/**
 * One sound of a bed: its recipe, its params and when it plays, as a fraction of the hold, or, `after`,
 * of the entry's motion once the hold is over (R2801000).
 */
export type SetPieceBedStep = { id: SfxId; at: number; params?: SfxParams; after?: true };

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

/** R2801000: the first countdown beep's clockTick `amount`; each later count beeps one higher. */
export const STARSHIP_BEEP_AMOUNT = 8;

/**
 * R2801000: a Starship launch's bed: a beep at each count of its countdown, rising, the engines' burn at
 * the ignition and, after the hold, the landing's heavy thud. Its own row's roar (`starshipLaunched`'s
 * `entrance`) sounds as the hold ends, when it lifts off (director.ts).
 */
export const STARSHIP_LAUNCH_BED: readonly SetPieceBedStep[] = [
  ...FX_STARSHIP_COUNT_AT.map((at, k): SetPieceBedStep => ({ id: "clockTick", at, params: { amount: STARSHIP_BEEP_AMOUNT + k } })),
  { id: "burn", at: FX_STARSHIP_IGNITE_AT },
  { id: "summon", at: FX_STARSHIP_LAND_AT, after: true, params: { slamTier: "huge" } },
];

function bedOf(key: string): readonly SetPieceBedStep[] {
  if (key === STARSHIP_SET_PIECE) return STARSHIP_LAUNCH_BED;
  return isSetPieceKey(key) ? SET_PIECE_BEDS[key] : [];
}

/**
 * R2800000: the bed of an entry's held set piece, each sound at its share of the hold (R2801000: an
 * `after` step at its share of the motion that follows); none for any other entry.
 */
export function setPieceBed(entry: Pick<AnimationEntry, "setPiece"> & Partial<Pick<AnimationEntry, "durationMs">>): SoundCue[] {
  const setPiece = entry.setPiece;
  if (setPiece === undefined) return [];
  const holdMs = setPiece.holdMs;
  const motionMs = Math.max(0, (entry.durationMs ?? holdMs) - holdMs);
  return bedOf(setPiece.key).map((step) => ({
    kind: "sfx",
    id: step.id,
    ...(step.params === undefined ? {} : { params: { ...step.params } }),
    delayMs: Math.round(step.after === true ? holdMs + step.at * motionMs : step.at * holdMs),
  }));
}
