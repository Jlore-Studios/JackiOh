// R2789000: the combat keyword beats (docs/polish/9-card-animations.md, "Keyword beats"). A beat is
// the smallest tier of #2782: it decorates the entry of the event it keys on, holds nothing and ends
// inside it. Cleave, Trample, Pierce, First Strike, Lifesteal and Poisonous key on the `via` the engine
// sets on an event (R2783001); Windfury, Flying and Ranged on an `attackDeclared` whose attacker the
// view shows with the keyword. R200: each beat is planned in the D left after its stagger, so every
// cue starts inside the entry and ends within FX_MAX_TAIL_MS of it. R201: D carries the effects speed,
// and the stagger scales with it. R202: a beat reads `via`, the keywords a view shows and the ids it
// shows, never a defId; a hidden source has no `via` and no beat. Built from the motif helpers
// (build.ts) and the shared callout (callout.ts), over cues that exist already.
//
// Issue #2790: the defensive and life keywords' beats, on the same machinery and the same caps.
// Divine Shield and Armor fold into the `ward` motif (R2784000): the row draws the gold ring, shards
// and shield flash, the beat only the callout (and Armor's grey absorbed-amount splat, R1361).
// Indestructible keys on the absorption R1361 reports when Armor takes a hit whole on its holder (a
// hit meeting Indestructible itself is stopped unreported, so no `damage` key could fire); Immutable
// and Immune to Spells key on the grant that raises their shell and ward (a refused change emits
// nothing, and a Spell can never name an Immune unit, so no bounce or fizzle key could fire); Can't
// attack keys on a forced declaration naming its holder (a declared one is refused with no event, a
// forced attack skips the validator, R53). Brittle, Taunt and Can't Defend read the keywords the view
// shows; Reborn keys on the `via` the engine sets on its `summoned` (R2783001).

import { hasKeyword, type GameEvent, type KeywordKind, type PlayerView, type UnitView } from "@jackioh/shared";

import { animationRow, HIDDEN_ID, locateInstance, targetFor, type AnimationEntry } from "../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst, type BurstTuning } from "../build.ts";
import {
  FX_BEAT_STAGGER_MAX,
  FX_BEAT_STAGGER_MS,
  FX_BEAT_TRAUMA,
  FX_CALLOUTS_PER_HIT,
  FX_CRACK_TAIL_MS,
  FX_FIRST_STRIKE_BREAK_AT,
  FX_FIRST_STRIKE_BREAK_REACH,
  FX_PIERCE_REACH,
  FX_SPLAT_HOLD_MS,
} from "../constants.ts";
import { FLAVOR_LOOKS, type FlavorLook } from "../flavors.ts";
import { shieldCues } from "../shield.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxShotCue } from "../types.ts";
import { flies, hit, hitAnchor, MOTIF_FOOT, MOTIF_TUNING, motifBurst, motifRing, motifShot, placeOf, timesOf, type CueSpec } from "./build.ts";
import { calloutCue } from "./callout.ts";

/** The keywords with a beat, in the order a hit's beats play. */
export const FX_BEAT_KEYWORDS = ["First Strike", "Cleave", "Trample", "Pierce", "Lifesteal", "Poisonous", "Windfury", "Flying", "Ranged"] as const;

/** Issue #2790: the defensive and life keywords with a beat, in the order a hit's beats play them after the combat ones. */
export const FX_DEFENSIVE_BEAT_KEYWORDS = [
  "Divine Shield",
  "Armor",
  "Indestructible",
  "Immutable",
  "Reborn",
  "Brittle",
  "Immune to Spells",
  "Taunt",
  "Can't Defend",
  "Can't attack",
] as const;

type BeatKeyword = (typeof FX_BEAT_KEYWORDS)[number] | (typeof FX_DEFENSIVE_BEAT_KEYWORDS)[number];

/** The beats' own particle counts at scale 1 (rule 9); the landing bursts reuse MOTIF_TUNING's. */
const BEAT_TUNING = {
  glint: { count: 10, power: 0.7 },
  fizzle: { count: 10, power: 0.7 },
} as const satisfies Record<string, BurstTuning>;

/** Lifesteal's red motes: the blood preset, thrown as a bolt. */
const LIFESTEAL_LOOK: FlavorLook = {
  colors: ["#ffd6dc", "#e8283f", "#7a0a1c"],
  particle: "blood",
  projectile: { motion: "bolt", trail: "blood" },
};

/** The middle of a card, across or down. */
const MID = 0.5;

/** One keyword's beat on one event: where its callout goes and, planned in the D it has left, what it draws. */
type Beat = {
  keyword: BeatKeyword;
  event: GameEvent;
  /** The unit its callout pops over; null draws none. */
  holder: FxAnchor | null;
  /** Whether it draws the event's row in place of the row recipe (cues.ts). */
  replaces: boolean;
  plan: (D: number) => FxCue[];
};

type Ctx = { view: PlayerView; env: FxPlanEnv; entry: AnimationEntry; history: readonly GameEvent[] };

type HitEvent = Extract<GameEvent, { type: "damage" | "damageAbsorbed" }>;

const isHit = (event: GameEvent): event is HitEvent => event.type === "damage" || event.type === "damageAbsorbed";

/** The events before this entry's: the memory's recent ones, less the entry's own when it remembered them last. */
function priorOf(entry: AnimationEntry, env: FxPlanEnv): readonly GameEvent[] {
  const recent = env.memory.recent();
  const own = entry.events.length;
  const tail = recent.slice(recent.length - own);
  const remembered = own > 0 && tail.length === own && tail.every((event, k) => event === entry.events[k]);
  return remembered ? recent.slice(0, recent.length - own) : recent;
}

/** The unit `view` shows with this id, on the field or standing in a face-up backrow zone (as stage.ts's `rangedLook`). */
function unitOf(view: PlayerView, id: string): UnitView | null {
  if (id === HIDDEN_ID) return null;
  for (const side of [view.you, view.opponent]) {
    const standing = side.backrow.map((slot) => (slot !== null && slot.faceDown === false ? slot.unit : undefined));
    for (const unit of [...side.units, ...standing]) {
      if (unit !== null && unit !== undefined && unit.instanceId === id) return unit;
    }
  }
  return null;
}

/** Where `id` is drawn, or null for a missing or hidden one. */
function anchorOf(view: PlayerView, id: string | null): FxAnchor | null {
  return id === null || id === HIDDEN_ID ? null : hitAnchor(view, id);
}

const looking = (look: FlavorLook, env: FxPlanEnv, row: CueSpec["row"]): CueSpec => ({ look, scale: env.intensity, row });

/** Where a combat's extra blow starts: the target the source declared its attack on (while it is drawn), else the source. */
function blowFrom(event: HitEvent, ctx: Ctx, holder: FxAnchor | null): FxAnchor | null {
  const declared = event.sourceId === null ? undefined : ctx.env.memory.attacksThisTurn(event.sourceId).at(-1);
  return (declared === undefined ? null : anchorOf(ctx.view, declared)) ?? holder;
}

/** Cleave: the blow carries on as a wide arc across the lanes beside the target, and each neighbour takes a smaller hit. */
function cleaveBeat(event: HitEvent, ctx: Ctx): Beat[] {
  const to = anchorOf(ctx.view, event.targetId);
  if (to === null) return [];
  const holder = anchorOf(ctx.view, event.sourceId);
  const from = blowFrom(event, ctx, holder);
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "replace");
  return [
    {
      keyword: "Cleave",
      event,
      holder,
      replaces: true,
      plan(D) {
        const t = timesOf(D);
        const cues: FxCue[] = [];
        if (flies(from, to)) {
          const arc: FxShotCue = {
            kind: "shot",
            shot: { motion: "swipe" },
            colors: spec.look.colors,
            preset: spec.look.particle,
            from,
            to,
            delayMs: t.T,
            flightMs: t.L - t.T,
            density: spec.scale,
          };
          cues.push(arc);
        }
        cues.push(...hit(spec, event, to, D, t.L, MOTIF_TUNING.small));
        if (event.type === "damageAbsorbed") cues.push(...shieldCues("full", to, D, t.L, ctx.env.intensity));
        cues.push(...shakeCues(spec.scale, FX_BEAT_TRAUMA, t.L));
        return cues;
      },
    },
  ];
}

/** Trample: the overflow carries on past the dead unit as a bolt to the hero (or the next unit), and lands a second splat there. */
function trampleBeat(event: HitEvent, ctx: Ctx): Beat[] {
  const to = anchorOf(ctx.view, event.targetId);
  if (to === null || event.type !== "damage") return [];
  const holder = anchorOf(ctx.view, event.sourceId);
  const from = blowFrom(event, ctx, holder);
  const spec = looking(FLAVOR_LOOKS.Brute, ctx.env, "replace");
  return [
    {
      keyword: "Trample",
      event,
      holder,
      replaces: true,
      plan(D) {
        const t = timesOf(D);
        const cues: FxCue[] = flies(from, to) ? [motifShot(spec, "bolt", from, to, t.T, t.L - t.T)] : [];
        cues.push(...hit(spec, event, to, D, t.L), ...shakeCues(spec.scale, FX_BEAT_TRAUMA, t.L));
        return cues;
      },
    },
  ];
}

/** Pierce: a spear line drawn through the defender, from the side the blow comes from to the far side. */
function pierceBeat(event: HitEvent, ctx: Ctx): Beat[] {
  const to = anchorOf(ctx.view, event.targetId);
  const place = placeOf(ctx.view, event.targetId);
  if (to === null || to.kind !== "testid" || place === null || event.type !== "damage") return [];
  const spec = looking(FLAVOR_LOOKS.Tech, ctx.env, "keep");
  // The blow comes from the viewer's side: up through an opponent's unit, down through the viewer's own.
  const near = tid(to.testid, { x: MID, y: 1 + FX_PIERCE_REACH });
  const far = tid(to.testid, { x: MID, y: -FX_PIERCE_REACH });
  const [start, end] = place.side === "opponent" ? [near, far] : [far, near];
  return [
    {
      keyword: "Pierce",
      event,
      holder: anchorOf(ctx.view, event.sourceId),
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [motifShot(spec, "beam", start, end, t.T, t.L - t.T), motifBurst(spec, to, "point", t.L, MOTIF_TUNING.small)];
      },
    },
  ];
}

/** First Strike's strike: a quick-draw glint on the attacker, who hits first and alone. */
function strikeBeat(event: HitEvent, ctx: Ctx): Beat[] {
  const holder = anchorOf(ctx.view, event.sourceId);
  if (holder === null) return [];
  const spec = looking(FLAVOR_LOOKS.Holy, ctx.env, "keep");
  return [
    {
      keyword: "First Strike",
      event,
      holder,
      replaces: false,
      plan(D) {
        return [{ kind: "sheen", at: holder, delayMs: 0, durationMs: timesOf(D).L }, motifBurst(spec, holder, "point", 0, BEAT_TUNING.glint)];
      },
    },
  ];
}

/**
 * First Strike's survivor: a counter-blow that follows a First Strike's hit, from the unit it struck to
 * the one that struck, lands after the strike's beat (at the land phase) rather than at once.
 */
function counterBeat(event: HitEvent, ctx: Ctx): Beat[] {
  if (event.type !== "damage" || event.sourceId === null) return [];
  const strike = ctx.history.findLast(isHit);
  if (strike === undefined || strike.via !== "First Strike" || strike.sourceId !== event.targetId || strike.targetId !== event.sourceId) return [];
  const to = anchorOf(ctx.view, event.targetId);
  if (to === null) return [];
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "replace");
  return [
    {
      keyword: "First Strike",
      event,
      holder: null,
      replaces: true,
      plan(D) {
        const t = timesOf(D);
        return [...hit(spec, event, to, D, t.L), ...shakeCues(spec.scale, FX_BEAT_TRAUMA, t.L)];
      },
    },
  ];
}

function hitBeats(event: HitEvent, ctx: Ctx): Beat[] {
  switch (event.via) {
    case "First Strike":
      return event.combat ? strikeBeat(event, ctx) : [];
    case "Cleave":
      return event.combat ? cleaveBeat(event, ctx) : [];
    case "Trample":
      return event.combat ? trampleBeat(event, ctx) : [];
    case "Pierce":
      return pierceBeat(event, ctx);
    case undefined:
      return event.combat ? counterBeat(event, ctx) : [];
    default:
      return [];
  }
}

/** Lifesteal: red motes stream from the damage back to the controller's hero and land with the heal's splat. */
function lifestealBeat(event: Extract<GameEvent, { type: "healed" }>, ctx: Ctx): Beat[] {
  const strike = ctx.history.findLast((past): past is Extract<GameEvent, { type: "damage" }> => past.type === "damage");
  const to = anchorOf(ctx.view, event.targetId);
  const from = strike === undefined ? null : anchorOf(ctx.view, strike.targetId);
  const holder = strike === undefined ? null : anchorOf(ctx.view, strike.sourceId);
  if (to === null || from === null) return [{ keyword: "Lifesteal", event, holder, replaces: false, plan: () => [] }];
  const spec = looking(LIFESTEAL_LOOK, ctx.env, "replace");
  return [
    {
      keyword: "Lifesteal",
      event,
      holder,
      replaces: true,
      plan(D) {
        const t = timesOf(D);
        return [
          { kind: "projectile", preset: "blood", from, to, delayMs: 0, flightMs: t.L, density: spec.scale },
          motifBurst(spec, to, "area", t.L, MOTIF_TUNING.small),
          { kind: "splat", tone: "heal", amount: event.amount, at: to, delayMs: t.L, durationMs: D - t.L + FX_SPLAT_HOLD_MS },
        ];
      },
    },
  ];
}

/** What a unit's death shows of the keyword behind it. */
function deathBeats(event: Extract<GameEvent, { type: "destroyed" }>, ctx: Ctx): Beat[] {
  const killer = anchorOf(ctx.view, event.killerId);
  switch (event.via) {
    case "Poisonous":
      // The poison itself is the death recipe's (cues.ts); the beat adds the callout over the poisoner.
      return [{ keyword: "Poisonous", event, holder: killer ?? anchorOf(ctx.view, event.instanceId), replaces: false, plan: () => [] }];
    case "First Strike":
      return breakOffBeat(event, ctx, killer);
    default:
      return [];
  }
}

/** First Strike's dying defender: its counter-blow starts toward the attacker and breaks off in smoke, and never lands. */
function breakOffBeat(event: Extract<GameEvent, { type: "destroyed" }>, ctx: Ctx, holder: FxAnchor | null): Beat[] {
  const card = locateInstance(ctx.view, event.instanceId);
  const place = placeOf(ctx.view, event.instanceId);
  if (card === null || !card.startsWith("card-") || place === null) return [];
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "keep");
  // The attacker stands on the other side of the board: below an opponent's unit, above the viewer's.
  const broken = tid(card, { x: MID, y: place.side === "opponent" ? 1 + FX_FIRST_STRIKE_BREAK_REACH : -FX_FIRST_STRIKE_BREAK_REACH });
  return [
    {
      keyword: "First Strike",
      event,
      holder,
      replaces: false,
      plan(D) {
        const flightMs = frac(FX_FIRST_STRIKE_BREAK_AT, D);
        return [
          { kind: "projectile", preset: "spark", from: tid(card), to: broken, delayMs: 0, flightMs, density: spec.scale },
          tunedBurst(spec.scale, "smoke", broken, "point", flightMs, BEAT_TUNING.fizzle),
        ];
      },
    },
  ];
}

/** Divine Shield: the `ward` row's gold ring and shards draw the pop, so the beat is the callout alone: the hit did nothing. */
function divineShieldBeat(event: Extract<GameEvent, { type: "divineShieldLost" }>, ctx: Ctx): Beat[] {
  const holder = anchorOf(ctx.view, event.instanceId);
  if (holder === null) return [];
  return [{ keyword: "Divine Shield", event, holder, replaces: false, plan: () => [] }];
}

/**
 * Armor (R1361): the `ward` row draws the shield flash, the beat the absorbed amount as a grey
 * splat chipping the plate. A routed hit's own combat beat draws it instead (Cleave keeps its
 * full shield), so this keys on an absorption with no `via`.
 */
function armorBeat(event: Extract<GameEvent, { type: "damageAbsorbed" }>, ctx: Ctx): Beat[] {
  if (event.via !== undefined) return [];
  const holder = anchorOf(ctx.view, event.targetId);
  if (holder === null) return [];
  const spec = looking(FLAVOR_LOOKS.Holy, ctx.env, "keep");
  return [
    {
      keyword: "Armor",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          { kind: "splat", tone: "absorb", amount: event.absorbed, at: holder, delayMs: t.L, durationMs: D - t.L + FX_SPLAT_HOLD_MS },
          motifBurst(spec, holder, "ring", t.L, MOTIF_TUNING.small, "shard"),
        ];
      },
    },
  ];
}

/**
 * Indestructible (R1361): a hit its Armor takes whole, lethal and with no `destroyed` of the unit on
 * the way: the unit flashes white-hot and rings, and it does not die. A hit meeting Indestructible
 * itself is stopped unreported (§4.4 step 4, before any event), so a `damage` key could never fire;
 * the absorption its Armor reports is the reachable half. A routed hit's own combat beat draws it
 * instead, so this keys on an absorption with no `via`, as the Armor beat does. The ward row's flash
 * and the Armor beat's grey splat stay: the beat adds only the flash and the ring.
 */
function indestructibleBeat(event: Extract<GameEvent, { type: "damageAbsorbed" }>, ctx: Ctx): Beat[] {
  if (event.via !== undefined) return [];
  const unit = unitOf(ctx.view, event.targetId);
  const holder = anchorOf(ctx.view, event.targetId);
  if (unit === null || holder === null) return [];
  if (!hasKeyword(unit.keywords, "Indestructible") || event.absorbed < unit.health) return [];
  const destroyed = [...ctx.history, ...ctx.entry.events].some((past) => past.type === "destroyed" && past.instanceId === event.targetId);
  if (destroyed) return [];
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "keep");
  return [
    {
      keyword: "Indestructible",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          { kind: "sheen", at: holder, delayMs: 0, durationMs: t.L },
          motifBurst(spec, holder, "point", 0, MOTIF_TUNING.small),
          ringCue(D, "spark", holder, t.L),
          ...shakeCues(spec.scale, FX_BEAT_TRAUMA, t.L),
        ];
      },
    },
  ];
}

/**
 * Immutable (R23, R386): gaining the keyword snaps a glassy shell shut around the unit. A nerf,
 * tune-up, transform or number set aimed at it is refused with no event, so no bounce key could
 * fire; the grant is the reachable half. Keyed on the granted kind, as Taunt's arrival is: the
 * pre-action view has no keyword to read yet.
 */
function immutableBeat(event: Extract<GameEvent, { type: "keywordGranted" }>, ctx: Ctx): Beat[] {
  if (event.keyword.kind !== "Immutable" || event.lost === true) return [];
  const holder = anchorOf(ctx.view, event.instanceId);
  if (holder === null) return [];
  const spec = looking(FLAVOR_LOOKS.Tech, ctx.env, "keep");
  return [
    {
      keyword: "Immutable",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [ringCue(D, "arcane", holder, t.T), motifBurst(spec, holder, "point", t.L, MOTIF_TUNING.small)];
      },
    },
  ];
}

/** Reborn: on a `summoned` with `via: Reborn` (R2783001), the unit rises from ashes in its own lane. */
function rebornBeat(event: Extract<GameEvent, { type: "summoned" }>, ctx: Ctx): Beat[] {
  if (event.via !== "Reborn") return [];
  const zone = targetFor(event, ctx.view);
  const holder = anchorOf(ctx.view, event.instanceId);
  if (zone === null || holder === null) return [];
  const spec = looking(FLAVOR_LOOKS.Spirit, ctx.env, "keep");
  return [
    {
      keyword: "Reborn",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          { kind: "ghost", from: tid(zone, MOTIF_FOOT), to: holder, delayMs: t.T, durationMs: t.L - t.T },
          motifBurst(spec, holder, "area", t.L, MOTIF_TUNING.land),
          motifRing(spec, D, holder, t.L),
        ];
      },
    },
  ];
}

/** Brittle: hairline cracks grow on the unit as it takes damage. */
function brittleCrackBeat(event: Extract<GameEvent, { type: "damage" }>, ctx: Ctx): Beat[] {
  const unit = unitOf(ctx.view, event.targetId);
  const holder = anchorOf(ctx.view, event.targetId);
  if (unit === null || holder === null) return [];
  if (!hasKeyword(unit.keywords, "Brittle")) return [];
  return [
    {
      keyword: "Brittle",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [{ kind: "crack", at: holder, delayMs: t.L, durationMs: D - t.L + FX_CRACK_TAIL_MS }];
      },
    },
  ];
}

/**
 * Brittle: the shatter on its death (R385). Only Brittle crumbles, so the event alone keys it;
 * the holder is the row's target, so a pile stands in off the board. The death row draws the
 * shatter itself, so the beat is the callout alone.
 */
function brittleShatterBeat(event: Extract<GameEvent, { type: "crumbled" }>, ctx: Ctx): Beat[] {
  const tgt = targetFor(event, ctx.view);
  if (tgt === null) return [];
  return [{ keyword: "Brittle", event, holder: tid(tgt), replaces: false, plan: () => [] }];
}

/**
 * Immune to Spells (R81): gaining the keyword raises a rune ward around the unit, in the `negate`
 * motif's ring-and-smoke vocabulary. A Spell can never name the unit (no declaration offers it, and
 * every resolving effect passes it by in silence), so no fizzle key could fire; the grant is the
 * reachable half. Keyed on the granted kind, as Taunt's arrival is.
 */
function immuneBeat(event: Extract<GameEvent, { type: "keywordGranted" }>, ctx: Ctx): Beat[] {
  if (event.keyword.kind !== "Immune to Spells" || event.lost === true) return [];
  const holder = anchorOf(ctx.view, event.instanceId);
  if (holder === null) return [];
  const spec = looking(FLAVOR_LOOKS.Tech, ctx.env, "keep");
  return [
    {
      keyword: "Immune to Spells",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [motifRing(spec, D, holder, t.G), motifBurst(spec, holder, "point", t.L, MOTIF_TUNING.linger, "smoke")];
      },
    },
  ];
}

/** Taunt arriving: a dust-ring shield wall over the unit, beside the bless row's arrows. */
function tauntArrivalBeat(event: Extract<GameEvent, { type: "keywordGranted" }>, ctx: Ctx): Beat[] {
  if (event.keyword.kind !== "Taunt" || event.lost === true) return [];
  const holder = anchorOf(ctx.view, event.instanceId);
  if (holder === null) return [];
  return [
    {
      keyword: "Taunt",
      event,
      holder,
      replaces: false,
      plan(D) {
        return [ringCue(D, "dust", holder, timesOf(D).T)];
      },
    },
  ];
}

/** A chain clink over the unit the game stops from the action it cannot take. */
function chainBeat(keyword: "Can't attack" | "Can't Defend", event: GameEvent, ctx: Ctx, holder: FxAnchor): Beat[] {
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "keep");
  return [
    {
      keyword,
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [ringCue(D, "dust", holder, t.T), motifBurst(spec, holder, "ring", t.L, MOTIF_TUNING.small, "shard")];
      },
    },
  ];
}

/** Can't Defend: the unit is skipped by the position switch it cannot make. */
function cantDefendBeat(event: Extract<GameEvent, { type: "positionSwitched" }>, ctx: Ctx): Beat[] {
  const unit = unitOf(ctx.view, event.instanceId);
  const holder = anchorOf(ctx.view, event.instanceId);
  if (unit === null || holder === null) return [];
  if (!hasKeyword(unit.keywords, "Can't Defend")) return [];
  return chainBeat("Can't Defend", event, ctx, holder);
}

/** Windfury, Flying and Ranged on an attack: read off the keywords the view shows on its attacker. */
function attackBeats(event: Extract<GameEvent, { type: "attackDeclared" }>, ctx: Ctx): Beat[] {
  const unit = unitOf(ctx.view, event.attackerId);
  const holder = anchorOf(ctx.view, event.attackerId);
  if (unit === null || holder === null) return [];
  const beats: Beat[] = [];
  const attacks = ctx.env.memory.attacksThisTurn(event.attackerId);
  if (hasKeyword(unit.keywords, "Windfury") && attacks.length >= 2) {
    // An after-image of the first attack's flight as the second begins.
    const first = anchorOf(ctx.view, attacks[0] ?? null) ?? anchorOf(ctx.view, event.targetId);
    const spec = looking(FLAVOR_LOOKS.Spirit, ctx.env, "keep");
    if (first !== null) {
      beats.push({
        keyword: "Windfury",
        event,
        holder,
        replaces: false,
        plan: (D) => [{ kind: "ghost", from: holder, to: first, delayMs: 0, durationMs: timesOf(D).L }, motifBurst(spec, holder, "ring", 0, MOTIF_TUNING.small)],
      });
    }
  }
  for (const keyword of ["Flying", "Ranged"] as const satisfies readonly KeywordKind[]) {
    if (hasKeyword(unit.keywords, keyword)) beats.push({ keyword, event, holder, replaces: false, plan: () => [] });
  }
  // Issue #2790: Taunt forced the attack onto its wall: a brief pull of the wall toward the attacker.
  const wall = unitOf(ctx.view, event.targetId);
  const wallAt = anchorOf(ctx.view, event.targetId);
  if (event.forced === true && wall !== null && wallAt !== null && hasKeyword(wall.keywords, "Taunt")) {
    beats.push({
      keyword: "Taunt",
      event,
      holder: wallAt,
      replaces: false,
      plan: (D) => {
        const t = timesOf(D);
        return [{ kind: "ghost", from: wallAt, to: holder, delayMs: t.T, durationMs: t.L - t.T }];
      },
    });
  }
  // Issue #2790: Can't attack. A declared attack by the holder is refused with no event, but a
  // forced attack skips the validator (R53), so a forced declaration naming it is the reachable
  // "chosen for the action it can't take": chains clink over it. The `forced` gate matters: a
  // non-forced declaration by it is what the engine refuses, so that stream cannot occur live.
  if (event.forced === true && hasKeyword(unit.keywords, "Can't attack")) {
    beats.push(...chainBeat("Can't attack", event, ctx, holder));
  }
  return beats;
}

function beatsAt(event: GameEvent, ctx: Ctx): Beat[] {
  switch (event.type) {
    case "damage":
      return [...hitBeats(event, ctx), ...brittleCrackBeat(event, ctx)];
    case "damageAbsorbed":
      return [...hitBeats(event, ctx), ...armorBeat(event, ctx), ...indestructibleBeat(event, ctx)];
    case "healed":
      return event.via === "Lifesteal" ? lifestealBeat(event, ctx) : [];
    case "destroyed":
      return deathBeats(event, ctx);
    case "attackDeclared":
      return attackBeats(event, ctx);
    case "divineShieldLost":
      return divineShieldBeat(event, ctx);
    case "summoned":
      return rebornBeat(event, ctx);
    case "crumbled":
      return brittleShatterBeat(event, ctx);
    case "keywordGranted":
      return [...tauntArrivalBeat(event, ctx), ...immutableBeat(event, ctx), ...immuneBeat(event, ctx)];
    case "positionSwitched":
      return cantDefendBeat(event, ctx);
    default:
      return [];
  }
}

/** Every beat this entry's events key, in event order. */
function beatsOf(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): Beat[] {
  const prior = priorOf(entry, env);
  return entry.events.flatMap((event, k) => beatsAt(event, { view, env, entry, history: [...prior, ...entry.events.slice(0, k)] }));
}

const shifted = (cue: FxCue, ms: number): FxCue => (ms > 0 ? { ...cue, delayMs: cue.delayMs + ms } : cue);

/**
 * The cues of the entry's beats, and the events whose row recipe they draw in place of it. A hit's beats
 * start FX_BEAT_STAGGER_MS apart (scaled with the effects speed, at most FX_BEAT_STAGGER_MAX of D in),
 * and at most FX_CALLOUTS_PER_HIT of them pop a callout. Nothing at intensity 0 or in an entry of no duration.
 */
export function planBeats(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): { cues: FxCue[]; replaces: ReadonlySet<GameEvent> } {
  const cues: FxCue[] = [];
  const replaces = new Set<GameEvent>();
  const D = entry.durationMs;
  if (!(env.intensity > 0) || !(D > 0)) return { cues, replaces };
  const pace = D / (animationRow(entry.type)?.durationMs ?? D);
  let callouts = 0;
  beatsOf(entry, view, env).forEach((beat, k) => {
    const offset = Math.min(Math.round(k * FX_BEAT_STAGGER_MS * pace), Math.floor(FX_BEAT_STAGGER_MAX * D));
    if (beat.holder !== null && callouts < FX_CALLOUTS_PER_HIT) {
      cues.push(calloutCue(beat.keyword, beat.holder, offset, false));
      callouts += 1;
    }
    for (const cue of beat.plan(D - offset)) cues.push(shifted(cue, offset));
    if (beat.replaces) replaces.add(beat.event);
  });
  return { cues, replaces };
}

/** R2789000: under reduced motion, the entry's first FX_CALLOUTS_PER_HIT callouts as static icons, whatever its duration. */
export function planStillCallouts(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  if (!(env.intensity > 0)) return [];
  const holders = beatsOf(entry, view, env).flatMap((beat) => (beat.holder === null ? [] : [calloutCue(beat.keyword, beat.holder, 0, true)]));
  return holders.slice(0, FX_CALLOUTS_PER_HIT);
}
