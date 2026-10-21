// #1446: a Unit's death exit takes its flavor. The `death` recipe (cues.ts) keeps today's
// motion underneath and appends one exit per flavor from `FLAVOR_EXITS` (exits.ts).
// A death the view does not show face-up keeps today's cue (R202), and reduced motion and
// intensity off keep the death's fallback.

import type { BackrowView, Flavor, GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, animTestid, planEntries, type AnimationEntry } from "../game/animations.ts";
import { testid, type Side } from "../game/contract.ts";
import { faceUpBackrow, fullBoardView, unit, withEvents } from "../test/fixtures.ts";
import { flavorOfId } from "../cards/flavor.ts";
import {
  FX_CRACK_TAIL_MS,
  FX_DEATH_BOLT_FLIGHT_FRACTION,
  FX_DEATH_EMBER_AT,
  FX_DEATH_SMOKE_AT,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_RING_MS,
} from "./constants.ts";
import { planFx } from "./cues.ts";
import { FLAVOR_EXITS, type FlavorExitPlan } from "./exits.ts";
import { chaosFlavor, FLAVOR_LOOKS } from "./flavors.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCue, FxPlanEnv, FxPoint } from "./types.ts";

type Loose = Record<string, unknown>;

const T = FX_MAX_TAIL_MS;
const r = Math.round;

function tid(value: string, point?: FxPoint): { kind: "testid"; testid: string; at?: FxPoint } {
  return point === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at: point };
}

function burst(preset: string, at: ReturnType<typeof tid>, spread: string, delayMs: number): Loose {
  return { kind: "burst", preset, at, spread, delayMs };
}
function ring(preset: string, at: ReturnType<typeof tid>, delayMs: number, D: number): Loose {
  return { kind: "ring", preset, at, delayMs, durationMs: Math.min(FX_RING_MS, D - delayMs + T) };
}
function crack(at: ReturnType<typeof tid>, delayMs: number, D: number): Loose {
  return { kind: "crack", at, delayMs, durationMs: D - delayMs + FX_CRACK_TAIL_MS };
}

function shape(cue: FxCue): Loose {
  if (cue.kind === "burst") {
    const { count: _count, power: _power, ...rest } = cue;
    return rest;
  }
  return { ...cue };
}

function stable(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return `{${Object.keys(record)
      .filter((key) => record[key] !== undefined)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stable(record[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function expectCues(actual: readonly FxCue[], expected: readonly Loose[]): void {
  const sort = (cues: readonly Loose[]): Loose[] => [...cues].sort((a, b) => (stable(a) < stable(b) ? -1 : stable(a) > stable(b) ? 1 : 0));
  expect(sort(actual.map(shape))).toEqual(sort(expected));
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the fixture is missing ${what}`);
  return value;
}

function entryFor(events: readonly GameEvent[], view: PlayerView, D: number, reduced = false): AnimationEntry {
  const entries = planEntries(events, view, reduced);
  expect(entries, "these events should plan exactly one entry").toHaveLength(1);
  return { ...must(entries[0], "an entry"), durationMs: D };
}

const envOf = (over: Partial<FxPlanEnv> = {}): FxPlanEnv => ({ intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over });
function plan(events: readonly GameEvent[], D: number, options: { view?: PlayerView; env?: FxPlanEnv; reduced?: boolean } = {}): FxCue[] {
  const view = withEvents(options.view ?? fullBoardView(), [...events]);
  const env = options.env ?? envOf();
  const entry = entryFor(events, view, options.reduced === true ? 0 : D, options.reduced);
  env.memory.remember(entry.events);
  return planFx(entry, view, env);
}
const destroyed = (instanceId: string, defId: string, owner: PlayerId): GameEvent => ({ type: "destroyed", instanceId, defId, owner, controller: owner, attack: 1, maxHealth: 1, killerId: null });
const cardT = (id: string, point?: FxPoint) => tid(testid.card(id), point);
const today = (id: string, D: number): Loose[] => [crack(cardT(id), 0, D), burst("ember", cardT(id), "area", r(FX_DEATH_EMBER_AT * D)), burst("smoke", cardT(id), "area", r(FX_DEATH_SMOKE_AT * D))];
const exitPlan = (D: number, id: string, side: Side, intensity: number = FX_INTENSITY_SCALE.normal): FlavorExitPlan =>
  ({ D, tgt: testid.card(id), intensity, instanceId: id, pile: animTestid.graveyard(side), fallAt: r(FX_DEATH_EMBER_AT * D), settleAt: r(FX_DEATH_SMOKE_AT * D) });
const exitOf = (flavor: Flavor, id: string, D: number, side: Side): Loose[] => FLAVOR_EXITS[flavor](exitPlan(D, id, side)).map(shape);

function withYourUnit(over: Partial<UnitView>): PlayerView {
  const view = fullBoardView();
  const units = [...view.you.units];
  units[0] = unit("p1", over);
  return { ...view, you: { ...view.you, units } };
}

function withTrap(over: Partial<Extract<BackrowView, { faceDown: false }>>): PlayerView {
  const view = fullBoardView();
  const backrow = [...view.you.backrow];
  backrow[2] = faceUpBackrow("p1", { instanceId: "t1", defId: "core-084", type: "Trap", ...over });
  return { ...view, you: { ...view.you, backrow } };
}

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
      return cue.delayMs + cue.flightMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const BOARD = fullBoardView();
const MINE_3 = must(BOARD.you.units[2], "your unit 3").instanceId;
const ENEMY_2 = must(BOARD.opponent.units[1], "enemy unit 2").instanceId;
const FLAVORS = Object.keys(FLAVOR_LOOKS) as Flavor[];

describe("#1446 a Unit's death exit takes its flavor", () => {
  it("#1446 every Flavor has an exit, each planning at least one burst", () => {
    expect(Object.keys(FLAVOR_EXITS).sort()).toEqual(Object.keys(FLAVOR_LOOKS).sort());
    for (const flavor of FLAVORS) {
      const cues = FLAVOR_EXITS[flavor](exitPlan(400, "n1", "you"));
      expect(cues.some((cue) => cue.kind === "burst"), flavor).toBe(true);
    }
  });

  it("#1446 R200 every flavor exit starts inside its entry, lands its streak inside it and ends within FX_MAX_TAIL_MS of its end", () => {
    for (const D of [120, 350, 400, 732, 1400]) {
      for (const flavor of FLAVORS) {
        const cues = FLAVOR_EXITS[flavor](exitPlan(D, "n1", "you"));
        for (const cue of cues) {
          const label = `${flavor} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          expect(Number.isInteger(cue.delayMs), label).toBe(true);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + T);
          if (cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
        }
      }
    }
  });

  it("#1446 low, normal and high plan the same kinds with strictly ordered burst totals", () => {
    const levels = [FX_INTENSITY_SCALE.low, FX_INTENSITY_SCALE.normal, FX_INTENSITY_SCALE.high] as const;
    const totals = [0, 0, 0];
    for (const flavor of FLAVORS) {
      const planned = levels.map((level) => FLAVOR_EXITS[flavor](exitPlan(400, "n1", "you", level)));
      expect(planned[1]?.map((cue) => cue.kind), flavor).toEqual(planned[0]?.map((cue) => cue.kind));
      expect(planned[2]?.map((cue) => cue.kind), flavor).toEqual(planned[1]?.map((cue) => cue.kind));
      const sums = planned.map((cues) => cues.reduce((sum, cue) => sum + (cue.kind === "burst" ? cue.count : 0), 0));
      expect(sums[0], `${flavor} low < normal`).toBeLessThan(sums[1] ?? 0);
      expect(sums[1], `${flavor} normal < high`).toBeLessThan(sums[2] ?? 0);
      sums.forEach((sum, k) => {
        totals[k] = (totals[k] ?? 0) + sum;
      });
      for (const cues of planned) {
        for (const cue of cues) {
          if (cue.kind !== "burst") continue;
          expect(Number.isInteger(cue.count), flavor).toBe(true);
          expect(cue.count, flavor).toBeGreaterThanOrEqual(1);
        }
      }
    }
    expect(totals[0]).toBeLessThan(totals[1] ?? 0);
    expect(totals[1]).toBeLessThan(totals[2] ?? 0);
  });

  it("#1446 a face-up destroyed plans today's death plus its flavor's exit", () => {
    const board = fullBoardView();
    const cards: { id: string; defId: string; owner: PlayerId }[] = [];
    for (const side of ["you", "opponent"] as const) {
      for (const found of board[side].units) {
        if (found !== null) cards.push({ id: found.instanceId, defId: found.defId, owner: found.owner });
      }
    }
    for (const index of [0, 4] as const) {
      const slot = must(board.you.backrow[index], `your backrow ${String(index)}`);
      if (slot.faceDown) throw new Error(`your backrow ${String(index)} is not face-up`);
      cards.push({ id: slot.instanceId, defId: slot.defId, owner: slot.owner });
    }
    for (const D of [ANIMATIONS.destroyed.durationMs, 732]) {
      for (const card of cards) {
        const cues = plan([destroyed(card.id, card.defId, card.owner)], D);
        expectCues(cues, [...today(card.id, D), ...exitOf(flavorOfId(card.defId), card.id, D, card.owner === "p1" ? "you" : "opponent")]);
        expect(cues.length, `${card.defId} at D=${String(D)}`).toBeGreaterThan(3);
      }
    }
  });

  it("#1446 a Beast bolts off to its owner's graveyard in a streak", () => {
    const D = 400;
    const view = withYourUnit({ instanceId: "beast1", defId: "core-t-sheep" });
    const cues = plan([destroyed("beast1", "core-t-sheep", "p1")], D, { view });
    expect(cues.map(shape)).toContainEqual({
      kind: "projectile",
      preset: "shard",
      from: cardT("beast1"),
      to: tid("graveyard-you"),
      delayMs: r(FX_DEATH_EMBER_AT * 400),
      flightMs: r(FX_DEATH_BOLT_FLIGHT_FRACTION * 400),
      density: 1,
    });
  });

  it("#1446 a face-down card's death keeps today's cue", () => {
    for (const D of [350, 732]) {
      // The viewer's own unrevealed Trap keeps today's death; revealed, it rises in light.
      expectCues(plan([destroyed("t1", "core-084", "p1")], D, { view: withTrap({ unrevealed: true }) }), today("t1", D));
      expectCues(plan([destroyed("t1", "core-084", "p1")], D, { view: withTrap({}) }), [...today("t1", D), ...exitOf("Holy", "t1", D, "you")]);
      // The opponent's face-down card dies at its pile, naming nothing.
      const hidden: Loose[] = [burst("smoke", tid("graveyard-opponent"), "point", 0)];
      expectCues(plan([destroyed("t9", "core-084", "p2")], D), hidden);
      expect(JSON.stringify(plan([destroyed("t9", "core-084", "p2")], D))).not.toContain("core-084");
      expectCues(plan([destroyed("hidden", "hidden", "p2")], D), hidden);
    }
  });

  it("#1446 exiled, bounced, crumbled and transformed cues are unchanged", () => {
    for (const D of [350, 732]) {
      expectCues(plan([{ type: "exiled", instanceId: ENEMY_2, defId: "core-013", owner: "p2" }], D), [
        ring("void", cardT(ENEMY_2), 0, D),
        burst("void", cardT(ENEMY_2), "area", 0),
      ]);
      expectCues(plan([{ type: "bounced", instanceId: MINE_3, defId: "core-017", owner: "p1" }], D), [
        burst("smoke", cardT(MINE_3), "area", 0),
        { kind: "ghost", from: cardT(MINE_3), to: tid("hand-you"), delayMs: 0, durationMs: D },
      ]);
      expectCues(plan([{ type: "crumbled", instanceId: MINE_3, defId: "core-017", owner: "p1", zone: "field" }], D), [
        crack(cardT(MINE_3), 0, D),
        burst("shard", cardT(MINE_3), "ring", r(FX_DEATH_EMBER_AT * D)),
        burst("dust", cardT(MINE_3), "area", r(FX_DEATH_SMOKE_AT * D)),
      ]);
      expectCues(plan([{ type: "transformed", instanceId: MINE_3, fromDefId: "core-017", toDefId: "token-sheep", newInstanceId: "c90" }], D), [
        burst("smoke", cardT(MINE_3), "point", 0),
        burst("arcane", cardT(MINE_3), "point", 0),
      ]);
    }
  });

  it("#1446 Chaos is deterministic and never Chaos", () => {
    const D = 400;
    for (const id of ["c1", "n7", "hello", "chaos-42"]) {
      const view = withYourUnit({ instanceId: id, defId: "classic-002" });
      expect(plan([destroyed(id, "classic-002", "p1")], D, { view })).toEqual(plan([destroyed(id, "classic-002", "p1")], D, { view }));
      const pick = chaosFlavor(id);
      expect(pick).not.toBe("Chaos");
      expectCues(plan([destroyed(id, "classic-002", "p1")], D, { view }), [...today(id, D), ...exitOf(pick, id, D, "you")]);
      expect(FLAVOR_EXITS.Chaos(exitPlan(D, id, "you"))).toEqual(FLAVOR_EXITS[pick](exitPlan(D, id, "you")));
    }
    for (let k = 0; k < 200; k += 1) expect(chaosFlavor(`chaos-${String(k)}`), `chaos-${String(k)}`).not.toBe("Chaos");
  });

  it("#1446 reduced motion and intensity off take the death's fallback", () => {
    expectCues(plan([destroyed(MINE_3, "core-017", "p1")], 0, { reduced: true }), today(MINE_3, 0));
    expect(plan([destroyed(MINE_3, "core-017", "p1")], 400, { env: envOf({ intensity: FX_INTENSITY_SCALE.off }) })).toEqual([]);
  });
});
