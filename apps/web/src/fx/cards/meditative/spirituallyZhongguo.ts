// R2784000 #2796: Meditative #32 Spiritually 中国 (`meditative-032`, bespoke "cnSweep"): it transforms
// the caster's whole hand and summons in one beat, which no motif reads both of, so one sweep crosses
// the caster's hand, each card the viewer may read glints as it turns, and each summon rings its zone.
// It reads public ids, `hiddenFrom` and the view alone (R202): a hand the viewer may not read glints no card.

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_CN_SWEEP_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, isFirstEntry, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, flourishes } from "./flourish.ts";

const SWEPT = ["transformed"] as const;

/** Meditative #32 Spiritually 中国: the hand swept once at the play's first transform, a glint on each readable card, a ring on each summon. */
export const cnSweep: CardRecipe = (event, p, play) => {
  if (!flourishes(p)) return null;
  const t = timesOf(p.D);
  const spec = flourishSpec(p, play);
  if (event.type === "transformed") {
    const cues: FxCue[] = [];
    const first = p.entry.events.find((candidate) => candidate.type === "transformed") === event;
    if (first && isFirstEntry(p.env, p.entry, SWEPT)) {
      const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
      cues.push(
        motifBurst(spec, hand, "area", t.G, MOTIF_TUNING.gather),
        { kind: "sheen", at: hand, delayMs: t.T, durationMs: t.L - t.T },
        motifRing(spec, t.D, hand, t.L),
      );
    }
    if (event.hiddenFrom?.includes(p.view.viewer) !== true) {
      const at = locateInstance(p.view, event.newInstanceId) ?? locateInstance(p.view, event.instanceId);
      if (at?.startsWith("hand-card-") === true) cues.push(motifBurst(spec, tid(at), "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
    return cues.length === 0 ? null : { cues, row: "keep" };
  }
  if (event.type === "summoned" && event.player === play.player && event.row === "units") {
    const zone = tid(testid.zone(sideOf(p.view, event.player), "units", event.lane));
    return {
      cues: [motifRing(spec, t.D, zone, t.L), motifBurst(spec, zone, "area", t.L, MOTIF_TUNING.land), ...shakeCues(spec.scale, FX_CN_SWEEP_TRAUMA, t.L)],
      row: "keep",
    };
  }
  return null;
};
