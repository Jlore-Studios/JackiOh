// #2793: #83 Transmogulate (`core-083`, bespoke "legendaryTide"), bespoke because its transforms land
// in a hand and a deck the board does not show, which no motif anchors on. R3402001: its signature
// (R3332000), a Legendary Spell's, so everything it draws ends within its share of the budget. At its
// cast a cracked crucible tips over the caster's hero: a poison gather, Legendary rays, a prismatic ring
// and glass vials bursting into prismatic shards. At its first transform the alchemical wave crests over
// the caster's Deck and hand piles in rings and a sheen; each hand card the viewer reads takes a sheen
// as it turns, and Embiggened, each Unit it re-forms ripples in a ring and a sheen. Past the cast the
// budget leaves no room for particles, so the transforms draw in place of their own row's smoke. The
// piles and heroes are public on both seats; a transform the viewer may not read draws nothing of its
// own (R202).

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRUCIBLE_TRAUMA, FX_TIDE_TRAUMA } from "../../constants.ts";
import { isFirstEntry, MOTIF_TUNING, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";
import { flourishSpec, isFirstBeat } from "./flourish.ts";

/** Every particle count the crucible throws, at intensity "normal" (rule 9). */
const TUNING = {
  vials: { count: 24, power: 1.2 },
} as const;

export const legendaryTide: CardRecipe = (event, p, play) => {
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  if (isFirstBeat(event, play)) {
    const hero = heroOf(p.view, play);
    return {
      cues: [
        motifBurst(spec, hero, "ring", t.G, MOTIF_TUNING.gather, "poison"),
        raysCue(p.D, "legendary", hero, t.G),
        motifRing(spec, p.D, hero, t.T),
        tunedBurst(p.env.intensity, "prismatic", hero, "area", t.L, TUNING.vials),
        motifBurst(spec, hero, "point", t.L, MOTIF_TUNING.small, "shard"),
        ...shakeCues(p.env.intensity, FX_CRUCIBLE_TRAUMA, t.L),
      ],
      row: "keep",
    };
  }
  if (event.type !== "transformed") return null;
  const cues: FxCue[] = [];
  const first = p.entry.events.find((candidate) => candidate.type === "transformed") === event && isFirstEntry(p.env, p.entry, ["transformed"]);
  if (first) {
    const side = sideOf(p.view, play.player);
    for (const pile of [tid(animTestid.library(side)), tid(animTestid.hand(side))]) {
      cues.push(ringCue(p.D, "prismatic", pile, t.G), { kind: "sheen", at: pile, delayMs: t.G, durationMs: t.L });
    }
    cues.push(...shakeCues(p.env.intensity, FX_TIDE_TRAUMA, t.T));
  }
  if (event.hiddenFrom?.includes(p.view.viewer) !== true) {
    const at = locateInstance(p.view, event.newInstanceId) ?? locateInstance(p.view, event.instanceId);
    if (at !== null && at.startsWith("hand-card-")) cues.push({ kind: "sheen", at: tid(at), delayMs: t.T, durationMs: t.L - t.T });
    // Embiggened, it sweeps the caster's Units too: each re-forms in a ripple.
    if (at !== null && at.startsWith("card-")) cues.push(ringCue(p.D, "prismatic", tid(at), t.T), { kind: "sheen", at: tid(at), delayMs: t.T, durationMs: t.L - t.T });
  }
  return { cues, row: "replace" };
};
