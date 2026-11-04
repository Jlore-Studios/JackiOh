// R2784000: the life and stats motifs (docs/polish/9-card-animations.md): drain, bless, curse,
// ward, and radiance and mend (#2793). Like the damage motifs (damage.ts) each plays in the entry's
// phases (build.ts) and plans nothing without an event it reads.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { sideOf, testid } from "../../game/contract.ts";
import { raysCue, ringCue, tid } from "../build.ts";
import { projectile } from "../cards/recipe.ts";
import { FX_ARROWS_TAIL_MS, FX_SPLAT_HOLD_MS } from "../constants.ts";
import { shieldCues } from "../shield.ts";
import type { FxAnchor, FxCue } from "../types.ts";
import {
  flies,
  hitAnchor,
  isFirstEntry,
  MOTIF_TUNING,
  motifBurst,
  motifRing,
  motifShake,
  readEvents,
  timesOf,
  type MotifPlanner,
  type MotifSpec,
} from "./build.ts";
import type { AnimationEntry } from "../../game/animations.ts";
import type { FxPlanEnv } from "../types.ts";

const DRAIN_READS = ["healthLost", "healthSet"] as const;

/** How far a `healthSet` moved the hero: down is a loss, up a heal (cues.ts's B5 E7). */
function setLoss(event: Extract<GameEvent, { type: "healthSet" }>, view: PlayerView): number {
  const side = sideOf(view, event.player);
  const before = (side === "you" ? view.you : view.opponent).hero.health;
  return before - event.health;
}

/**
 * Where a drain lands: the same entry's `healed` target when one is there, else the resolving
 * play's hero, else nowhere (bursts only). The bloodDrain recipe (core-027) is the bespoke
 * reference; this is its motif, a liquid `projectile` rather than a bolt.
 */
function drainGainer(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxAnchor | null {
  const healed = entry.events.find((event) => event.type === "healed");
  if (healed !== undefined) {
    const at = hitAnchor(view, healed.targetId);
    if (at !== null) return at;
  }
  const play = env.memory.resolving();
  if (play === undefined) return null;
  return tid(testid.hero(sideOf(view, play.player)));
}

/**
 * drain: a stream from the victim to the gainer, which can be the hero or a unit. Like the damage
 * motifs (build.ts `hit`) it draws the loss's number only in place of the life row it replaces, and
 * under `keep` leaves it to that row, so one life loss shows one number.
 */
export const drain: MotifPlanner = {
  reads: DRAIN_READS,
  replaces: [...DRAIN_READS],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, DRAIN_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const gainer = drainGainer(entry, view, env);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type !== "healthLost" && h.event.type !== "healthSet") continue;
      cues.push(motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.gather));
      if (gainer !== null && flies(h.at, gainer)) cues.push(projectile(spec.scale, "blood", h.at, gainer, t.T, t.L - t.T));
      cues.push(motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land));
      const loss = h.event.type === "healthLost" ? h.event.amount : setLoss(h.event, view);
      if (spec.row === "replace" && loss !== 0) {
        cues.push({
          kind: "splat",
          tone: loss > 0 ? "loss" : "heal",
          amount: Math.abs(loss),
          at: h.at,
          delayMs: t.L,
          durationMs: t.D - t.L + FX_SPLAT_HOLD_MS,
        });
      }
      cues.push(motifBurst(spec, h.at, "ring", t.Li, MOTIF_TUNING.linger));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const BLESS_READS = ["buffed", "upgraded", "keywordGranted"] as const;

/** Whether `bless` draws this event: the positive side of the shared buff and keyword rows. */
function isBlessed(event: GameEvent): boolean {
  if (event.type === "buffed") return event.attack + event.health > 0;
  if (event.type === "upgraded") return true;
  if (event.type === "keywordGranted") return event.lost !== true;
  return false;
}

/** bless: arrows up, a burst and a sheen over a card made better. The numbers stay on the row. */
export const bless: MotifPlanner = {
  reads: BLESS_READS,
  replaces: [...BLESS_READS],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, BLESS_READS).filter((h) => isBlessed(h.event));
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push({ kind: "arrows", direction: "up", at: h.at, delayMs: t.G, durationMs: t.D - t.G + FX_ARROWS_TAIL_MS });
      cues.push({ kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const CURSE_READS = ["degraded", "buffed", "keywordGranted"] as const;

/** Whether `curse` draws this event: the negative side of the shared buff and keyword rows. */
function isCursed(event: GameEvent): boolean {
  if (event.type === "degraded") return true;
  if (event.type === "buffed") return event.attack + event.health < 0;
  if (event.type === "keywordGranted") return event.lost === true;
  return false;
}

/** curse: arrows down and a void burst over a card made worse. */
export const curse: MotifPlanner = {
  reads: CURSE_READS,
  replaces: [...CURSE_READS],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, CURSE_READS).filter((h) => isCursed(h.event));
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push({ kind: "arrows", direction: "down", at: h.at, delayMs: t.G, durationMs: t.D - t.G + FX_ARROWS_TAIL_MS });
      cues.push(motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land, "void"));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const WARD_READS = ["damageAbsorbed", "divineShieldLost"] as const;

/** ward: Armor's full shield over a hit it took whole (under `keep`, a burst over the row's flash), gold and shards where a Divine Shield broke. */
export const ward: MotifPlanner = {
  reads: WARD_READS,
  replaces: [...WARD_READS],
  plan(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv) {
    const hits = readEvents(entry, view, env, WARD_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type === "damageAbsorbed" && spec.row === "keep") {
        // Armor's flash is a number of the row's (R2784000): under keep the armor row draws it, so the motif only lands.
        cues.push(motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.land, "spark"));
      } else if (h.event.type === "damageAbsorbed") {
        // shieldCues throws colorless bursts; the motif recolors them into its look.
        for (const cue of shieldCues("full", h.at, t.D, t.L, spec.scale)) {
          cues.push(cue.kind === "burst" ? { ...cue, colors: spec.look.colors } : cue);
        }
      } else {
        cues.push(ringCue(t.D, "gold", h.at, t.G));
        cues.push(motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.small, "shard"));
      }
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const RADIANCE_READS = ["radiantSet"] as const;

/**
 * radiance (#2793): a card made Radiant, in a hand or on the board: a sheen over it and a burst as
 * it lands, and once a play the rays of the Radiant tone. A hand card the viewer may not read is its
 * hand pile (`targetFor`), never a definition (R202). The Radiant row still plays.
 */
export const radiance: MotifPlanner = {
  reads: RADIANCE_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, RADIANCE_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    // The rays once a play: on its first entry of Radiant cards, at the first of them.
    const rays = isFirstEntry(env, entry, RADIANCE_READS);
    const cues: FxCue[] = [];
    hits.forEach((h, k) => {
      cues.push({ kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land));
      if (rays && k === 0) cues.push(raysCue(t.D, "radiant", h.at, t.L));
    });
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const MEND_READS = ["healed"] as const;

/**
 * mend (#2793): a heal: a sheen over the healed hero or unit, a ring of the look closing as it
 * lands and a soft afterglow. The heal's number stays on its row. No shake, like `flight`.
 */
export const mend: MotifPlanner = {
  reads: MEND_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, MEND_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push({ kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.land));
      cues.push(motifRing(spec, t.D, h.at, t.L));
      cues.push(motifBurst(spec, h.at, "area", t.Li, MOTIF_TUNING.linger));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};
