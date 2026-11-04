// R2784000: the board motifs (docs/polish/9-card-animations.md): conjure, rise, offering, shift,
// bind, tether and transmute (R2803000 folds `mirror` into `transmute`), and R2797000's hex.
// Card-table-only overlays (`replaces: []` but bind and transmute): the event rows still play
// underneath, and each plans nothing without an event it reads.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance, type AnimationEntry } from "../../game/animations.ts";
import { LANES, sideOf, testid, type Side } from "../../game/contract.ts";
import { FX_CRACK_TAIL_MS } from "../constants.ts";
import { raysCue, ringCue, tid } from "../build.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../types.ts";
import {
  flies,
  hitAnchor,
  MOTIF_FOOT,
  MOTIF_TUNING,
  motifBurst,
  motifRing,
  motifShake,
  motifShot,
  placeOf,
  readEvents,
  timesOf,
  type MotifPlanner,
  type MotifSpec,
} from "./build.ts";
import { hitIdOf } from "../memory.ts";

const CONJURE_READS = ["summoned"] as const;

/** conjure: a summoning circle where the card arrives: a gather, a landing and an afterglow. */
export const conjure: MotifPlanner = {
  reads: CONJURE_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, CONJURE_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push(
        motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.gather),
        motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land),
        motifRing(spec, t.D, h.at, t.L),
        motifBurst(spec, h.at, "ring", t.Li, MOTIF_TUNING.linger),
      );
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const RISE_READS = ["summoned", "animated", "awakened"] as const;

/** rise: a card clawing up out of its zone, from below to where it stands. */
export const rise: MotifPlanner = {
  reads: RISE_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, RISE_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      cues.push({ kind: "ghost", from: tid(h.tgt, MOTIF_FOOT), to: h.at, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.land), motifRing(spec, t.D, h.at, t.L));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const OFFERING_READS = ["destroyed"] as const;

/**
 * offering: a cost paid, never a kill. The unit sinks where it stood under a glyph in its lane:
 * a ring on the zone, bursts on the card, a lingering ring, and no crack.
 */
export const offering: MotifPlanner = {
  reads: OFFERING_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, OFFERING_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      const place = placeOf(view, hitIdOf(h.event) ?? "");
      const zone = place !== null && place.lane !== null ? tid(testid.zone(place.side, "units", place.lane)) : h.at;
      cues.push(
        motifRing(spec, t.D, zone, t.G),
        motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.gather),
        motifBurst(spec, h.at, "point", t.L, MOTIF_TUNING.small),
        motifRing(spec, t.D, zone, t.Li),
      );
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const SHIFT_READS = ["swapped", "rotated", "positionSwitched", "flickered"] as const;

function otherSide(side: Side): Side {
  return side === "you" ? "opponent" : "you";
}

type BoardZone = { side: Side; lane: number };

/** The rows a board swap and a rotation carry (the engine's SWAP_ROWS and ROTATION_ROWS). */
const BOARD_ROWS = ["units", "backrow"] as const;

/** Whether a card stands on the zone (a face-down trap's marker included: that it is there is public, §10.8). */
function occupied(view: PlayerView, row: (typeof BOARD_ROWS)[number], zone: BoardZone): boolean {
  const side = view[zone.side];
  return ((row === "units" ? side.units : side.backrow)[zone.lane - 1] ?? null) !== null;
}

/** R14's ring of one row from the rotating player's seat: their lane 1 to 5, then the other side's 5 down to 1. */
function ringOf(rotator: Side): BoardZone[] {
  return [...LANES.map((lane) => ({ side: rotator, lane })), ...[...LANES].reverse().map((lane) => ({ side: otherSide(rotator), lane }))];
}

/**
 * Each zone a board-wide move carries a card from, and the zone it lands on, in both rows: across the
 * centre line for a board swap; one step along R14's ring for a rotation, "right" forward from the
 * rotating player's seat, so an end card crosses to the other side and the two sides step opposite ways.
 * `swapped` and `rotated` name no instances (R1855000, B3), so the path is drawn for every card on the
 * board. The ring turns from the seat of the player whose card rotates it (Silly Silas's controller, the
 * engine's `RotationArgs.perspective`), the play resolving now; with none, which way it ran is unknown and
 * no path is drawn.
 */
function boardMoves(event: GameEvent, view: PlayerView, env: FxPlanEnv): { from: FxAnchor; to: FxAnchor }[] {
  let zones: BoardZone[];
  let landing: (zone: BoardZone, k: number) => BoardZone | undefined;
  if (event.type === "rotated") {
    const play = env.memory.resolving();
    if (play === undefined) return [];
    const ring = ringOf(sideOf(view, play.player));
    const step = event.direction === "left" ? -1 : 1;
    zones = ring;
    landing = (_zone, k) => ring[(k + step + ring.length) % ring.length];
  } else {
    zones = (["you", "opponent"] as const).flatMap((side) => LANES.map((lane) => ({ side, lane })));
    landing = (zone) => ({ side: otherSide(zone.side), lane: zone.lane });
  }
  const moves: { from: FxAnchor; to: FxAnchor }[] = [];
  for (const row of BOARD_ROWS) {
    zones.forEach((zone, k) => {
      const to = landing(zone, k);
      if (to === undefined || !occupied(view, row, zone)) return;
      moves.push({ from: tid(testid.zone(zone.side, row, zone.lane)), to: tid(testid.zone(to.side, row, to.lane)) });
    });
  }
  return moves;
}

/**
 * shift: the path of every unit that moved: swapped, rotated, switched position or flickered; a board
 * swap or a rotation carries the backrow too, so its cards walk their paths as well.
 */
export const shift: MotifPlanner = {
  reads: SHIFT_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    const path = (from: FxAnchor | null, to: FxAnchor): void => {
      if (from !== null && flies(from, to)) cues.push({ kind: "ghost", from, to, delayMs: t.T, durationMs: t.L - t.T });
      cues.push(motifBurst(spec, to, "point", t.L, MOTIF_TUNING.small));
    };
    for (const h of readEvents(entry, view, env, SHIFT_READS)) {
      const event = h.event;
      if (event.type === "positionSwitched") {
        const place = placeOf(view, event.instanceId);
        path(place === null || place.lane === null ? null : tid(testid.zone(place.side, "units", place.lane)), h.at);
      } else if (event.type === "flickered") {
        path(tid(testid.zone(sideOf(view, event.player), event.row, event.lane)), h.at);
      } else if (event.type === "swapped" && event.what === "health") {
        path(tid(testid.hero("you")), tid(testid.hero("opponent")));
        cues.push(motifBurst(spec, tid(testid.hero("you")), "point", t.L, MOTIF_TUNING.small));
      } else if (event.type === "swapped" && event.what === "library") {
        path(tid(animTestid.library("you")), tid(animTestid.library("opponent")));
        cues.push(motifBurst(spec, tid(animTestid.library("you")), "point", t.L, MOTIF_TUNING.small));
      } else {
        for (const move of boardMoves(event, view, env)) path(move.from, move.to);
      }
    }
    if (cues.length === 0) return [];
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const BIND_READS = ["locked", "unlocked"] as const;

/** bind: frost and chains over a Locked unit; gold shards when it releases. */
export const bind: MotifPlanner = {
  reads: BIND_READS,
  replaces: [...BIND_READS],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, BIND_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type === "unlocked") {
        cues.push(ringCue(t.D, "gold", h.at, t.G));
        cues.push(motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.small, "shard"));
      } else {
        cues.push(ringCue(t.D, "dust", h.at, t.G));
        cues.push(motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.small, "dust"));
      }
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const TETHER_READS = ["stolen", "controlChanged", "held"] as const;

/** Where a stolen card was taken from, as the row recipe reads it (cues.ts, R202: backs only). */
function stolenFrom(event: Extract<GameEvent, { type: "stolen" }>, view: PlayerView): FxAnchor | null {
  const side = sideOf(view, event.from);
  switch (event.zone) {
    case "hand":
      return tid(animTestid.hand(side));
    case "library":
      return tid(animTestid.library(side));
    case "graveyard":
      return tid(animTestid.graveyard(side));
    case "exile":
      return tid(animTestid.exile(side));
    case "field": {
      const at = locateInstance(view, event.instanceId);
      return at === null ? null : tid(at);
    }
    default:
      return tid(testid.board);
  }
}

/** tether: an arcane line from where a card was to whose it is now: stolen, taken or held. */
export const tether: MotifPlanner = {
  reads: TETHER_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, TETHER_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      const event = h.event;
      if (event.type !== "stolen" && event.type !== "controlChanged" && event.type !== "held") continue;
      const from =
        event.type === "stolen"
          ? stolenFrom(event, view)
          : event.type === "held"
            ? h.at
            : ((): FxAnchor | null => {
                const at = locateInstance(view, event.instanceId);
                return at === null ? null : tid(at);
              })();
      const to = event.type === "held" ? (hitAnchor(view, event.holderId) ?? h.at) : h.at;
      if (from !== null && flies(from, to)) cues.push(motifShot(spec, "bolt", from, to, t.T, t.L - t.T));
      cues.push(motifBurst(spec, to, "area", t.L, MOTIF_TUNING.land));
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const TRANSMUTE_READS = ["transformed", "fused", "upgraded", "degraded"] as const;

/**
 * transmute: smoke and sparks as one card becomes another: today's `smoke` beats, phased. R2797000: a
 * fusion too (an Equip pick fused into its host), landing on the card it makes; #2908 an upgrade and a
 * degrade besides.
 */
export const transmute: MotifPlanner = {
  reads: TRANSMUTE_READS,
  replaces: [...TRANSMUTE_READS],
  plan(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv) {
    const hits = readEvents(entry, view, env, TRANSMUTE_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      const landAt = h.event.type === "fused" ? (hitAnchor(view, h.event.resultInstanceId) ?? h.at) : h.at;
      cues.push(
        motifBurst(spec, h.at, "point", t.G, MOTIF_TUNING.small, "smoke"),
        { kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T },
        motifBurst(spec, landAt, "area", t.L, MOTIF_TUNING.land, "arcane"),
        motifBurst(spec, landAt, "point", t.Li, MOTIF_TUNING.linger, "smoke"),
      );
      if (h.event.type === "upgraded") {
        cues.push(motifRing(spec, t.D, landAt, t.L), raysCue(t.D, "holy", landAt, t.L));
      } else if (h.event.type === "degraded") {
        cues.push({ kind: "crack", at: landAt, delayMs: t.L, durationMs: t.D - t.L + FX_CRACK_TAIL_MS });
      }
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};

const HEX_READS = ["marked"] as const;

/**
 * R2797000 hex: a mark laid on a card (a Freeze, an Allure, any `marked`): a ring closing on it, a sheen
 * over it and a burst as the mark takes; a mark lifted flares once and is gone. Over the rows
 * (`replaces: []`): the brand row (fx/brand.ts) keeps the mark's own colors (R437).
 */
export const hex: MotifPlanner = {
  reads: HEX_READS,
  replaces: [],
  plan(spec, entry, view, env) {
    const hits = readEvents(entry, view, env, HEX_READS);
    if (hits.length === 0) return [];
    const t = timesOf(entry.durationMs);
    const cues: FxCue[] = [];
    for (const h of hits) {
      if (h.event.type === "marked" && h.event.added) {
        cues.push(
          motifRing(spec, t.D, h.at, t.G),
          { kind: "sheen", at: h.at, delayMs: t.T, durationMs: t.L - t.T },
          motifBurst(spec, h.at, "area", t.L, MOTIF_TUNING.small),
        );
      } else {
        cues.push(motifBurst(spec, h.at, "ring", t.L, MOTIF_TUNING.small, "sparkle"));
      }
    }
    cues.push(...motifShake(spec, t.L));
    return cues;
  },
};
