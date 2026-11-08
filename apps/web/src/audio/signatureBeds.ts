// R3389000: the sound bed of each signature (#3389): a short sequence of the existing `sfx.ts` recipes
// laid across the entry its card makes its entrance in (a Unit's or a Field Spell's landing, a Spell's
// play, a Trap's firing), sent as that entry starts (director.ts), as a set piece's bed is laid across
// its hold (setPieceBeds.ts, R2800000). Each card's is its own, and it plays beside the card's intro
// sting (R3332002) and its events' own sounds, which sound as ever. It keys only on the entry's
// `entrance`, which names a card the seat reads (R202); an entry with no entrance has no bed. At most
// SIGNATURE_BED_MAX_STEPS sounds, so a bed stays inside SFX_MAX_VOICES (audio/constants.ts).

import type { AnimationEntry } from "../game/animations.ts";
import type { SfxId, SfxParams, SoundCue } from "./types.ts";

/** One sound of a signature's bed: its recipe, its params and when it plays, as a share of the entry. */
export type SignatureBedStep = { id: SfxId; at: number; params?: SfxParams };

/** R3389000: the most sounds a signature's bed lays. */
export const SIGNATURE_BED_MAX_STEPS = 4;

/** R3389000: every signature's bed, by its card's id. */
export const SIGNATURE_BEDS: Readonly<Record<string, readonly SignatureBedStep[]>> = {
  // Palantir: a glassy hum as the orb opens, then a shutter's click.
  "classic-004": [
    { id: "radiant", at: 0 },
    { id: "lock", at: 0.6 },
  ],
  // InfiniScepter: a crystal chime as it plants, a rising ring as its locket opens.
  "classic-007": [
    { id: "radiant", at: 0.1 },
    { id: "upgrade", at: 0.6 },
  ],
  // Income Tax: the strongbox slams, coins clatter, the quill scratches.
  "classic-009": [
    { id: "lock", at: 0 },
    { id: "goldBurst", at: 0.3 },
    { id: "steal", at: 0.6 },
  ],
  // Second Wind: a rushing gust, sand blown off, a long sigh of wind.
  "classic-028": [
    { id: "whoosh", at: 0 },
    { id: "sand", at: 0.5 },
    { id: "drain", at: 0.8 },
  ],
  // Joro: a silk zip, a boing as it lands, a cheeky chuckle.
  "classic-033": [
    { id: "whoosh", at: 0 },
    { id: "summon", at: 0.6 },
    { id: "emoteLaugh", at: 0.85 },
  ],
  // Back from the GY: a thunder rumble, the earth cracking, a chorus of groans.
  "classic-044": [
    { id: "impact", at: 0 },
    { id: "crumble", at: 0.3 },
    { id: "death", at: 0.7 },
  ],
  // Nature Titan: a deep wooden groan, a stone thud, the heal's birdsong.
  "classic-045": [
    { id: "tapWood", at: 0 },
    { id: "summon", at: 0.6, params: { slamTier: "huge" } },
    { id: "heal", at: 0.85 },
  ],
  // Spell Tyrant: a commanding horn blast, then a whoosh as it rises.
  "classic-056": [
    { id: "horn", at: 0 },
    { id: "whoosh", at: 0.6 },
  ],
  // BOOM! Big Max: the fuse hisses, a huge boom, rubble rains.
  "classic-080": [
    { id: "fuse", at: 0 },
    { id: "overkill", at: 0.6 },
    { id: "crumble", at: 0.8 },
  ],
  // King Wagtoggle: a hearty laugh, a handshake slap, a riffle.
  "classic-085": [
    { id: "emoteLaugh", at: 0 },
    { id: "give", at: 0.4 },
    { id: "riffle", at: 0.7 },
  ],
  // In Too Deep: a deep water rumble, the rush down the shaft, a far bell.
  "classic-090": [
    { id: "drain", at: 0 },
    { id: "whoosh", at: 0.4 },
    { id: "notify", at: 0.9 },
  ],
};

/** R3389000: the bed of the signature whose entrance `entry` makes, each sound at its share of the entry; none for any other. */
export function signatureBed(entry: Pick<AnimationEntry, "entrance" | "durationMs">): SoundCue[] {
  const defId = entry.entrance?.defId;
  const bed = defId === undefined ? undefined : SIGNATURE_BEDS[defId];
  if (bed === undefined) return [];
  return bed.map((step) => ({
    kind: "sfx",
    id: step.id,
    ...(step.params === undefined ? {} : { params: { ...step.params } }),
    delayMs: Math.round(step.at * entry.durationMs),
  }));
}
