// R3340000: a Unit's art wears its state. Damage is cracks, one more for each step of its missing health
// (`damageLevel`), each a stroke grown from the picture's edge toward its middle (`crackPaths`) from the card's id, so the
// same card cracks the same way everywhere and a crack at one level is the start of the same crack at the next. The marks
// that need no path (a buffed unit's glow, a frozen one's frost) are CSS (artLive.css), drawn by ArtWear.tsx.
//
// Presentation only (CLAUDE.md rule 7): the levels read a view's health and maximum health, nothing else.

import { ART_WEAR_LEVELS } from "../constants.ts";
import { fmt, hashId, seededRandom } from "./hash.ts";
import { ART_BOX } from "./procedural.ts";

/** Salts the cracks' own stream, so they move nothing the picture draws. */
const WEAR_SALT = 0x3c6ef372;
/** Each crack's stream is the card's, this far on from the one before it (the golden ratio's 32-bit multiplier). */
const CRACK_STRIDE = 0x9e3779b1;
/** A crack's segments at the first level; each level above it adds one, so a deeper crack runs further in. */
export const CRACK_STEPS_BASE = 3;
/** How far one segment runs toward the picture's middle, in the picture's units. */
export const CRACK_STEP_LEN = 9;
/** How far a segment may stray from that line to either side, in the picture's units. */
export const CRACK_JITTER = 6;

/**
 * R3340000: how worn a unit's art is for the health it has left: 0 for a whole unit (or one with no maximum), then
 * 1 to `ART_WEAR_LEVELS`, one more for each share of its maximum it is missing, the last at a quarter left or less.
 */
export function damageLevel(health: number, maxHealth: number): number {
  if (maxHealth <= 0 || health >= maxHealth) return 0;
  const missing = Math.max(0, maxHealth - health);
  return Math.min(ART_WEAR_LEVELS, Math.max(1, Math.ceil((missing / maxHealth) * ART_WEAR_LEVELS)));
}

/**
 * R3340000: the cracks of a unit worn to `level` (0 to `ART_WEAR_LEVELS`), one path each, in the picture's own units. A
 * crack starts on the picture's border at a place its own stream picks and runs toward the middle in `CRACK_STEPS_BASE`
 * plus `level` segments, each straying by a drawn jitter; the jitter is drawn in step order, so the crack at `level + 1`
 * begins with the crack at `level`.
 */
export function crackPaths(seedId: string, level: number): string[] {
  const count = Math.max(0, Math.min(ART_WEAR_LEVELS, Math.floor(level)));
  const half = ART_BOX / 2;
  const paths: string[] = [];
  for (let k = 0; k < count; k += 1) {
    const rng = seededRandom((((hashId(seedId) ^ WEAR_SALT) >>> 0) + k * CRACK_STRIDE) >>> 0);
    // The crack's start: a point of the border, an angle round the picture picked by its stream.
    const angle = rng() * Math.PI * 2;
    const reach = half / Math.max(Math.abs(Math.cos(angle)), Math.abs(Math.sin(angle)));
    let x = half + Math.cos(angle) * reach;
    let y = half + Math.sin(angle) * reach;
    const steps = CRACK_STEPS_BASE + count;
    const parts = [`M${fmt(x)} ${fmt(y)}`];
    for (let step = 0; step < steps; step += 1) {
      // Toward the middle, with a sideways stray drawn for this step whatever the level.
      const toward = Math.atan2(half - y, half - x);
      const stray = (rng() * 2 - 1) * CRACK_JITTER;
      x += Math.cos(toward) * CRACK_STEP_LEN - Math.sin(toward) * stray;
      y += Math.sin(toward) * CRACK_STEP_LEN + Math.cos(toward) * stray;
      parts.push(`L${fmt(x)} ${fmt(y)}`);
    }
    paths.push(parts.join(" "));
  }
  return paths;
}
