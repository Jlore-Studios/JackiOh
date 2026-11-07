// R3336000: the Drone Battleship's moments, Stinkyscape #36's three layers drawn as one great ship (R1842000):
// its Shield taking a hit, its Propagation bays launching drones, its Meltdown glowing to the end, and a
// layer breaking off. Planned from public events and the layers the view draws (R202), never from a card's
// text: a redirect onto the Shield (R1842001) is a beam from the protected target to the dome, which flares
// and ripples, and the armor plates glow down for what Armor took of the hit; an ordinary hit is a flicker.
// An Activate of the Propagation layer opens a bay (the lights stay on for the bays that still hold
// drones) and the drone it summons leaves it: an Interceptor shot out fast, a Corvette rolled out heavy. The
// Meltdown's end-of-turn damage pulses its core, harder the more of it is gone; its attacks fire from the
// reactor and its death is the reactor's blast. A Shield or a Propagation layer that dies with a deck beneath
// it shears off in plates, the next deck's lights coming up. The arrival is `cards/stinkyscape/battleshipWarp.ts`.
// Everything starts inside its entry and ends within FX_SHIP_TAIL_MS of it, inside FX_MAX_TAIL_MS (R200); the
// effects speed scales it with the entry (R201). No cue names a card's definition, and a hidden id never
// reaches a lookup (R202).

import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";

import { SHIP_BAYS, SHIP_DRONE_IDS, shipLayerOf, type ShipLayer } from "../cards/art/stinkyscape.ts";
import { HIDDEN_ID, targetFor, type AnimationEntry } from "../game/animations.ts";
import { testid, type Side } from "../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst, type BurstTuning } from "./build.ts";
import {
  FX_CRACK_TAIL_MS,
  FX_REDIRECT_FLIGHT_FRACTION,
  FX_SHIP_BAY_AT,
  FX_SHIP_BLAST_TRAUMA,
  FX_SHIP_BREAK_TRAUMA,
  FX_SHIP_CORE_AT,
  FX_SHIP_CORVETTE_FLIGHT,
  FX_SHIP_CORVETTE_TRAUMA,
  FX_SHIP_DOME_AT,
  FX_SHIP_DOME_TRAUMA,
  FX_SHIP_HIT_TRAUMA,
  FX_SHIP_INTERCEPTOR_FLIGHT,
  FX_SHIP_LAUNCH_AT,
  FX_SHIP_LIGHTS_AT,
  FX_SHIP_PLATES_MAX,
  FX_SHIP_RIPPLES,
  FX_SHIP_RIPPLE_STEP,
  FX_SHIP_TAIL_MS,
  FX_SHIP_TICK_TRAUMA,
} from "./constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "./flavors.ts";
import { MOTIF_TUNING, hitAnchor, motifBurst, motifShot, timesOf, type CueSpec } from "./motifs/build.ts";
import { shieldCues } from "./shield.ts";
import { tintOf, type SetPiecePlan } from "./setPieces.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxShipCue } from "./types.ts";

/** Every particle count the Battleship throws at intensity "normal" (rule 9): the reactor's pulse from a spent tick to a nearly melted one, the blast, the shear and a launch's landing. */
export const SHIP_TUNING = {
  tickMin: { count: 8, power: 0.7 },
  tickMax: { count: 36, power: 1.3 },
  blast: { count: 64, power: 1.6 },
  shards: { count: 30, power: 1.3 },
  shear: { count: 18, power: 0.6 },
  attack: { count: 14, power: 1 },
  launch: { count: 10, power: 0.9 },
} as const satisfies Record<string, BurstTuning>;

/** One layer of a Battleship as the view draws it: its unit and where it stands. */
type Layer = { unit: UnitView; layer: ShipLayer; side: Side; lane: number };

/** The Battleship layer the view draws with this id, or null: no unit, a hidden one or any other card (R202). */
function layerAt(view: PlayerView, id: string): Layer | null {
  if (id === HIDDEN_ID) return null;
  for (const side of ["you", "opponent"] as const) {
    const units = view[side].units;
    for (let lane = 0; lane < units.length; lane += 1) {
      const unit = units[lane];
      if (unit === null || unit === undefined || unit.instanceId !== id) continue;
      const layer = shipLayerOf(unit.defId);
      return layer === null ? null : { unit, layer, side, lane: lane + 1 };
    }
  }
  return null;
}

/** Tech's starfield look: the dome and the arrival; its rocket look: the bays; the Fire look: the core. */
const starfield = (): FlavorLook => paletteLook("Tech/starfield") ?? FLAVOR_LOOKS.Tech;
const rocket = (): FlavorLook => paletteLook("Tech/rocket") ?? FLAVOR_LOOKS.Tech;

const specOf = (look: FlavorLook, env: Pick<FxPlanEnv, "intensity">): CueSpec => ({ look, scale: env.intensity, row: "keep" });

/** The events before this one, oldest first: the memory's earlier ones (less this entry's own, if it remembered them last), then this entry's own up to `index`. */
function beforeOf(entry: AnimationEntry, env: FxPlanEnv, index: number): readonly GameEvent[] {
  const recent = env.memory.recent();
  const own = entry.events.length;
  const tail = recent.slice(Math.max(0, recent.length - own));
  const remembered = own > 0 && tail.length === own && tail.every((event, k) => event === entry.events[k]);
  return [...(remembered ? recent.slice(0, recent.length - own) : recent), ...entry.events.slice(0, index)];
}

/** Whether the hit on `targetId` is the one a redirect just sent there: the latest redirect onto it, with no hit on it since. */
function wasRedirected(before: readonly GameEvent[], targetId: string): boolean {
  for (let k = before.length - 1; k >= 0; k -= 1) {
    const event = before[k];
    if (event === undefined) continue;
    if (event.type === "redirected" && event.toId === targetId) return true;
    if ((event.type === "damage" || event.type === "damageAbsorbed") && event.targetId === targetId) return false;
  }
  return false;
}

/** `from` to `to` by `level` (0 to 1): a tuning between a pulse's smallest and its largest. */
function between(from: BurstTuning, to: BurstTuning, level: number): BurstTuning {
  return { count: from.count + (to.count - from.count) * level, power: from.power + (to.power - from.power) * level };
}

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));

/** The dome over `card`: a full flare (the shield bloom) and its ripples, from `land`. */
function domeCues(card: string, D: number, land: number, i: number): FxCue[] {
  const dome = tid(card, FX_SHIP_DOME_AT);
  const cues: FxCue[] = shieldCues("full", dome, D, land, i);
  for (let k = 1; k <= FX_SHIP_RIPPLES; k += 1) cues.push(ringCue(D, "frost", dome, Math.min(D, land + frac(k * FX_SHIP_RIPPLE_STEP, D))));
  return cues;
}

/** The ship cue over a layer's card, `delayMs` into an entry of D. */
function shipCue(move: FxShipCue["move"], card: string, look: FlavorLook, count: number, lit: number, D: number, delayMs: number): FxShipCue {
  return { kind: "ship", move, at: tid(card), tint: tintOf(look), count, lit, delayMs, durationMs: D - delayMs + FX_SHIP_TAIL_MS };
}

/** A sheen over `at`, from `delayMs` to the entry's end and the Battleship's tail. */
const sheenCue = (at: FxAnchor, D: number, delayMs: number): FxCue => ({ kind: "sheen", at, delayMs, durationMs: D - delayMs + FX_SHIP_TAIL_MS });

type Plan = { cues: FxCue[]; replaces: Set<GameEvent> };

/** A redirect onto the Shield: a beam from the protected target to the dome, which flares and ripples as it lands. */
function planRedirect(event: Extract<GameEvent, { type: "redirected" }>, shield: Layer, view: PlayerView, D: number, i: number, out: Plan): void {
  const card = testid.card(shield.unit.instanceId);
  const dome = tid(card, FX_SHIP_DOME_AT);
  const from = hitAnchor(view, event.fromId);
  const flies = from !== null && !(from.kind === "testid" && from.testid === card);
  const land = flies ? frac(FX_REDIRECT_FLIGHT_FRACTION, D) : 0;
  if (flies) out.cues.push(motifShot(specOf(starfield(), { intensity: i }), "bolt", from, dome, 0, land));
  out.cues.push(...domeCues(card, D, land, i), ...shakeCues(i, FX_SHIP_DOME_TRAUMA, land));
  out.replaces.add(event);
}

/** A hit on the Shield: the redirected one flares the dome and glows down the plates Armor took; any other is a flicker. */
function planShieldHit(event: Extract<GameEvent, { type: "damage" | "damageAbsorbed" }>, shield: Layer, redirected: boolean, D: number, i: number, out: Plan): void {
  const card = testid.card(shield.unit.instanceId);
  const dome = tid(card, FX_SHIP_DOME_AT);
  if (redirected) {
    const took = event.absorbed ?? 0;
    const count = Math.min(Math.max(0, shield.unit.armor), FX_SHIP_PLATES_MAX);
    out.cues.push(...domeCues(card, D, 0, i), ...shakeCues(i, FX_SHIP_DOME_TRAUMA, 0));
    if (count > 0) out.cues.push(shipCue("plates", card, starfield(), count, Math.min(count, Math.max(0, took)), D, 0));
  } else {
    out.cues.push(sheenCue(tid(card), D, 0), ...shieldCues("small", dome, D, 0, i), ...shakeCues(i, FX_SHIP_HIT_TRAUMA, 0));
  }
  // The armor row would bloom the same shield over the card: the dome's own flare stands in for it.
  if (event.type === "damageAbsorbed") out.replaces.add(event);
}

/** An Activate of the Propagation layer: a bay door opens, its lights staying on for the bays that still hold drones. */
function planActivate(event: Extract<GameEvent, { type: "activated" }>, hangar: Layer, D: number, out: Plan): void {
  const card = testid.card(hangar.unit.instanceId);
  const activations = hangar.unit.activations;
  // The view is the board before this Activate, so this one's use is spent: the bays that still have a use after it.
  const lit =
    activations === undefined
      ? SHIP_BAYS
      : activations.filter((a) => (a.ability === event.ability ? a.usesLeft === null || a.usesLeft > 1 : a.usesLeft !== 0)).length;
  out.cues.push(shipCue("bay", card, rocket(), lit, lit, D, 0), sheenCue(tid(card), D, 0));
}

/** A drone the Propagation layer's Activate summoned leaves its bay for its zone: an Interceptor shot out, a Corvette rolled out heavy. */
function planLaunch(event: Extract<GameEvent, { type: "summoned" }>, hangar: Layer, view: PlayerView, D: number, i: number, out: Plan): void {
  const zone = targetFor(event, view);
  if (zone === null) return;
  const from = tid(testid.card(hangar.unit.instanceId), FX_SHIP_BAY_AT);
  const to = tid(zone);
  const look = rocket();
  const spec = specOf(look, { intensity: i });
  const start = frac(FX_SHIP_LAUNCH_AT, D);
  if (event.defId === SHIP_DRONE_IDS.corvette) {
    const flight = frac(FX_SHIP_CORVETTE_FLIGHT, D);
    out.cues.push(
      { kind: "projectile", preset: "exhaust", from, to, delayMs: start, flightMs: flight, density: i },
      motifBurst(spec, to, "area", start + flight, MOTIF_TUNING.land),
      ...shakeCues(i, FX_SHIP_CORVETTE_TRAUMA, start + flight),
    );
    return;
  }
  const flight = frac(FX_SHIP_INTERCEPTOR_FLIGHT, D);
  out.cues.push(motifShot(spec, "bolt", from, to, start, flight), motifBurst(spec, to, "point", start + flight, SHIP_TUNING.launch));
}

/** The Meltdown's end-of-turn damage to itself: its core pulses, harder the more of it has melted. */
function planTick(event: Extract<GameEvent, { type: "damage" }>, meltdown: Layer, D: number, i: number, out: Plan): void {
  const { health, maxHealth } = meltdown.unit;
  const level = maxHealth > 0 ? clamp01((maxHealth - health + event.amount) / maxHealth) : 0;
  const core = tid(testid.card(meltdown.unit.instanceId), FX_SHIP_CORE_AT);
  const fire = FLAVOR_LOOKS.Fire;
  out.cues.push(ringCue(D, "fire", core, 0), { ...tunedBurst(i, "ember", core, "point", 0, between(SHIP_TUNING.tickMin, SHIP_TUNING.tickMax, level)), colors: fire.colors }, ...shakeCues(i, FX_SHIP_TICK_TRAUMA * level, 0));
}

/** The Meltdown's attack, fired from the reactor. */
function planAttack(event: Extract<GameEvent, { type: "attackDeclared" }>, meltdown: Layer, view: PlayerView, D: number, i: number, out: Plan): void {
  const target = hitAnchor(view, event.targetId);
  const core = tid(testid.card(meltdown.unit.instanceId), FX_SHIP_CORE_AT);
  const spec = specOf(FLAVOR_LOOKS.Fire, { intensity: i });
  out.cues.push(motifBurst(spec, core, "point", 0, SHIP_TUNING.attack));
  if (target !== null && !(target.kind === "testid" && target.testid === testid.card(meltdown.unit.instanceId))) {
    out.cues.push(motifShot(spec, "beam", core, target, 0, timesOf(D).L));
  }
}

/** The Meltdown's death: the reactor's blast, the hardest shake the Battleship makes. */
function planBlast(meltdown: Layer, D: number, i: number, out: Plan): void {
  const card = testid.card(meltdown.unit.instanceId);
  const core = tid(card, FX_SHIP_CORE_AT);
  const whole = tid(card);
  const fire = FLAVOR_LOOKS.Fire;
  out.cues.push(
    ringCue(D, "fire", core, 0),
    ringCue(D, "fire", core, Math.min(D, frac(FX_SHIP_RIPPLE_STEP, D))),
    { ...tunedBurst(i, "fire", core, "area", 0, SHIP_TUNING.blast), colors: fire.colors },
    { ...tunedBurst(i, "ember", whole, "area", 0, SHIP_TUNING.blast), colors: fire.colors },
    tunedBurst(i, "shard", whole, "ring", 0, SHIP_TUNING.shards),
    raysCue(D, "legendary", core, 0),
    ...shakeCues(i, FX_SHIP_BLAST_TRAUMA, 0),
  );
}

/** A deck breaking off the ship: plates shear away and drift down, then the next deck's lights come up. */
function planShear(deck: Layer, D: number, i: number, out: Plan): void {
  const card = testid.card(deck.unit.instanceId);
  const whole = tid(card);
  const zone = tid(testid.zone(deck.side, "units", deck.lane));
  const lights = Math.min(D, frac(FX_SHIP_LIGHTS_AT, D));
  out.cues.push(
    { kind: "crack", at: whole, delayMs: 0, durationMs: D + FX_CRACK_TAIL_MS },
    tunedBurst(i, "shard", whole, "area", 0, SHIP_TUNING.shear),
    ...shakeCues(i, FX_SHIP_BREAK_TRAUMA, 0),
    sheenCue(zone, D, lights),
    ringCue(D, rocket().particle, zone, lights),
  );
}

/**
 * R3336000: the Battleship's cues for an entry's events, and the rows they draw in place of their own; null for an
 * entry that holds none of its beats, the effects off, an entry of no duration (Reduce Motion) or one of the shapes
 * that plays as a whole (a zone, a sweep, a wipe, a fuse or a launch).
 */
export function planBattleship(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): SetPiecePlan | null {
  const i = env.intensity;
  const D = entry.durationMs;
  if (!(i > 0) || !(D > 0)) return null;
  if (entry.zone !== undefined || entry.sweep !== undefined || entry.wipe !== undefined || entry.fuse !== undefined || entry.launch !== undefined) return null;
  const out: Plan = { cues: [], replaces: new Set<GameEvent>() };
  entry.events.forEach((event, index) => {
    switch (event.type) {
      case "redirected": {
        const shield = layerAt(view, event.toId);
        if (shield?.layer === "shield") planRedirect(event, shield, view, D, i, out);
        return;
      }
      case "damage":
      case "damageAbsorbed": {
        const target = layerAt(view, event.targetId);
        if (target === null) return;
        if (target.layer === "shield") {
          planShieldHit(event, target, wasRedirected(beforeOf(entry, env, index), event.targetId), D, i, out);
        } else if (target.layer === "meltdown" && event.type === "damage" && event.sourceId === event.targetId) {
          planTick(event, target, D, i, out);
        }
        return;
      }
      case "activated": {
        const hangar = layerAt(view, event.instanceId);
        if (hangar?.layer === "propagation") planActivate(event, hangar, D, out);
        return;
      }
      case "summoned": {
        if (event.defId !== SHIP_DRONE_IDS.interceptor && event.defId !== SHIP_DRONE_IDS.corvette) return;
        const play = env.memory.resolving();
        const hangar = play?.hook === "activate" ? layerAt(view, play.instanceId) : null;
        if (hangar?.layer === "propagation") planLaunch(event, hangar, view, D, i, out);
        return;
      }
      case "attackDeclared": {
        const attacker = layerAt(view, event.attackerId);
        if (attacker?.layer === "meltdown") planAttack(event, attacker, view, D, i, out);
        return;
      }
      case "destroyed": {
        const dying = layerAt(view, event.instanceId);
        if (dying === null) return;
        if (dying.layer === "meltdown") planBlast(dying, D, i, out);
        else if (dying.unit.buried > 0) planShear(dying, D, i, out);
        return;
      }
      default:
        return;
    }
  });
  return out.cues.length === 0 && out.replaces.size === 0 ? null : out;
}
