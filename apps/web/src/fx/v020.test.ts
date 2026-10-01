// Patch v0.2.0's events on the effects layer (docs/classic-sets.md B3, B5; SPEC §10.10). Each new
// event's ANIMATIONS row names a recipe, and the planner gives it a look of its own: an announced play
// hangs under a sheen, a countered one shatters, a stolen card flies to its thief's hand, an unlock
// snaps the chains, an Animated card lifts from its backrow zone into its unit zone and back, Brittle
// crumbles like glass, a redirected hit flies from its old target to its new one, a hero's health set
// reads as the way it went, a flicker blinks out and back, and Rollback rewinds the board. R200 binds
// every one (inside the entry, gone within the tail) and R202 too (a ghost is a card back; the
// sentinel plans what any card would).

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries, type AnimationEntry } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { FX_FLICKER_RETURN_AT, FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_REDIRECT_FLIGHT_FRACTION } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "./types.ts";

const VIEW: PlayerView = fullBoardView();

function unitOf(side: "you" | "opponent", index: number): string {
  const unit = VIEW[side].units[index];
  if (unit === null || unit === undefined) throw new Error(`the fixture has no ${side} unit ${String(index)}`);
  return unit.instanceId;
}

const MINE = unitOf("you", 0);
const ENEMY = unitOf("opponent", 0);
const ME = VIEW.viewer;
const THEM = ME === "p1" ? "p2" : "p1";

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

function plan(events: readonly GameEvent[], view: PlayerView = VIEW): { cues: FxCue[]; D: number } {
  const shown = withEvents(view, [...events]);
  const entries = planEntries(events, shown, false);
  expect(entries).toHaveLength(1);
  const entry = entries[0] as AnimationEntry;
  const env = envOf();
  env.memory.remember(entry.events);
  return { cues: planFx(entry, shown, env), D: entry.durationMs };
}

function kinds(cues: readonly FxCue[]): string[] {
  return [...new Set(cues.map((cue) => cue.kind))].sort();
}

function tid(value: string): FxAnchor {
  return { kind: "testid", testid: value };
}

/** R200: a cue starts inside its entry, and whatever it leaves behind is gone within the tail. */
function expectInsideR200(cues: readonly FxCue[], D: number): void {
  for (const cue of cues) {
    expect(cue.delayMs, JSON.stringify(cue)).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, JSON.stringify(cue)).toBeLessThanOrEqual(D);
    if ("durationMs" in cue && cue.kind !== "hold" && cue.kind !== "conceal") {
      expect(cue.delayMs + cue.durationMs, JSON.stringify(cue)).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    }
    if (cue.kind === "projectile") expect(cue.delayMs + cue.flightMs).toBeLessThanOrEqual(D);
  }
}

const SAMPLES: readonly { name: string; event: GameEvent }[] = [
  {
    name: "cardAnnounced",
    event: { type: "cardAnnounced", player: ME, instanceId: MINE, defId: "core-008", cardType: "Unit", costPaid: 1, targets: [] },
  },
  { name: "countered", event: { type: "countered", player: ME, instanceId: MINE, defId: "core-008", byInstanceId: null, to: "graveyard" } },
  { name: "stolen", event: { type: "stolen", instanceId: "hidden", defId: "hidden", from: THEM, to: ME, zone: "library" } },
  { name: "unlocked", event: { type: "unlocked", player: ME, row: "units", lane: 3 } },
  { name: "animated", event: { type: "animated", player: ME, instanceId: MINE, defId: "classic-005", backrowLane: 2, unitLane: 1 } },
  { name: "deanimated", event: { type: "deanimated", player: ME, instanceId: MINE, defId: "classicplus-012-8", unitLane: 1, backrowLane: 2 } },
  { name: "crumbled", event: { type: "crumbled", instanceId: MINE, defId: "core-008", owner: ME, zone: "field" } },
  { name: "numberChanged", event: { type: "numberChanged", instanceId: MINE, defId: "core-008", key: "attack", value: 3 } },
  { name: "redirected", event: { type: "redirected", what: "damage", fromId: MINE, toId: ENEMY, byInstanceId: null } },
  { name: "healthSet", event: { type: "healthSet", player: ME, health: 13, sourceId: null } },
  { name: "rolledBack", event: { type: "rolledBack", player: ME, turnsAgo: 2, sides: [ME, THEM] } },
  { name: "flickered", event: { type: "flickered", player: ME, instanceId: MINE, defId: "core-008", row: "units", lane: 1 } },
];

describe("patch v0.2.0's events look like what they do", () => {
  it("every one plans a look of its own, inside R200's bounds", () => {
    for (const { name, event } of SAMPLES) {
      const { cues, D } = plan([event]);
      expect(cues.length, `${name} plans something`).toBeGreaterThan(0);
      expect(ANIMATIONS[event.type].fx, `${name}'s row names a recipe`).toBeDefined();
      expectInsideR200(cues, D);
    }
  });

  it("an announced play hangs under a sheen and a ring while the window for a Counter is open", () => {
    const { cues } = plan([SAMPLES[0]?.event as GameEvent]);
    expect(kinds(cues)).toEqual(["burst", "ring", "sheen"]);
  });

  it("a countered card shatters where it hung, with a small shake", () => {
    const { cues } = plan([SAMPLES[1]?.event as GameEvent]);
    expect(kinds(cues)).toEqual(["burst", "crack", "shake"]);
  });

  it("R202 a stolen card flies to its thief's hand as a back, from the pile it came out of", () => {
    const { cues } = plan([SAMPLES[2]?.event as GameEvent]);
    const ghost = cues.find((cue) => cue.kind === "ghost");
    expect(ghost).toMatchObject({ from: tid("library-opponent"), to: tid("hand-you") });
    expect(JSON.stringify(cues)).not.toContain("core-");
  });

  it("an unlock snaps its chains in gold shards", () => {
    const { cues } = plan([SAMPLES[3]?.event as GameEvent]);
    expect(cues).toContainEqual(expect.objectContaining({ kind: "ring", preset: "gold", at: tid(testid.zone("you", "units", 3)) }));
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: "shard" }));
  });

  it("an Animated card lifts from its backrow zone into its unit zone, and back again", () => {
    const up = plan([SAMPLES[4]?.event as GameEvent]).cues.find((cue) => cue.kind === "ghost");
    expect(up).toMatchObject({ from: tid(testid.zone("you", "backrow", 2)), to: tid(testid.zone("you", "units", 1)) });
    const down = plan([SAMPLES[5]?.event as GameEvent]).cues.find((cue) => cue.kind === "ghost");
    expect(down).toMatchObject({ from: tid(testid.zone("you", "units", 1)), to: tid(testid.zone("you", "backrow", 2)) });
  });

  it("Brittle at 0 shatters the card like glass", () => {
    const { cues } = plan([SAMPLES[6]?.event as GameEvent]);
    expect(cues).toContainEqual(expect.objectContaining({ kind: "crack" }));
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: "shard" }));
  });

  it("a redirected hit flies from its old target onto the new one", () => {
    const { cues, D } = plan([SAMPLES[8]?.event as GameEvent]);
    const shot = cues.find((cue) => cue.kind === "projectile");
    expect(shot).toMatchObject({ from: tid(testid.card(MINE)), to: tid(testid.card(ENEMY)) });
    expect(shot?.kind === "projectile" ? shot.flightMs : -1).toBe(Math.round(FX_REDIRECT_FLIGHT_FRACTION * D));
  });

  it("a hero's health set reads as the way it went: a gain in holy light, a loss in void, each with its size", () => {
    const health = VIEW.you.hero.health;
    const up = plan([{ type: "healthSet", player: ME, health: health + 5, sourceId: null }]).cues;
    expect(up).toContainEqual(expect.objectContaining({ kind: "splat", tone: "heal", amount: 5 }));
    expect(up).toContainEqual(expect.objectContaining({ kind: "rays", tone: "holy" }));
    const down = plan([{ type: "healthSet", player: ME, health: health - 7, sourceId: null }]).cues;
    expect(down).toContainEqual(expect.objectContaining({ kind: "splat", tone: "loss", amount: 7 }));
    const same = plan([{ type: "healthSet", player: ME, health, sourceId: null }]).cues;
    expect(kinds(same)).toEqual(["ring"]);
  });

  it("Rollback rewinds the whole board with a jolt", () => {
    const { cues } = plan([SAMPLES[10]?.event as GameEvent]);
    expect(kinds(cues)).toEqual(["burst", "shake", "sheen"]);
  });

  it("a flicker blinks the card out through the void and back in halfway", () => {
    const { cues, D } = plan([SAMPLES[11]?.event as GameEvent]);
    expect(cues).toContainEqual(expect.objectContaining({ kind: "ring", preset: "void", delayMs: 0 }));
    expect(cues).toContainEqual(expect.objectContaining({ kind: "ring", preset: "arcane", delayMs: Math.round(FX_FLICKER_RETURN_AT * D) }));
  });

  it("R202 the sentinel plans the same as any card would: a hidden announce and a hidden counter name nothing", () => {
    const hidden: GameEvent = { type: "countered", player: THEM, instanceId: "hidden", defId: "hidden", byInstanceId: null, to: "graveyard" };
    const { cues } = plan([hidden]);
    expect(JSON.stringify(cues)).not.toMatch(/core-|classic/);
  });
});
