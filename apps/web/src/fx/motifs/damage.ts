// R2784000: the damage motifs (docs/polish/9-card-animations.md): bolt, volley, barrage, lance,
// nova and scorch (R2803000 folds `chain` into `nova`). Each gathers at its source, travels, lands on
// its targets and lingers, every beat a phase of the entry (build.ts), and returns nothing when the
// entry holds nothing it reads.

import type { PlayerView } from "@jackioh/shared";

import { testid, type Side } from "../../game/contract.ts";
import { tid, tunedBurst } from "../build.ts";
import { FX_MOTIF_BARRAGE_MAX, FX_MOTIF_BARRAGE_STEP, FX_MOTIF_CENTER_LANE, FX_MOTIF_VOLLEY_MAX } from "../constants.ts";
import { hitIdOf } from "../memory.ts";
import type { FxAnchor, FxCue } from "../types.ts";
import {
  earlierHits,
  flies,
  hit,
  MOTIF_FOOT,
  MOTIF_TUNING,
  motifBurst,
  motifRing,
  motifShake,
  motifShot,
  otherSide,
  placeOf,
  readEvents,
  sourceOf,
  timesOf,
  type MotifHit,
  type MotifPlanner,
  type MotifSpec,
  type MotifTimes,
} from "./build.ts";

const HITS = ["damage", "destroyed"] as const;

/** One bolt from the source to the first target, every target hit as it lands, a ring lingering on the first. */
function boltCues(spec: MotifSpec, hits: readonly MotifHit[], s: FxAnchor | null, t: MotifTimes): FxCue[] {
  const first = hits[0];
  if (first === undefined) return [];
  const cues: FxCue[] = [];
  if (s !== null) cues.push(motifBurst(spec, s, "point", t.G, MOTIF_TUNING.gather));
  if (flies(s, first.at)) cues.push(motifShot(spec, "bolt", s, first.at, t.T, t.L - t.T));
  for (const h of hits) cues.push(...hit(spec, h.event, h.at, t.D, t.L));
  cues.push(motifBurst(spec, first.at, "ring", t.Li, MOTIF_TUNING.linger), ...motifShake(spec, t.L));
  return cues;
}

/** bolt: one glowing bolt from the source into its target. */
export const bolt: MotifPlanner = {
  reads: HITS,
  replaces: ["damage"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, HITS);
    const first = hits[0];
    if (first === undefined) return [];
    return boltCues(spec, hits, sourceOf(first.event, view, env), timesOf(entry.durationMs));
  },
};

/** volley: a fan of bolts that leave together, one per target up to FX_MOTIF_VOLLEY_MAX a play, after one gather. */
export const volley: MotifPlanner = {
  reads: HITS,
  replaces: ["damage"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, HITS);
    const first = hits[0];
    if (first === undefined) return [];
    const t = timesOf(entry.durationMs);
    const s = sourceOf(first.event, view, env);
    const earlier = earlierHits(env, entry).length;
    const cues: FxCue[] = [];
    if (s !== null && earlier === 0) cues.push(motifBurst(spec, s, "ring", t.G, MOTIF_TUNING.gather));
    let shots = earlier;
    for (const h of hits) {
      if (shots < FX_MOTIF_VOLLEY_MAX && flies(s, h.at)) {
        cues.push(motifShot(spec, "bolt", s, h.at, t.T, t.L - t.T));
        shots += 1;
      }
      cues.push(...hit(spec, h.event, h.at, t.D, t.L));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

/** barrage: bolts raining in one after another, FX_MOTIF_BARRAGE_STEP of D apart; the rest land together. */
export const barrage: MotifPlanner = {
  reads: HITS,
  replaces: ["damage"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, HITS);
    const first = hits[0];
    if (first === undefined) return [];
    const t = timesOf(entry.durationMs);
    const s = sourceOf(first.event, view, env);
    const cues: FxCue[] = [];
    if (s !== null && earlierHits(env, entry).length === 0) cues.push(motifBurst(spec, s, "ring", t.G, MOTIF_TUNING.gather));
    hits.forEach((h, k) => {
      if (k >= FX_MOTIF_BARRAGE_MAX) {
        cues.push(...hit(spec, h.event, h.at, t.D, t.L, MOTIF_TUNING.small));
        return;
      }
      const lands = Math.min(t.Li, t.L + Math.round(k * FX_MOTIF_BARRAGE_STEP * t.D));
      if (flies(s, h.at)) cues.push(motifShot(spec, "bolt", s, h.at, t.T + lands - t.L, t.L - t.T));
      cues.push(...hit(spec, h.event, h.at, t.D, lands, MOTIF_TUNING.small));
    });
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

/**
 * lance: JackiOh's lane. A beam runs down the first target's whole lane, from the far side's backrow
 * through both unit rows into its own side's backrow, every zone of the lane flaring as it lands; the
 * play's later hits in that lane flare it again without a second beam. A target in no lane (a hero)
 * takes bolt's cues instead.
 */
const LANCE_READS = ["damage", "destroyed", "exiled"] as const;

/** The side and lane of the first target standing in a lane, or null. */
function laneOf(hits: readonly MotifHit[], view: PlayerView): { side: Side; lane: number } | null {
  for (const h of hits) {
    const place = placeOf(view, hitIdOf(h.event) ?? "");
    if (place !== null && place.lane !== null) return { side: place.side, lane: place.lane };
  }
  return null;
}

export const lance: MotifPlanner = {
  reads: LANCE_READS,
  replaces: ["damage"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, LANCE_READS);
    const first = hits[0];
    if (first === undefined) return [];
    const t = timesOf(entry.durationMs);
    const lined = laneOf(hits, view);
    if (lined === null) return boltCues(spec, hits, sourceOf(first.event, view, env), t);
    const { side, lane } = lined;
    const far = otherSide(side);
    const start = tid(testid.zone(far, "backrow", lane));
    const end = tid(testid.zone(side, "backrow", lane));
    // One beam a lane: a later hit of the play in the lane its last hit was in (each its own entry) only flares it.
    const last = earlierHits(env, entry).at(-1);
    const beamed = last !== undefined && placeOf(view, last)?.lane === lane;
    const cues: FxCue[] = beamed ? [] : [motifBurst(spec, start, "ring", t.G, MOTIF_TUNING.gather), motifShot(spec, "beam", start, end, t.T, t.L - t.T)];
    for (const zone of [testid.zone(far, "backrow", lane), testid.zone(far, "units", lane), testid.zone(side, "units", lane), testid.zone(side, "backrow", lane)]) {
      cues.push(motifBurst(spec, tid(zone), "area", t.L, MOTIF_TUNING.small));
    }
    cues.push(motifRing(spec, t.D, tid(testid.zone(side, "units", lane)), t.L), motifRing(spec, t.D, end, t.L));
    for (const h of hits) cues.push(...hit(spec, h.event, h.at, t.D, t.L));
    cues.push(motifBurst(spec, end, "point", t.Li, MOTIF_TUNING.linger), ...motifShake(spec, t.L));
    return cues;
  },
};

/** nova's centre: its one target, the middle of the one side every target is on, else the board. */
function novaCentre(hits: readonly MotifHit[], view: PlayerView): FxAnchor {
  const first = hits[0];
  if (first !== undefined && hits.every((h) => h.tgt === first.tgt)) return first.at;
  const sides = new Set(hits.map((h) => placeOf(view, hitIdOf(h.event) ?? "")?.side ?? null));
  const [only] = [...sides];
  if (sides.size === 1 && only !== undefined && only !== null) return tid(testid.zone(only, "units", FX_MOTIF_CENTER_LANE));
  return tid(testid.board);
}

/** nova: a blast that gathers at one centre and bursts out over every target at once. */
export const nova: MotifPlanner = {
  reads: HITS,
  replaces: ["damage"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, HITS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const c = novaCentre(hits, view);
    const cues: FxCue[] = [
      motifBurst(spec, c, "ring", t.G, MOTIF_TUNING.gather),
      motifRing(spec, t.D, c, t.T),
      motifBurst(spec, c, "area", t.L, MOTIF_TUNING.nova),
      motifRing(spec, t.D, c, t.L),
      motifRing(spec, t.D, c, t.Li),
    ];
    for (const h of hits) cues.push(...hit(spec, h.event, h.at, t.D, t.L));
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

/** scorch: each target heats, flares, and burns down to embers and smoke where it stood. */
export const scorch: MotifPlanner = {
  reads: HITS,
  replaces: ["damage"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, HITS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push(
        motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.gather),
        ...hit(spec, h.event, h.at, t.D, t.L),
        motifBurst(spec, tid(h.tgt, MOTIF_FOOT), "area", t.L, MOTIF_TUNING.ember, "ember"),
        tunedBurst(spec.scale, "smoke", h.at, "point", t.Li, MOTIF_TUNING.linger),
      );
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};
