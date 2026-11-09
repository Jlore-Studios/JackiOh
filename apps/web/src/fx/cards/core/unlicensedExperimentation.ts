// R3402001: #85 Unlicensed Experimentation (`core-085`, bespoke "clampWeld"), a Legendary Trap's signature
// (R3332000). Face down it is a card back like any other and draws nothing (R202); its first beat is its
// firing: the card bursts open into a floor hatch in its zone, green arc light and smoke pouring out. On
// the `fused` it reports, the permanent the opponent just played and the caster's own flash where they
// stand (the clamps seizing one and the tether landing on the other) and weld with a stitched seam, a
// sheen and a green flash. A Trap's text plays a beat after its firing, inside its budget, so the weld
// draws in rings, a seam and one flash at its start, in place of the fuse's own row. It anchors on zones
// and cards the viewer reads and names none.

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CLAMP_TRAUMA, FX_CRACK_TAIL_MS, FX_FRACTURE_TAIL_MS } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardFxPlan, CardRecipe } from "../recipe.ts";
import { isFirstBeat } from "./flourish.ts";

/** Every particle count the hatch and the weld throw, at intensity "normal" (rule 9). */
const TUNING = {
  arc: { count: 22, power: 1.1 },
  smoke: { count: 14, power: 0.7 },
  weld: { count: 26, power: 1.3 },
  sizzle: { count: 16, power: 0.9 },
} as const;

/** The hatch's look: Dark's curse, a sickly green over shadow. */
const HATCH_PALETTE = "Dark/curse";

/**
 * Where the seized permanent stands as the weld lands: the zone its newest readable `summoned` names (the board shows
 * the view from before the batch until the burst ends, so a Unit whose play fired the Trap is still in its owner's
 * hand there, hidden from the Trap's owner and a hand card to its own), else the board card the view shows.
 */
function seizedAt(p: CardFxPlan, instanceId: string): string | null {
  const recent = p.env.memory.recent();
  for (let k = recent.length - 1; k >= 0; k -= 1) {
    const event = recent[k];
    if (event?.type === "summoned" && event.instanceId === instanceId) return testid.zone(sideOf(p.view, event.player), event.row, event.lane);
  }
  const shown = locateInstance(p.view, instanceId);
  return shown === testid.card(instanceId) ? shown : null;
}

export const clampWeld: CardRecipe = (event, p, play) => {
  if (play.hook !== "cast") return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const look = paletteLook(HATCH_PALETTE) ?? FLAVOR_LOOKS.Dark;
  if (isFirstBeat(event, play) && event.type === "trapFired") {
    const at = tid(testid.zone(sideOf(p.view, event.controller), event.row, event.lane));
    return {
      cues: [
        { kind: "fracture", at, delayMs: t.G, durationMs: p.D - t.G + FX_FRACTURE_TAIL_MS },
        { ...tunedBurst(i, "poison", at, "area", t.T, TUNING.arc), colors: look.colors },
        tunedBurst(i, "smoke", at, "area", t.T, TUNING.smoke),
        tunedBurst(i, "spark", at, "point", t.L, TUNING.sizzle),
      ],
      row: "keep",
    };
  }
  if (event.type !== "fused") return null;
  const cues: FxCue[] = [];
  const seized = event.instanceIds[0];
  const from = seized === undefined ? null : seizedAt(p, seized);
  const to = locateInstance(p.view, event.resultInstanceId);
  if (from !== null && from !== to) cues.push(ringCue(p.D, "poison", tid(from), t.G));
  if (to !== null) {
    const at = tid(to);
    cues.push(
      ringCue(p.D, "spark", at, t.G),
      { kind: "crack", at, delayMs: t.G, durationMs: p.D - t.G + FX_CRACK_TAIL_MS },
      { kind: "sheen", at, delayMs: t.G, durationMs: t.L },
      tunedBurst(i, "spark", at, "ring", t.G, TUNING.weld),
      { ...tunedBurst(i, "poison", at, "area", t.G, TUNING.sizzle), colors: look.colors },
    );
  }
  cues.push(...shakeCues(i, FX_CLAMP_TRAUMA, t.G));
  return { cues, row: "replace" };
};
