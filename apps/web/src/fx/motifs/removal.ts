// R2784000: the removal motifs (docs/polish/9-card-animations.md): wipe, vanish and negate. Like the
// damage motifs (damage.ts) each plays in the entry's phases (build.ts) and plans nothing without an
// event it reads.

import { LANES, testid } from "../../game/contract.ts";
import { tid, tunedBurst } from "../build.ts";
import { FX_CRACK_TAIL_MS } from "../constants.ts";
import type { FxCue } from "../types.ts";
import { isFirstEntry, MOTIF_TUNING, motifBurst, motifRing, motifShake, readEvents, timesOf, type MotifPlanner } from "./build.ts";

const WIPED = ["destroyed", "exiled"] as const;

/**
 * wipe: a wave rolls across both sides lane by lane at the play's first removal, and ends in one ring
 * over the board; every card removed flares where it stood. Each card's own death or exile still plays.
 */
export const wipe: MotifPlanner = {
  reads: WIPED,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, WIPED);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    if (isFirstEntry(env, entry, WIPED)) {
      const last = Math.max(1, LANES.length - 1);
      for (const lane of LANES) {
        const at = t.T + Math.round(((lane - 1) * (t.L - t.T)) / last);
        for (const side of ["you", "opponent"] as const) cues.push(motifBurst(spec, tid(testid.zone(side, "units", lane)), "area", at, MOTIF_TUNING.wave));
      }
      cues.push(motifRing(spec, t.D, tid(testid.board), t.L), ...motifShake(spec, t.L));
    }
    for (const h of hits) cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small));
    return cues;
  },
};

const VANISHED = ["exiled", "bounced"] as const;

/** vanish: each card drawn in, glinting, and gone in a soft flare; no shake. */
export const vanish: MotifPlanner = {
  reads: VANISHED,
  replaces: ["exiled"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, VANISHED);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push(
        motifBurst(spec, h.at, "ring", t.G, MOTIF_TUNING.gather),
        { kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T },
        motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land),
        motifRing(spec, t.D, h.at, t.L),
      );
    }
    return cues;
  },
};

const COUNTERED = ["countered"] as const;

/** negate: a ring closes on the countered card, which shatters and fizzles out in smoke. */
export const negate: MotifPlanner = {
  reads: COUNTERED,
  replaces: ["countered"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, COUNTERED);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push(
        motifRing(spec, t.D, h.at, t.G),
        motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.land),
        { kind: "crack", at: h.at, delayMs: t.L, durationMs: t.D - t.L + FX_CRACK_TAIL_MS },
        tunedBurst(spec.scale, "smoke", h.at, "point", t.Li, MOTIF_TUNING.linger),
      );
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};
