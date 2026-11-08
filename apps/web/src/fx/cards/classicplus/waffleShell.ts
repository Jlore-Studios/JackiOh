// R3399000: Classic+ Anti-Waffle Shell (`classicplus-012-5`, entrance "waffleIron", bespoke "waffleGrid"), its signature (#3399):
// Entrance: a waffle iron slams shut over the zone with a hiss (steam and a ring at the slam) and lifts,
// leaving a pale blue dome of frost-light over the caster's side. Text: on its Cry a waffle-grid bubble
// clicks into place on each Unit it shields (`keywordGranted`, a ring and sparkle), and the dome's glow
// lifts them all a little taller (`buffed`, a rising arrow, when the engine emits it; its aura does not).
// Nothing moves. R202: only public board targets and zones are anchors.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { targetFor } from "../../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the iron and the grid throw, at intensity "normal" (rule 9). */
const TUNING = {
  hiss: { count: 16, power: 0.9 },
  dome: { count: 14, power: 0.6 },
  click: { count: 8, power: 0.6 },
  lift: { count: 6, power: 0.5 },
} as const;

/** The iron lifts, leaving the dome, after the slam; the bubbles click in at this fraction of the entry. */
const DOME_AT = 0.8;
const CLICK_AT = 0.15;
const LIFT_AT = 0.4;
const ARROWS_MS = 500;
const LOOK = paletteLook("Spirit") ?? FLAVOR_LOOKS.Spirit;

export const waffleIron: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    { ...tunedBurst(i, "smoke", at, "area", slam, TUNING.hiss), colors: LOOK.colors },
    ringCue(D, LOOK.particle, at, slam),
    { ...tunedBurst(i, LOOK.particle, at, "ring", frac(DOME_AT, D), TUNING.dome), colors: LOOK.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.waffleIron, slam),
  ];
};

/** Each Unit it shields clicks its bubble in; each it lifts rises. */
export const waffleGrid: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  if (event.type === "keywordGranted") {
    const at = hitAnchor(p.view, event.instanceId);
    if (at === null) return null;
    return {
      cues: [ringCue(p.D, LOOK.particle, at, frac(CLICK_AT, p.D)), { ...tunedBurst(p.env.intensity, "frost", at, "ring", frac(CLICK_AT, p.D), TUNING.click), colors: LOOK.colors }],
      row: "keep",
    };
  }
  if (event.type === "buffed") {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const at = tid(tgt);
    return {
      cues: [{ kind: "arrows", direction: "up", at, delayMs: frac(LIFT_AT, p.D), durationMs: ARROWS_MS }, { ...tunedBurst(p.env.intensity, "sparkle", at, "area", frac(LIFT_AT, p.D), TUNING.lift), colors: LOOK.colors }],
      row: "keep",
    };
  }
  return null;
};

/** Its Cry shields each of the caster's Units. */
export function waffleGridFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const own = player === view.viewer ? view.you : view.opponent;
  return own.units.flatMap((unit): GameEvent[] => (unit === null ? [] : [{ type: "keywordGranted", instanceId: unit.instanceId, keyword: { kind: "Divine Shield" } }]));
}
