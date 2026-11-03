// R2784000: the trap motif (docs/polish/9-card-animations.md): snap. Like the damage motifs
// (damage.ts) it plays in the entry's phases (build.ts) and plans nothing without an event it
// reads. It replaces the generic trap reveal, and still looks the same for every Trap until the
// view names the card (R154, R202).

import type { PlayerView } from "@jackioh/shared";

import { frac, ringCue } from "../build.ts";
import { FX_TRAP_BURST_AT } from "../constants.ts";
import { flavorLook, type FlavorLook } from "../flavors.ts";
import { flavorSourceOf } from "../flavorSource.ts";
import type { FxCue } from "../types.ts";
import { locateInstance, type AnimationEntry } from "../../game/animations.ts";
import type { FxPlanEnv } from "../types.ts";
import {
  MOTIF_TUNING,
  motifBurst,
  motifShake,
  readEvents,
  timesOf,
  type MotifPlanner,
  type MotifSpec,
} from "./build.ts";

const SNAP_READS = ["trapFired"] as const;

/**
 * What a reveal bursts in: the card's flavor, but only when the view shows this very instance
 * (cues.ts's #1447 condition) — read off the view, never off the event's defId (R154, R202).
 * Otherwise the entry's palette.
 */
function revealLook(spec: MotifSpec, view: PlayerView, instanceId: string): FlavorLook {
  if (locateInstance(view, instanceId) === null) return spec.look;
  const source = flavorSourceOf(instanceId, view);
  return source === null ? spec.look : flavorLook(source.flavor, instanceId);
}

/** snap: a tight ring on the trap's own zone, then its reveal. A firing again is a short flare. */
export const snap: MotifPlanner = {
  reads: SNAP_READS,
  replaces: [...SNAP_READS],
  plan(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv) {
    const hits = readEvents(entry, view, env, SNAP_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    let revealed = false;
    for (const h of hits) {
      cues.push(ringCue(t.D, "arcane", h.at, t.G));
      if (h.event.type === "trapFired" && h.event.again === true) {
        cues.push(motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.small, "arcane"));
        continue;
      }
      // The reveal lands on jk-trap-flip's stop; the shake lands with the motif, on Land.
      revealed = true;
      const look = h.event.type === "trapFired" ? revealLook(spec, view, h.event.instanceId) : spec.look;
      cues.push({ ...motifBurst(spec, h.at, "ring", frac(FX_TRAP_BURST_AT, t.D), MOTIF_TUNING.land, look.particle), colors: look.colors });
    }
    if (revealed) cues.push(...motifShake(spec, t.L));
    return cues;
  },
};
