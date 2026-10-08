// R1363 (docs/meditative-set.md M8, MN05): a shield flashes up as Armor takes a hit.
//
// The `damage` event carries the Armor's part of the hit (`absorbed`, R1360) and `damageAbsorbed`
// reports a hit the Armor took whole (R1361). Where the Armor took half or more of a hit that still
// landed (`armorTookHalf`, game/damageFeel.ts, the test the sound's dull clank keys on too), a small
// shield glances up over the target as the hit lands, with a few sparks; where it took all of it, a
// full shield blooms with a bright ring and a burst of sparks thrown off its rim.
//
// R202: the numbers are the event's, public on both seats, and the anchor is the target the row
// resolves; nothing here reads a card. R200: the flash starts with the hit, inside its entry, and is
// gone FX_SHIELD_TAIL_MS after the entry ends (≤ T). Under reduced motion the layer plans nothing, so
// the shield is never drawn (FxLayer.tsx).

import { ringCue, tunedBurst } from "./build.ts";
import { FX_SHIELD_TAIL_MS } from "./constants.ts";
import type { FxAnchor, FxCue, FxShieldCue } from "./types.ts";

/** The shield's particle counts at intensity "normal" (rule 9). */
const SHIELD_TUNING = {
  glance: { count: 10, power: 0.8 },
  bloom: { count: 24, power: 1.1 },
} as const;

/**
 * The shield over `at` from `delayMs` (the moment the hit lands) to FX_SHIELD_TAIL_MS after the entry
 * of length `D`: a small glance with a few sparks, or a full bloom with a ring and a spray of sparks.
 */
export function shieldCues(size: FxShieldCue["size"], at: FxAnchor, D: number, delayMs: number, intensity: number): FxCue[] {
  const start = Math.max(0, Math.min(D, delayMs));
  const flash: FxShieldCue = { kind: "shield", size, at, delayMs: start, durationMs: D - start + FX_SHIELD_TAIL_MS };
  if (size === "small") return [flash, tunedBurst(intensity, "spark", at, "ring", start, SHIELD_TUNING.glance)];
  return [flash, ringCue(D, "frost", at, start), tunedBurst(intensity, "spark", at, "ring", start, SHIELD_TUNING.bloom)];
}
