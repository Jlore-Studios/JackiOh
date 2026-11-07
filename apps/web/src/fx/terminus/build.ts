// R3338000: what every Terminus set piece is built from (fx/terminus/): its moment's context (the entry,
// its hold H, the span S its beats use and the motion M after the hold), its piece cue and its particle
// counts. Every cue starts inside the entry and ends within FX_MAX_TAIL_MS of it (R200); a planner reads
// the run's own events and the view alone, never which card a hidden one is (R202).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { FLAVOR_LOOKS, type FlavorLook } from "../flavors.ts";
import type { AnimationEntry, EntrySignature } from "../../game/animations.ts";
import { testid } from "../../game/contract.ts";
import { frac, tid } from "../build.ts";
import { FX_TERMINUS_TAIL_MS } from "../constants.ts";
import { SET_PIECES, SET_PIECE_SWALLOW_AT, beatAt, tintOf, type SetPieceKey, type SetPiecePlan } from "../setPieces.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxSetPieceCue, SetPieceShape } from "../types.ts";

/** Every particle count and share of a span the Terminus set pieces throw and keep, at intensity "normal" (rule 9). */
export const TERMINUS_TUNING = {
  // Olivian Expanse: the rift's tear, and the star's landing in the hand.
  rift: { count: 26, power: 1.1 },
  arrive: { count: 12, power: 0.9 },
  // KY's Last Exam: the chalk dust where the problem cracks, and the lines it writes.
  chalk: { count: 16, power: 0.6 },
  chalkLines: 4,
  // Event Horizon: the turn banner shows at these shares of the span, each this long; its snap back.
  horizonBannersAt: [0.15, 0.55] as const,
  horizonBannerSpan: 0.35,
  snap: { count: 30, power: 1.4 },
  // Last Word: the hero's shatter, at this share of the span; the countdown's last ember.
  freezeAt: 0.5,
  shatter: { count: 22, power: 1.2 },
  countEmber: { count: 14, power: 0.6 },
  // 1v1 Final Destination: a lane chaining shut, the health counters at this share, the Spells burning from this one.
  chain: { count: 10, power: 0.7 },
  healthAt: 0.7,
  burnAt: 0.75,
  burn: { count: 6, power: 0.6 },
  burnMax: 6,
  // Dice of Doom: the die lands at this share of the span; a jackpot's rain falls this share of the motion; a
  // miss crumbles at this share; an Embiggened die's Lucky glint at this one.
  doomLandAt: 0.5,
  rainFall: 0.3,
  rain: { count: 8, power: 0.8 },
  crumbleAt: 0.6,
  crumble: { count: 20, power: 1 },
  glintAt: 0.8,
  glint: { count: 16, power: 0.9 },
} as const;

/** One Terminus moment as its planner reads it. */
export type TerminusMoment = {
  key: SetPieceKey;
  entry: AnimationEntry;
  view: PlayerView;
  env: FxPlanEnv;
  signature: EntrySignature;
  /** The entry's duration, its hold included. */
  D: number;
  /** Its hold, when it is this card's set piece; else 0 (Big moments "off", a second claim in the action). */
  H: number;
  /** The span its beats use: the hold, or unheld, up to SET_PIECE_SWALLOW_AT of the entry. */
  S: number;
  /** The motion after the hold (the whole entry, unheld). */
  M: number;
  /** The intensity. */
  i: number;
  look: FlavorLook;
};

export type TerminusPlanner = (moment: TerminusMoment) => SetPiecePlan;

/** The moment of `key` in `entry`, or null for the effects off or an entry of no duration (Reduce motion). */
export function momentOf(key: SetPieceKey, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): TerminusMoment | null {
  const signature = entry.signature;
  const D = entry.durationMs;
  const i = env.intensity;
  if (signature === undefined || !(i > 0) || !(D > 0)) return null;
  const H = entry.setPiece?.key === key ? entry.setPiece.holdMs : 0;
  const S = H > 0 ? H : frac(SET_PIECE_SWALLOW_AT, D);
  const palette = SET_PIECES[key].palette;
  return { key, entry, view, env, signature, D, H, S, M: D - H, i, look: FLAVOR_LOOKS[palette ?? "Dark"] };
}

/** `n` beats evenly over a span `S` (`beatAt`). */
export function beats(n: number, S: number): number[] {
  return Array.from({ length: n }, (_, k) => beatAt(k, n, S));
}

/** The set piece's own cue over the whole layer, centred on `at`, its parts timed off `holdMs` and marked at each beat. */
export function pieceCue(shape: SetPieceShape, look: FlavorLook, at: FxAnchor, holdMs: number, marks: readonly number[], durationMs: number): FxSetPieceCue {
  return {
    kind: "setPiece",
    shape,
    tint: tintOf(look),
    spin: 1,
    side: null,
    at,
    holdMs,
    delayMs: 0,
    durationMs,
    ...(marks.length > 0 ? { beats: [...marks] } : {}),
  };
}

/** A moment's piece cue: its shape, its span and its beats, lasting the entry and FX_TERMINUS_TAIL_MS more. */
export function momentPiece(m: TerminusMoment, at: FxAnchor, marks: readonly number[]): FxSetPieceCue {
  return pieceCue(SET_PIECES[m.key].shape, m.look, at, m.S, marks, m.D + FX_TERMINUS_TAIL_MS);
}

/** `cues` moved `ms` later (a cue planned over the motion after a hold, M, shifted past the hold). */
export function shiftCues(cues: readonly FxCue[], ms: number): FxCue[] {
  return ms === 0 ? [...cues] : cues.map((cue) => ({ ...cue, delayMs: cue.delayMs + ms }));
}

/** The board's centre. */
export const BOARD: FxAnchor = tid(testid.board);

/** The run's events of `types`. */
export function eventsOf<T extends GameEvent["type"]>(entry: AnimationEntry, types: readonly T[]): Extract<GameEvent, { type: T }>[] {
  return entry.events.filter((event): event is Extract<GameEvent, { type: T }> => (types as readonly string[]).includes(event.type));
}
