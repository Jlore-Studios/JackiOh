// #2793: #83 Transmogulate (`core-083`, bespoke "legendaryTide"), bespoke because its transforms land
// in a hand and a deck the board does not show, which no motif anchors on. At its first transform a
// Legendary tide washes over the caster's Deck and hand piles; each hand card the viewer reads takes a
// sheen as it turns. The piles are public on both seats; a transform the viewer may not read draws
// nothing of its own (R202). Each transform's own row still plays.

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { raysCue, shakeCues, tid } from "../../build.ts";
import { FX_TIDE_TRAUMA } from "../../constants.ts";
import { isFirstEntry, MOTIF_TUNING, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec } from "./flourish.ts";

export const legendaryTide: CardRecipe = (event, p, play) => {
  if (event.type !== "transformed") return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  const cues: FxCue[] = [];
  const first = p.entry.events.find((candidate) => candidate.type === "transformed") === event && isFirstEntry(p.env, p.entry, ["transformed"]);
  if (first) {
    const side = sideOf(p.view, play.player);
    for (const pile of [tid(animTestid.library(side)), tid(animTestid.hand(side))]) {
      cues.push(raysCue(p.D, "legendary", pile, t.T), motifRing(spec, p.D, pile, t.L), motifBurst(spec, pile, "area", t.L, MOTIF_TUNING.land, "gold"));
    }
    cues.push(...shakeCues(spec.scale, FX_TIDE_TRAUMA, t.L));
  }
  if (event.hiddenFrom?.includes(p.view.viewer) !== true) {
    const at = locateInstance(p.view, event.newInstanceId) ?? locateInstance(p.view, event.instanceId);
    if (at !== null && at.startsWith("hand-card-")) {
      cues.push({ kind: "sheen", at: tid(at), delayMs: t.T, durationMs: t.L - t.T }, motifBurst(spec, tid(at), "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
  }
  return cues.length === 0 ? null : { cues, row: "keep" };
};
