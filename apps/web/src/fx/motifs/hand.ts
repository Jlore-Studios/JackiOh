// R2784000: the cards and mana motifs (docs/polish/9-card-animations.md): flight, glimpse, mana,
// tithe and tally, and #2795's deal (R2795000). Like the damage motifs (damage.ts) each plays in the
// entry's phases (build.ts) and plans nothing without an event it reads.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance, type AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid, type Side } from "../../game/contract.ts";
import { tid } from "../build.ts";
import { FX_MANA_MAX_SPARKS, FX_MOTIF_DEAL_MAX, FX_MOTIF_TITHE_MAX } from "../constants.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../types.ts";
import {
  MOTIF_TUNING,
  flies,
  isFirstEntry,
  motifBurst,
  motifShake,
  readEvents,
  sourceOf,
  timesOf,
  type MotifPlanner,
  type MotifSpec,
} from "./build.ts";

const FLIGHT_READS = ["drawn"] as const;

/**
 * flight: a card back flying from the library into the hand, landing on the Land phase with its
 * sparkle (today's `draw` beats, phased so the flight lands inside the entry, R200).
 */
export const flight: MotifPlanner = {
  reads: FLIGHT_READS,
  replaces: [...FLIGHT_READS],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, FLIGHT_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      const event = h.event;
      if (event.type !== "drawn") continue;
      const side = sideOf(view, event.player);
      const from = tid(animTestid.library(side));
      const to = tid(animTestid.hand(side));
      cues.push({ kind: "ghost", from, to, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, to, "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
    return cues;
  },
};

const GLIMPSE_READS = ["predicted", "secretRevealed"] as const;

/** glimpse: a secret kept, revealed or judged: a sheen and a small flare on the hero's badge. */
export const glimpse: MotifPlanner = {
  reads: GLIMPSE_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, GLIMPSE_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push({ kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "arcane"));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const MANA_READS = ["manaChanged", "manaSpent"] as const;

/** How many crystals `manaChanged` fills, from the planning view's current mana up (cues.ts). */
function gainedCrystals(event: Extract<GameEvent, { type: "manaChanged" }>, view: PlayerView): { side: Side; old: number; n: number } {
  const side = sideOf(view, event.player);
  const old = (side === "you" ? view.you : view.opponent).mana.current;
  return { side, old, n: Math.min(event.current, old + FX_MANA_MAX_SPARKS) - old };
}

/**
 * mana: today's crystal fill, re-phased: a gather where the mana sits, then one sparkle per
 * crystal gained, staggered inside Travel to Land. A spend (`manaSpent`, which has no row of its
 * own) pulses softly where the crystals drained. No shake, like `vanish`.
 */
export const mana: MotifPlanner = {
  reads: MANA_READS,
  replaces: ["manaChanged"],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, MANA_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type !== "manaChanged" && h.event.type !== "manaSpent") continue;
      if (h.event.type === "manaSpent") {
        cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "sparkle"));
        continue;
      }
      const { side, old, n } = gainedCrystals(h.event, view);
      if (n <= 0) continue;
      cues.push(motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.small, "sparkle"));
      // Spread across Travel to Land, so every speed lands the same beats (R201); the fill lands on Land.
      const step = (t.L - t.T) / Math.max(n, 1);
      for (let k = 0; k < n; k += 1) {
        const crystal: FxAnchor = { kind: "crystal", side, index: old + k };
        cues.push(motifBurst(spec, crystal, "point", t.T + Math.round(k * step), MOTIF_TUNING.small, "sparkle"));
      }
      cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
    return cues;
  },
};

const TITHE_READS = ["costChanged"] as const;

/**
 * tithe: a glint on the cost gem of each hand card whose cost changed. A card the viewer may not
 * read (its `hiddenFrom` names the viewer, or it is nowhere in the view) glints by count only
 * (R202): one glint per event, capped, on the other seat's hand backs, never keyed to a back.
 */
export const tithe: MotifPlanner = {
  reads: TITHE_READS,
  replaces: [...TITHE_READS],
  plan(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv) {
    const events = entry.events.filter((event): event is Extract<GameEvent, { type: "costChanged" }> => event.type === "costChanged");
    if (events.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const play = env.memory.resolving();
    const side = play === undefined ? "opponent" : sideOf(view, play.player);
    // The glints so far in this play, so every hidden change gets the next back, at most
    // FX_MOTIF_TITHE_MAX a play: the memory already holds this entry's events when this plans.
    let veiled = (play?.seen.costChanged ?? events.length) - events.length;
    const cues: FxCue[] = [];
    for (const event of events) {
      const at = locateInstance(view, event.instanceId);
      if (event.hiddenFrom?.includes(view.viewer) !== true && at !== null) {
        cues.push(motifBurst(spec, tid(at), "point", t.L, MOTIF_TUNING.small, "arcane"));
      } else if (veiled < FX_MOTIF_TITHE_MAX) {
        const back: FxAnchor = { kind: "handCard", side, pick: veiled };
        cues.push(motifBurst(spec, back, "point", t.L, MOTIF_TUNING.small, "arcane"));
        veiled += 1;
      }
    }
    if (cues.length === 0) return [];
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const TALLY_READS = ["counterChanged", "questProgressed"] as const;

/** tally: a quest's or a counter's tick: a sparkle where the card stands. */
export const tally: MotifPlanner = {
  reads: TALLY_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, TALLY_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "sparkle"));
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const DEAL_READS = ["addedToHand", "shuffledIn", "fused"] as const;

/** Where a dealt card comes from: the resolving play's hero, else the hero it goes to; none for a fuse with no play. */
function dealtFrom(event: GameEvent, view: PlayerView, env: FxPlanEnv): FxAnchor | null {
  const from = sourceOf(event, view, env);
  if (from !== null) return from;
  return event.type === "addedToHand" || event.type === "shuffledIn" ? tid(testid.hero(sideOf(view, event.player))) : null;
}

/**
 * deal (R2795000): a card made and handed out: a gather at the source (once a play, on its first
 * entry of them), then a card back flying from it to the hand, the library or the card it fuses into,
 * landing on the Land phase with a sparkle, at most FX_MOTIF_DEAL_MAX an entry. It reads the event's pile or card alone, never the card dealt (a
 * back on both seats, R202), and its events' rows still play.
 */
export const deal: MotifPlanner = {
  reads: DEAL_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, DEAL_READS).slice(0, FX_MOTIF_DEAL_MAX);
    const first = hits[0];
    if (first === undefined) return [];
    const t = timesOf(entry.durationMs);
    const from = dealtFrom(first.event, view, env);
    const cues: FxCue[] = [];
    if (from !== null && isFirstEntry(env, entry, DEAL_READS)) cues.push(motifBurst(spec, from, "point", t.G, MOTIF_TUNING.gather));
    for (const h of hits) {
      if (flies(from, h.at)) cues.push({ kind: "ghost", from, to: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
    return cues;
  },
};
