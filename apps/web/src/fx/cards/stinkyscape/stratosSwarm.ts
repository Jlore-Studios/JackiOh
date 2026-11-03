// #2798: Stinkyscape #158 Stratos Swarm (`stinkyscape-158`, bespoke "stratosSwarm"), bespoke because
// conjure's circle lands one Unit where the Swarm fills the board: each Stratos streaks in on a rocket
// trail from its caster's hero to the zone it fills, landing in a burst and a ring with exhaust smoke
// lingering, and the board shakes once a play. It anchors on the hero and the zones the summons name,
// the same on both seats (R202); each summon's own entrance row still plays.

import { targetFor } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_STRATOS_SWARM_TRAUMA } from "../../constants.ts";
import { paletteLook } from "../../flavors.ts";
import { isFirstEntry, MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { flourishSpec } from "../core/flourish.ts";
import type { CardRecipe } from "../recipe.ts";

/** #2792: the space ships' sub-look. */
const PALETTE = "Tech/rocket";

const SUMMONED = ["summoned"] as const;

export const stratosSwarm: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned") return null;
  const tgt = targetFor(event, p.view);
  if (tgt === null) return null;
  const base = flourishSpec(p, play);
  const spec: CueSpec = { ...base, look: paletteLook(PALETTE) ?? base.look };
  const t = timesOf(p.D);
  const hero = tid(testid.hero(sideOf(p.view, play.player)));
  const at = tid(tgt);
  const cues: FxCue[] = [
    motifBurst(spec, hero, "point", t.G, MOTIF_TUNING.small),
    motifShot(spec, "bolt", hero, at, t.T, t.L - t.T),
    motifBurst(spec, at, "area", t.L, MOTIF_TUNING.land),
    motifRing(spec, p.D, at, t.L),
    motifBurst(spec, at, "ring", t.Li, MOTIF_TUNING.linger, "smoke"),
  ];
  const first = p.entry.events.find((candidate) => candidate.type === "summoned") === event && isFirstEntry(p.env, p.entry, SUMMONED);
  if (first) cues.push(...shakeCues(spec.scale, FX_STRATOS_SWARM_TRAUMA, t.L));
  return { cues, row: "keep" };
};
