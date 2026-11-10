// R3402001: #52 Silly Silas (`core-052`), a Legendary Unit's signature (R3332000): entrance "silasSpotlight",
// recipe "mirrorballShuffle" and a `mirrorball` aura on his own zone (R3402000). He pirouettes down a
// spotlight from the top of the screen, lands heel-first in a ring-shaped crater and tips an imaginary hat
// in a glint; his Cry rotates the field, so on its `rotated` a mirrorball ring and confetti spin at the
// board's centre and every occupied unit zone takes a sheen as its card glides a step, and each card the
// rotation hands to the other side wears a brand in its new owner's colour on its `controlChanged`. It
// anchors on the zones, the board and the viewport alone and names no card (R202).

import type { PlayerView } from "@jackioh/shared";

import { sideOf, testid, type Side } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_CRACK_TAIL_MS, FX_SIGNATURE_BURSTS_MAX, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import { timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint, FxShotCue, FxTint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { withinBursts } from "./flourish.ts";

/** Every particle count Silas throws, at intensity "normal" (rule 9). */
const TUNING = {
  dust: { count: 22, power: 1.1 },
  hat: { count: 10, power: 0.8 },
  confetti: { count: 26, power: 1.1 },
} as const;

/** His look: a violet-and-teal hologram, the mirrorball's. */
const SILAS_PALETTE = "Tech/hologram";

/** Where his spotlight drops from: the top of the screen, over the board's middle. */
const SPOTLIGHT_FROM: FxPoint = { x: 0.5, y: 0 };

/** Where he tips his hat: the top corner of his card. */
const HAT_AT: FxPoint = { x: 0.75, y: 0.1 };

/** R3402001: the colour a card's border takes as the rotation hands it to a side, by its new owner's side. */
export const SILAS_SIDE_TINTS: Readonly<Record<Side, FxTint>> = {
  you: { rim: "#7dffe0", core: "#f0fffb", glow: "#2bd4b4" },
  opponent: { rim: "#b48cff", core: "#f4ecff", glow: "#7c4ddb" },
};

function silasLook(): FlavorLook {
  return paletteLook(SILAS_PALETTE) ?? FLAVOR_LOOKS.Chaos;
}

export const silasSpotlight: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const look = silasLook();
  const pirouette: FxShotCue = {
    kind: "shot",
    shot: { motion: "arc", sprites: ["emoji"], turn: "spin", count: 1, trail: "sparkle" },
    colors: look.colors,
    preset: "sparkle",
    from: { kind: "viewport", at: SPOTLIGHT_FROM },
    to: at,
    delayMs: 0,
    flightMs: slam,
    density: i,
  };
  return [
    raysCue(D, "holy", at, 0),
    pirouette,
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    ringCue(D, "dust", at, slam),
    tunedBurst(i, "dust", tid(tgt, FOOT), "ring", slam, TUNING.dust),
    { ...tunedBurst(i, "sparkle", tid(tgt, HAT_AT), "point", slam, TUNING.hat), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.silasSpotlight, slam),
  ];
};

/** The unit zones the view shows a card in, both sides, the first FX_SIGNATURE_BURSTS_MAX of them. */
function occupiedZones(view: PlayerView, max: number): string[] {
  const sides: readonly Side[] = ["you", "opponent"];
  return sides.flatMap((side) => view[side].units.flatMap((unit, k) => (unit === null ? [] : [testid.zone(side, "units", k + 1)]))).slice(0, max);
}

export const mirrorballShuffle: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const look = silasLook();
  if (event.type === "rotated") {
    const centre: FxAnchor = tid(testid.board, FX_CENTER);
    const cues: FxCue[] = [
      ringCue(p.D, "prismatic", centre, t.T),
      { ...tunedBurst(i, "confetti", centre, "ring", t.T, TUNING.confetti), colors: look.colors },
      ...occupiedZones(p.view, FX_SIGNATURE_BURSTS_MAX).map((zone): FxCue => ({ kind: "sheen", at: tid(zone), delayMs: t.T, durationMs: t.L - t.T })),
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "controlChanged" && withinBursts(event, p)) {
    const side = sideOf(p.view, event.controller);
    const at = tid(testid.zone(side, event.row, event.lane));
    // R3402001: the brand stands in for the row's own mind-control flight, inside Silas's budget.
    return { cues: [{ kind: "brand", at, tint: SILAS_SIDE_TINTS[side], delayMs: 0, durationMs: p.D }], row: "replace" };
  }
  return null;
};
