// #1445: a Unit's summon entrance takes its flavor. The `summon` recipe (cues.ts) keeps today's
// motion underneath and appends one entrance per flavor from `FLAVOR_ENTRANCES` (entrances.ts).
// A summon the view does not show face-up keeps today's cue (R202), and reduced motion and
// intensity off keep the summon's fallback.

import type { Flavor, GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries, type AnimationEntry } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import {
  FX_CRACK_TAIL_MS,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_RING_MS,
  FX_SLAM_AT,
  FX_SUMMON_BRUTE_TRAUMA,
} from "./constants.ts";
import { planFx } from "./cues.ts";
import { FLAVOR_ENTRANCES } from "./entrances.ts";
import { chaosFlavor, FLAVOR_LOOKS } from "./flavors.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCardFacts, FxCue, FxPlanEnv, FxPoint } from "./types.ts";

type Loose = Record<string, unknown>;

const T = FX_MAX_TAIL_MS;
const r = Math.round;

const FOOT: FxPoint = { x: 0.5, y: 1 };

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
function shake(trauma: number, delayMs: number): Loose {
  return { kind: "shake", trauma: Math.round(trauma * 1e6) / 1e6, delayMs };
}
function sheen(at: ReturnType<typeof tid>, D: number): Loose {
  return { kind: "sheen", at, delayMs: 0, durationMs: D };
}

function shape(cue: FxCue): Loose {
  if (cue.kind === "burst") {
    const { count: _count, power: _power, ...rest } = cue;
    return rest;
  }
  if (cue.kind === "shake") return { ...cue, trauma: Math.round(cue.trauma * 1e6) / 1e6 };
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

const FACTS: Record<string, FxCardFacts> = {
  "core-002": { rarity: "Common", attack: 1, health: 2 },
  "core-066": { rarity: "Common", attack: 1, health: 1 },
  "core-t-sheep": { rarity: "Token", attack: 1, health: 1 },
  "classic-002": { rarity: "Common", attack: 1, health: 1 },
  "classic-005": { rarity: "Epic", attack: 0, health: 4 },
};

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: (defId) => FACTS[defId], memory: createFxMemory(), ...over };
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

function plan(events: readonly GameEvent[], D: number, options: { env?: FxPlanEnv; reduced?: boolean } = {}): FxCue[] {
  const view = withEvents(fullBoardView(), [...events]);
  const env = options.env ?? envOf();
  const entry = entryFor(events, view, options.reduced === true ? 0 : D, options.reduced);
  env.memory.remember(entry.events);
  return planFx(entry, view, env);
}

const summon = (player: "p1" | "p2", instanceId: string, defId: string, row: "units" | "backrow", lane: number): GameEvent => ({
  type: "summoned",
  player,
  instanceId,
  defId,
  row,
  lane,
});

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const FLAVORS = Object.keys(FLAVOR_LOOKS) as Flavor[];
const ZONE = testid.zone("you", "units", 3);

describe("#1445 a Unit's summon entrance takes its flavor", () => {
  it("#1445 every Flavor has an entrance, each planning at least one cue", () => {
    expect(Object.keys(FLAVOR_ENTRANCES).sort()).toEqual(Object.keys(FLAVOR_LOOKS).sort());
    for (const flavor of FLAVORS) {
      const cues = FLAVOR_ENTRANCES[flavor]({ D: 400, tgt: ZONE, intensity: FX_INTENSITY_SCALE.normal, landAt: r(FX_SLAM_AT * 400), instanceId: "n1" });
      expect(cues.length, flavor).toBeGreaterThan(0);
    }
  });

  it("#1445 R200 every flavor entrance starts inside its entry and ends within FX_MAX_TAIL_MS of its end", () => {
    for (const D of [120, 250, 400, 732, 1400]) {
      const landAt = r(FX_SLAM_AT * D);
      for (const flavor of FLAVORS) {
        const cues = FLAVOR_ENTRANCES[flavor]({ D, tgt: ZONE, intensity: FX_INTENSITY_SCALE.normal, landAt, instanceId: "n1" });
        for (const cue of cues) {
          const label = `${flavor} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + T);
          expect(Number.isInteger(cue.delayMs), label).toBe(true);
        }
      }
    }
  });

  it("#1445 low, normal and high plan the same kinds with strictly ordered burst totals", () => {
    const levels = [FX_INTENSITY_SCALE.low, FX_INTENSITY_SCALE.normal, FX_INTENSITY_SCALE.high] as const;
    const totals = [0, 0, 0];
    for (const flavor of FLAVORS) {
      const planned = levels.map((level) =>
        FLAVOR_ENTRANCES[flavor]({ D: 400, tgt: ZONE, intensity: level, landAt: r(FX_SLAM_AT * 400), instanceId: "n1" }),
      );
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

  it("#1445 a face-up summoned plans today's summon plus its flavor's entrance", () => {
    for (const D of [ANIMATIONS.summoned.durationMs, 732]) {
      const slam = r(FX_SLAM_AT * D);
      expectCues(plan([summon("p1", "n1", "core-002", "units", 3)], D), [
        ring("dust", tid(ZONE), slam, D),
        burst("dust", tid(ZONE, FOOT), "ring", slam),
        burst("spark", tid(ZONE, FOOT), "point", slam),
      ]);
    }
  });

  it("#1445 a Brute summon lands heavy", () => {
    const D = 400;
    const slam = r(FX_SLAM_AT * D);
    const cues = plan([summon("p1", "n1", "core-066", "units", 3)], D);
    expectCues(cues, [
      ring("dust", tid(ZONE), slam, D),
      burst("dust", tid(ZONE, FOOT), "ring", slam),
      ring("dust", tid(ZONE), slam, D),
      burst("dust", tid(ZONE, FOOT), "ring", slam),
      burst("dust", tid(ZONE, FOOT), "area", slam),
      crack(tid(ZONE), slam, D),
      shake(FX_SUMMON_BRUTE_TRAUMA, slam),
    ]);
    const shakes = cues.filter((cue) => cue.kind === "shake");
    expect(shakes).toHaveLength(1);
  });

  it("#1445 a hidden summon keeps today's cue", () => {
    for (const D of [ANIMATIONS.summoned.durationMs, 732]) {
      const slam = r(FX_SLAM_AT * D);
      const zone = testid.zone("opponent", "units", 2);
      for (const [instanceId, defId] of [["hidden", "hidden"], ["n99", "core-999"]] as const) {
        const cues = plan([summon("p2", instanceId, defId, "units", 2)], D);
        expectCues(cues, [burst("dust", tid(zone, FOOT), "ring", slam)]);
        expect(cues.some((cue) => cue.kind === "rays" || cue.kind === "crack" || cue.kind === "shake" || cue.kind === "sheen")).toBe(false);
        expect(JSON.stringify(cues)).not.toContain("core-999");
        expect(JSON.stringify(cues)).not.toContain("hidden");
      }
    }
  });

  it("#1445 Chaos is deterministic and never Chaos", () => {
    const D = 400;
    const slam = r(FX_SLAM_AT * D);
    const base = [ring("dust", tid(ZONE), slam, D), burst("dust", tid(ZONE, FOOT), "ring", slam)];
    for (const id of ["c1", "n7", "hello", "chaos-42"]) {
      expect(plan([summon("p1", id, "classic-002", "units", 3)], D)).toEqual(plan([summon("p1", id, "classic-002", "units", 3)], D));
      const pick = chaosFlavor(id);
      expect(pick).not.toBe("Chaos");
      const entrance = FLAVOR_ENTRANCES[pick]({ D, tgt: ZONE, intensity: FX_INTENSITY_SCALE.normal, landAt: slam, instanceId: id }).map(shape);
      expectCues(plan([summon("p1", id, "classic-002", "units", 3)], D), [...base, ...entrance]);
    }
    for (let k = 0; k < 200; k += 1) expect(chaosFlavor(`chaos-${String(k)}`), `chaos-${String(k)}`).not.toBe("Chaos");
  });

  it("#1445 reduced motion and intensity off take the summon's fallback", () => {
    // Reduced motion zeroes the entry: no flavor cue, exactly today's ring and burst.
    const reduced = plan([summon("p1", "n1", "core-002", "units", 3)], 0, { reduced: true });
    expectCues(reduced, [ring("dust", tid(ZONE), 0, 0), burst("dust", tid(ZONE, FOOT), "ring", 0)]);
    // Intensity off plans nothing.
    expect(plan([summon("p1", "n1", "core-002", "units", 3)], 400, { env: envOf({ intensity: FX_INTENSITY_SCALE.off }) })).toEqual([]);
    expect(plan([summon("p1", "n1", "core-066", "units", 3)], 400, { env: envOf({ intensity: FX_INTENSITY_SCALE.off }) })).toEqual([]);
  });

  it("#1445 tokens and Animated cards take it", () => {
    const D = 400;
    const slam = r(FX_SLAM_AT * D);
    // core-t-sheep is a Beast token: its entrance scratches shard marks around it.
    const token = plan([summon("p1", "n1", "core-t-sheep", "units", 3)], D);
    expect(token.map(shape)).toContainEqual(burst("shard", tid(ZONE), "ring", slam));
    expect(token.some((cue) => cue.kind === "rays")).toBe(false);
    // classic-005 is Tech: animating into its unit zone keeps the lift ghost plus a frost burst.
    const animated: GameEvent = { type: "animated", player: "p1", instanceId: "a1", defId: "classic-005", backrowLane: 2, unitLane: 1 };
    const zone = testid.zone("you", "units", 1);
    const lift = plan([animated], 350);
    expect(lift.some((cue) => cue.kind === "ghost")).toBe(true);
    expect(lift.map(shape)).toContainEqual(burst("frost", tid(zone), "ring", 0));
    expect(lift.map(shape)).toContainEqual(sheen(tid(zone), 350));
  });
});
