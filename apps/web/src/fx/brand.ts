// R437: a mark branded onto a card (#50 K-Pop Fanatic's pending steal on its target).
//
// The `marked` event (via `markEventOf`, cards/marks.ts) says a mark came onto a card or left it. As
// it comes, a sigil in the mark's colours slams on and fades into the board's lasting aura
// (cards/CardMarks.tsx); as it goes, the colour puffs away. Colours come from the palette table the
// aura reads, so the two always match.
//
// R202: a card the viewer may not read is the sentinel, so nothing plays. R200: the sigil lasts
// D + FX_BRAND_TAIL_MS (≤ T) and the burst lands at FX_BRAND_SLAM_AT of the entry.

import type { GameEvent } from "@jackioh/shared";

import { markEventOf, paletteFor } from "../cards/marks.ts";
import { frac, ringCue, tunedBurst } from "./build.ts";
import { FX_BRAND_SLAM_AT, FX_BRAND_TAIL_MS } from "./constants.ts";
import type { FxAnchor, FxBrandCue, FxCue } from "./types.ts";

/** The brand's particle counts at intensity "normal" (rule 9). */
const BRAND_TUNING = {
  slam: { count: 28, power: 1.1 },
  glitter: { count: 14, power: 0.8 },
  fade: { count: 12, power: 0.6 },
} as const;

export function brandCues(event: GameEvent, at: FxAnchor, D: number, intensity: number): FxCue[] {
  const change = markEventOf(event);
  if (change === null) return [];
  const palette = paletteFor(change.color);
  if (!change.added) return [tunedBurst(intensity, palette.preset, at, "area", 0, BRAND_TUNING.fade)];
  const slam = frac(FX_BRAND_SLAM_AT, D);
  const sigil: FxBrandCue = {
    kind: "brand",
    at,
    tint: { rim: palette.rim, core: palette.core, glow: palette.glow },
    delayMs: 0,
    durationMs: D + FX_BRAND_TAIL_MS,
  };
  return [
    sigil,
    ringCue(D, palette.preset, at, slam),
    tunedBurst(intensity, palette.preset, at, "ring", slam, BRAND_TUNING.slam),
    tunedBurst(intensity, "sparkle", at, "area", slam, BRAND_TUNING.glitter),
  ];
}
