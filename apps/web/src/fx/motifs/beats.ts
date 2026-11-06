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
// Divine Shield and Armor draw in the rows (R2803000 folds `ward`, R2784000): the row draws the gold
// ring, shards and shield flash, the beat only the callout (and Armor's grey absorbed-amount splat, R1361).
// Indestructible keys on the absorption R1361 reports when Armor takes a hit whole on its holder (a
// hit meeting Indestructible itself is stopped unreported, so no `damage` key could fire); Immutable
// and Immune to Spells key on the grant that raises their shell and ward (a refused change emits
// nothing, and a Spell can never name an Immune unit, so no bounce or fizzle key could fire); Can't
// attack keys on a forced declaration naming its holder (a declared one is refused with no event, a
// forced attack skips the validator, R53). Brittle, Taunt and Can't Defend read the keywords the view
// shows; Reborn keys on the `via` the engine sets on its `summoned` (R2783001).
//
// R2791000: the tempo keyword beats. Charge, Rush, Equip, Tradable, Stack, Spell Damage, Lucky, Deft,
// Flashback and Temporary key on `via` where one exists, else on the existing events plus the keywords
// and ids the view shows: a landing reads the played card's hand keywords (its delta, else its printed
// face), a Stack landing the pile it lands on, a spell's bonus the resolving play and the view's Spell
// Damage total, Lucky the batch's extra draws, Deft the turn's switches, Flashback the graveyard play in
// the history and Tradable the trade and shuffle before the draw. R200 and R201 as above; R202: never
// a hidden card, on either seat.

import { hasKeyword, type GameEvent, type Keyword, type KeywordKind, type PlayerId, type PlayerView, type UnitView } from "@jackioh/shared";

import { animTestid, animationRow, HIDDEN_ID, locateInstance, targetFor, type AnimationEntry } from "../../game/animations.ts";
import { temporaryLeft } from "../../cards/keywordVisuals.ts";
import { sideOf, testid, type Side } from "../../game/contract.ts";
import { CATALOG } from "../../wire/cards.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst, type BurstTuning } from "../build.ts";
import {
  FX_BEAT_STAGGER_MAX,
  FX_BEAT_STAGGER_MS,
  FX_BEAT_TRAUMA,
  FX_CALLOUTS_PER_HIT,
  FX_CENTER,
  FX_CHARGE_LINE_SPREAD,
  FX_CRACK_TAIL_MS,
  FX_DEFT_FEINT_STEP,
  FX_FIRST_STRIKE_BREAK_AT,
  FX_FIRST_STRIKE_BREAK_REACH,
  FX_PIERCE_REACH,
  FX_SPLAT_HOLD_MS,
  FX_STACK_LAND,
  FX_ZONE_COUNT_AT,
  FX_ZONE_TAIL_MS,
} from "../constants.ts";
import { FLAVOR_LOOKS, type FlavorLook } from "../flavors.ts";
import { ZONE_DEFAULT, ZONE_LOOKS } from "../looks.ts";
import { extraDraws, rollBatchOf, rollLandAt } from "../rolls.ts";
import { shieldCues } from "../shield.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxShotCue } from "../types.ts";
import { flies, hit, hitAnchor, isFirstEntry, MOTIF_FOOT, MOTIF_TUNING, motifBurst, motifRing, motifShot, placeOf, sourceOf, timesOf, type CueSpec } from "./build.ts";
import { calloutCue } from "./callout.ts";

/** #2437: the Temporary badge's glyph, an hourglass in text presentation (U+FE0E), as the face model draws it. */
const TEMPORARY_GLYPH = "\u231B\uFE0E";

/** The keywords with a beat: the nine combat ones in the order a hit's beats play, then R2791000's ten. */
export const FX_BEAT_KEYWORDS = [
  "First Strike",
  "Cleave",
  "Trample",
  "Pierce",
  "Lifesteal",
  "Poisonous",
  "Windfury",
  "Flying",
  "Ranged",
  "Charge",
  "Rush",
  "Equip",
  "Tradable",
  "Stack",
  "Spell Damage",
  "Lucky",
  "Deft",
  "Flashback",
  "Temporary",
] as const;

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

/**
 * R2791000: the keywords the hand card `instanceId` would carry onto the field: its delta keywords
 * where they differ from its printed face (B5 E38), else that face (the Radiant one when Radiant)
 * through the catalog or the view's definitions, as a motif reads its scale. Null when the hand is
 * a count, the card is gone or hidden, or no definition names it: unknown, never a guess.
 */
function handKeywordsOf(view: PlayerView, side: Side, instanceId: string): readonly Keyword[] | null {
  const hand = side === "you" ? view.you.hand : view.opponent.hand;
  if (!Array.isArray(hand)) return null;
  const card = hand.find((one) => one.instanceId === instanceId);
  if (card === undefined || card.defId === HIDDEN_ID) return null;
  if (card.keywords !== undefined) return card.keywords;
  const def = CATALOG[card.defId] ?? view.defs?.[card.defId];
  if (def === undefined) return null;
  return card.radiant ? def.radiant.keywords : def.base.keywords;
}

/** R2791000: `player`'s Spell Damage total as the view shows it: over its units and its face-up backrow carriers. */
function spellDamageTotal(view: PlayerView, player: PlayerId): number {
  const sv = sideOf(view, player) === "you" ? view.you : view.opponent;
  let total = 0;
  for (const unit of sv.units) {
    if (unit === null) continue;
    for (const keyword of unit.keywords) if (keyword.kind === "Spell Damage") total += keyword.n;
  }
  for (const slot of sv.backrow) {
    const carried = slot !== null && slot.faceDown === false ? slot.unit : undefined;
    if (carried === undefined || carried === null) continue;
    for (const keyword of carried.keywords) if (keyword.kind === "Spell Damage") total += keyword.n;
  }
  return total;
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
 * Whether the view shows `instanceId` as Brittle: a unit's keywords, or a backrow card's (a
 * Brittle Trap crumbles where it stands, and `unitOf` sees no backrow card). R202 draws nothing
 * beyond what the view shows, so a crumble the view does not show as Brittle draws no beat.
 */
function brittleShown(view: PlayerView, instanceId: string): boolean {
  const unit = unitOf(view, instanceId);
  if (unit !== null) return hasKeyword(unit.keywords, "Brittle");
  for (const side of [view.you, view.opponent]) {
    for (const slot of side.backrow) {
      if (slot !== null && slot.faceDown === false && slot.instanceId === instanceId) {
        return hasKeyword(slot.keywords ?? [], "Brittle");
      }
    }
  }
  return false;
}

/**
 * Brittle: the shatter on its death (R385). The engine only ever crumbles a Brittle count, but
 * the beat still keys on the keywords the view shows (R202), as the crack beat does; the holder
 * is the row's target, so a pile stands in off the board. The death row draws the shatter
 * itself, so the beat is the callout alone.
 */
function brittleShatterBeat(event: Extract<GameEvent, { type: "crumbled" }>, ctx: Ctx): Beat[] {
  if (!brittleShown(ctx.view, event.instanceId)) return [];
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
  beats.push(...dashBeats(event, ctx), ...deftBeat(event, ctx));
  return beats;
}

type SummonedEvent = Extract<GameEvent, { type: "summoned" }>;
type AttackEvent = Extract<GameEvent, { type: "attackDeclared" }>;

/** The first unit `foe` shows on the field, in lane order, or null. */
function firstFoeUnit(view: PlayerView, foe: Side): string | null {
  const units = foe === "you" ? view.you.units : view.opponent.units;
  for (const unit of units) if (unit !== null) return unit.instanceId;
  return null;
}

/**
 * R2791000: a Charge landing's speed lines and forward lean, shared with Rush: three bolts from
 * the hand region into the slot, a small landing, then a lean from the slot at the land. Charge
 * leans at the enemy hero; Rush at the first enemy unit the view shows, and lands without a lean
 * when none shows. Gated on the played card's hand keywords, so the other seat plans nothing.
 */
function chargeLandingBeat(kind: "Charge" | "Rush", event: SummonedEvent, ctx: Ctx): Beat[] {
  const side = sideOf(ctx.view, event.player);
  const keywords = handKeywordsOf(ctx.view, side, event.instanceId);
  if (keywords === null || !hasKeyword(keywords, kind)) return [];
  const holder = tid(testid.zone(side, event.row, event.lane));
  const handRegion = animTestid.hand(side);
  const foe: Side = side === "you" ? "opponent" : "you";
  const foeUnit = kind === "Charge" ? null : firstFoeUnit(ctx.view, foe);
  const leanTo = kind === "Charge" ? tid(testid.hero(foe)) : foeUnit === null ? null : tid(testid.card(foeUnit));
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "keep");
  return [
    {
      keyword: kind,
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        const cues: FxCue[] = [
          motifShot(spec, "bolt", tid(handRegion), holder, 0, t.L),
          motifShot(spec, "bolt", tid(handRegion, { x: MID - FX_CHARGE_LINE_SPREAD, y: MID }), holder, 0, t.L),
          motifShot(spec, "bolt", tid(handRegion, { x: MID + FX_CHARGE_LINE_SPREAD, y: MID }), holder, 0, t.L),
          motifBurst(spec, holder, "point", t.L, MOTIF_TUNING.small),
        ];
        if (leanTo !== null && flies(holder, leanTo)) {
          cues.push(motifShot(spec, "bolt", holder, leanTo, t.L, t.Li - t.L));
          cues.push(...shakeCues(spec.scale, FX_BEAT_TRAUMA, t.Li));
        }
        return cues;
      },
    },
  ];
}

/** R2791000: a Charge's and a Rush's dash on the first attack of the turn: a bolt into the target, a small landing and a shake. */
function dashBeats(event: AttackEvent, ctx: Ctx): Beat[] {
  const unit = unitOf(ctx.view, event.attackerId);
  const holder = anchorOf(ctx.view, event.attackerId);
  if (unit === null || holder === null) return [];
  if (ctx.env.memory.attacksThisTurn(event.attackerId).length !== 1) return [];
  const to = anchorOf(ctx.view, event.targetId);
  if (to === null) return [];
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "keep");
  const beats: Beat[] = [];
  for (const kind of ["Charge", "Rush"] as const) {
    if (!hasKeyword(unit.keywords, kind)) continue;
    beats.push({
      keyword: kind,
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          ...(flies(holder, to) ? [motifShot(spec, "bolt", holder, to, 0, t.L)] : []),
          motifBurst(spec, to, "point", t.L, MOTIF_TUNING.small),
          ...shakeCues(spec.scale, FX_BEAT_TRAUMA, t.L),
        ];
      },
    });
  }
  return beats;
}

/**
 * R2791000: Deft's feint before an attack that follows a same-turn switch: two Spirit bolts, out to
 * the sidestep and back, under a sheen. No event marks an aimed random cast apart, so the
 * switch-then-attack is the observable trigger.
 */
function deftBeat(event: AttackEvent, ctx: Ctx): Beat[] {
  const unit = unitOf(ctx.view, event.attackerId);
  const holder = anchorOf(ctx.view, event.attackerId);
  if (unit === null || holder === null || holder.kind !== "testid" || !hasKeyword(unit.keywords, "Deft")) return [];
  const sinceTurn = ctx.history.findLastIndex((past) => past.type === "turnStarted") + 1;
  const switched = ctx.history.slice(sinceTurn).some((past) => past.type === "positionSwitched" && past.instanceId === event.attackerId);
  if (!switched) return [];
  const spec = looking(FLAVOR_LOOKS.Spirit, ctx.env, "keep");
  const feint = tid(holder.testid, { x: MID + FX_DEFT_FEINT_STEP, y: MID });
  return [
    {
      keyword: "Deft",
      event,
      holder,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          motifShot(spec, "bolt", holder, feint, 0, t.T),
          motifShot(spec, "bolt", feint, holder, t.T, t.L - t.T),
          { kind: "sheen", at: holder, delayMs: 0, durationMs: t.L },
        ];
      },
    },
  ];
}

/**
 * R2791000: Equip's landing: the equipment flies from the hand region onto its host and snaps on,
 * with a gold ring. Gated on the kept result standing on the field while no ingredient does (a
 * fusion from hand, R1086), and on the top card's hand keywords showing Equip where they are known:
 * a fresh result plans nothing, and a hidden top still lands. The host's stat tick-up stays the
 * bless row's.
 */
function equipBeat(event: Extract<GameEvent, { type: "fused" }>, ctx: Ctx): Beat[] {
  // R2801000: a dead piece's copy fusing into its Starship is no Equip: its fuse run draws it (fx/starship.ts).
  if (ctx.entry.fuse !== undefined) return [];
  const at = locateInstance(ctx.view, event.resultInstanceId);
  if (at === null || !at.startsWith("card-")) return [];
  const place = placeOf(ctx.view, event.resultInstanceId);
  if (place === null) return [];
  const others = event.instanceIds.filter((id) => id !== event.resultInstanceId);
  const top = others[0];
  if (top === undefined) return [];
  if (
    others.some((id) => {
      const found = locateInstance(ctx.view, id);
      return found !== null && found.startsWith("card-");
    })
  ) {
    return [];
  }
  const keywords = handKeywordsOf(ctx.view, place.side, top);
  if (keywords !== null && !hasKeyword(keywords, "Equip")) return [];
  const host = tid(at);
  const spec = looking(FLAVOR_LOOKS.Physical, ctx.env, "keep");
  return [
    {
      keyword: "Equip",
      event,
      holder: host,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          { kind: "ghost", from: tid(animTestid.hand(place.side)), to: host, delayMs: 0, durationMs: t.L },
          ringCue(D, "gold", host, t.L),
          motifBurst(spec, host, "point", t.L, MOTIF_TUNING.small),
        ];
      },
    },
  ];
}

/** R2791000: Tradable's swirl back into the Deck: a card back from the hand region into the library. The shuffle row still draws its own. */
function tradableOutBeat(event: Extract<GameEvent, { type: "traded" }>, ctx: Ctx): Beat[] {
  const side = sideOf(ctx.view, event.player);
  const library = tid(animTestid.library(side));
  return [
    {
      keyword: "Tradable",
      event,
      holder: library,
      replaces: false,
      plan(D) {
        return [{ kind: "ghost", from: tid(animTestid.hand(side)), to: library, delayMs: 0, durationMs: timesOf(D).L }];
      },
    },
  ];
}

/**
 * R2791000: Tradable's drawn card swirling out: a sparkle over the hand region where the trade's
 * shuffle put it, only on the trade's own first draw. A Trade is one immediate action — the
 * `shuffledIn` after its `traded` and the `drawn` after that (R1838001) — so the two most recent
 * same-player trade events before this draw must be that `traded` then that `shuffledIn`, with no
 * draw of theirs between: anything looser fires on a later unrelated draw within memory. No
 * instance link is read: the trade's ids are hidden per zone (R202). No second ghost: the
 * `flight` motif owns the flight.
 */
function tradableInBeat(event: Extract<GameEvent, { type: "drawn" }>, ctx: Ctx): Beat[] {
  let last: GameEvent | undefined;
  let prev: GameEvent | undefined;
  for (let i = ctx.history.length - 1; i >= 0; i -= 1) {
    const past = ctx.history[i];
    if (past === undefined) continue;
    if (past.type !== "traded" && past.type !== "shuffledIn" && past.type !== "drawn") continue;
    if (past.player !== event.player) continue;
    if (last === undefined) last = past;
    else {
      prev = past;
      break;
    }
  }
  if (last?.type !== "shuffledIn" || prev?.type !== "traded") return [];
  const side = sideOf(ctx.view, event.player);
  const handRegion = tid(animTestid.hand(side));
  const spec = looking(FLAVOR_LOOKS.Spirit, ctx.env, "keep");
  return [
    {
      keyword: "Tradable",
      event,
      holder: handRegion,
      replaces: false,
      plan(D) {
        return [motifBurst(spec, handRegion, "point", timesOf(D).L, MOTIF_TUNING.small, "sparkle")];
      },
    },
  ];
}

/**
 * R2791000: a Stack landing: a gold burst up-right of the pile top and a count pulse of the new
 * pile total, gated on the arrival's slot standing occupied in the view (§3.2: only a Stack card
 * lands on one). The callout sits over the occupant, over the zone slot when it shows no face.
 */
function stackBeat(event: SummonedEvent, ctx: Ctx): Beat[] {
  const side = sideOf(ctx.view, event.player);
  const zone = tid(testid.zone(side, event.row, event.lane));
  let occupant: FxAnchor = zone;
  let buried = 0;
  if (event.row === "units") {
    const standing = ctx.view[side].units[event.lane - 1];
    if (standing === null || standing === undefined) return [];
    occupant = tid(testid.card(standing.instanceId));
    buried = standing.buried;
  } else {
    const lying = ctx.view[side].backrow[event.lane - 1];
    if (lying === null || lying === undefined) return [];
    if (lying.faceDown === false) {
      occupant = tid(testid.card(lying.instanceId));
      buried = lying.buried ?? 0;
    }
  }
  const burstAt = occupant.kind === "testid" ? tid(occupant.testid, { ...FX_STACK_LAND }) : occupant;
  const tint = ZONE_LOOKS.upgraded?.look.tint ?? ZONE_DEFAULT.look.tint;
  const spec = looking(FLAVOR_LOOKS.Hustle, ctx.env, "keep");
  return [
    {
      keyword: "Stack",
      event,
      holder: occupant,
      replaces: false,
      plan(D) {
        const pop = frac(FX_ZONE_COUNT_AT, D);
        return [
          motifBurst(spec, burstAt, "point", timesOf(D).L, MOTIF_TUNING.small, "gold"),
          {
            kind: "zone",
            at: occupant,
            tint,
            text: `×${String(buried + 2)}`,
            direction: "up",
            delayMs: pop,
            durationMs: D - pop + FX_ZONE_TAIL_MS,
          },
        ];
      },
    },
  ];
}

/**
 * R2791000: Spell Damage's "+N" sigil: a resolving Spell's non-combat hit burns hotter, with the
 * view's Spell Damage total over its source. Once per resolution: only its first entry of hits.
 */
function spellDamageBeat(event: HitEvent, ctx: Ctx): Beat[] {
  if (event.combat) return [];
  const play = ctx.env.memory.resolving();
  if (play === undefined) return [];
  if (ctx.env.card(play.defId)?.type !== "Spell") return [];
  const total = spellDamageTotal(ctx.view, play.player);
  if (total <= 0) return [];
  if (!isFirstEntry(ctx.env, ctx.entry, ["damage", "damageAbsorbed"])) return [];
  const to = hitAnchor(ctx.view, event.targetId);
  if (to === null) return [];
  const from = (event.sourceId === null ? null : hitAnchor(ctx.view, event.sourceId)) ?? sourceOf(event, ctx.view, ctx.env);
  if (from === null) return [];
  const spec = looking(FLAVOR_LOOKS.Fire, ctx.env, "keep");
  const tint = ZONE_LOOKS.upgraded?.look.tint ?? ZONE_DEFAULT.look.tint;
  return [
    {
      keyword: "Spell Damage",
      event,
      holder: from,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          motifBurst(spec, to, "area", t.L, MOTIF_TUNING.small),
          {
            kind: "zone",
            at: from,
            tint,
            text: `+${String(total)}`,
            direction: "up",
            delayMs: t.L,
            durationMs: D - t.L + FX_ZONE_TAIL_MS,
          },
        ];
      },
    },
  ];
}

/**
 * R2791000: Lucky's clover glints: a leaf burst and a gold ring over the roll panel at each extra
 * roll's landing, with one callout over the panel. The panel row is kept, so nothing pauses longer.
 */
function luckyBeat(event: Extract<GameEvent, { type: "rolled" }>, ctx: Ctx): Beat[] {
  const batch = rollBatchOf(event);
  if (batch === null || extraDraws(batch) <= 0) return [];
  const panel: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };
  const spec = looking(FLAVOR_LOOKS.Nature, ctx.env, "keep");
  return [
    {
      keyword: "Lucky",
      event,
      holder: panel,
      replaces: false,
      plan(D) {
        const cues: FxCue[] = [];
        const n = batch.rolls.length;
        batch.rolls.forEach((roll, i) => {
          if (roll.faces.length <= 1) return;
          const land = frac(rollLandAt(i, n), D);
          cues.push(motifBurst(spec, panel, "point", land, MOTIF_TUNING.small, "leaf"));
          cues.push(ringCue(D, "gold", panel, land));
        });
        return cues;
      },
    },
  ];
}

/**
 * R2791000: Flashback's rise: the card climbs from the graveyard pile wreathed in a ghost frame,
 * under a sheen and a Dark burst.
 */
function flashbackRiseBeat(event: Extract<GameEvent, { type: "cardPlayed" }>, ctx: Ctx): Beat[] {
  if (event.from !== "graveyard") return [];
  const side = sideOf(ctx.view, event.player);
  const pile = tid(animTestid.graveyard(side));
  const spec = looking(FLAVOR_LOOKS.Dark, ctx.env, "keep");
  return [
    {
      keyword: "Flashback",
      event,
      holder: pile,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          { kind: "ghost", from: pile, to: tid(animTestid.graveyard(side), { x: MID, y: 0 }), delayMs: 0, durationMs: t.L },
          { kind: "sheen", at: pile, delayMs: 0, durationMs: t.L },
          motifBurst(spec, pile, "area", t.L, MOTIF_TUNING.small),
        ];
      },
    },
  ];
}

/**
 * R2791000: Flashback's exile: the risen card dissolves where it is exiled, once its graveyard
 * play stands in the history.
 */
function flashbackExileBeat(event: Extract<GameEvent, { type: "exiled" }>, ctx: Ctx): Beat[] {
  const risen = ctx.history.some((past) => past.type === "cardPlayed" && past.instanceId === event.instanceId && past.from === "graveyard");
  if (!risen) return [];
  const pile = tid(animTestid.exile(sideOf(ctx.view, event.owner)));
  return [
    {
      keyword: "Flashback",
      event,
      holder: pile,
      replaces: false,
      plan(D) {
        const land = timesOf(D).L;
        return [tunedBurst(ctx.env.intensity, "smoke", pile, "area", land, MOTIF_TUNING.small), ringCue(D, "void", pile, land)];
      },
    },
  ];
}

/**
 * R2791000: Temporary's dissolve: the hourglass count over the hand card that leaves at the end
 * of its time, in smoke and embers. The standing hourglass is the card's own treatment; the beat
 * is the leaving.
 */
function temporaryBeat(event: Extract<GameEvent, { type: "discarded" }>, ctx: Ctx): Beat[] {
  const side = sideOf(ctx.view, event.owner);
  const keywords = handKeywordsOf(ctx.view, side, event.instanceId);
  const left = keywords === null ? null : temporaryLeft(keywords);
  if (left === null) return [];
  const at = locateInstance(ctx.view, event.instanceId);
  if (at === null || !at.startsWith("hand-card-")) return [];
  const card = tid(at);
  const tint = ZONE_LOOKS.exiled?.look.tint ?? ZONE_DEFAULT.look.tint;
  return [
    {
      keyword: "Temporary",
      event,
      holder: card,
      replaces: false,
      plan(D) {
        const t = timesOf(D);
        return [
          {
            kind: "zone",
            at: card,
            tint,
            text: `${TEMPORARY_GLYPH}${String(left)}`,
            direction: "none",
            delayMs: 0,
            durationMs: D + FX_ZONE_TAIL_MS,
          },
          tunedBurst(ctx.env.intensity, "smoke", card, "area", 0, MOTIF_TUNING.small),
          tunedBurst(ctx.env.intensity, "ember", card, "area", t.T, MOTIF_TUNING.small),
        ];
      },
    },
  ];
}

function beatsAt(event: GameEvent, ctx: Ctx): Beat[] {
  switch (event.type) {
    case "damage":
      // R2791000's Spell Damage answers a non-combat hit on either hit event; Issue #2790's
      // crack reads the view's Brittle on a `damage`.
      return [...hitBeats(event, ctx), ...brittleCrackBeat(event, ctx), ...spellDamageBeat(event, ctx)];
    case "damageAbsorbed":
      return [...hitBeats(event, ctx), ...armorBeat(event, ctx), ...indestructibleBeat(event, ctx), ...spellDamageBeat(event, ctx)];
    case "healed":
      return event.via === "Lifesteal" ? lifestealBeat(event, ctx) : [];
    case "destroyed":
      return deathBeats(event, ctx);
    case "attackDeclared":
      return attackBeats(event, ctx);
    case "divineShieldLost":
      return divineShieldBeat(event, ctx);
    case "summoned":
      return [...chargeLandingBeat("Charge", event, ctx), ...chargeLandingBeat("Rush", event, ctx), ...stackBeat(event, ctx), ...rebornBeat(event, ctx)];
    case "crumbled":
      return brittleShatterBeat(event, ctx);
    case "keywordGranted":
      return [...tauntArrivalBeat(event, ctx), ...immutableBeat(event, ctx), ...immuneBeat(event, ctx)];
    case "positionSwitched":
      return cantDefendBeat(event, ctx);
    case "fused":
      return equipBeat(event, ctx);
    case "traded":
      return tradableOutBeat(event, ctx);
    case "drawn":
      return tradableInBeat(event, ctx);
    case "rolled":
      return luckyBeat(event, ctx);
    case "cardPlayed":
      return flashbackRiseBeat(event, ctx);
    case "exiled":
      return flashbackExileBeat(event, ctx);
    case "discarded":
      return temporaryBeat(event, ctx);
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
