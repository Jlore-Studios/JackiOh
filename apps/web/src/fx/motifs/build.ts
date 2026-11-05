// R2784000: what every motif is built from (docs/polish/9-card-animations.md). A motif is a pure planner,
// `(spec, entry, view, env) => FxCue[]`, over the cue kinds and builders the event rows already use
// (`../build.ts`): no new rendering. Its four phases are fractions of the entry's D (FX_MOTIF_PHASES), so
// every cue starts inside the entry and its tail is an existing one (R200), and D carries the effects
// speed (R201). It reads the redacted stream only: the entry, its view and the readable resolving play
// (R202). Every count is MOTIF_TUNING's, every fraction and shake constants.ts's (CLAUDE.md rule 9).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, locateInstance, targetFor, type AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid, type Side } from "../../game/contract.ts";
import { armorTookHalf } from "../../game/damageFeel.ts";
import { CATALOG } from "../../wire/cards.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst, type BurstTuning } from "../build.ts";
import {
  FX_CRACK_TAIL_MS,
  FX_MOTIF_COST_CAP,
  FX_MOTIF_PHASES,
  FX_MOTIF_RARITY_SCALE,
  FX_MOTIF_SCALE_MAX,
  FX_MOTIF_SCALE_PER_COST,
  FX_MOTIF_TRAUMA,
  FX_MOTIF_X_COST,
  FX_SPLAT_HOLD_MS,
} from "../constants.ts";
import type { FlavorLook } from "../flavors.ts";
import { hitIdOf } from "../memory.ts";
import { shieldCues } from "../shield.ts";
import type { FxAnchor, FxBurstCue, FxCue, FxPlanEnv, FxPoint, FxPreset, FxRingCue, FxShotCue, FxSpread } from "../types.ts";
import type { DrawnMotif } from "./names.ts";

/** What one motif plays with: its look (a flavor's by default), its scale (intensity included) and whether the event rows it draws over still play. */
export type MotifSpec = { motif: DrawnMotif; look: FlavorLook; scale: number; row: "keep" | "replace" };

/** R2789000: what the cue helpers read of a spec, so a keyword beat (beats.ts) builds with them and names no motif. */
export type CueSpec = Pick<MotifSpec, "look" | "scale" | "row">;

export type MotifPlanner = {
  /** The event types it draws on. */
  reads: readonly GameEvent["type"][];
  /** Those of `reads` whose own row recipe it replaces when its row is `replace`. */
  replaces: readonly GameEvent["type"][];
  plan: (spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv) => FxCue[];
};

/** Every particle count the motifs throw at scale 1 (rule 9). */
export const MOTIF_TUNING = {
  gather: { count: 14, power: 0.8 },
  land: { count: 26, power: 1.2 },
  linger: { count: 10, power: 0.6 },
  small: { count: 8, power: 0.9 },
  nova: { count: 40, power: 1.5 },
  wave: { count: 12, power: 0.9 },
  ember: { count: 18, power: 0.7 },
} as const;

/** The foot of a zone, where embers rise (cues.ts's FOOT). */
export const MOTIF_FOOT: FxPoint = { x: 0.5, y: 1 };

export type MotifPhase = keyof typeof FX_MOTIF_PHASES;

export function phaseAt(phase: MotifPhase, D: number): number {
  return frac(FX_MOTIF_PHASES[phase], D);
}

/** The entry's D and the start of each phase in it: Gather, Travel, Land and Linger. */
export type MotifTimes = { D: number; G: number; T: number; L: number; Li: number };

export function timesOf(D: number): MotifTimes {
  return { D, G: phaseAt("gather", D), T: phaseAt("travel", D), L: phaseAt("land", D), Li: phaseAt("linger", D) };
}

/** A burst of the look's particle (or `preset`) in the look's colors, its count scaled. */
export function motifBurst(
  spec: CueSpec,
  at: FxAnchor,
  spread: FxSpread,
  delayMs: number,
  tuning: BurstTuning,
  preset: FxPreset = spec.look.particle,
): FxBurstCue {
  return { ...tunedBurst(spec.scale, preset, at, spread, delayMs, tuning), colors: spec.look.colors };
}

/** A glowing straight bolt, or an instant beam, in the look's colors, landing at `delayMs + flightMs`. */
export function motifShot(spec: CueSpec, motion: "bolt" | "beam", from: FxAnchor, to: FxAnchor, delayMs: number, flightMs: number): FxShotCue {
  return {
    kind: "shot",
    shot: motion === "bolt" ? { motion: "bolt", trail: spec.look.particle } : { motion: "beam" },
    colors: spec.look.colors,
    preset: spec.look.particle,
    from,
    to,
    delayMs,
    flightMs,
    density: spec.scale,
  };
}

export function motifRing(spec: CueSpec, D: number, at: FxAnchor, delayMs: number): FxRingCue {
  return ringCue(D, spec.look.particle, at, delayMs);
}

export function motifShake(spec: MotifSpec, delayMs: number): FxCue[] {
  return shakeCues(spec.scale, FX_MOTIF_TRAUMA[spec.motif], delayMs);
}

/** One event a motif draws on, and where: its target's testid as the row recipe would find it. */
export type MotifHit = { event: GameEvent; tgt: string; at: FxAnchor };

/**
 * The entry's events of `types` a motif draws on, each with its target. Combat damage is never a card's,
 * and nor is a hit another card dealt while the play resolves (a Trap answering it): a damage counts when
 * it names no source or the play's own instance. A card the play's hits already reached (a bolt's
 * damage, then its death) leaves the stage under its own row, so the motif lands once. An event with no
 * target is dropped.
 */
export function readEvents(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv, types: readonly GameEvent["type"][]): MotifHit[] {
  const play = env.memory.resolving();
  const struck = new Set(earlierHits(env, entry));
  const hits: MotifHit[] = [];
  for (const event of entry.events) {
    const id = hitIdOf(event);
    const again = id !== null && struck.has(id);
    if (id !== null) struck.add(id);
    if (!types.includes(event.type)) continue;
    if (again && event.type !== "damage") continue;
    if (event.type === "damage") {
      if (event.combat) continue;
      if (play !== undefined && event.sourceId !== null && event.sourceId !== play.instanceId) continue;
    }
    const tgt = targetFor(event, view);
    if (tgt !== null) hits.push({ event, tgt, at: tid(tgt) });
  }
  return hits;
}

/** Where the motif gathers and fires from: a damage's source on the board, else the resolving play's hero. */
export function sourceOf(event: GameEvent, view: PlayerView, env: FxPlanEnv): FxAnchor | null {
  if (event.type === "damage" && event.sourceId !== null && event.sourceId !== HIDDEN_ID) {
    const at = locateInstance(view, event.sourceId);
    if (at !== null) return tid(at);
  }
  const play = env.memory.resolving();
  return play === undefined ? null : tid(testid.hero(sideOf(view, play.player)));
}

/** Whether a shot from `from` to `to` would fly anywhere. */
export function flies(from: FxAnchor | null, to: FxAnchor): from is FxAnchor {
  if (from === null) return false;
  return !(from.kind === "testid" && to.kind === "testid" && from.testid === to.testid);
}

const HERO_ID = /^hero-(p1|p2)$/;

/** The seat across the board from `side`. */
export function otherSide(side: Side): Side {
  return side === "you" ? "opponent" : "you";
}

/** The side and lane (1 on) of a unit or face-up backrow card; a hero has a side and no lane. */
export function placeOf(view: PlayerView, id: string): { side: Side; lane: number | null } | null {
  if (id === HIDDEN_ID) return null;
  const hero = HERO_ID.exec(id)?.[1];
  if (hero !== undefined) return { side: sideOf(view, hero as "p1" | "p2"), lane: null };
  for (const side of ["you", "opponent"] as const) {
    const sv = view[side];
    const unit = sv.units.findIndex((u) => u !== null && u.instanceId === id);
    if (unit >= 0) return { side, lane: unit + 1 };
    const slot = sv.backrow.findIndex((b) => b !== null && b.faceDown === false && (b.instanceId === id || b.unit?.instanceId === id));
    if (slot >= 0) return { side, lane: slot + 1 };
  }
  return null;
}

/**
 * The ids the resolving play's hits landed on before this entry, oldest first. The runner gives each
 * hit of a run that sweeps no whole side its own entry, so a motif that strings its hits together (a
 * volley's one gather) reads the ones before from here; none with no play.
 */
export function earlierHits(env: FxPlanEnv, entry: AnimationEntry): readonly string[] {
  const hits = env.memory.resolving()?.hits ?? [];
  const own = entry.events.filter((event) => hitIdOf(event) !== null).length;
  return hits.slice(0, Math.max(0, hits.length - own));
}

/** Where a hit's id is drawn in `view` (a hero, or a card still shown), or null. */
export function hitAnchor(view: PlayerView, id: string): FxAnchor | null {
  const hero = HERO_ID.exec(id)?.[1];
  if (hero !== undefined) return tid(testid.hero(sideOf(view, hero as "p1" | "p2")));
  const at = locateInstance(view, id);
  return at === null ? null : tid(at);
}

/**
 * The landing on one target at `land`: a burst, and the numbers when the motif replaces the row (a
 * damage's splat and its Armor flash, R1363), and a crack under a card destroyed. Each tail is the row's.
 */
export function hit(spec: CueSpec, event: GameEvent, at: FxAnchor, D: number, land: number, tuning: BurstTuning = MOTIF_TUNING.land): FxCue[] {
  const cues: FxCue[] = [motifBurst(spec, at, "area", land, tuning)];
  if (spec.row === "replace" && event.type === "damage") {
    if (event.amount > 0) cues.push({ kind: "splat", tone: "damage", amount: event.amount, at, delayMs: land, durationMs: D - land + FX_SPLAT_HOLD_MS });
    if (armorTookHalf(event.absorbed, event.amount)) cues.push(...shieldCues("small", at, D, land, spec.scale));
  }
  if (event.type === "destroyed") cues.push({ kind: "crack", at, delayMs: land, durationMs: D - land + FX_CRACK_TAIL_MS });
  return cues;
}

/** Whether this entry holds the resolving play's first events of `types` (true with no play). */
export function isFirstEntry(env: FxPlanEnv, entry: AnimationEntry, types: readonly GameEvent["type"][]): boolean {
  const play = env.memory.resolving();
  if (play === undefined) return true;
  const seen = types.reduce((sum, type) => sum + (play.seen[type] ?? 0), 0);
  const own = entry.events.filter((event) => types.includes(event.type)).length;
  return seen - own === 0;
}

/**
 * A card's motif scale: its rarity's × (1 + its cost, capped, × a step), at most FX_MOTIF_SCALE_MAX,
 * times the intensity. An X cost counts as FX_MOTIF_X_COST and an Embiggen cost as its base; a card
 * the catalog and the match's definitions do not know takes the intensity alone.
 */
export function motifScale(defId: string, view: PlayerView, intensity: number): number {
  const def = defId === HIDDEN_ID ? undefined : (CATALOG[defId] ?? view.defs?.[defId]);
  if (def === undefined) return intensity;
  const cost = typeof def.cost === "number" ? def.cost : def.cost === "X" ? FX_MOTIF_X_COST : def.cost.base;
  const grown = FX_MOTIF_RARITY_SCALE[def.rarity] * (1 + Math.min(Math.max(0, cost), FX_MOTIF_COST_CAP) * FX_MOTIF_SCALE_PER_COST);
  return intensity * Math.min(FX_MOTIF_SCALE_MAX, grown);
}
