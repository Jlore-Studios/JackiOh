// R2789000: the combat keyword beats (fx/motifs/beats.ts) pinned on fixture streams built from #2783's `via`
// and First Strike marker, each held inside its entry (R200), scaled through it (R201), silent under
// reduced motion but for a still callout, and blind to what the view hides (R202). Planned the way FxLayer
// plans them: remember an entry's events, then plan it. Issue #2790 pins the defensive ten the same way.

import type { GameEvent, Keyword, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { KEYWORD_CALLOUT_ICONS } from "../../cards/keywordVisuals.ts";
import { planEntries, type AnimationEntry } from "../../game/animations.ts";
import { testid } from "../../game/contract.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { must } from "../../test/stubServer.ts";
import { FX_CALLOUT_MS, FX_CALLOUTS_PER_HIT, FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS } from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { planStage } from "../stage.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";
import { FX_BEAT_KEYWORDS, FX_DEFENSIVE_BEAT_KEYWORDS, planBeats, planStillCallouts } from "./beats.ts";

const VIEW = fullBoardView();
const id = (side: "you" | "opponent", index: number): string => must(VIEW[side].units[index], `${side} unit ${index}`).instanceId;

/** The viewer's lane-1 unit: Taunt only (a beat reads no keyword of it unless a test gives it one). */
const MINE = id("you", 0);
/** The viewer's lane-2 unit: every keyword, Windfury, Flying and Ranged included. */
const POISONER = id("you", 1);
/** The viewer's lane-3 unit: no keyword. */
const MINE_3 = id("you", 2);
const FOE = id("opponent", 0);
const FOE_2 = id("opponent", 1);
const FOE_3 = id("opponent", 2);

const envOf = (over: Partial<FxPlanEnv> = {}): FxPlanEnv => ({ intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over });

/** The board with `MINE`'s keywords replaced. */
function viewWith(keywords: readonly Keyword[]): PlayerView {
  const view = fullBoardView();
  const first = must(view.you.units[0], "the viewer's first unit");
  view.you.units[0] = { ...first, keywords: [...keywords] };
  return view;
}

/** The board with one unit lane emptied: R2791000's Stack answers an occupied landing slot, so a non-Stack `summoned` pin lands here. */
function openLane(view: PlayerView, lane: number): PlayerView {
  view.you.units[lane - 1] = null;
  return view;
}

const attack = (attackerId: string, targetId: string): GameEvent => ({ type: "attackDeclared", attackerId, targetId, forced: false });
const hitOf = (sourceId: string | null, targetId: string, amount: number, via?: Extract<GameEvent, { type: "damage" }>["via"], combat = true): GameEvent => ({
  type: "damage",
  sourceId,
  targetId,
  amount,
  combat,
  ...(via === undefined ? {} : { via }),
});
const diedOf = (instanceId: string, killerId: string | null, via?: Extract<GameEvent, { type: "destroyed" }>["via"]): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId: "core-013",
  owner: "p2",
  controller: "p2",
  attack: 3,
  maxHealth: 5,
  killerId,
  ...(via === undefined ? {} : { via }),
});

type Played = { entry: AnimationEntry; beats: ReturnType<typeof planBeats>; env: FxPlanEnv };

/** Each entry of `events` as FxLayer plans it on `start`: remember its events, then plan; `D` overrides every entry's duration. */
function stream(events: readonly GameEvent[], view: PlayerView = fullBoardView(), D?: number, env: FxPlanEnv = envOf()): Played[] {
  return planEntries(events, withEvents(view, [...events]), false).map((planned) => {
    const entry = D === undefined ? planned : { ...planned, durationMs: D };
    env.memory.remember(entry.events);
    return { entry, beats: planBeats(entry, entry.view, env), env };
  });
}

const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);

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

/** R200: every cue starts inside the entry, a flight lands inside it and no tail outlives FX_MAX_TAIL_MS. */
function expectInside(cues: readonly FxCue[], D: number, label: string): void {
  for (const cue of cues) {
    const name = `${label} ${cue.kind} at D=${String(D)}`;
    expect(cue.delayMs, name).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, name).toBeLessThanOrEqual(D);
    if (cue.kind === "shot" || cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, name).toBeLessThanOrEqual(D);
    expect(endOf(cue), name).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    if (cue.kind === "shake") expect(cue.trauma).toBeLessThanOrEqual(1);
  }
}

/** The beats of the entry `at` of `events`, pinned at its table duration and held inside it at three others. */
function pinned(events: readonly GameEvent[], at: number, view: PlayerView = fullBoardView()): FxCue[] {
  const own = must(stream(events, view)[at], "the entry");
  for (const D of [120, 300, 1200]) {
    const cues = must(stream(events, view, D)[at], "the entry").beats.cues;
    expectInside(cues, D, `entry ${String(at)}`);
  }
  return own.beats.cues;
}

const cardOf = (instanceId: string): { kind: "testid"; testid: string } => ({ kind: "testid", testid: testid.card(instanceId) });

describe("R2789000 the combat keyword beats", () => {
  it("R2789000 Cleave: the blow continues as an arc from the primary target to each neighbour, which takes a smaller splat", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 3), hitOf(MINE, FOE_3, 2, "Cleave")];
    expect(kinds(stream(events)[1]?.beats.cues ?? [])).toEqual([]);
    expect(kinds(pinned(events, 2))).toEqual(["callout@0", "shot@60", "burst@180", "splat@180", "shake@180"]);
    const cues = must(stream(events)[2], "the Cleave entry").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Cleave", at: cardOf(MINE), durationMs: FX_CALLOUT_MS });
    expect(cues[1]).toMatchObject({ kind: "shot", shot: { motion: "swipe" }, from: cardOf(FOE_2), to: cardOf(FOE_3) });
    // The beat's splat is the entry's only one: the row's own is replaced.
    const played = must(stream(events)[2], "the Cleave entry");
    const splats = planFx(played.entry, played.entry.view, played.env).filter((cue) => cue.kind === "splat");
    expect(splats.map((cue) => cue.delayMs)).toEqual([180]);
  });

  it("R2789000 Cleave: a hit Armor took whole flashes the full shield at the landing instead of the row's", () => {
    const events: GameEvent[] = [attack(MINE, FOE_2), { type: "damageAbsorbed", sourceId: MINE, targetId: FOE_3, absorbed: 2, combat: true, via: "Cleave" }];
    const cues = pinned(events, 1);
    expect(kinds(cues)).toEqual(["callout@0", "shot@60", "burst@180", "shield@180", "ring@180", "burst@180", "shake@180"]);
  });

  it("R2789000 Trample: the overflow carries on past the dead unit as a bolt to the hero, with a second splat there", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 5), diedOf(FOE_2, MINE), hitOf(MINE, "hero-p2", 2, "Trample")];
    expect(kinds(pinned(events, 3))).toEqual(["callout@0", "shot@60", "burst@180", "splat@180", "shake@180"]);
    const cues = must(stream(events)[3], "the Trample entry").beats.cues;
    expect(cues[1]).toMatchObject({ kind: "shot", shot: { motion: "bolt" }, from: cardOf(FOE_2), to: { kind: "testid", testid: testid.hero("opponent") } });
    const played = must(stream(events)[3], "the Trample entry");
    expect(planFx(played.entry, played.entry.view, played.env).filter((cue) => cue.kind === "splat")).toHaveLength(1);
  });

  it("R2789000 Pierce: a spear line runs through the defender from the attacker's side, the damage row kept", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 3, "Pierce")];
    expect(kinds(pinned(events, 1))).toEqual(["callout@0", "shot@60", "burst@180"]);
    const spear = must(stream(events)[1], "the Pierce entry").beats.cues[1];
    expect(spear).toMatchObject({ kind: "shot", shot: { motion: "beam" } });
    // Up through an opponent's unit; down through the viewer's own.
    if (spear?.kind !== "shot" || spear.from.kind !== "testid" || spear.to.kind !== "testid") throw new Error("a beam between two testids");
    expect(spear.from.at?.y).toBeGreaterThan(1);
    expect(spear.to.at?.y).toBeLessThan(0);
    const own = [attack(FOE_2, MINE), hitOf(FOE_2, MINE, 3, "Pierce")];
    const down = must(stream(own)[1], "the Pierce entry").beats.cues[1];
    if (down?.kind !== "shot" || down.from.kind !== "testid" || down.to.kind !== "testid") throw new Error("a beam between two testids");
    expect(down.from.at?.y).toBeLessThan(0);
    expect(down.to.at?.y).toBeGreaterThan(1);
    const played = must(stream(events)[1], "the Pierce entry");
    expect(planFx(played.entry, played.entry.view, played.env).filter((cue) => cue.kind === "splat")).toHaveLength(1);
  });

  it("R2789000 First Strike's strike: a quick-draw glint on the attacker, the damage row kept", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 3, "First Strike")];
    expect(kinds(pinned(events, 1))).toEqual(["callout@0", "sheen@0", "burst@0"]);
    expect(must(stream(events)[1], "the strike").beats.cues[1]).toMatchObject({ kind: "sheen", at: cardOf(MINE) });
  });

  it("R2789000 First Strike: a dying defender's counter breaks off, and no counter-blow lands on the attacker in any entry", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 9, "First Strike"), diedOf(FOE_2, MINE, "First Strike")];
    expect(kinds(pinned(events, 2))).toEqual(["callout@0", "projectile@0", "burst@140"]);
    const played = stream(events);
    const breaking = must(played[2], "the death").beats.cues[1];
    expect(breaking).toMatchObject({ kind: "projectile", from: cardOf(FOE_2) });
    for (const { entry, env } of played) {
      const cues = planFx(entry, entry.view, env);
      const onAttacker = cues.filter((cue) => cue.kind === "splat" && cue.at.kind === "testid" && cue.at.testid === testid.card(MINE));
      expect(onAttacker, entry.type).toEqual([]);
    }
  });

  it("R2789000 First Strike: a survivor's counter lands after the beat, not at once", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 1, "First Strike"), hitOf(FOE_2, MINE, 3)];
    expect(kinds(pinned(events, 2))).toEqual(["burst@180", "splat@180", "shake@180"]);
    const played = must(stream(events)[2], "the counter");
    const splats = planFx(played.entry, played.entry.view, played.env).filter((cue) => cue.kind === "splat");
    expect(splats.map((cue) => cue.delayMs)).toEqual([180]);
    // Without a First Strike before it, the same blow keeps its own row: it lands at 0.
    const plain = must(stream([attack(MINE, FOE_2), hitOf(MINE, FOE_2, 1), hitOf(FOE_2, MINE, 3)])[2], "the blow");
    expect(plain.beats.cues).toEqual([]);
  });

  it("R2789000 Lifesteal: red motes stream from the damage back to the hero, landing with the heal's splat", () => {
    const events: GameEvent[] = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 3), { type: "healed", targetId: "hero-p1", amount: 3, via: "Lifesteal" }];
    expect(kinds(pinned(events, 2))).toEqual(["callout@0", "projectile@0", "burst@180", "splat@180"]);
    const cues = must(stream(events)[2], "the heal").beats.cues;
    expect(cues[1]).toMatchObject({ kind: "projectile", preset: "blood", from: cardOf(FOE_2), to: { kind: "testid", testid: testid.hero("you") } });
    expect(cues[3]).toMatchObject({ kind: "splat", tone: "heal", amount: 3 });
    // The heal row's own rays and splat are replaced.
    const played = must(stream(events)[2], "the heal");
    const row = planFx(played.entry, played.entry.view, played.env);
    expect(row.filter((cue) => cue.kind === "splat")).toHaveLength(1);
    expect(row.some((cue) => cue.kind === "rays")).toBe(false);
  });

  it("R2789000 Windfury: the second attack of the turn starts as an after-image of the first, the first nothing", () => {
    const view = viewWith([{ kind: "Windfury" }]);
    const events = [attack(MINE, FOE), attack(MINE, FOE_2)];
    expect(kinds(pinned(events, 0, view))).toEqual([]);
    expect(kinds(pinned(events, 1, view))).toEqual(["callout@0", "ghost@0", "burst@0"]);
    expect(must(stream(events, view)[1], "the second attack").beats.cues[1]).toMatchObject({ kind: "ghost", from: cardOf(MINE), to: cardOf(FOE) });
  });

  it("R2789000 Windfury: a new turn forgets the first attack", () => {
    const view = viewWith([{ kind: "Windfury" }]);
    const events: GameEvent[] = [attack(MINE, FOE), { type: "turnStarted", player: "p2", turn: 3 }, attack(MINE, FOE_2)];
    const played = stream(events, view);
    expect(played.map((one) => kinds(one.beats.cues))).toEqual([[], [], []]);
  });

  it("R2789000 Poisonous is keyed on the death's `via`: a callout over the poisoner, the poison burst the death row's", () => {
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 1), diedOf(FOE_2, MINE, "Poisonous")];
    expect(kinds(pinned(events, 2))).toEqual(["callout@0"]);
    expect(must(stream(events)[2], "the death").beats.cues[0]).toMatchObject({ keyword: "Poisonous", at: cardOf(MINE) });
    const played = must(stream(events)[2], "the death");
    const poison = (cues: readonly FxCue[]): string[] => kinds(cues.filter((cue) => cue.kind === "burst" && cue.preset === "poison"));
    expect(poison(planFx(played.entry, played.entry.view, played.env))).toEqual(["burst@0"]);
    // No `via`, no poison and no callout, though the attacker is Poisonous in the view.
    const bare = stream([attack(POISONER, FOE_2), hitOf(POISONER, FOE_2, 1), diedOf(FOE_2, POISONER)]);
    const death = must(bare[2], "the death");
    expect(death.beats.cues).toEqual([]);
    expect(poison(planFx(death.entry, death.entry.view, death.env))).toEqual([]);
  });

  it("R2789000 Ranged keeps its flavor shot and adds a callout", () => {
    const view = viewWith([{ kind: "Ranged" }]);
    const events = [attack(MINE, FOE_2)];
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0"]);
    const played = must(stream(events, view)[0], "the attack");
    expect(planStage(played.entry, played.entry.view, played.env).some((cue) => cue.kind === "shot")).toBe(true);
    expect(planStage(played.entry, played.entry.view, played.env).find((cue) => cue.kind === "lunge")).toMatchObject({ still: true });
  });

  it("R2789000 Flying: a callout, and its lunge arcs over the lane", () => {
    const view = viewWith([{ kind: "Flying" }]);
    const events = [attack(MINE, FOE_2)];
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0"]);
    const played = must(stream(events, view)[0], "the attack");
    expect(planStage(played.entry, played.entry.view, played.env).find((cue) => cue.kind === "lunge")).toMatchObject({ arc: true });
    // A walker's lunge stays along the lane.
    const plain = must(stream(events, viewWith([]))[0], "the attack");
    expect(planStage(plain.entry, plain.entry.view, plain.env).find((cue) => cue.kind === "lunge")).not.toHaveProperty("arc");
  });

  it("R2784000 R2789000 Divine Shield: the ward row's pop gains only a callout, no splat, so the hit did nothing", () => {
    const events: GameEvent[] = [{ type: "divineShieldLost", instanceId: FOE }];
    expect(kinds(pinned(events, 0))).toEqual(["callout@0"]);
    const cues = must(stream(events)[0], "the pop").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Divine Shield", at: cardOf(FOE), durationMs: FX_CALLOUT_MS });
    // The ward row still draws the gold ring and shards, and no number lands.
    const played = must(stream(events)[0], "the pop");
    const row = planFx(played.entry, played.entry.view, played.env);
    expect(row.filter((cue) => cue.kind === "ring")).toHaveLength(1);
    expect(row.filter((cue) => cue.kind === "burst")).toHaveLength(1);
    expect(row.filter((cue) => cue.kind === "splat")).toEqual([]);
  });

  it("R1361 R2789000 Armor: the absorbed amount lands as a grey splat beside the ward row's flash", () => {
    const events: GameEvent[] = [{ type: "damageAbsorbed", sourceId: null, targetId: FOE_2, absorbed: 2, combat: false }];
    expect(kinds(pinned(events, 0))).toEqual(["callout@0", "splat@180", "burst@180"]);
    const cues = must(stream(events)[0], "the absorption").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Armor", at: cardOf(FOE_2), durationMs: FX_CALLOUT_MS });
    expect(cues[1]).toMatchObject({ kind: "splat", tone: "absorb", amount: 2, at: cardOf(FOE_2) });
    // The ward row's shield flash still draws in planFx.
    const played = must(stream(events)[0], "the absorption");
    const row = planFx(played.entry, played.entry.view, played.env);
    expect(row.filter((cue) => cue.kind === "shield")).toHaveLength(1);
    expect(row.filter((cue) => cue.kind === "splat" && cue.tone === "absorb")).toHaveLength(1);
    // A routed hit's own combat beat draws it instead: Cleave keeps its full shield, with no Armor beat.
    const routed: GameEvent[] = [attack(MINE, FOE_2), { type: "damageAbsorbed", sourceId: MINE, targetId: FOE_3, absorbed: 2, combat: true, via: "Cleave" }];
    const blow = must(stream(routed)[1], "the routed absorption").beats.cues;
    expect(blow.some((cue) => cue.kind === "callout" && cue.keyword === "Armor")).toBe(false);
    expect(blow.some((cue) => cue.kind === "splat" && cue.tone === "absorb")).toBe(false);
  });

  it("R1361 R2789000 Indestructible: a hit its Armor takes whole, lethal and with no destroyed after it, flashes and rings", () => {
    // MINE has 1 health: a hit of 2 its Armor takes whole is lethal (R1361 reports it), and the
    // unit does not die. A `damage` event could never key this: the engine stops a hit meeting
    // Indestructible before any event, so the pin keys on the absorption it does report.
    const view = viewWith([
      { kind: "Armor", n: 2 },
      { kind: "Indestructible" },
    ]);
    const events: GameEvent[] = [{ type: "damageAbsorbed", sourceId: null, targetId: MINE, absorbed: 2, combat: false }];
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0", "splat@180", "burst@180", "callout@60", "sheen@60", "burst@60", "ring@204", "shake@204"]);
    const cues = must(stream(events, view)[0], "the absorption").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Armor", at: cardOf(MINE) });
    expect(cues[3]).toMatchObject({ kind: "callout", keyword: "Indestructible", at: cardOf(MINE) });
    // The ward row's shield flash and the Armor beat's grey splat stay: the beat adds the flash.
    const played = must(stream(events, view)[0], "the absorption");
    const row = planFx(played.entry, played.entry.view, played.env);
    expect(row.filter((cue) => cue.kind === "shield")).toHaveLength(1);
    expect(row.filter((cue) => cue.kind === "splat" && cue.tone === "absorb")).toHaveLength(1);
  });

  it("R1361 R2789000 Indestructible: no beat when the absorption is not lethal, is routed, or the unit dies after all", () => {
    const view = viewWith([
      { kind: "Armor", n: 2 },
      { kind: "Indestructible" },
    ]);
    // POISONER has 7 health with Armor 2 and Indestructible: 2 absorbed whole is not lethal, so the
    // Armor beat plays alone.
    const scratch = stream([{ type: "damageAbsorbed", sourceId: null, targetId: POISONER, absorbed: 2, combat: false }], fullBoardView());
    const calls = scratch.flatMap(({ beats }) => beats.cues.filter((cue) => cue.kind === "callout"));
    expect(calls).toEqual([expect.objectContaining({ keyword: "Armor" })]);
    // A routed absorption is its combat beat's: Cleave draws it, with no Indestructible beat.
    const routed: GameEvent[] = [attack(FOE_2, MINE), { type: "damageAbsorbed", sourceId: FOE_2, targetId: MINE, absorbed: 2, combat: true, via: "Cleave" }];
    const blow = must(stream(routed, view)[1], "the routed absorption").beats.cues;
    expect(blow.some((cue) => cue.kind === "callout" && cue.keyword === "Indestructible")).toBe(false);
    expect(blow.some((cue) => cue.kind === "callout" && cue.keyword === "Cleave")).toBe(true);
    // A lethal absorption the destroyed follows draws none either: a sacrifice still kills it, so
    // the memory already knowing the destroyed means no "it does not die" beat.
    const events: GameEvent[] = [
      { type: "damageAbsorbed", sourceId: null, targetId: MINE, absorbed: 2, combat: false },
      diedOf(MINE, FOE_2),
    ];
    const entries = planEntries(events, withEvents(view, events), false);
    const absorption = must(
      entries.find((entry) => entry.events.some((event) => event.type === "damageAbsorbed")),
      "the absorption",
    );
    const env = envOf();
    env.memory.remember(events);
    const survived = planBeats(absorption, absorption.view, env).cues;
    expect(survived.filter((cue) => cue.kind === "callout")).toEqual([expect.objectContaining({ keyword: "Armor" })]);
  });

  it("R23 R386 R2789000 Immutable: gaining the keyword snaps a glassy shell shut", () => {
    // A nerf, tune-up, transform or number set aimed at an Immutable unit is refused with no event,
    // so no bounce stream can occur live; the pin keys on the grant that raises the shell instead.
    const arrival: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Immutable" } }];
    expect(kinds(pinned(arrival, 0))).toEqual(["callout@0", "ring@40", "burst@120"]);
    const cues = must(stream(arrival)[0], "the arrival").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Immutable", at: cardOf(MINE_3) });
    expect(cues[1]).toMatchObject({ kind: "ring", preset: "arcane" });
    // Losing Immutable snaps nothing shut, and another keyword's grant draws no Immutable beat.
    const lost: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Immutable" }, lost: true }];
    expect(stream(lost)[0]?.beats.cues).toEqual([]);
    const other: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Taunt" } }];
    expect(must(stream(other)[0], "the other grant").beats.cues.some((cue) => cue.kind === "callout" && cue.keyword === "Immutable")).toBe(false);
  });

  it("R2783001 R2789000 Reborn: a summoned with via Reborn rises from ashes in its own lane", () => {
    // Lane 5 stands open: R2791000's Stack answers an occupied landing slot, and a Reborn
    // return lands in an empty lane live. The holder is still MINE's card, which the view shows.
    const view = openLane(fullBoardView(), 5);
    const reborned: GameEvent = { type: "summoned", player: "p1", instanceId: MINE, defId: "core-004", row: "units", lane: 5, via: "Reborn" };
    expect(kinds(pinned([reborned], 0, view))).toEqual(["callout@0", "ghost@50", "burst@150", "ring@150"]);
    const cues = must(stream([reborned], view)[0], "the return").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Reborn", at: cardOf(MINE), durationMs: FX_CALLOUT_MS });
    expect(cues[1]).toMatchObject({ kind: "ghost", from: { kind: "testid", testid: testid.zone("you", "units", 5) }, to: cardOf(MINE) });
    // The same arrival without `via` draws no beat.
    const arrival: GameEvent = { type: "summoned", player: "p1", instanceId: MINE, defId: "core-004", row: "units", lane: 5 };
    expect(stream([arrival], view)[0]?.beats.cues).toEqual([]);
  });

  it("R385 R2789000 Brittle: damage grows cracks on the unit, and its crumble gains only a callout", () => {
    const view = viewWith([{ kind: "Brittle", n: 2 }]);
    const events = [attack(FOE_2, MINE), hitOf(FOE_2, MINE, 1)];
    expect(kinds(pinned(events, 1, view))).toEqual(["callout@0", "crack@180"]);
    const cues = must(stream(events, view)[1], "the hit").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Brittle", at: cardOf(MINE) });
    expect(cues[1]).toMatchObject({ kind: "crack", at: cardOf(MINE) });
    // The crumble's shatter stays the death row's: the beat is the callout, the bursts the row's.
    const crumbled: GameEvent[] = [{ type: "crumbled", instanceId: MINE, defId: "core-004", owner: "p1", zone: "field" }];
    expect(kinds(pinned(crumbled, 0, view))).toEqual(["callout@0"]);
    const played = must(stream(crumbled, view)[0], "the crumble");
    expect(planFx(played.entry, played.entry.view, played.env).filter((cue) => cue.kind === "burst").length).toBeGreaterThan(0);
    // A plain unit's hit grows no cracks.
    expect(stream(events)[1]?.beats.cues).toEqual([]);
    // A crumble the view does not show as Brittle draws no beat (R202): the death row alone.
    // MINE_3 carries no keyword, so this is the flavorExits pin's stream staying silent.
    const plainCrumble: GameEvent[] = [{ type: "crumbled", instanceId: MINE_3, defId: "core-017", owner: "p1", zone: "field" }];
    expect(stream(plainCrumble)[0]?.beats.cues).toEqual([]);
  });

  it("R81 R2789000 Immune to Spells: gaining the keyword raises a rune ward", () => {
    // No declaration offers an Immune unit and every resolving Spell effect passes it by in silence,
    // so no fizzle stream can occur live; the pin keys on the grant that raises the ward instead,
    // in the `negate` motif's ring-and-smoke vocabulary.
    const arrival: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Immune to Spells" } }];
    expect(kinds(pinned(arrival, 0))).toEqual(["callout@0", "ring@0", "burst@120"]);
    const cues = must(stream(arrival)[0], "the arrival").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Immune to Spells", at: cardOf(MINE_3) });
    expect(cues[1]).toMatchObject({ kind: "ring", preset: "circuit" });
    // Losing the ward raises nothing, and another keyword's grant draws no Immune beat.
    const lost: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Immune to Spells" }, lost: true }];
    expect(stream(lost)[0]?.beats.cues).toEqual([]);
    const other: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Taunt" } }];
    expect(must(stream(other)[0], "the other grant").beats.cues.some((cue) => cue.kind === "callout" && cue.keyword === "Immune to Spells")).toBe(false);
  });

  it("R46 R2789000 Taunt: arrival raises a dust-ring wall, and a forced attack pulls it to the attacker", () => {
    const arrival: GameEvent[] = [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Taunt" } }];
    expect(kinds(pinned(arrival, 0))).toEqual(["callout@0", "ring@40"]);
    const granted = must(stream(arrival)[0], "the arrival").beats.cues;
    expect(granted[0]).toMatchObject({ kind: "callout", keyword: "Taunt", at: cardOf(MINE_3) });
    expect(granted[1]).toMatchObject({ kind: "ring", preset: "dust" });
    // Losing Taunt raises no wall.
    const lost: GameEvent[] = [{ type: "keywordGranted", instanceId: FOE, keyword: { kind: "Taunt" }, lost: true }];
    expect(stream(lost)[0]?.beats.cues).toEqual([]);
    // A forced attack on the wall pulls it toward the attacker; an unforced one does not.
    const forced: GameEvent[] = [{ type: "attackDeclared", attackerId: FOE_2, targetId: MINE, forced: true }];
    expect(kinds(pinned(forced, 0))).toEqual(["callout@0", "ghost@70"]);
    const pull = must(stream(forced)[0], "the forced attack").beats.cues;
    expect(pull[0]).toMatchObject({ kind: "callout", keyword: "Taunt", at: cardOf(MINE) });
    expect(pull[1]).toMatchObject({ kind: "ghost", from: cardOf(MINE), to: cardOf(FOE_2) });
    const free: GameEvent[] = [{ type: "attackDeclared", attackerId: FOE_2, targetId: MINE, forced: false }];
    expect(stream(free)[0]?.beats.cues).toEqual([]);
  });

  it("R53 R2789000 Can't attack: a forced attack naming the chained unit clinks", () => {
    // A declared attack by it is refused with no event, so no cancellation stream can occur live;
    // a forced attack skips the validator (R53), so the pin keys on a forced declaration instead.
    const view = viewWith([{ kind: "Can't attack" }]);
    const events: GameEvent[] = [{ type: "attackDeclared", attackerId: MINE, targetId: FOE, forced: true }];
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0", "ring@70", "burst@210"]);
    const cues = must(stream(events, view)[0], "the forced attack").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Can't attack", at: cardOf(MINE) });
    // A non-forced declaration by it is what the engine refuses: no beat. Another unit's forced
    // attack draws none either (MINE shows no Taunt here to pull).
    const declared: GameEvent[] = [{ type: "attackDeclared", attackerId: MINE, targetId: FOE, forced: false }];
    expect(stream(declared, view)[0]?.beats.cues).toEqual([]);
    const others: GameEvent[] = [{ type: "attackDeclared", attackerId: FOE_2, targetId: MINE, forced: true }];
    expect(stream(others, view)[0]?.beats.cues).toEqual([]);
  });

  it("R2657000 R2789000 Can't Defend: a skipped position switch clinks chains over the unit", () => {
    const view = viewWith([{ kind: "Can't Defend" }]);
    const events: GameEvent[] = [{ type: "positionSwitched", instanceId: MINE, position: "ATK" }];
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0", "ring@50", "burst@150"]);
    const cues = must(stream(events, view)[0], "the switch").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Can't Defend", at: cardOf(MINE) });
    // Another unit's switch draws no beat.
    expect(stream(events)[0]?.beats.cues).toEqual([]);
  });

  it("R2789000 several keywords on one hit stagger by FX_BEAT_STAGGER_MS, and at most FX_CALLOUTS_PER_HIT pop a callout", () => {
    // POISONER has Windfury, Flying and Ranged: three beats on its second attack. Its Can't attack
    // stays silent: neither declaration is forced, and only a forced one can name it (R53).
    const events = [attack(POISONER, FOE), attack(POISONER, FOE_2)];
    expect(kinds(pinned(events, 1))).toEqual(["callout@0", "ghost@0", "burst@0", "callout@60"]);
    const cues = must(stream(events)[1], "the second attack").beats.cues;
    expect(cues.filter((cue) => cue.kind === "callout")).toHaveLength(FX_CALLOUTS_PER_HIT);
    expect(cues.filter((cue) => cue.kind === "callout").map((cue) => (cue.kind === "callout" ? cue.keyword : ""))).toEqual(["Windfury", "Flying"]);
    // The first attack has Flying and Ranged: two callouts, the second staggered; R2791000: and
    // POISONER's Charge and Rush dash behind them, staggered third and fourth, with no callout left.
    expect(kinds(must(stream(events)[0], "the first attack").beats.cues)).toEqual([
      "callout@0",
      "callout@60",
      "shot@120",
      "burst@258",
      "shake@258",
      "shot@175",
      "burst@280",
      "shake@280",
    ]);
  });

  it("R201 R2789000 speed scales every beat: the same cues at 0.5, 1 and 2, each within 4 ms of its scaled time", () => {
    const cases: { events: GameEvent[]; at: number; D: number; view?: PlayerView }[] = [
      { events: [attack(POISONER, FOE), attack(POISONER, FOE_2)], at: 1, D: 350 },
      { events: [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 3), hitOf(MINE, FOE_3, 2, "Cleave")], at: 2, D: 300 },
      { events: [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 9, "First Strike"), diedOf(FOE_2, MINE, "First Strike")], at: 2, D: 350 },
      { events: [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 3), { type: "healed", targetId: "hero-p1", amount: 3, via: "Lifesteal" }], at: 2, D: 300 },
      {
        events: [{ type: "damageAbsorbed", sourceId: null, targetId: FOE_2, absorbed: 2, combat: false }],
        at: 0,
        D: 300,
      },
      {
        events: [{ type: "summoned", player: "p1", instanceId: MINE, defId: "core-004", row: "units", lane: 1, via: "Reborn" }],
        at: 0,
        D: 250,
      },
      // The re-keyed beats scale too: the Indestructible absorption, the Immutable arrival and the
      // forced declaration of a chained unit.
      {
        events: [{ type: "damageAbsorbed", sourceId: null, targetId: MINE, absorbed: 2, combat: false }],
        at: 0,
        D: 300,
        view: viewWith([
          { kind: "Armor", n: 2 },
          { kind: "Indestructible" },
        ]),
      },
      {
        events: [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Immutable" } }],
        at: 0,
        D: 300,
      },
      {
        events: [{ type: "attackDeclared", attackerId: MINE, targetId: FOE, forced: true }],
        at: 0,
        D: 300,
        view: viewWith([{ kind: "Can't attack" }]),
      },
    ];
    for (const { events, at, D, view } of cases) {
      const seen = view ?? fullBoardView();
      const base = must(stream(events, seen, D)[at], "the entry").beats.cues;
      expect(base.length).toBeGreaterThan(0);
      for (const speed of [0.5, 2]) {
        const cues = must(stream(events, seen, D / speed)[at], "the entry").beats.cues;
        expect(cues.map((cue) => cue.kind)).toEqual(base.map((cue) => cue.kind));
        cues.forEach((cue, k) => {
          expect(Math.abs(cue.delayMs - (base[k]?.delayMs ?? 0) / speed), `${cue.kind} at speed ${String(speed)}`).toBeLessThanOrEqual(4);
        });
      }
    }
  });

  it("R2789000 reduced motion keeps only still callouts: no motion cue, and no beat on an entry of no duration", () => {
    const events = [attack(POISONER, FOE), attack(POISONER, FOE_2)];
    const played = must(stream(events)[1], "the second attack");
    const still = { ...played.entry, durationMs: 0 };
    expect(planBeats(still, still.view, played.env)).toEqual({ cues: [], replaces: new Set() });
    const callouts = planStillCallouts(still, still.view, played.env);
    expect(callouts).toHaveLength(FX_CALLOUTS_PER_HIT);
    expect(callouts.every((cue) => cue.kind === "callout" && cue.still === true && cue.delayMs === 0 && cue.durationMs === FX_CALLOUT_MS)).toBe(true);
    expect(callouts.map((cue) => (cue.kind === "callout" ? cue.keyword : ""))).toEqual(["Windfury", "Flying"]);
    for (const cue of callouts) expectInside([cue], 0, "a still callout");
    // A hit with no keyword draws none.
    const plain = must(stream([attack(MINE, FOE_2), hitOf(MINE, FOE_2, 1)])[1], "the hit");
    expect(planStillCallouts(plain.entry, plain.entry.view, plain.env)).toEqual([]);
    // A defensive beat keeps its still callout too: Reborn's return rises still. Lane 5
    // stands open, as in its pin: Stack answers an occupied landing slot, not this return.
    const rebornView = openLane(fullBoardView(), 5);
    const reborn = must(stream([{ type: "summoned", player: "p1", instanceId: MINE, defId: "core-004", row: "units", lane: 5, via: "Reborn" }], rebornView)[0], "the return");
    const restill = { ...reborn.entry, durationMs: 0 };
    const rebornStill = planStillCallouts(restill, restill.view, reborn.env);
    expect(rebornStill).toHaveLength(1);
    expect(rebornStill[0]).toMatchObject({ kind: "callout", keyword: "Reborn", still: true, delayMs: 0, durationMs: FX_CALLOUT_MS });
    for (const cue of rebornStill) expectInside([cue], 0, "a still callout");
  });

  it("R2789000 intensity off draws nothing, still callouts included", () => {
    const events = [attack(POISONER, FOE), attack(POISONER, FOE_2)];
    const env = envOf({ intensity: FX_INTENSITY_SCALE.off });
    for (const { entry } of stream(events, fullBoardView(), undefined, env)) {
      expect(planBeats(entry, entry.view, env)).toEqual({ cues: [], replaces: new Set() });
      expect(planStillCallouts(entry, entry.view, env)).toEqual([]);
    }
    // A defensive entry draws nothing either.
    const defensive: GameEvent[] = [{ type: "divineShieldLost", instanceId: FOE }];
    for (const { entry } of stream(defensive, fullBoardView(), undefined, env)) {
      expect(planBeats(entry, entry.view, env)).toEqual({ cues: [], replaces: new Set() });
      expect(planStillCallouts(entry, entry.view, env)).toEqual([]);
    }
  });

  it("R202 R2789000 a hidden source keys no callout, a hidden killer's Poisonous death calls out over the unit that died, and no cue names a definition", () => {
    const hidden: GameEvent[] = [attack("hidden", FOE_2), hitOf("hidden", FOE_3, 2, "Cleave"), diedOf(FOE_2, "hidden", "Poisonous")];
    const played = stream(hidden);
    const callouts = played.flatMap(({ beats }) => beats.cues.filter((cue) => cue.kind === "callout"));
    expect(callouts).toEqual([expect.objectContaining({ keyword: "Poisonous", at: cardOf(FOE_2) })]);
    expect(must(played[0], "the attack").beats.cues).toEqual([]);
    expect(JSON.stringify(played.flatMap(({ beats }) => beats.cues))).not.toMatch(/core-\d{3}/);
    // A damage with no `via` (which is how a view hides one) plans none.
    expect(stream([attack(MINE, FOE_2), hitOf("hidden", FOE_3, 2)]).flatMap(({ beats }) => beats.cues)).toEqual([]);
    // A hidden target draws no defensive beat either, even on a Brittle board, and names no definition.
    const shrouded = stream([attack(MINE, FOE_2), hitOf(MINE, "hidden", 2)], viewWith([{ kind: "Brittle", n: 2 }]));
    expect(shrouded.flatMap(({ beats }) => beats.cues)).toEqual([]);
    expect(JSON.stringify(shrouded.flatMap(({ beats }) => beats.cues))).not.toMatch(/core-\d{3}/);
    // A hit Armor takes whole, hidden, draws no Indestructible beat either.
    const swallowed = stream(
      [{ type: "damageAbsorbed", sourceId: null, targetId: "hidden", absorbed: 2, combat: false }],
      viewWith([
        { kind: "Armor", n: 2 },
        { kind: "Indestructible" },
      ]),
    );
    expect(swallowed.flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R2789000 the nine keywords with a beat each have a callout icon, R2791000 the ten tempo ones", () => {
    expect(FX_BEAT_KEYWORDS).toHaveLength(19);
    for (const keyword of FX_BEAT_KEYWORDS) expect(KEYWORD_CALLOUT_ICONS[keyword]?.d, keyword).toMatch(/^M/);
  });

  it("R2789000 the ten defensive keywords with a beat each have a callout icon", () => {
    expect(FX_DEFENSIVE_BEAT_KEYWORDS).toHaveLength(10);
    for (const keyword of FX_DEFENSIVE_BEAT_KEYWORDS) expect(KEYWORD_CALLOUT_ICONS[keyword]?.d, keyword).toMatch(/^M/);
  });
});

describe("R2789000 what the memory keeps for the beats", () => {
  it("R2789000 attacksThisTurn lists an attacker's targets oldest first, until the next turn starts, and never a hidden attacker's", () => {
    const memory = createFxMemory();
    memory.remember([attack(MINE, FOE), attack("hidden", FOE), attack(MINE, FOE_2)]);
    expect(memory.attacksThisTurn(MINE)).toEqual([FOE, FOE_2]);
    expect(memory.attacksThisTurn("hidden")).toEqual([]);
    // A copy: a caller cannot change what the memory holds.
    (memory.attacksThisTurn(MINE) as string[]).push("x");
    expect(memory.attacksThisTurn(MINE)).toEqual([FOE, FOE_2]);
    memory.remember([{ type: "turnStarted", player: "p2", turn: 4 }]);
    expect(memory.attacksThisTurn(MINE)).toEqual([]);
    memory.remember([attack(MINE, FOE_3)]);
    expect(memory.attacksThisTurn(MINE)).toEqual([FOE_3]);
    memory.clear();
    expect(memory.attacksThisTurn(MINE)).toEqual([]);
  });

  it("R2789000 recent lists the last events in order, planned entry last, as a copy", () => {
    const memory = createFxMemory();
    const events = [attack(MINE, FOE), hitOf(MINE, FOE, 1)];
    memory.remember(events);
    expect(memory.recent()).toEqual(events);
    (memory.recent() as GameEvent[]).pop();
    expect(memory.recent()).toEqual(events);
    memory.clear();
    expect(memory.recent()).toEqual([]);
  });

  it("R2789000 a beat reads its history off the events before its entry, not those it was planned with", () => {
    // Planned with no memory of the strike, the survivor's counter is just a hit: it keeps its own row.
    const events = [attack(MINE, FOE_2), hitOf(MINE, FOE_2, 1, "First Strike"), hitOf(FOE_2, MINE, 3)];
    const entries = planEntries(events, withEvents(fullBoardView(), events), false);
    const counter = must(entries[2], "the counter");
    const env = envOf();
    env.memory.remember(counter.events);
    expect(planBeats(counter, counter.view, env).cues).toEqual([]);
    env.memory.clear();
    env.memory.remember(events);
    expect(kinds(planBeats(counter, counter.view, env).cues)).toEqual(["burst@180", "splat@180", "shake@180"]);
  });
});
