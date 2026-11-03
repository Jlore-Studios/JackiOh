// R2784000: the cards and mana motifs (docs/polish/9-card-animations.md): flight, glimpse, mana,
// tithe, tally and send (#2793), then #2796's deal (cards dealt into a hand or deck) and seal (a
// lasting modifier laid on a seat), and R2794000's recall. Like the damage motifs (damage.ts) each
// plays in the entry's phases (build.ts) and plans nothing without an event it reads.
import type { GameEvent, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance, type AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid, type Side } from "../../game/contract.ts";
import { tid } from "../build.ts";
import { FX_MANA_MAX_SPARKS, FX_MOTIF_DEAL_MAX, FX_MOTIF_TITHE_MAX } from "../constants.ts";
import { dormantStarshipOf } from "../starship.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../types.ts";
import {
  flies,
  MOTIF_TUNING,
  motifBurst,
  motifRing,
  motifShake,
  motifShot,
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

const RECALL_READS = ["addedToHand"] as const;

/**
 * R2794000 recall: a card back flying from its owner's graveyard pile into the hand (#2794): a small
 * gather on the pile, the flight from Travel to Land and the hand's sparkle on Land. The hand's own
 * glint still plays, and nothing shakes.
 */
export const recall: MotifPlanner = {
  reads: RECALL_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, RECALL_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      const event = h.event;
      if (event.type !== "addedToHand") continue;
      const side = sideOf(view, event.player);
      const grave = tid(animTestid.graveyard(side));
      const hand = tid(animTestid.hand(side));
      cues.push(motifBurst(spec, grave, "point", t.G, MOTIF_TUNING.small));
      cues.push({ kind: "ghost", from: grave, to: hand, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, hand, "point", t.L, MOTIF_TUNING.small, "sparkle"));
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

const TITHE_READS = ["costChanged", "modifierChanged"] as const;

/**
 * tithe: a glint on the cost gem of each hand card whose cost changed. A card the viewer may not
 * read (its `hiddenFrom` names the viewer, or it is nowhere in the view) glints by count only
 * (R202): one glint per event, capped, on the other seat's hand backs, never keyed to a back.
 * R2801000: a price set for a player (an added modifier, public on both seats) glints on their
 * modifier badges, and on their Dormant Starship, whose launch it may price (R1852006); the badge's
 * own row still plays.
 */
export const tithe: MotifPlanner = {
  reads: TITHE_READS,
  replaces: ["costChanged"],
  plan(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv) {
    const events = entry.events.filter((event): event is Extract<GameEvent, { type: "costChanged" }> => event.type === "costChanged");
    const priced = entry.events.filter((event): event is Extract<GameEvent, { type: "modifierChanged" }> => event.type === "modifierChanged" && event.added);
    if (events.length === 0 && priced.length === 0) return [];
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
    for (const event of priced) {
      const owner = sideOf(view, event.player);
      cues.push(motifBurst(spec, tid(animTestid.modifiers(owner)), "point", t.L, MOTIF_TUNING.small, "arcane"));
      const ship = dormantStarshipOf(view, owner);
      if (ship !== null) cues.push(motifBurst(spec, tid(ship), "point", t.L, MOTIF_TUNING.small, "arcane"));
    }
    if (cues.length === 0) return [];
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const TALLY_READS = ["counterChanged", "questProgressed", "jadeChanged"] as const;

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

const SEND_READS = ["addedToHand", "shuffledIn"] as const;

/**
 * send (#2793): a card made and sent into a hand or a deck: a gather at the caster's hero (the
 * receiver's with no play), a card back flying to the hand or the library, and its sparkle there.
 * The back names no card (R202); the hand's or the library's own row still plays.
 */
export const send: MotifPlanner = {
  reads: SEND_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, SEND_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type !== "addedToHand" && h.event.type !== "shuffledIn") continue;
      const from = sourceOf(h.event, view, env) ?? tid(testid.hero(sideOf(view, h.event.player)));
      cues.push(motifBurst(spec, from, "point", t.G, MOTIF_TUNING.small));
      if (flies(from, h.at)) cues.push({ kind: "ghost", from, to: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const DEAL_READS = ["addedToHand", "shuffledIn", "fused"] as const;

/**
 * Where a deal's cards fly from: the first event's source (the resolving play's hero), else, for a card
 * added to a hand or shuffled into a deck with no play to read, its receiver's hero; null for a fuse.
 */
function dealtFrom(event: GameEvent, view: PlayerView, env: FxPlanEnv): FxAnchor | null {
  const from = sourceOf(event, view, env);
  if (from !== null) return from;
  if (event.type === "addedToHand" || event.type === "shuffledIn") return tid(testid.hero(sideOf(view, event.player)));
  return null;
}

/** deal: cards dealt from the caster into a hand or a deck, each a card back flying in and landing with a sparkle; no shake. */
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
    if (from !== null) cues.push(motifBurst(spec, from, "point", t.G, MOTIF_TUNING.gather));
    for (const h of hits) {
      if (flies(from, h.at)) cues.push({ kind: "ghost", from, to: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small, "sparkle"));
    }
    return cues;
  },
};

const SEAL_READS = ["modifierChanged"] as const;

/** seal: a lasting modifier laid on a seat: a bolt from the caster, a ring closing on the seat's modifiers and a slow afterglow. */
export const seal: MotifPlanner = {
  reads: SEAL_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, SEAL_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type !== "modifierChanged" || !h.event.added) continue;
      const from = sourceOf(h.event, view, env);
      if (flies(from, h.at)) {
        cues.push(motifBurst(spec, from, "point", t.G, MOTIF_TUNING.gather));
        cues.push(motifShot(spec, "bolt", from, h.at, t.T, t.L - t.T));
      }
      cues.push(motifRing(spec, t.D, h.at, t.L));
      cues.push(motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.land));
      cues.push(motifBurst(spec, h.at, "area", t.Li, MOTIF_TUNING.linger));
    }
    if (cues.length === 0) return [];
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};
