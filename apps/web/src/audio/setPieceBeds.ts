// R2800000: the sound bed of each board clear's set piece (R2801000: and of a Starship's launch), #2785's
// sound hook: a short sequence of the existing `sfx.ts` recipes laid across its hold, sent as the held
// entry starts (director.ts). The card's own cast (Twisting Nether's "alien" voice and "rumble") sounds
// as ever with its play, and the removals the wipe makes sound once, condensed, after the hold.
// R3338000: each Terminus card's set piece, Ceaseless Void's fold among them, is its own sting: its bed
// opens with an `organ` chord in a key of its own (R1883000's Terminus intros' organ), lays two or three
// recipes of its moment over it and ends in a Mythic's choir (`entrance`), and its signature run sounds
// once, condensed, after the hold, as a wipe does. It reads only the entry's set piece, which keys on a
// definition the seat reads (R202); an entry with no hold (Big moments "off", Reduce motion, the effects
// off) has no bed. R3334001: a launch's bed is the Starship's own ignition and roar (sfx.ts), timed to
// its cinematic; its other moments sound from starshipSounds.ts.

import { STARSHIP_SET_PIECE, type AnimationEntry } from "../game/animations.ts";
import { FX_STARSHIP_LAND_AT } from "../fx/constants.ts";
import { isSetPieceKey, type SetPieceKey } from "../fx/setPieceKeys.ts";
import { STARSHIP_CINEMATIC } from "../fx/starship.ts";
import type { SfxId, SfxParams, SoundCue } from "./types.ts";

/**
 * One sound of a bed: its recipe, its params and when it plays, as a fraction of the hold, or, `after`,
 * of the entry's motion once the hold is over (R2801000).
 */
export type SetPieceBedStep = { id: SfxId; at: number; params?: SfxParams; after?: true };

/** R3338000: a Terminus bed's first step, its own organ chord (`amount`, sfx.ts ORGAN_ROOTS_HZ), at once. */
const organ = (amount: number): SetPieceBedStep => ({ id: "organ", at: 0, params: { amount } });

/** R3338000: a Terminus bed's last step, a Mythic's sting and its choir as the hold ends. */
const MYTHIC_CHOIR: SetPieceBedStep = { id: "entrance", at: 1, params: { mythic: true } };

/** R2800000: every set piece's bed; a missing one fails `typecheck`. */
export const SET_PIECE_BEDS: { readonly [K in SetPieceKey]: readonly SetPieceBedStep[] } = {
  // The Nether opens with a rush of air, drains the board into the vortex, and swallows it.
  nether: [
    { id: "whoosh", at: 0 },
    { id: "drain", at: 0.4 },
    { id: "death", at: 1 },
  ],
  // The Void's fold: its organ, the light drains, it folds in, and the Void stands alone in a Mythic's sting.
  voidFold: [
    organ(1),
    { id: "drain", at: 0 },
    { id: "whoosh", at: 0.5 },
    MYTHIC_CHOIR,
  ],
  // Every other board clear: the rush, the drain and a puff as the board empties.
  vortex: [
    { id: "whoosh", at: 0 },
    { id: "drain", at: 0.5 },
    { id: "poof", at: 1 },
  ],
  // R3338000: Olivian Expanse's rift tears open, its seven stars glint as they circle, and the one it
  // gives is drawn out toward its hand.
  terminusRift: [
    organ(2),
    { id: "whoosh", at: 0.2 },
    { id: "radiant", at: 0.5 },
    { id: "draw", at: 0.9 },
    MYTHIC_CHOIR,
  ],
  // Farewell: the light drains, each card it takes lifts softly into the lamp, and a puff of smoke.
  farewell: [
    organ(3),
    { id: "drain", at: 0.2 },
    { id: "brand", at: 0.5, params: { release: true } },
    { id: "poof", at: 0.85 },
    MYTHIC_CHOIR,
  ],
  // KY's Last Exam: the chalk scrawls down the slate and back, and the slate cracks under the stamp.
  lastExam: [
    organ(4),
    { id: "attack", at: 0.2 },
    { id: "draw", at: 0.45 },
    { id: "crumble", at: 0.85 },
    MYTHIC_CHOIR,
  ],
  // Event Horizon: the clock ticks at each of its turn banners, the second lower and longer as time
  // stretches at the edge, and the bent board rushes past.
  eventHorizon: [
    organ(5),
    { id: "clockTick", at: 0.15, params: { amount: 5 } },
    { id: "clockTick", at: 0.55, params: { amount: 1 } },
    { id: "whoosh", at: 0.8 },
    MYTHIC_CHOIR,
  ],
  // Last Word: frost glints across the board, then cracks and shatters at its caster's hero.
  lastWord: [
    organ(6),
    { id: "radiant", at: 0.1 },
    { id: "shieldShatter", at: 0.5 },
    MYTHIC_CHOIR,
  ],
  // 1v1 Final Destination: chains clank and lock on the lanes, and the heroes' life drains to its mark.
  finalDestination: [
    organ(7),
    { id: "armorClank", at: 0.25 },
    { id: "lock", at: 0.5 },
    { id: "drain", at: 0.7 },
    MYTHIC_CHOIR,
  ],
  // The Big Crunch: everything is pulled into one point, and it implodes.
  bigCrunch: [
    organ(8),
    { id: "fuse", at: 0.3 },
    { id: "overkill", at: 0.8 },
    MYTHIC_CHOIR,
  ],
  // Dice of Doom: the d100 rattles in, Call to Chaos's roll with no ding, and lands heavily as it stops.
  diceOfDoom: [
    organ(9),
    { id: "chaosRoll", at: 0, params: { amount: 0 } },
    { id: "summon", at: 0.5, params: { slamTier: "large" } },
    MYTHIC_CHOIR,
  ],
};

/**
 * R2801000, R3334001: a Starship launch's bed: its engines igniting as the cinematic lights them, its
 * roar as it climbs out of frame and, after the hold, the landing's heavy thud. Its own row's
 * `entrance` sounds as the hold ends, when it lifts off (director.ts).
 */
export const STARSHIP_LAUNCH_BED: readonly SetPieceBedStep[] = [
  { id: "starshipIgnite", at: STARSHIP_CINEMATIC.enginesFrom },
  { id: "starshipRoar", at: STARSHIP_CINEMATIC.climbAt },
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
