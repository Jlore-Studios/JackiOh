// R1363 (docs/meditative-set.md M8, MN05): a shield flashes up as Armor takes a hit. Where the Armor
// took half or more of a hit that still landed (`absorbed`, R1360; `armorTookHalf`, game/damageFeel.ts),
// a small shield glances up with a few sparks; where it took all of it (`damageAbsorbed`, R1361), a
// full shield blooms with a ring and a burst of sparks.
//
// R202: the numbers are the event's, public on both seats; nothing here reads a card. R200: the flash
// starts with the hit, inside its entry, and is gone FX_SHIELD_TAIL_MS after it ends (≤ T). Under
// reduced motion the layer plans nothing (FxLayer.tsx).

import { ringCue, tunedBurst } from "./build.ts";
import { FX_SHIELD_TAIL_MS } from "./constants.ts";
import type { FxAnchor, FxCue, FxShieldCue } from "./types.ts";

/** The shield's particle counts at intensity "normal" (rule 9). */
const SHIELD_TUNING = {
  glance: { count: 10, power: 0.8 },
  bloom: { count: 24, power: 1.1 },
} as const;

/** The shield over `at` from `delayMs` (when the hit lands) to FX_SHIELD_TAIL_MS after the entry of length `D`. */
export function shieldCues(size: FxShieldCue["size"], at: FxAnchor, D: number, delayMs: number, intensity: number): FxCue[] {
  const start = Math.max(0, Math.min(D, delayMs));
  const flash: FxShieldCue = { kind: "shield", size, at, delayMs: start, durationMs: D - start + FX_SHIELD_TAIL_MS };
  if (size === "small") return [flash, tunedBurst(intensity, "spark", at, "ring", start, SHIELD_TUNING.glance)];
  return [flash, ringCue(D, "frost", at, start), tunedBurst(intensity, "spark", at, "ring", start, SHIELD_TUNING.bloom)];
}
