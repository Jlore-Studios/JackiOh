// R3336000: the Drone Battleship's moments as the effects layer plans them (battleship.ts). A redirect onto the
// Shield beams from the protected target to the dome, which flares and ripples, and the plates glow for what
// Armor took; an ordinary hit is a flicker. The Propagation layer's Activates open a bay and the drone leaves
// it, an Interceptor fast and a Corvette heavy; the Meltdown's tick pulses its core harder as it melts, its
// attacks fire from the reactor and its death is the hardest shake; a layer with a deck beneath it shears off
// and lights the next. Everything lands inside its entry and is gone within FX_MAX_TAIL_MS of it (R200); no cue
// names a card, and a card the view cannot place plans nothing (R202).

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { SHIP_LAYER_IDS, SHIP_DRONE_IDS } from "../cards/art/stinkyscape.ts";
import { planEntries, type AnimationEntry } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { baseView, emptySide, unit, withEvents } from "../test/fixtures.ts";
import { planBattleship, SHIP_TUNING } from "./battleship.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
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
  FX_SHIP_TICK_TRAUMA,
} from "./constants.ts";
import { frac } from "./build.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxAnchor, FxBurstCue, FxCue, FxPlanEnv, FxProjectileCue, FxRingCue, FxShakeCue, FxShieldCue, FxShipCue, FxShotCue } from "./types.ts";

const { shield: SHIELD, propagation: HANGAR, meltdown: MELTDOWN } = SHIP_LAYER_IDS;

const envOf = (intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv => ({ intensity, card: () => undefined, memory: createFxMemory() });

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });

/** Where a cue ends, in ms after its entry starts (R200's bound is D + FX_MAX_TAIL_MS). */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
    case "shot":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const activations = [
  { ability: "interceptor", label: "Activate 2", usesLeft: 2, usable: true },
  { ability: "corvette", label: "Activate", usesLeft: 1, usable: true },
];

/** p1's board: its layer in lane 3 (the top card of its pile), the view the entries are planned against. */
function board(layer: UnitView, side: "you" | "opponent" = "you"): PlayerView {
  const owner: PlayerId = side === "you" ? "p1" : "p2";
  const units: (UnitView | null)[] = [null, null, layer, null, null];
  return baseView(side === "you" ? { you: emptySide("p1", { units }), opponent: emptySide("p2") } : { you: emptySide("p1"), opponent: emptySide(owner, { units }) });
}

const shield = (over: Partial<UnitView> = {}): UnitView => unit("p1", { instanceId: "sh", defId: SHIELD, armor: 4, health: 12, maxHealth: 12, buried: 2, ...over });
const hangar = (over: Partial<UnitView> = {}): UnitView => unit("p1", { instanceId: "pr", defId: HANGAR, health: 12, maxHealth: 12, buried: 1, activations, ...over });
const meltdown = (over: Partial<UnitView> = {}): UnitView => unit("p1", { instanceId: "me", defId: MELTDOWN, health: 24, maxHealth: 24, buried: 0, ...over });

const card = (id: string): string => testid.card(id);
const lane3 = tid(testid.zone("you", "units", 3));

const redirected = (toId = "sh", fromId = "hero-p1"): GameEvent => ({ type: "redirected", what: "damage", fromId, toId, byInstanceId: toId });
const absorbed = (absorb: number, targetId = "sh"): GameEvent => ({ type: "damageAbsorbed", sourceId: null, targetId, absorbed: absorb, combat: false });
const hit = (amount: number, targetId = "sh", over: Partial<Extract<GameEvent, { type: "damage" }>> = {}): GameEvent => ({
  type: "damage",
  sourceId: null,
  targetId,
  amount,
  combat: false,
  ...over,
});
const activated = (instanceId = "pr", ability = "interceptor"): GameEvent => ({ type: "activated", player: "p1", instanceId, defId: HANGAR, ability });
const marker = (sourceId = "pr"): GameEvent => ({ type: "hookResolving", sourceId, defId: HANGAR, controller: "p1", hook: "activate" });
const summoned = (defId: string, instanceId = "dr", lane = 4): GameEvent => ({ type: "summoned", player: "p1", instanceId, defId, row: "units", lane });
const attacked = (attackerId: string, targetId: string): GameEvent => ({ type: "attackDeclared", attackerId, targetId, forced: false });
const destroyed = (instanceId: string, defId: string): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId,
  owner: "p1",
  controller: "p1",
  attack: 0,
  maxHealth: 12,
  killerId: null,
});

type Planned = { entry: AnimationEntry; cues: FxCue[]; replaces: ReadonlySet<GameEvent> };

/** Plans each entry of `events` in order as FxLayer does on every `start`: remember, then plan; the entries with nothing planned give no cues. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): Planned[] {
  return planEntries(events, withEvents(view, [...events]), false).map((entry) => {
    env.memory.remember(entry.events);
    const plan = planBattleship(entry, entry.view, env);
    return { entry, cues: plan?.cues ?? [], replaces: plan?.replaces ?? new Set<GameEvent>() };
  });
}

/** The cues of the entry holding `event`. */
function cuesOf(planned: readonly Planned[], event: GameEvent): FxCue[] {
  const found = planned.find(({ entry }) => entry.events.includes(event));
  if (found === undefined) throw new Error(`no entry holds ${event.type}`);
  return found.cues;
}

const ofKind = <K extends FxCue["kind"]>(cues: readonly FxCue[], kind: K): Extract<FxCue, { kind: K }>[] => cues.filter((c): c is Extract<FxCue, { kind: K }> => c.kind === kind);
const shakesOf = (cues: readonly FxCue[]): FxShakeCue[] => ofKind(cues, "shake");
const shipsOf = (cues: readonly FxCue[]): FxShipCue[] => ofKind(cues, "ship");
const burstsOf = (cues: readonly FxCue[], preset: FxBurstCue["preset"]): FxBurstCue[] => ofKind(cues, "burst").filter((c) => c.preset === preset);

describe("R3336000 the shield takes the hit", () => {
  it("R3336000 a redirect onto the Shield beams from the protected target to the dome, which flares and ripples", () => {
    const event = redirected("sh", "hero-p1");
    const planned = planAll([event], board(shield()));
    const cues = cuesOf(planned, event);
    const dome = tid(card("sh"), FX_SHIP_DOME_AT);
    const beam = ofKind(cues, "shot");
    expect(beam).toHaveLength(1);
    expect(beam[0]).toEqual(expect.objectContaining({ from: tid(testid.hero("you")), to: dome, delayMs: 0 }));
    const flare = ofKind(cues, "shield");
    expect(flare).toEqual([expect.objectContaining({ size: "full", at: dome, delayMs: (beam[0] as FxShotCue).flightMs })]);
    // The bloom's own ring and each ripple, all on the dome and after the beam lands.
    const rings = ofKind(cues, "ring").filter((ring) => ring.at.kind === "testid" && ring.at.testid === card("sh"));
    expect(rings).toHaveLength(1 + FX_SHIP_RIPPLES);
    for (const ring of rings) expect((ring as FxRingCue).delayMs).toBeGreaterThanOrEqual((flare[0] as FxShieldCue).delayMs);
    expect(shakesOf(cues)).toEqual([expect.objectContaining({ trauma: FX_SHIP_DOME_TRAUMA, delayMs: (beam[0] as FxShotCue).flightMs })]);
    // The redirect's own row gives way to it.
    expect(planned[0]?.replaces.has(event)).toBe(true);
  });

  it("R3336000 a redirect from a Unit beams from that Unit, and one onto no Shield plans nothing", () => {
    const from = unit("p1", { instanceId: "ally", attack: 1 });
    const view = baseView({ you: emptySide("p1", { units: [from, null, shield(), null, null] }), opponent: emptySide("p2") });
    const event = redirected("sh", "ally");
    expect(ofKind(cuesOf(planAll([event], view), event), "shot")[0]?.from).toEqual(tid(card("ally")));
    // A hangar or a keel redirected onto is no dome.
    expect(planAll([redirected("pr")], board(hangar()))[0]?.cues).toEqual([]);
    expect(planAll([redirected("me")], board(meltdown()))[0]?.cues).toEqual([]);
  });

  it("R3336000 an absorbed redirected hit lights min(armor, absorbed) plates, and flares the dome in place of the armor row", () => {
    const hitEvent = absorbed(3);
    const planned = planAll([redirected(), hitEvent], board(shield({ armor: 4 })));
    const cues = cuesOf(planned, hitEvent);
    expect(shipsOf(cues)).toEqual([expect.objectContaining({ move: "plates", count: 4, lit: 3, at: tid(card("sh")) })]);
    expect(ofKind(cues, "shield")[0]).toEqual(expect.objectContaining({ size: "full" }));
    expect(planned.some(({ replaces }) => replaces.has(hitEvent))).toBe(true);
    // More absorbed than armor lights every plate; the plates never pass FX_SHIP_PLATES_MAX.
    const all = planAll([redirected(), absorbed(9)], board(shield({ armor: 4 })));
    expect(shipsOf(all.flatMap(({ cues: c }) => c))[0]).toEqual(expect.objectContaining({ count: 4, lit: 4 }));
    const heavy = planAll([redirected(), absorbed(20)], board(shield({ armor: 30 })));
    expect(shipsOf(heavy.flatMap(({ cues: c }) => c))[0]).toEqual(expect.objectContaining({ count: FX_SHIP_PLATES_MAX, lit: FX_SHIP_PLATES_MAX }));
  });

  it("R3336000 a redirected hit that landed past its Armor keeps its row and lights what Armor took", () => {
    const hitEvent = hit(5, "sh", { absorbed: 2 });
    const planned = planAll([redirected(), hitEvent], board(shield({ armor: 4 })));
    expect(shipsOf(cuesOf(planned, hitEvent))).toEqual([expect.objectContaining({ move: "plates", count: 4, lit: 2 })]);
    expect(planned.some(({ replaces }) => replaces.has(hitEvent))).toBe(false);
    // No absorbed field: the plates stand dim, none lit.
    const bare = planAll([redirected(), hit(5)], board(shield({ armor: 4 })));
    expect(shipsOf(bare.flatMap(({ cues }) => cues))).toEqual([expect.objectContaining({ count: 4, lit: 0 })]);
    // A Shield with no Armor has no plates to glow.
    const none = planAll([redirected(), hit(5)], board(shield({ armor: 0 })));
    expect(shipsOf(none.flatMap(({ cues }) => cues))).toEqual([]);
  });

  it("R3336000 an ordinary hit on the Shield is a dome flicker: a sheen and a small shield, no beam and no plates", () => {
    const hitEvent = hit(3, "sh", { sourceId: "foe", combat: true });
    const cues = cuesOf(planAll([hitEvent], board(shield())), hitEvent);
    expect(ofKind(cues, "sheen")).toEqual([expect.objectContaining({ at: tid(card("sh")) })]);
    expect(ofKind(cues, "shield")).toEqual([expect.objectContaining({ size: "small", at: tid(card("sh"), FX_SHIP_DOME_AT) })]);
    expect(ofKind(cues, "shot")).toEqual([]);
    expect(shipsOf(cues)).toEqual([]);
    expect(shakesOf(cues)).toEqual([expect.objectContaining({ trauma: FX_SHIP_HIT_TRAUMA })]);
    // The redirected hit is the one a redirect just sent: the next hit on the Shield is ordinary again.
    const second = hit(2, "sh");
    const after = planAll([redirected(), absorbed(2), second], board(shield()));
    expect(shipsOf(cuesOf(after, second))).toEqual([]);
    expect(ofKind(cuesOf(after, second), "shield")[0]).toEqual(expect.objectContaining({ size: "small" }));
    // A whole-armor hit that no redirect sent is a flicker too, its armor row replaced by it.
    const whole = absorbed(3);
    const wholePlan = planAll([whole], board(shield()));
    expect(ofKind(cuesOf(wholePlan, whole), "shield")[0]).toEqual(expect.objectContaining({ size: "small" }));
    expect(wholePlan[0]?.replaces.has(whole)).toBe(true);
  });
});

describe("R3336000 drones launch from the bays", () => {
  it("R3336000 an Activate opens a bay, its lights staying on for the bays that still hold drones", () => {
    const event = activated("pr", "interceptor");
    const cues = cuesOf(planAll([event], board(hangar())), event);
    // Activate 2 has a use left after this one, and Activate its one.
    expect(shipsOf(cues)).toEqual([expect.objectContaining({ move: "bay", count: 2, lit: 2, at: tid(card("pr")) })]);
    expect(ofKind(cues, "sheen")).toHaveLength(1);
    // Spent: the Corvette's single use is gone after it, so one bay stays lit.
    const corvette = activated("pr", "corvette");
    expect(shipsOf(cuesOf(planAll([corvette], board(hangar())), corvette))[0]).toEqual(expect.objectContaining({ count: 1 }));
    // The opponent's view carries no Activates: both bays show lit.
    const seen = activated("pr", "interceptor");
    const theirs = board(hangar({ activations: undefined, owner: "p2", controller: "p2" }), "opponent");
    expect(shipsOf(cuesOf(planAll([seen], theirs), seen))[0]).toEqual(expect.objectContaining({ count: 2 }));
    // No other layer opens a bay.
    expect(planAll([activated("me")], board(meltdown()))[0]?.cues).toEqual([]);
  });

  it("R3336000 a drone leaves its bay for its tile: an Interceptor shot out fast, a Corvette rolled out heavy", () => {
    const view = board(hangar());
    const interceptor = summoned(SHIP_DRONE_IDS.interceptor, "d1", 4);
    const corvette = summoned(SHIP_DRONE_IDS.corvette, "d2", 5);
    const planned = planAll([marker(), activated(), interceptor, marker(), activated("pr", "corvette"), corvette], view);
    const fast = cuesOf(planned, interceptor);
    const heavy = cuesOf(planned, corvette);
    const from = tid(card("pr"), FX_SHIP_BAY_AT);
    const bolt = ofKind(fast, "shot");
    expect(bolt).toEqual([expect.objectContaining({ from, to: tid(testid.zone("you", "units", 4)), delayMs: frac(FX_SHIP_LAUNCH_AT, planned[2]?.entry.durationMs ?? 0) })]);
    const roll = ofKind(heavy, "projectile");
    expect(roll).toEqual([expect.objectContaining({ preset: "exhaust", from, to: tid(testid.zone("you", "units", 5)) })]);
    // The Interceptor flies faster than the Corvette, by the constants' ratio of the entry.
    const D = planned[2]?.entry.durationMs ?? 0;
    expect((bolt[0] as FxShotCue).flightMs).toBe(frac(FX_SHIP_INTERCEPTOR_FLIGHT, D));
    expect((roll[0] as FxProjectileCue).flightMs).toBe(frac(FX_SHIP_CORVETTE_FLIGHT, D));
    expect((roll[0] as FxProjectileCue).flightMs).toBeGreaterThan((bolt[0] as FxShotCue).flightMs);
    // The Corvette lands heavy: a shake the Interceptor does not make.
    expect(shakesOf(heavy)).toEqual([expect.objectContaining({ trauma: FX_SHIP_CORVETTE_TRAUMA })]);
    expect(shakesOf(fast)).toEqual([]);
  });

  it("R3336000 a drone summoned by anything but a bay's Activate draws no launch", () => {
    const loose = summoned(SHIP_DRONE_IDS.interceptor, "d1", 4);
    expect(planAll([loose], board(hangar()))[0]?.cues).toEqual([]);
    // Nor does an Activate of any other card.
    const other: GameEvent = { type: "hookResolving", sourceId: "ally", defId: "core-004", controller: "p1", hook: "activate" };
    const ally = unit("p1", { instanceId: "ally" });
    const view = baseView({ you: emptySide("p1", { units: [ally, null, hangar(), null, null] }), opponent: emptySide("p2") });
    expect(planAll([other, loose], view)[1]?.cues).toEqual([]);
    // A card that is no drone, summoned in a bay's Activate, is none of it.
    const token = summoned("core-004", "x", 4);
    expect(planAll([marker(), token], board(hangar()))[1]?.cues).toEqual([]);
  });
});

describe("R3336000 the meltdown glows to the end", () => {
  const tick = (amount: number): GameEvent => hit(amount, "me", { sourceId: "me" });

  it("R3336000 the end-of-turn tick pulses the core, its burst and shake growing with how much is melted", () => {
    const light = tick(2);
    const heavy = tick(6);
    const early = cuesOf(planAll([light], board(meltdown({ health: 24 }))), light);
    const late = cuesOf(planAll([heavy], board(meltdown({ health: 8 }))), heavy);
    const core = tid(card("me"), FX_SHIP_CORE_AT);
    expect(ofKind(early, "ring")).toEqual([expect.objectContaining({ preset: "fire", at: core })]);
    const small = burstsOf(early, "ember")[0] as FxBurstCue;
    const big = burstsOf(late, "ember")[0] as FxBurstCue;
    expect(small.at).toEqual(core);
    expect(big.count).toBeGreaterThan(small.count);
    expect(big.power).toBeGreaterThan(small.power);
    expect(small.count).toBeGreaterThanOrEqual(SHIP_TUNING.tickMin.count);
    expect(big.count).toBeLessThanOrEqual(SHIP_TUNING.tickMax.count);
    const lowShake = shakesOf(early)[0]?.trauma ?? 0;
    const highShake = shakesOf(late)[0]?.trauma ?? 0;
    expect(lowShake).toBeGreaterThan(0);
    expect(highShake).toBeGreaterThan(lowShake);
    expect(highShake).toBeLessThanOrEqual(FX_SHIP_TICK_TRAUMA);
  });

  it("R3336000 a hit on the Meltdown by anything else is no pulse", () => {
    const struck = hit(4, "me", { sourceId: "foe", combat: true });
    expect(planAll([struck], board(meltdown()))[0]?.cues).toEqual([]);
    // Nor is a Shield's own damage to itself a reactor tick.
    const own = hit(1, "sh", { sourceId: "sh" });
    expect(shipsOf(planAll([own], board(shield()))[0]?.cues ?? [])).toEqual([]);
  });

  it("R3336000 the Meltdown's attack fires from the reactor at its target", () => {
    const event = attacked("me", "hero-p2");
    const cues = cuesOf(planAll([event], board(meltdown())), event);
    const beam = ofKind(cues, "shot");
    expect(beam).toEqual([expect.objectContaining({ shot: { motion: "beam" }, from: tid(card("me"), FX_SHIP_CORE_AT), to: tid(testid.hero("opponent")) })]);
    expect(burstsOf(cues, "spark").length + burstsOf(cues, "ember").length + burstsOf(cues, "fire").length).toBeGreaterThan(0);
    // An attack by any other layer fires nothing of the reactor's.
    expect(planAll([attacked("sh", "hero-p2")], board(shield()))[0]?.cues).toEqual([]);
  });

  it("R3336000 the reactor's blast shakes the board harder than anything else the Battleship does", () => {
    const event = destroyed("me", MELTDOWN);
    const cues = cuesOf(planAll([event], board(meltdown())), event);
    expect(shakesOf(cues)).toEqual([expect.objectContaining({ trauma: FX_SHIP_BLAST_TRAUMA })]);
    expect(ofKind(cues, "ring").length).toBeGreaterThanOrEqual(2);
    expect(ofKind(cues, "rays")).toHaveLength(1);
    expect(burstsOf(cues, "fire").length).toBeGreaterThan(0);
    expect(burstsOf(cues, "ember").length).toBeGreaterThan(0);
    expect(burstsOf(cues, "shard").length).toBeGreaterThan(0);
    for (const other of [FX_SHIP_BREAK_TRAUMA, FX_SHIP_DOME_TRAUMA, FX_SHIP_HIT_TRAUMA, FX_SHIP_CORVETTE_TRAUMA, FX_SHIP_TICK_TRAUMA]) {
      expect(FX_SHIP_BLAST_TRAUMA).toBeGreaterThan(other);
    }
  });
});

describe("R3336000 a layer breaks", () => {
  it("R3336000 a Shield or a Propagation layer with a deck beneath shears off and lights the next deck", () => {
    for (const [layer, id, defId] of [
      [shield({ buried: 2 }), "sh", SHIELD],
      [hangar({ buried: 1 }), "pr", HANGAR],
    ] as const) {
      const event = destroyed(id, defId);
      const planned = planAll([event], board(layer));
      const cues = cuesOf(planned, event);
      const D = planned[0]?.entry.durationMs ?? 0;
      expect(ofKind(cues, "crack")).toEqual([expect.objectContaining({ at: tid(card(id)), delayMs: 0 })]);
      expect(burstsOf(cues, "shard")).toEqual([expect.objectContaining({ at: tid(card(id)), delayMs: 0, power: SHIP_TUNING.shear.power })]);
      expect(shakesOf(cues)).toEqual([expect.objectContaining({ trauma: FX_SHIP_BREAK_TRAUMA })]);
      // The next deck's lights come up on the zone partway through.
      expect(ofKind(cues, "sheen")).toEqual([expect.objectContaining({ at: lane3, delayMs: frac(FX_SHIP_LIGHTS_AT, D) })]);
      expect(ofKind(cues, "ring").some((ring) => ring.at.kind === "testid" && ring.at.testid === testid.zone("you", "units", 3) && ring.delayMs === frac(FX_SHIP_LIGHTS_AT, D))).toBe(true);
    }
  });

  it("R3336000 a break shears only over a card: a layer with nothing beneath, or no layer at all, gives none", () => {
    expect(planAll([destroyed("sh", SHIELD)], board(shield({ buried: 0 })))[0]?.cues).toEqual([]);
    expect(planAll([destroyed("pr", HANGAR)], board(hangar({ buried: 0 })))[0]?.cues).toEqual([]);
    const other = unit("p1", { instanceId: "plain", buried: 2 });
    expect(planAll([destroyed("plain", "core-004")], board(other))[0]?.cues).toEqual([]);
    // The keel's death is the blast, not a shear: no crack.
    const keel = planAll([destroyed("me", MELTDOWN)], board(meltdown({ buried: 2 })));
    expect(ofKind(keel[0]?.cues ?? [], "crack")).toEqual([]);
  });
});

describe("R3336000 every beat holds inside its entry and its tail, and names no card", () => {
  const SCENES: readonly (readonly [string, readonly GameEvent[], PlayerView])[] = [
    ["redirect", [redirected()], board(shield())],
    ["absorbed", [redirected(), absorbed(3)], board(shield())],
    ["partial", [redirected(), hit(5, "sh", { absorbed: 2 })], board(shield())],
    ["flicker", [hit(3, "sh", { sourceId: "foe", combat: true })], board(shield())],
    ["activate", [marker(), activated(), summoned(SHIP_DRONE_IDS.interceptor, "d1", 4)], board(hangar())],
    ["corvette", [marker(), activated("pr", "corvette"), summoned(SHIP_DRONE_IDS.corvette, "d2", 5)], board(hangar())],
    ["tick", [hit(4, "me", { sourceId: "me" })], board(meltdown({ health: 12 }))],
    ["attack", [attacked("me", "hero-p2")], board(meltdown())],
    ["blast", [destroyed("me", MELTDOWN)], board(meltdown())],
    ["shield break", [destroyed("sh", SHIELD)], board(shield())],
    ["hangar break", [destroyed("pr", HANGAR)], board(hangar())],
  ];

  it("R3336000 every cue starts in its entry and ends within FX_MAX_TAIL_MS of it (R200)", () => {
    for (const [name, events, view] of SCENES) {
      for (const { entry, cues } of planAll(events, view)) {
        for (const cue of cues) {
          expect(cue.kind === "banner" ? 0 : cue.delayMs, `${name}: ${cue.kind} starts in its entry`).toBeLessThanOrEqual(entry.durationMs);
          expect(endOf(cue), `${name}: ${cue.kind} ends within the tail`).toBeLessThanOrEqual(entry.durationMs + FX_MAX_TAIL_MS);
        }
      }
    }
  });

  it("R3336000 no cue names a card's definition, and each entry plans cues for something the view shows (R202)", () => {
    const names = [...Object.values(SHIP_LAYER_IDS), ...Object.values(SHIP_DRONE_IDS), "stinkyscape"];
    for (const [name, events, view] of SCENES) {
      const planned = planAll(events, view);
      expect(planned.some(({ cues }) => cues.length > 0), name).toBe(true);
      const text = JSON.stringify(planned.map(({ cues }) => cues));
      for (const defId of names) expect(text, `${name} names ${defId}`).not.toContain(defId);
    }
  });

  it("R3336000 a card the view cannot place plans nothing: a hidden id, or a layer the seat is not shown", () => {
    const empty = baseView({ you: emptySide("p1"), opponent: emptySide("p2") });
    for (const events of [
      [redirected("hidden")],
      [hit(3, "hidden")],
      [destroyed("hidden", "hidden")],
      [attacked("hidden", "hero-p2")],
      [redirected()],
      [hit(3, "sh")],
      [destroyed("me", MELTDOWN)],
    ]) {
      expect(planAll(events, empty).flatMap(({ cues, replaces }) => [...cues, ...replaces]), events[0]?.type).toEqual([]);
    }
    // The planner reads the view's units, never the event's own defId, so a layer drawn on the board is the only way in.
    const hiddenLayer = unit("p1", { instanceId: "sh", defId: "hidden", armor: 4, buried: 2 });
    expect(planAll([destroyed("sh", "hidden")], board(hiddenLayer))[0]?.cues).toEqual([]);
  });

  it("R3336000 the effects off, an entry of no duration or one that plays as a whole plans nothing", () => {
    const view = board(shield());
    const [first] = planEntries([redirected()], withEvents(view, [redirected()]), false);
    if (first === undefined) throw new Error("no entry");
    expect(planBattleship(first, first.view, envOf(0))).toBeNull();
    expect(planBattleship({ ...first, durationMs: 0 }, first.view, envOf())).toBeNull();
    expect(planBattleship(first, first.view, envOf())).not.toBeNull();
    for (const shape of [{ zone: {} }, { sweep: {} }, { wipe: {} }, { fuse: {} }, { launch: {} }]) {
      expect(planBattleship({ ...first, ...shape } as unknown as AnimationEntry, first.view, envOf())).toBeNull();
    }
  });

  it("R3336000 planFx draws the dome in place of the redirect's and the armor's rows", () => {
    const events = [redirected(), absorbed(3)];
    const view = board(shield());
    const env = envOf();
    const planned = planEntries(events, withEvents(view, [...events]), false);
    const all = planned.flatMap((entry) => {
      env.memory.remember(entry.events);
      return planFx(entry, entry.view, env);
    });
    // Without the planner each row would draw its own shield (the armor row's full bloom) and arcane redirect ring.
    const shields = ofKind(all, "shield").filter((cue) => cue.size === "full");
    expect(shields).toHaveLength(2);
    for (const cue of shields) expect(cue.at).toEqual(tid(card("sh"), FX_SHIP_DOME_AT));
    expect(ofKind(all, "ring").some((ring) => ring.preset === "arcane")).toBe(false);
    expect(shipsOf(all)).toHaveLength(1);
  });
});
