// The cue planner (docs/polish/1-animations.md S6 and S7).
//
// Turns one animation entry into the effects that decorate it: particle bursts, projectiles, cracks,
// rings, a board shake and short DOM flourishes. Every function here is pure. It reads only its
// arguments, never the DOM, and returns plain data the director schedules.
//
// R200: effects pace nothing. Every delay and duration is derived from the duration the runner gave
// the entry (D, `entry.durationMs`) through the constants in `constants.ts`, so for any D an effect
// starts inside the entry (0 <= delay <= D), a projectile lands inside it (delay + flight <= D) and
// whatever trails after it is gone within FX_MAX_TAIL_MS of the entry's end.
//
// R202: effects draw from the redacted stream only. The planner reads the event, the view the entry
// was planned against and the public catalog facts in `env.card`. An id or defId redacted to
// "hidden" is never looked up, so a hidden card plans the same cues whatever it hides, gets no rarity
// entrance, and no cue ever carries a defId or a card name.
//
// CLAUDE.md rule 9: particle counts live in `TUNING`, the game-over timings in `RESULT_TUNING`, and
// every other number in `constants.ts`.
//
// Issue #124: the runner gives three new shapes of entry (`game/runs.ts`), and each plans as one: a
// whole pile reached at once washes one wave over the pile (`zoneCues`), a sweep rolls a fog over each
// row it swept and lands each hit as the fog reaches its lane (`sweepCues`), and a card another card
// cast flares at its caster's hero, bigger with each cast of the burst (`castCues`). A readable card's
// family lends its look (looks.ts) to those and, as an accent, to a play, a summon and a spell's hit:
// the same recipe in the colours, particles and emblem of what made it.

import { hasKeyword, type GameEvent, type PlayerView } from "@jackioh/shared";

import { GLITCH_WORDS } from "../cards/glitch.ts";
import { ANIMATIONS, animTestid, locateInstance, pileTestid, targetFor, type AnimationEntry, type EntrySlam } from "../game/animations.ts";
import { sideOf, testid, type Side } from "../game/contract.ts";
import { armorTookHalf, damageFeel, damageTier, UNIT_SLAM } from "../game/damageFeel.ts";
import { brandCues } from "./brand.ts";
import { castOnDrawCues, planCardFx } from "./cardFx.ts";
import { chaosCues } from "./chaos.ts";
import { lookOf, TONE_LOOKS, ZONE_DEFAULT, ZONE_LOOKS, type ZoneLook } from "./looks.ts";
import { shieldCues } from "./shield.ts";
import {
  FX_ARROWS_TAIL_MS,
  FX_BANNER_TAIL_MS,
  FX_BURN_AT,
  FX_CAST_SCALE_MAX,
  FX_CAST_SCALE_STEP,
  FX_CAST_TRAUMA,
  FX_CENTER,
  FX_COUNTER_TRAUMA,
  FX_CRACK_TAIL_MS,
  FX_DEATH_EMBER_AT,
  FX_DEATH_SMOKE_AT,
  FX_FATIGUE_FLIGHT_FRACTION,
  FX_FATIGUE_STREAK_AT,
  FX_FLICKER_RETURN_AT,
  FX_FOG_HIT_FROM,
  FX_FOG_HIT_TO,
  FX_FOG_TAIL_MS,
  FX_FUSE_FLIGHT_FRACTION,
  FX_HANDOVER_BANNER_MS,
  FX_HEAL_SPLAT_AT,
  FX_LEGENDARY_TRAUMA,
  FX_LETHAL_LEAD_MAX_MS,
  FX_MANA_MAX_SPARKS,
  FX_MANA_STAGGER_MS,
  FX_MAX_TAIL_MS,
  FX_MIND_CONTROL_FLIGHT_FRACTION,
  FX_OVERFLOW_FIZZLE_AT,
  FX_PROJECTILE_FLIGHT_FRACTION,
  FX_RADIANT_BURST_AT,
  FX_RAYS_TAIL_MS,
  FX_REDIRECT_FLIGHT_FRACTION,
  FX_RESULT_MS,
  FX_RESULT_TRAUMA,
  FX_REWIND_TRAUMA,
  FX_RING_MS,
  FX_SHAKE_MAX_PX,
  FX_SLAM_AT,
  FX_SLAM_MAX_TRAUMA,
  FX_SLAM_STATS_MIN,
  FX_SLAM_TRAUMA_PER_STAT,
  FX_SPLAT_HOLD_MS,
  FX_TEXT,
  FX_TRAP_BURST_AT,
  FX_TRAP_TRAUMA,
  FX_ZONE_COUNT_AT,
  FX_ZONE_TAIL_MS,
} from "./constants.ts";
import type {
  FxAnchor,
  FxBannerTone,
  FxBurstCue,
  FxCardFacts,
  FxCrackCue,
  FxCue,
  FxGhostCue,
  FxMemory,
  FxOutcome,
  FxPlanEnv,
  FxPoint,
  FxPreset,
  FxProjectileCue,
  FxRayTone,
  FxRaysCue,
  FxRecipe,
  FxRingCue,
  FxShakeCue,
  FxSplatCue,
  FxSplatTone,
  FxSpread,
} from "./types.ts";

/* ------------------------------------------------------------------------------------------- *
 * Tuning tables (rule 9)
 * ------------------------------------------------------------------------------------------- */

/**
 * Every burst's base particle count at intensity "normal", and the emit power it is thrown with.
 * A planned count is `max(1, round(count × intensity))`; power does not scale with intensity.
 * Every count is at least 6, so "low" plans strictly fewer particles than "normal" and "high" more.
 */
const TUNING = {
  cast: { count: 24, power: 1 },
  castOpponent: { count: 12, power: 0.8 },
  summonDust: { count: 30, power: 1.1 },
  // #185: a Small Unit shifts a few grains, a Large one kicks up a cloud, a Huge or MASSIVE one heavy dust.
  slamGrains: { count: 8, power: 0.5 },
  slamCloud: { count: 48, power: 1.3 },
  slamHeavy: { count: 64, power: 1.6 },
  summonGold: { count: 40, power: 1.3 },
  summonPrismatic: { count: 44, power: 1.3 },
  impactSpark: { count: 24, power: 1.2 },
  impactHeavy: { count: 44, power: 1.45 },
  impactDust: { count: 22, power: 0.9 },
  impactPoison: { count: 20, power: 0.8 },
  drainVoid: { count: 16, power: 0.8 },
  healHoly: { count: 26, power: 0.9 },
  shieldShard: { count: 28, power: 1.3 },
  deathEmber: { count: 36, power: 0.9 },
  deathSmoke: { count: 16, power: 0.7 },
  pileSmoke: { count: 8, power: 0.6 },
  exileVoid: { count: 18, power: 0.9 },
  bounceSmoke: { count: 12, power: 0.7 },
  burnFire: { count: 32, power: 1 },
  burnEmber: { count: 18, power: 0.9 },
  burnGraveEmber: { count: 8, power: 0.6 },
  fatigueDust: { count: 16, power: 0.8 },
  fatigueSmoke: { count: 8, power: 0.6 },
  fatigueVoid: { count: 12, power: 0.8 },
  overflowSmoke: { count: 12, power: 0.7 },
  overflowEmber: { count: 8, power: 0.6 },
  discardEmber: { count: 8, power: 0.6 },
  drawSparkle: { count: 6, power: 0.6 },
  handSparkle: { count: 10, power: 0.7 },
  shuffleArcane: { count: 8, power: 0.6 },
  buffSparkle: { count: 12, power: 0.8 },
  debuffVoid: { count: 12, power: 0.8 },
  keywordHoly: { count: 16, power: 0.9 },
  keywordPoison: { count: 14, power: 0.8 },
  keywordOther: { count: 10, power: 0.8 },
  counterPoison: { count: 12, power: 0.7 },
  counterSparkle: { count: 8, power: 0.7 },
  glintArcane: { count: 8, power: 0.6 },
  radiantGold: { count: 34, power: 1.1 },
  transformSmoke: { count: 10, power: 0.6 },
  transformArcane: { count: 10, power: 0.9 },
  fuseSmoke: { count: 12, power: 0.7 },
  fuseArcane: { count: 20, power: 1 },
  controlArcane: { count: 18, power: 1 },
  lockDust: { count: 12, power: 0.7 },
  trapArcane: { count: 30, power: 1 },
  lungeDust: { count: 12, power: 0.8 },
  fizzleSmoke: { count: 10, power: 0.7 },
  manaSparkle: { count: 6, power: 0.5 },
  // Patch v0.2.0's events (docs/classic-sets.md B3, B5).
  announceSparkle: { count: 10, power: 0.6 },
  animateArcane: { count: 12, power: 0.8 },
  crumbleShard: { count: 26, power: 1.1 },
  crumbleDust: { count: 14, power: 0.8 },
  counterShard: { count: 22, power: 1.2 },
  counterVoid: { count: 16, power: 0.9 },
  flickerArcane: { count: 14, power: 0.9 },
  unlockShard: { count: 18, power: 1 },
  healthSetHoly: { count: 18, power: 0.9 },
  rewindArcane: { count: 40, power: 1.2 },
  resultConfetti: { count: 90, power: 1.4 },
  resultShard: { count: 36, power: 1.5 },
  resultSmoke: { count: 24, power: 0.9 },
  resultEmber: { count: 30, power: 1 },
  resultDust: { count: 16, power: 0.8 },
  // Issue #124: the looks (looks.ts) a card's family lends a recipe, and the runner's new shapes.
  castAccent: { count: 14, power: 0.9 },
  summonAccent: { count: 16, power: 1 },
  impactAccent: { count: 12, power: 1 },
  multicastFlare: { count: 28, power: 1.2 },
  multicastMotes: { count: 12, power: 0.7 },
  zoneWave: { count: 22, power: 0.9 },
  zoneMotes: { count: 10, power: 0.6 },
  fogHit: { count: 10, power: 0.8 },
} as const;

type TuningKey = keyof typeof TUNING;

/** The game-over sequence's own timings (S7 `planResult`), all inside FX_RESULT_MS. */
const RESULT_TUNING = {
  /** How long the loser's hero crack runs. */
  crackMs: 900,
  /** When the hero bursts into shards and the board shakes. */
  impactDelayMs: 200,
  /** When the smoke (and, on a defeat, the embers) rise from the broken hero. */
  smokeDelayMs: 300,
} as const;

/** The bottom centre of a box: where a landing unit kicks up dust. */
const FOOT: FxPoint = { x: 0.5, y: 1 };

const HIDDEN_ID = "hidden";

const SIDES: readonly Side[] = ["you", "opponent"];

/* ------------------------------------------------------------------------------------------- *
 * Anchors
 * ------------------------------------------------------------------------------------------- */

function anchor(id: string, at?: FxPoint): FxAnchor {
  return at === undefined ? { kind: "testid", testid: id } : { kind: "testid", testid: id, at: { x: at.x, y: at.y } };
}

function viewportCenter(): FxAnchor {
  return { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };
}

const isCard = (tgt: string): boolean => tgt.startsWith("card-");
const isHandCard = (tgt: string): boolean => tgt.startsWith("hand-card-");
/* ------------------------------------------------------------------------------------------- *
 * Cue builders
 * ------------------------------------------------------------------------------------------- */

function frac(fraction: number, durationMs: number): number {
  return Math.round(fraction * durationMs);
}

function burst(
  intensity: number,
  preset: FxPreset,
  at: FxAnchor,
  spread: FxSpread,
  delayMs: number,
  key: TuningKey,
  scale?: number,
): FxBurstCue {
  const tuning = TUNING[key];
  return {
    kind: "burst",
    preset,
    at,
    delayMs,
    count: Math.max(1, Math.round(tuning.count * intensity)),
    spread,
    power: tuning.power,
    ...(scale === undefined || scale === 1 ? {} : { scale }),
  };
}

/** `density` is the intensity: it scales the trail and the arrival burst as `count` scales a burst. */
function projectile(intensity: number, preset: FxPreset, from: FxAnchor, to: FxAnchor, flightMs: number): FxProjectileCue {
  return { kind: "projectile", preset, from, to, delayMs: 0, flightMs, density: intensity };
}

function ring(D: number, preset: FxPreset, at: FxAnchor, delayMs: number): FxRingCue {
  return { kind: "ring", preset, at, delayMs, durationMs: Math.min(FX_RING_MS, D - delayMs + FX_MAX_TAIL_MS) };
}

function rays(D: number, tone: FxRayTone, at: FxAnchor, delayMs: number): FxRaysCue {
  return { kind: "rays", tone, at, delayMs, durationMs: D - delayMs + FX_RAYS_TAIL_MS };
}

function splat(D: number, tone: FxSplatTone, amount: number, at: FxAnchor, delayMs: number): FxSplatCue {
  return { kind: "splat", tone, amount, at, delayMs, durationMs: D - delayMs + FX_SPLAT_HOLD_MS };
}

function crack(D: number, at: FxAnchor, delayMs: number): FxCrackCue {
  return { kind: "crack", at, delayMs, durationMs: D - delayMs + FX_CRACK_TAIL_MS };
}

function ghost(D: number, from: FxAnchor, to: FxAnchor): FxGhostCue {
  return { kind: "ghost", from, to, delayMs: 0, durationMs: D };
}

function banner(D: number, text: string, tone: FxBannerTone): FxCue {
  return { kind: "banner", text, tone, delayMs: 0, durationMs: D + FX_BANNER_TAIL_MS };
}

/** Appends a shake of `min(1, base × intensity)` at `delayMs`, only when that is above 0. */
/**
 * The trauma that shakes the board by `px` at its peak: the shake's offset is FX_SHAKE_MAX_PX times
 * trauma squared (shake.ts), so a tuning table can name its shake in pixels (#57, #185).
 */
export function traumaForShakePx(px: number): number {
  return px <= 0 ? 0 : Math.min(1, Math.sqrt(px / FX_SHAKE_MAX_PX));
}

function pushShake(cues: FxCue[], intensity: number, base: number, delayMs: number): void {
  const trauma = Math.min(1, base * intensity);
  if (trauma > 0) {
    const cue: FxShakeCue = { kind: "shake", trauma, delayMs };
    cues.push(cue);
  }
}

/* ------------------------------------------------------------------------------------------- *
 * Sources
 * ------------------------------------------------------------------------------------------- */

type FxSourceKind = "unit" | "backrow" | "hand" | "trap" | "hero";

/** Where `id` is rendered in `view` and as what, mirroring `locateInstance`'s search order. */
function renderedSource(view: PlayerView, id: string): { anchor: FxAnchor; kind: FxSourceKind } | null {
  for (const side of SIDES) {
    const sv = side === "you" ? view.you : view.opponent;
    for (const unit of sv.units) {
      if (unit !== null && unit.instanceId === id) return { anchor: anchor(testid.card(id)), kind: "unit" };
    }
    for (const slot of sv.backrow) {
      if (slot !== null && slot.faceDown === false && slot.instanceId === id) {
        return { anchor: anchor(testid.card(id)), kind: "backrow" };
      }
    }
  }
  const hand = view.you.hand;
  if (Array.isArray(hand) && hand.some((card) => card.instanceId === id)) {
    return { anchor: anchor(testid.handCard(id)), kind: "hand" };
  }
  return null;
}

/** True when `sourceId` is a unit on either board of `view` that has Poisonous. */
function sourceIsPoisonous(sourceId: string | null, view: PlayerView): boolean {
  if (sourceId === null || sourceId === HIDDEN_ID) return false;
  for (const side of SIDES) {
    const sv = side === "you" ? view.you : view.opponent;
    for (const unit of sv.units) {
      if (unit !== null && unit.instanceId === sourceId) return hasKeyword(unit.keywords, "Poisonous");
    }
  }
  return false;
}

/** Where a non-combat damage source is: rendered instance → remembered trap zone → caster's hero → null. */
export function sourceAnchor(
  sourceId: string | null,
  view: PlayerView,
  memory: FxMemory,
): { anchor: FxAnchor; kind: FxSourceKind } | null {
  if (sourceId === null || sourceId === HIDDEN_ID) return null;
  const rendered = renderedSource(view, sourceId);
  if (rendered !== null) return rendered;
  const zone = memory.trapZoneOf(sourceId);
  if (zone !== undefined) {
    return { anchor: anchor(testid.zone(sideOf(view, zone.player), zone.row, zone.lane)), kind: "trap" };
  }
  const caster = memory.casterOf(sourceId);
  if (caster !== undefined) return { anchor: anchor(testid.hero(sideOf(view, caster))), kind: "hero" };
  return null;
}

/* ------------------------------------------------------------------------------------------- *
 * Recipes: one per FxRecipe (S7)
 * ------------------------------------------------------------------------------------------- */

type Plan = {
  entry: AnimationEntry;
  view: PlayerView;
  env: FxPlanEnv;
  /** The duration the runner gave this entry. */
  D: number;
  /** `targetFor(event, view)`, never null here. */
  tgt: string;
};

type Recipe = (event: GameEvent, p: Plan) => FxCue[];

const cast: Recipe = (event, p) => {
  // B5 E1: an announced play glows where it hangs; B3.2: an ability flares on its card.
  if (event.type === "cardAnnounced") {
    // It hangs where it was announced, under a sheen, while the window for a Counter is open.
    const at = anchor(p.tgt);
    const cues: FxCue[] = [ring(p.D, "arcane", at, 0), burst(p.env.intensity, "sparkle", at, "ring", 0, "announceSparkle")];
    if (isCard(p.tgt) || isHandCard(p.tgt)) cues.push({ kind: "sheen", at, delayMs: 0, durationMs: p.D });
    return cues;
  }
  if (event.type === "activated") {
    const at = anchor(p.tgt);
    return [ring(p.D, "arcane", at, 0), burst(p.env.intensity, "arcane", at, "point", 0, "cast")];
  }
  if (event.type !== "cardPlayed") return [];
  // R502: a card cast as it was drawn never was in a hand; it bursts out of the Deck pile instead.
  if (p.env.memory.castOnDraw(event)) return castOnDrawCues(event, p);
  const paired = p.entry.events.some((e) => e.type === "summoned" && e.instanceId === event.instanceId);
  if (paired) return [];
  const at = anchor(p.tgt);
  // Issue #124: a card the viewer reads flares in its family's look as well.
  const look = lookOf(factsOf(event.defId, p.env));
  const accent = look === undefined ? [] : [burst(p.env.intensity, look.preset, at, "ring", 0, "castAccent")];
  if (isCard(p.tgt) || isHandCard(p.tgt)) {
    return [burst(p.env.intensity, "arcane", at, "area", 0, "cast"), ring(p.D, "arcane", at, 0), ...accent];
  }
  return [burst(p.env.intensity, "arcane", at, "point", 0, "castOpponent"), ...accent];
};

/**
 * #185: when in its entry a Unit hits the table: after the slam's anticipation, at FX_SLAM_AT of the
 * landing motion that follows it.
 */
export function slamLandMs(entry: Pick<AnimationEntry, "slam">, D: number): number {
  const wait = entry.slam?.anticipationMs ?? 0;
  return wait + frac(FX_SLAM_AT, D - wait);
}

/** #185: what the board throws up under a landing Unit, by its tier (UNIT_SLAM's `board`). */
function slamCues(slam: EntrySlam, i: number, D: number, tgt: string, at: number): FxCue[] {
  const feel = UNIT_SLAM[slam.tier];
  const zone = anchor(tgt);
  const foot = anchor(tgt, FOOT);
  const cues: FxCue[] = [];
  switch (feel.board) {
    case "none":
      break;
    case "grains":
      cues.push(burst(i, "dust", foot, "ring", at, "slamGrains"));
      break;
    case "puff":
      cues.push(burst(i, "dust", foot, "ring", at, "summonDust"));
      break;
    case "cloud":
      cues.push(ring(D, "dust", zone, at), burst(i, "dust", foot, "ring", at, "slamCloud"));
      break;
    case "cracks":
    case "crater":
      cues.push(ring(D, "dust", zone, at), burst(i, "dust", foot, "ring", at, "slamCloud"), burst(i, "dust", foot, "area", at, "slamHeavy"));
      break;
  }
  if (feel.shockwave) cues.push(ring(D, "dust", viewportCenter(), at));
  pushShake(cues, i, traumaForShakePx(slam.shakePx), at);
  return cues;
}

const summon: Recipe = (event, p) => {
  // #185: the entry's slam, when it is this event's Unit landing (`slamEntries`).
  const slam = "instanceId" in event && p.entry.slam?.instanceId === event.instanceId ? p.entry.slam : undefined;
  const landAt = slamLandMs(p.entry, p.D);
  // B3.1: a backrow card stepping into its unit zone lands like a summon, with no entrance of its own.
  if (event.type === "animated") {
    // B3.1: the card lifts off its backrow zone and lands in its unit zone, where it slams down.
    const home = anchor(testid.zone(sideOf(p.view, event.player), "backrow", event.backrowLane));
    const lift = [ghost(p.D, home, anchor(p.tgt)), burst(p.env.intensity, "arcane", home, "point", 0, "animateArcane")];
    if (slam !== undefined) return [...lift, ...slamCues(slam, p.env.intensity, p.D, p.tgt, landAt)];
    return [...lift, ring(p.D, "dust", anchor(p.tgt), landAt), burst(p.env.intensity, "dust", anchor(p.tgt, FOOT), "ring", landAt, "summonDust")];
  }
  if (event.type !== "summoned") return [];
  const i = p.env.intensity;
  const facts = event.row === "units" && event.defId !== HIDDEN_ID ? p.env.card(event.defId) : undefined;
  if (facts === undefined) return [burst(i, "dust", anchor(p.tgt, FOOT), "ring", landAt, "summonDust")];

  const at = anchor(p.tgt);
  const cues: FxCue[] =
    slam === undefined
      ? [ring(p.D, "dust", at, landAt), burst(i, "dust", anchor(p.tgt, FOOT), "ring", landAt, "summonDust")]
      : slamCues(slam, i, p.D, p.tgt, landAt);
  let entrance = 0;
  if (facts.rarity === "Legendary") {
    cues.push(rays(p.D, "legendary", at, 0), burst(i, "gold", at, "area", landAt, "summonGold"));
    entrance = FX_LEGENDARY_TRAUMA;
  } else if (facts.rarity === "Mythic") {
    cues.push(rays(p.D, "mythic", at, 0), burst(i, "prismatic", at, "area", landAt, "summonPrismatic"));
    entrance = FX_LEGENDARY_TRAUMA;
  }
  if (slam !== undefined) {
    // The slam's own shake is in slamCues; a Legendary's entrance still lands on top of it.
    pushShake(cues, i, entrance, landAt);
    return cues;
  }
  const stats = (facts.attack ?? 0) + (facts.health ?? 0);
  const slamTrauma =
    stats >= FX_SLAM_STATS_MIN
      ? Math.min(FX_SLAM_MAX_TRAUMA, (stats - FX_SLAM_STATS_MIN + 1) * FX_SLAM_TRAUMA_PER_STAT)
      : 0;
  pushShake(cues, i, slamTrauma + entrance, landAt);
  return cues;
};

const impact: Recipe = (event, p) => {
  if (event.type !== "damage") return [];
  const i = p.env.intensity;
  const feel = damageFeel(event.amount);
  const tier = damageTier(event.amount);
  const particleIntensity = i * feel.particleScale;
  const at = anchor(p.tgt);
  const poisonous = sourceIsPoisonous(event.sourceId, p.view);
  const cues: FxCue[] = [];
  let hit = 0;
  if (!event.combat) {
    const src = sourceAnchor(event.sourceId, p.view, p.env.memory);
    if (src !== null && src.anchor.kind === "testid" && src.anchor.testid !== p.tgt) {
      const flight = frac(FX_PROJECTILE_FLIGHT_FRACTION, p.D);
      const preset: FxPreset =
        src.kind === "unit" && poisonous ? "poison" : src.kind === "hand" || src.kind === "hero" ? "fire" : "arcane";
      cues.push(projectile(i, preset, src.anchor, at, flight));
      hit = flight;
    }
  }
  cues.push(burst(particleIntensity, "spark", at, "point", hit, tier === "big" || tier === "giga" ? "impactHeavy" : "impactSpark"));
  if (event.amount > 0) cues.push(splat(p.D, "damage", event.amount, at, hit));
  if (poisonous) cues.push(burst(particleIntensity, "poison", at, "area", hit, "impactPoison"));
  // Issue #124: a spell's or a card's hit lands in its family's look as well.
  if (!event.combat) {
    const look = lookOf(sourceFacts(event.sourceId, p.view, p.env));
    if (look !== undefined) cues.push(burst(particleIntensity, look.preset, at, "area", hit, "impactAccent"));
  }
  if (tier === "moderate" || tier === "big" || tier === "giga") {
    cues.push(burst(particleIntensity, "dust", anchor(p.tgt, FOOT), "area", hit, "impactDust"));
  }
  if (tier === "giga") cues.push(ring(p.D, "dust", viewportCenter(), hit));
  // R1363: the Armor took half or more of the hit, and a small shield glances up as it lands.
  if (armorTookHalf(event.absorbed, event.amount)) cues.push(...shieldCues("small", at, p.D, hit, i));
  // B35: the board shakes here, by the tier's shakePx (#57's DAMAGE_FEEL); a Normal hit stays still.
  pushShake(cues, i, traumaForShakePx(feel.shakePx), hit);
  return cues;
};

/** R1363: the Armor took the whole hit (`damageAbsorbed`), and a full shield blooms over the target. */
const armor: Recipe = (event, p) => {
  if (event.type !== "damageAbsorbed") return [];
  return shieldCues("full", anchor(p.tgt), p.D, 0, p.env.intensity);
};

const drain: Recipe = (event, p) => {
  // B5 E7: a hero's health set outright — a drain of void with no number, since it is not a loss.
  if (event.type === "healthSet") return healthSetCues(event, p);
  if (event.type !== "healthLost") return [];
  const at = anchor(p.tgt);
  return [burst(p.env.intensity, "void", at, "area", 0, "drainVoid"), splat(p.D, "loss", event.amount, at, 0)];
};

const heal: Recipe = (event, p) => {
  if (event.type !== "healed") return [];
  const at = anchor(p.tgt);
  return [
    rays(p.D, "holy", at, 0),
    burst(p.env.intensity, "holy", at, "area", 0, "healHoly"),
    splat(p.D, "heal", event.amount, at, frac(FX_HEAL_SPLAT_AT, p.D)),
  ];
};

const shieldBreak: Recipe = (event, p) => {
  if (event.type !== "divineShieldLost") return [];
  const at = anchor(p.tgt);
  return [ring(p.D, "gold", at, 0), burst(p.env.intensity, "shard", at, "ring", 0, "shieldShard")];
};

const death: Recipe = (event, p) => {
  if (event.type !== "destroyed" && event.type !== "crumbled") return [];
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  if (event.type === "crumbled") {
    // B3.3: Brittle runs out and the card shatters like glass where it stands (a pile stands in).
    if (!isCard(p.tgt) && !isHandCard(p.tgt)) return [burst(i, "shard", at, "point", 0, "crumbleShard")];
    return [
      crack(p.D, at, 0),
      burst(i, "shard", at, "ring", frac(FX_DEATH_EMBER_AT, p.D), "crumbleShard"),
      burst(i, "dust", at, "area", frac(FX_DEATH_SMOKE_AT, p.D), "crumbleDust"),
    ];
  }
  if (!isCard(p.tgt)) return [burst(i, "smoke", at, "point", 0, "pileSmoke")];
  return [
    crack(p.D, at, 0),
    burst(i, "ember", at, "area", frac(FX_DEATH_EMBER_AT, p.D), "deathEmber"),
    burst(i, "smoke", at, "area", frac(FX_DEATH_SMOKE_AT, p.D), "deathSmoke"),
  ];
};

const exile: Recipe = (event, p) => {
  // B5 E22: a flicker blinks the card through the void and back into its zone.
  if (event.type !== "exiled" && event.type !== "flickered") return [];
  const at = anchor(p.tgt);
  const out: FxCue[] = [ring(p.D, "void", at, 0), burst(p.env.intensity, "void", at, "area", 0, "exileVoid")];
  if (event.type === "flickered") {
    // …and back in at once: an arcane ring and motes as it returns to the same zone.
    const back = frac(FX_FLICKER_RETURN_AT, p.D);
    out.push(ring(p.D, "arcane", at, back), burst(p.env.intensity, "arcane", at, "ring", back, "flickerArcane"));
  }
  return out;
};

const bounce: Recipe = (event, p) => {
  if (event.type !== "bounced") return [];
  const at = anchor(p.tgt);
  const cues: FxCue[] = [burst(p.env.intensity, "smoke", at, "area", 0, "bounceSmoke")];
  if (isCard(p.tgt)) cues.push(ghost(p.D, at, anchor(animTestid.hand(sideOf(p.view, event.owner)))));
  return cues;
};

/**
 * R318: the card a full hand cannot take burns over it, and what is left of it reaches the graveyard
 * as the entry ends. The card itself is the board's `burn-notice` (face or back by R97); this only
 * lights it.
 */
const burn: Recipe = (event, p) => {
  if (event.type !== "burned") return [];
  const at = anchor(p.tgt);
  const delay = frac(FX_BURN_AT, p.D);
  return [
    burst(p.env.intensity, "fire", at, "area", delay, "burnFire"),
    burst(p.env.intensity, "ember", at, "area", delay, "burnEmber"),
    burst(p.env.intensity, "ember", anchor(animTestid.graveyard(sideOf(p.view, event.owner))), "point", p.D, "burnGraveEmber"),
  ];
};

const discard: Recipe = (event, p) => {
  if (event.type !== "discarded") return [];
  const graveyard = animTestid.graveyard(sideOf(p.view, event.owner));
  return [
    ghost(p.D, anchor(p.tgt), anchor(graveyard)),
    burst(p.env.intensity, "ember", anchor(graveyard), "point", p.D, "discardEmber"),
  ];
};

const draw: Recipe = (event, p) => {
  if (event.type !== "drawn") return [];
  const side = sideOf(p.view, event.player);
  return [
    ghost(p.D, anchor(animTestid.library(side)), anchor(animTestid.hand(side))),
    burst(p.env.intensity, "sparkle", anchor(animTestid.hand(side)), "point", p.D, "drawSparkle"),
  ];
};

const handGlint: Recipe = (event, p) => {
  if (event.type !== "addedToHand") return [];
  const side = sideOf(p.view, event.player);
  return [burst(p.env.intensity, "sparkle", anchor(animTestid.hand(side)), "area", 0, "handSparkle")];
};

const shuffle: Recipe = (event, p) => {
  if (event.type !== "shuffledIn") return [];
  const library = animTestid.library(sideOf(p.view, event.player));
  return [
    ghost(p.D, viewportCenter(), anchor(library)),
    burst(p.env.intensity, "arcane", anchor(library), "point", p.D, "shuffleArcane"),
  ];
};

/** B3.4: which way a Degrade or Upgrade moved the card, for the arrows. */
function tuningDirection(event: Extract<GameEvent, { type: "degraded" | "upgraded" }>): number {
  return event.type === "upgraded" ? 1 : -1;
}

const buff: Recipe = (event, p) => {
  if (event.type !== "buffed" && event.type !== "degraded" && event.type !== "upgraded") return [];
  const net = event.type === "buffed" ? event.attack + event.health : tuningDirection(event);
  const at = anchor(p.tgt);
  if (net > 0) {
    return [
      { kind: "arrows", direction: "up", at, delayMs: 0, durationMs: p.D + FX_ARROWS_TAIL_MS },
      burst(p.env.intensity, "sparkle", at, "area", 0, "buffSparkle"),
    ];
  }
  if (net < 0) {
    return [
      { kind: "arrows", direction: "down", at, delayMs: 0, durationMs: p.D + FX_ARROWS_TAIL_MS },
      burst(p.env.intensity, "void", at, "area", 0, "debuffVoid"),
    ];
  }
  return [];
};

const keyword: Recipe = (event, p) => {
  if (event.type !== "keywordGranted") return [];
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  switch (event.keyword.kind) {
    case "Divine Shield":
      return [ring(p.D, "gold", at, 0), burst(i, "holy", at, "ring", 0, "keywordHoly")];
    case "Poisonous":
      return [burst(i, "poison", at, "area", 0, "keywordPoison")];
    case "Taunt":
      return [ring(p.D, "dust", at, 0)];
    default:
      return [burst(i, "arcane", at, "point", 0, "keywordOther")];
  }
};

const counter: Recipe = (event, p) => {
  // Classic #90: a quest's count ticks like a counter.
  if (event.type === "questProgressed") return [burst(p.env.intensity, "sparkle", anchor(p.tgt), "point", 0, "counterSparkle")];
  if (event.type !== "counterChanged") return [];
  const at = anchor(p.tgt);
  if (event.counter === "plague") return [burst(p.env.intensity, "poison", at, "area", 0, "counterPoison")];
  return [burst(p.env.intensity, "sparkle", at, "point", 0, "counterSparkle")];
};

const glint: Recipe = (event, p) => {
  if (event.type === "costChanged") return [burst(p.env.intensity, "arcane", anchor(p.tgt), "point", 0, "glintArcane")];
  if (event.type === "modifierChanged" && event.added) {
    return [burst(p.env.intensity, "arcane", anchor(p.tgt), "point", 0, "glintArcane")];
  }
  // B3.1: a Unit sinking back into its backrow zone glints there; Classic+ #41: a number set outright.
  if (event.type === "deanimated") {
    // B3.1: the Unit lifts out of its unit zone and settles back into its backrow zone.
    const from = anchor(testid.zone(sideOf(p.view, event.player), "units", event.unitLane));
    return [ghost(p.D, from, anchor(p.tgt)), burst(p.env.intensity, "arcane", anchor(p.tgt), "point", p.D, "glintArcane")];
  }
  if (event.type === "numberChanged") {
    // Classic+ #41: a number set outright shimmers on the card.
    const at = anchor(p.tgt);
    return [{ kind: "sheen", at, delayMs: 0, durationMs: p.D }, burst(p.env.intensity, "arcane", at, "point", 0, "glintArcane")];
  }
  return [];
};

const radiant: Recipe = (event, p) => {
  if (event.type !== "radiantSet" && event.type !== "questCompleted") return [];
  const at = anchor(p.tgt);
  const cues: FxCue[] = [
    { kind: "sheen", at, delayMs: 0, durationMs: p.D },
    burst(p.env.intensity, "gold", at, "area", frac(FX_RADIANT_BURST_AT, p.D), "radiantGold"),
  ];
  if (isCard(p.tgt)) cues.push(rays(p.D, "radiant", at, 0));
  return cues;
};

const smoke: Recipe = (event, p) => {
  if (event.type !== "transformed") return [];
  const at = anchor(p.tgt);
  // Centred on the unit that changed, and light: an area of big puffs read as smoke drifting over
  // the neighbouring lanes on a phone (integration QA).
  return [
    burst(p.env.intensity, "smoke", at, "point", 0, "transformSmoke"),
    burst(p.env.intensity, "arcane", at, "point", 0, "transformArcane"),
  ];
};

const fuse: Recipe = (event, p) => {
  if (event.type !== "fused") return [];
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  const flight = frac(FX_FUSE_FLIGHT_FRACTION, p.D);
  const cues: FxCue[] = [burst(i, "smoke", at, "area", 0, "fuseSmoke")];
  for (const id of event.instanceIds) {
    if (id === HIDDEN_ID) continue;
    const other = locateInstance(p.view, id);
    if (other === null || other === p.tgt) continue;
    const from = anchor(other);
    cues.push(burst(i, "smoke", from, "area", 0, "fuseSmoke"), projectile(i, "arcane", from, at, flight));
  }
  cues.push(burst(i, "arcane", at, "area", flight, "fuseArcane"));
  return cues;
};

const mindControl: Recipe = (event, p) => {
  // B5 E2, E16: a stolen card's arcane lands on the thief's hand.
  if (event.type === "stolen") return stolenCues(event, p);
  if (event.type !== "controlChanged") return [];
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  const from = event.instanceId === HIDDEN_ID ? null : locateInstance(p.view, event.instanceId);
  if (from === null) return [burst(i, "arcane", at, "area", 0, "controlArcane")];
  const flight = frac(FX_MIND_CONTROL_FLIGHT_FRACTION, p.D);
  return [projectile(i, "arcane", anchor(from), at, flight), burst(i, "arcane", at, "area", flight, "controlArcane")];
};

const lock: Recipe = (event, p) => {
  if (event.type !== "locked" && event.type !== "unlocked") return [];
  const at = anchor(p.tgt);
  // B5 E20: an unlock snaps the chains in gold shards.
  if (event.type === "unlocked") return [ring(p.D, "gold", at, 0), burst(p.env.intensity, "shard", at, "ring", 0, "unlockShard")];
  return [ring(p.D, "dust", at, 0), burst(p.env.intensity, "dust", at, "area", 0, "lockDust")];
};

const trap: Recipe = (event, p) => {
  if (event.type !== "trapFired") return [];
  const at = anchor(p.tgt);
  const delay = frac(FX_TRAP_BURST_AT, p.D);
  // A tight arcane ring on the trap's own zone, not motes over the whole area (integration QA).
  const cues: FxCue[] = [ring(p.D, "arcane", at, 0), burst(p.env.intensity, "arcane", at, "ring", delay, "trapArcane")];
  pushShake(cues, p.env.intensity, FX_TRAP_TRAUMA, delay);
  return cues;
};

const lunge: Recipe = (event, p) => {
  // B5 E9: the redirected hit, attack or pick kicks up dust at its new target.
  if (event.type === "redirected") return redirectedCues(event, p);
  if (event.type !== "attackDeclared") return [];
  return [burst(p.env.intensity, "dust", anchor(p.tgt, FOOT), "point", 0, "lungeDust")];
};

const fizzle: Recipe = (event, p) => {
  // B5 E1: a countered card goes up in smoke; B5 E3: a draw the limit stopped puffs from the deck;
  // R800: a discard a discard guard stopped fizzles over the hand.
  if (event.type !== "attackCancelled" && event.type !== "countered" && event.type !== "drawLimited" && event.type !== "discardPrevented") return [];
  if (event.type === "countered") return counteredCues(p);
  return [burst(p.env.intensity, "smoke", anchor(p.tgt), "point", 0, "fizzleSmoke")];
};

const mana: Recipe = (event, p) => {
  if (event.type !== "manaChanged") return [];
  const side = sideOf(p.view, event.player);
  const old = (side === "you" ? p.view.you : p.view.opponent).mana.current;
  const n = Math.min(event.current, old + FX_MANA_MAX_SPARKS) - old;
  if (n <= 0) return [];
  const step = Math.min(FX_MANA_STAGGER_MS, Math.floor(p.D / Math.max(n, 1)));
  const cues: FxCue[] = [];
  for (let k = 0; k < n; k += 1) {
    const crystal: FxAnchor = { kind: "crystal", side, index: old + k };
    cues.push(burst(p.env.intensity, "sparkle", crystal, "point", Math.min(p.D, k * step), "manaSparkle"));
  }
  return cues;
};

const turnBanner: Recipe = (event, p) => {
  if (event.type === "turnAutoEnded") return [banner(p.D, FX_TEXT.autoEnded, "muted")];
  // B5 E10: an effect cut the turn short.
  if (event.type === "turnCutShort") return [banner(p.D, FX_TEXT.turnCutShort, "muted")];
  // R676: a Glitch's banner is its own corrupted name (cards/glitch.ts), whatever it did.
  if (event.type === "glitched") return [banner(p.D, GLITCH_WORDS.name, "muted")];
  if (event.type !== "turnStarted") return [];
  if (event.player === p.view.viewer) {
    return [banner(p.D, FX_TEXT.yourTurn, "you"), rays(p.D, "victory", viewportCenter(), 0)];
  }
  return [banner(p.D, FX_TEXT.opponentTurn, "opponent")];
};

/**
 * R315, R318: a draw finds the library empty. Dust and a little smoke puff out of the pile at once,
 * then void wisps streak from it to its owner's hero, landing just inside the entry (R200), where the
 * `damage` entry after it pops the number and shakes by the amount.
 */
const fatigue: Recipe = (event, p) => {
  if (event.type !== "fatigue") return [];
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  const hero = anchor(testid.hero(sideOf(p.view, event.player)));
  const leave = frac(FX_FATIGUE_STREAK_AT, p.D);
  const flight = frac(FX_FATIGUE_FLIGHT_FRACTION, p.D);
  const streak: FxProjectileCue = { kind: "projectile", preset: "void", from: at, to: hero, delayMs: leave, flightMs: flight, density: i };
  return [
    burst(i, "dust", at, "area", 0, "fatigueDust"),
    burst(i, "smoke", at, "point", 0, "fatigueSmoke"),
    streak,
    burst(i, "void", hero, "point", leave + flight, "fatigueVoid"),
  ];
};

/**
 * R316, R318: a full library turns a card away. A refusal ring flares on the pile; a card that was
 * never made, or has ceased to exist, fizzles into smoke there, and one sent to the graveyard flies
 * there as a card back (R202: a ghost never names a card) and lands in embers. The card's face, when
 * the viewer may read it, is the board's `overflow-card`, not an effect.
 */
const overflow: Recipe = (event, p) => {
  if (event.type !== "libraryOverflow") return [];
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  const cues: FxCue[] = [ring(p.D, "fire", at, 0)];
  if (event.outcome === "graveyard") {
    const graveyard = anchor(animTestid.graveyard(sideOf(p.view, event.player)));
    cues.push(ghost(p.D, at, graveyard), burst(i, "ember", graveyard, "point", p.D, "overflowEmber"));
    return cues;
  }
  cues.push(burst(i, "smoke", at, "point", frac(FX_OVERFLOW_FIZZLE_AT, p.D), "overflowSmoke"));
  return cues;
};

/* ------------------------------------------------------------------------------------------- *
 * Patch v0.2.0's events (docs/classic-sets.md B3, B5): their own looks, inside R200's bounds
 * ------------------------------------------------------------------------------------------- */

/** The engine writes a hero target as `hero-<playerId>` (as `damage` does). */
const HERO_ID = /^hero-(p1|p2)$/;

/** Where an instance or hero id is drawn in `view`, or null. */
function idAnchor(view: PlayerView, id: string): FxAnchor | null {
  if (id === HIDDEN_ID) return null;
  const hero = HERO_ID.exec(id)?.[1];
  if (hero !== undefined) return anchor(testid.hero(sideOf(view, hero as "p1" | "p2")));
  const at = locateInstance(view, id);
  return at === null ? null : anchor(at);
}

/** B5 E1: a countered card shatters where it hung, then its smoke drifts off; a pile stands in for it. */
function counteredCues(p: Plan): FxCue[] {
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  if (!isCard(p.tgt) && !isHandCard(p.tgt)) {
    return [burst(i, "void", at, "point", 0, "counterVoid"), burst(i, "smoke", at, "point", 0, "fizzleSmoke")];
  }
  const cues: FxCue[] = [
    crack(p.D, at, 0),
    burst(i, "shard", at, "ring", 0, "counterShard"),
    burst(i, "void", at, "area", 0, "counterVoid"),
  ];
  pushShake(cues, i, FX_COUNTER_TRAUMA, 0);
  return cues;
}

/**
 * B5 E2, E16: a stolen card flies from where it was to its thief's hand. The ghost is a card back
 * (R202): which card it is shows only where the view names it.
 */
function stolenCues(event: Extract<GameEvent, { type: "stolen" }>, p: Plan): FxCue[] {
  const from = sideOf(p.view, event.from);
  const source: FxAnchor =
    event.zone === "hand"
      ? anchor(animTestid.hand(from))
      : event.zone === "library"
        ? anchor(animTestid.library(from))
        : event.zone === "graveyard"
          ? anchor(animTestid.graveyard(from))
          : event.zone === "exile"
            ? anchor(animTestid.exile(from))
            : event.zone === "field"
              ? (idAnchor(p.view, event.instanceId) ?? viewportCenter())
              : viewportCenter();
  const flight = frac(FX_MIND_CONTROL_FLIGHT_FRACTION, p.D);
  return [
    ghost(p.D, source, anchor(p.tgt)),
    burst(p.env.intensity, "arcane", source, "point", 0, "controlArcane"),
    burst(p.env.intensity, "arcane", anchor(p.tgt), "area", flight, "controlArcane"),
  ];
}

/** B5 E9: the hit, attack or pick flies off its old target onto the new one. */
function redirectedCues(event: Extract<GameEvent, { type: "redirected" }>, p: Plan): FxCue[] {
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  const from = idAnchor(p.view, event.fromId);
  if (from === null) return [ring(p.D, "arcane", at, 0), burst(i, "spark", at, "point", 0, "impactSpark")];
  const flight = frac(FX_REDIRECT_FLIGHT_FRACTION, p.D);
  return [ring(p.D, "arcane", from, 0), projectile(i, "arcane", from, at, flight), burst(i, "spark", at, "point", flight, "impactSpark")];
}

/**
 * B5 E7: a hero's health set outright. Not damage and not a heal (R18's lose health is the nearest
 * rule), so it reads as either by the way it went: holy light and the gain, or void and the loss.
 */
function healthSetCues(event: Extract<GameEvent, { type: "healthSet" }>, p: Plan): FxCue[] {
  const i = p.env.intensity;
  const at = anchor(p.tgt);
  const before = (sideOf(p.view, event.player) === "you" ? p.view.you : p.view.opponent).hero.health;
  const change = event.health - before;
  if (change > 0) return [rays(p.D, "holy", at, 0), burst(i, "holy", at, "area", 0, "healthSetHoly"), splat(p.D, "heal", change, at, 0)];
  if (change < 0) return [burst(i, "void", at, "area", 0, "drainVoid"), splat(p.D, "loss", -change, at, 0)];
  return [ring(p.D, "arcane", at, 0)];
}

/** Classic+ #35 Rollback: the whole board rewinds, a swirl of arcane at its centre and a jolt. */
const rewind: Recipe = (event, p) => {
  if (event.type !== "rolledBack") return [];
  const cues: FxCue[] = [
    { kind: "sheen", at: anchor(p.tgt), delayMs: 0, durationMs: p.D },
    burst(p.env.intensity, "arcane", viewportCenter(), "ring", 0, "rewindArcane"),
  ];
  pushShake(cues, p.env.intensity, FX_REWIND_TRAUMA, 0);
  return cues;
};

/* ------------------------------------------------------------------------------------------- *
 * Issue #124: the runner's new entry shapes (game/runs.ts), each planned as one
 * ------------------------------------------------------------------------------------------- */

/** The public facts of a readable defId; undefined for R97's sentinel or an unknown id (R202). */
function factsOf(defId: string, env: FxPlanEnv): FxCardFacts | undefined {
  return defId === HIDDEN_ID ? undefined : env.card(defId);
}

/** The defId the view reads on `instanceId`, or undefined when it shows the card nowhere. */
function defIdOfInstance(view: PlayerView, instanceId: string): string | undefined {
  for (const side of SIDES) {
    const sv = side === "you" ? view.you : view.opponent;
    for (const unit of sv.units) {
      if (unit !== null && unit.instanceId === instanceId) return unit.defId;
    }
    for (const unit of sv.carried ?? []) {
      if (unit !== null && unit.instanceId === instanceId) return unit.defId;
    }
    for (const slot of sv.backrow) {
      if (slot !== null && !slot.faceDown && slot.instanceId === instanceId) return slot.defId;
    }
    for (const card of sv.resolving) {
      if (card.instanceId === instanceId) return card.defId;
    }
  }
  const hand = view.you.hand;
  if (Array.isArray(hand)) {
    for (const card of hand) {
      if (card.instanceId === instanceId) return card.defId;
    }
  }
  return undefined;
}

/** The public facts of the card a non-combat hit's source instance is (R202: undefined when hidden). */
function sourceFacts(sourceId: string | null, view: PlayerView, env: FxPlanEnv): FxCardFacts | undefined {
  if (sourceId === null || sourceId === HIDDEN_ID) return undefined;
  const defId = defIdOfInstance(view, sourceId);
  return defId === undefined ? undefined : factsOf(defId, env);
}

/**
 * A card another card cast flares at its caster's hero, in the caster's family look (arcane when
 * the view hides the caster), bigger with each cast of the burst.
 */
function castCues(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  const cast = entry.cast;
  const played = entry.events.find((event) => event.type === "cardPlayed");
  if (cast === undefined || played === undefined || played.type !== "cardPlayed") return [];
  const at = anchor(testid.hero(sideOf(view, played.player)));
  const look = lookOf(factsOf(cast.by, env)) ?? TONE_LOOKS.cast;
  const scale = Math.min(FX_CAST_SCALE_MAX, 1 + (cast.ordinal - 1) * FX_CAST_SCALE_STEP);
  const cues: FxCue[] = [
    burst(env.intensity, look.preset, at, "area", 0, "multicastFlare", scale),
    burst(env.intensity, look.preset, at, "ring", 0, "multicastMotes", scale),
    ring(entry.durationMs, look.ring, at, 0),
  ];
  pushShake(cues, env.intensity, FX_CAST_TRAUMA, 0);
  return cues;
}

/**
 * A run that reached every card of one pile: one wave over the pile in the event's look, with the
 * count it reached popping in its middle, instead of one flash per card.
 */
function zoneCues(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  const zone = entry.zone;
  if (zone === undefined) return [];
  const D = entry.durationMs;
  const at = anchor(pileTestid(zone));
  const first = entry.events[0];
  const zl: ZoneLook = (first === undefined ? undefined : ZONE_LOOKS[first.type]) ?? ZONE_DEFAULT;
  const pop = frac(FX_ZONE_COUNT_AT, D);
  return [
    {
      kind: "zone",
      at,
      tint: zl.look.tint,
      text: `×${String(zone.count)}`,
      direction: zl.direction,
      delayMs: 0,
      durationMs: D + FX_ZONE_TAIL_MS,
    },
    burst(env.intensity, zl.look.preset, at, "area", pop, "zoneWave"),
    burst(env.intensity, zl.look.preset, at, "ring", pop, "zoneMotes"),
  ];
}

/**
 * The instance ids of `side`'s units `entry` reached, in lane order, with the side's hero last
 * when the sweep reached them too.
 */
function sweepOrder(entry: AnimationEntry, view: PlayerView, side: Side): string[] {
  const sv = side === "you" ? view.you : view.opponent;
  const reached = new Set<string>();
  for (const event of entry.events) {
    if (event.type === "damage" || event.type === "healed" || event.type === "damageAbsorbed") reached.add(event.targetId);
    else if (event.type === "divineShieldLost") reached.add(event.instanceId);
  }
  const ids: string[] = [];
  for (const unit of sv.units) {
    if (unit !== null && reached.has(unit.instanceId)) ids.push(unit.instanceId);
  }
  const hero = `hero-${sv.player}`;
  if (reached.has(hero)) ids.push(hero);
  return ids;
}

/**
 * A run of hits or heals that swept a side: one fog rolling over each swept side's row, in the
 * look of what cast it (fire and a flame for hits, holy light and a heart for heals, when nothing
 * readable did), with each hit landing as the fog reaches its lane.
 */
function sweepCues(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  const sweep = entry.sweep;
  if (sweep === undefined) return [];
  const D = entry.durationMs;
  const i = env.intensity;
  const look = lookOf(sourceFacts(sweep.sourceId, view, env)) ?? TONE_LOOKS[sweep.tone];
  const cues: FxCue[] = [];
  const fraction = new Map<string, number>();
  for (const side of sweep.sides) {
    const order = sweepOrder(entry, view, side);
    if (order.length === 0) continue;
    const first = order[0];
    const last = order[order.length - 1];
    if (first === undefined || last === undefined) continue;
    cues.push({
      kind: "fog",
      tone: sweep.tone,
      from: idAnchor(view, first) ?? viewportCenter(),
      to: idAnchor(view, last) ?? viewportCenter(),
      tint: look.tint,
      icon: look.icon,
      delayMs: 0,
      durationMs: D + FX_FOG_TAIL_MS,
    });
    order.forEach((id, k) => fraction.set(id, order.length > 1 ? k / (order.length - 1) : 0.5));
  }
  for (const event of entry.events) {
    if (event.type !== "damage" && event.type !== "healed" && event.type !== "divineShieldLost" && event.type !== "damageAbsorbed") {
      continue;
    }
    const target = event.type === "divineShieldLost" ? event.instanceId : event.targetId;
    const at = idAnchor(view, target);
    if (at === null) continue;
    const hit = Math.round(D * (FX_FOG_HIT_FROM + (FX_FOG_HIT_TO - FX_FOG_HIT_FROM) * (fraction.get(target) ?? 0.5)));
    if (event.type === "divineShieldLost") {
      cues.push(ring(D, "gold", at, hit), burst(i, "shard", at, "ring", hit, "shieldShard"));
    } else if (event.type === "damageAbsorbed") {
      // R1363: the fog reaches a unit whose Armor takes its hit whole, and the shield blooms there.
      cues.push(...shieldCues("full", at, D, hit, i));
    } else {
      cues.push(burst(i, look.preset, at, "area", hit, "fogHit"));
      if (event.amount > 0) cues.push(splat(D, event.type === "damage" ? "damage" : "heal", event.amount, at, hit));
      if (event.type === "damage" && armorTookHalf(event.absorbed, event.amount)) cues.push(...shieldCues("small", at, D, hit, i));
    }
  }
  return cues;
}

const RECIPES: { readonly [R in FxRecipe]: Recipe } = {
  cast,
  summon,
  impact,
  drain,
  heal,
  shieldBreak,
  death,
  void: exile,
  bounce,
  burn,
  discard,
  draw,
  handGlint,
  shuffle,
  buff,
  keyword,
  counter,
  glint,
  radiant,
  smoke,
  fuse,
  mindControl,
  lock,
  trap,
  lunge,
  fizzle,
  mana,
  banner: turnBanner,
  fatigue,
  overflow,
  // R436: the slot-machine reveal of what Call to Chaos rolled (chaos.ts).
  chaos: (event, p) => chaosCues(event, p.D, p.env.intensity),
  // R437: a mark branded onto its card in the mark's colours (brand.ts).
  brand: (event, p) => brandCues(event, anchor(p.tgt), p.D, p.env.intensity),
  rewind,
  // R1363: a hit the Armor took whole, a full shield over the target (shield.ts).
  armor,
};

/* ------------------------------------------------------------------------------------------- *
 * Public planners (S6)
 * ------------------------------------------------------------------------------------------- */

/**
 * Plans every event of one entry: each event's row recipe, in event order, concatenated. R502: while
 * a card with a signature recipe is resolving (`CARD_FX`, cardFx.ts), that recipe may claim an event
 * of its own resolution first, adding to the row's cues or replacing them.
 */
export function planFx(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  if (!(env.intensity > 0)) return [];
  // Issue #124: a sweep plays its fog and its timed hits, and a whole-pile impact its wave, instead
  // of one recipe per event; a cast plays its card's cues as usual, plus the flare at its caster.
  if (entry.sweep !== undefined) return sweepCues(entry, view, env);
  if (entry.zone !== undefined) return zoneCues(entry, view, env);
  const cues: FxCue[] = [];
  if (entry.cast !== undefined) cues.push(...castCues(entry, view, env));
  for (const event of entry.events) {
    const signature = planCardFx(event, { entry, view, env, D: entry.durationMs });
    if (signature !== null) {
      cues.push(...signature.cues);
      if (signature.row === "replace") continue;
    }
    const recipe = ANIMATIONS[event.type].fx?.recipe;
    if (recipe === undefined) continue;
    const tgt = targetFor(event, view);
    if (tgt === null) continue;
    cues.push(...RECIPES[recipe](event, { entry, view, env, D: entry.durationMs, tgt }));
  }
  return cues;
}

/** The game-over sequence for a view with a result; [] when view.result is null. */
export function planResult(view: PlayerView, env: Pick<FxPlanEnv, "intensity">): FxCue[] {
  const i = env.intensity;
  if (!(i > 0) || view.result === null) return [];
  const you = anchor(testid.hero("you"));
  const opponent = anchor(testid.hero("opponent"));
  const winner = view.result.winner;

  if (winner === "draw") {
    return [
      resultCue("draw", FX_TEXT.draw),
      burst(i, "dust", you, "area", 0, "resultDust"),
      burst(i, "dust", opponent, "area", 0, "resultDust"),
    ];
  }

  const cues: FxCue[] = [];
  if (winner === view.viewer) {
    const rayCue: FxRaysCue = { kind: "rays", tone: "victory", at: viewportCenter(), delayMs: 0, durationMs: FX_RESULT_MS };
    cues.push(
      resultCue("victory", FX_TEXT.victory),
      rayCue,
      burst(i, "confetti", viewportCenter(), "area", 0, "resultConfetti"),
      { kind: "crack", at: opponent, delayMs: 0, durationMs: RESULT_TUNING.crackMs },
      burst(i, "shard", opponent, "ring", RESULT_TUNING.impactDelayMs, "resultShard"),
      burst(i, "smoke", opponent, "area", RESULT_TUNING.smokeDelayMs, "resultSmoke"),
    );
  } else {
    cues.push(
      resultCue("defeat", FX_TEXT.defeat),
      { kind: "crack", at: you, delayMs: 0, durationMs: RESULT_TUNING.crackMs },
      burst(i, "shard", you, "ring", RESULT_TUNING.impactDelayMs, "resultShard"),
      burst(i, "smoke", you, "area", RESULT_TUNING.smokeDelayMs, "resultSmoke"),
      burst(i, "ember", you, "area", RESULT_TUNING.smokeDelayMs, "resultEmber"),
    );
  }
  pushShake(cues, i, FX_RESULT_TRAUMA, RESULT_TUNING.impactDelayMs);
  return cues;
}

/**
 * The killing blow, replayed ahead of the game-over sequence.
 *
 * A result settles the runner at once (Game.tsx drains it the moment a finished view arrives), so the
 * entry that dealt the lethal damage is cleared before it ever draws: without this, the last hit of the
 * match (the lunge's impact, the Fireball, the splat on the hero) would never show. `entries` are the
 * ones the drain cut short, in order. This takes the last of them whose `damage` or `healthLost` lands
 * on a losing hero and plans it again, given its own duration but never more than
 * FX_LETHAL_LEAD_MAX_MS, and reports that duration as `leadMs`, the beat the result waits for.
 *
 * Pure, like `planFx`: the caller remembers the drained entries' events in `env.memory` first, so a
 * spell's caster is known. Nothing is replayed when the entries were planned for another seat (a
 * hot-seat hand-over), because their `hero-you` would be the other hero now.
 */
export function planLethal(
  entries: readonly AnimationEntry[],
  view: PlayerView,
  env: FxPlanEnv,
): { cues: FxCue[]; leadMs: number } {
  const none = { cues: [], leadMs: 0 };
  if (!(env.intensity > 0) || view.result === null) return none;
  const winner = view.result.winner;
  const losers: readonly Side[] = winner === "draw" ? SIDES : [winner === view.viewer ? "opponent" : "you"];
  const heroes = new Set(losers.map((side) => testid.hero(side)));
  for (let k = entries.length - 1; k >= 0; k -= 1) {
    const entry = entries[k];
    if (entry === undefined || entry.view.viewer !== view.viewer) continue;
    const lethal = entry.events.some((event) => {
      if (event.type !== "damage" && event.type !== "healthLost") return false;
      const tgt = targetFor(event, entry.view);
      return tgt !== null && heroes.has(tgt);
    });
    if (!lethal) continue;
    // R318: a fatigue hit comes out of an empty library, and the entry just before it says so. A
    // lethal one is replayed with it, the two sharing the same lead, so the killing blow is not a hit
    // from nowhere; the board's "Fatigue N" never mounts under a finished view.
    const before = entries[k - 1];
    const fatigue =
      before !== undefined &&
      before.view.viewer === view.viewer &&
      before.events.some((event) => event.type === "fatigue" && heroes.has(testid.hero(sideOf(before.view, event.player))));
    if (fatigue) {
      const leadMs = Math.min(before.durationMs + entry.durationMs, FX_LETHAL_LEAD_MAX_MS);
      const emptyMs = Math.round((leadMs * before.durationMs) / (before.durationMs + entry.durationMs));
      const hitMs = leadMs - emptyMs;
      return {
        cues: [
          ...planFx({ ...before, durationMs: emptyMs }, before.view, env),
          ...delayCues(planFx({ ...entry, durationMs: hitMs }, entry.view, env), emptyMs),
        ],
        leadMs,
      };
    }
    const leadMs = Math.min(entry.durationMs, FX_LETHAL_LEAD_MAX_MS);
    return { cues: planFx({ ...entry, durationMs: leadMs }, entry.view, env), leadMs };
  }
  return none;
}

/** The same cues, each `ms` later. */
export function delayCues(cues: readonly FxCue[], ms: number): FxCue[] {
  return ms > 0 ? cues.map((cue) => ({ ...cue, delayMs: cue.delayMs + ms })) : [...cues];
}

function resultCue(outcome: FxOutcome, text: string): FxCue {
  return { kind: "result", outcome, text, delayMs: 0, durationMs: FX_RESULT_MS };
}

/** The "Your turn" banner on a hot-seat hand-over; [] unless active === viewer, phase ≠ "mulligan", no result. */
export function planHandover(view: PlayerView, env: Pick<FxPlanEnv, "intensity">): FxCue[] {
  if (!(env.intensity > 0)) return [];
  if (view.active !== view.viewer || view.phase === "mulligan" || view.result !== null) return [];
  const rayCue: FxRaysCue = {
    kind: "rays",
    tone: "victory",
    at: viewportCenter(),
    delayMs: 0,
    durationMs: FX_HANDOVER_BANNER_MS,
  };
  return [
    { kind: "banner", text: FX_TEXT.yourTurn, tone: "you", delayMs: 0, durationMs: FX_HANDOVER_BANNER_MS },
    rayCue,
  ];
}
