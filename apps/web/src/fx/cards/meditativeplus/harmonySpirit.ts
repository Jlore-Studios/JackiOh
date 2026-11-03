// R2784000 #2796: Meditative+ #21 Harmony Spirit (`meditativeplus-021`, bespoke "harmonyWave"): its Cry
// sets every other Unit to 3/3, which no motif reads (`numberChanged`), so one wave rings out from the
// Spirit and each unit it set settles under a sheen. It reads public ids, `hiddenFrom` and the view
// alone (R202), never the numbers' card.

import type { GameEvent } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_HARMONY_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, isFirstEntry, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, flourishes } from "../meditative/flourish.ts";

const SET = ["numberChanged"] as const;

/** The stat a unit's settling keys on: one sheen per unit, though its set reports Attack and Health. */
const SETTLES_ON = "attack";

function settles(event: GameEvent): event is Extract<GameEvent, { type: "numberChanged" }> {
  return event.type === "numberChanged" && event.key === SETTLES_ON;
}

/** Meditative+ #21 Harmony Spirit: a sheen and a glint on each unit it set, and at its first a ring out from the Spirit. */
export const harmonyWave: CardRecipe = (event, p, play) => {
  if (!flourishes(p) || !settles(event) || event.hiddenFrom?.includes(p.view.viewer) === true) return null;
  const at = locateInstance(p.view, event.instanceId);
  if (at === null) return null;
  const t = timesOf(p.D);
  const spec = flourishSpec(p, play);
  const unit = tid(at);
  const cues: FxCue[] = [{ kind: "sheen", at: unit, delayMs: t.T, durationMs: t.L - t.T }, motifBurst(spec, unit, "point", t.L, MOTIF_TUNING.small, "sparkle")];
  const spirit = locateInstance(p.view, play.instanceId);
  if (p.entry.events.find(settles) === event && isFirstEntry(p.env, p.entry, SET) && spirit !== null) {
    const from = tid(spirit);
    cues.push(motifRing(spec, t.D, from, t.G), motifBurst(spec, from, "ring", t.G, MOTIF_TUNING.wave), ...shakeCues(spec.scale, FX_HARMONY_TRAUMA, t.L));
  }
  return { cues, row: "keep" };
};
