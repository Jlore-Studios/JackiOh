// R2797000: Stinkyscape's Flourishes (fx/cards/stinkyscape/): Imitated Instrumentality's boots and Drone
// Core X's warp, planned the way FxLayer plans them (remember, then plan), each held inside its entry and
// its tail (R200), keyed by readable plays only (R202) and drawing nothing with the effects off. Olivian
// Expanse's rift became its set piece (R3338000, fx/terminus/terminus.test.ts).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries, type AnimationEntry } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_MAGI_BOOTS, FX_MAGI_STAGGER, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_WARP_FROM } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const MAGI = "stinkyscape-018";
const WARP = "stinkyscape-036";

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf(), D?: number): { entry: AnimationEntry; cues: FxCue[] }[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.map((original) => {
    const entry = D === undefined ? original : { ...original, durationMs: D };
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

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

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 });
const summoned = (player: PlayerId, instanceId: string, defId: string, row: "units" | "backrow", lane: number): GameEvent => ({ type: "summoned", player, instanceId, defId, row, lane });

/** Each recipe's card and a play of it, as the engine streams one: the events it keys on and those it leaves. */
const PLAYS: Readonly<Record<string, (player: PlayerId, instanceId: string) => GameEvent[]>> = {
  [MAGI]: (player, id) => [
    played(player, id, MAGI),
    summoned(player, "m1", "stinkyscape-018-1", "backrow", 3),
    summoned(player, "m2", "stinkyscape-018-2", "backrow", 4),
    summoned(player, "m3", "stinkyscape-018-3", "backrow", 5),
    resolved(player, id, MAGI),
  ],
  [WARP]: (player, id) => [played(player, id, WARP), summoned(player, "bs", "stinkyscape-t-shield-layer", "units", 3), resolved(player, id, WARP)],
};

/** The cues of the entries holding `type`. */
function cuesOn(results: ReturnType<typeof planAll>, type: GameEvent["type"]): FxCue[][] {
  return results.filter(({ entry }) => entry.events.some((event) => event.type === type)).map(({ cues }) => cues);
}

const isPlay = (event: GameEvent): boolean => event.type === "cardPlayed" || event.type === "cardResolved";

/** The cues of the play's other entries with no play around them: its events' own rows alone. */
function bare(events: readonly GameEvent[], view: PlayerView): FxCue[][] {
  return planAll(events.filter((event) => !isPlay(event)), view).map(({ cues }) => cues);
}

/** The cues of every entry but the play's own `cardPlayed` and `cardResolved`. */
function inside(results: ReturnType<typeof planAll>): FxCue[][] {
  return results.filter(({ entry }) => !entry.events.some(isPlay)).map(({ cues }) => cues);
}

describe("R2797000 Stinkyscape's Flourishes", () => {
  it("R2797000 the recipes are their cards' bespoke entries", () => {
    expect(FX_CARD_TABLE.get(MAGI)).toEqual({ kind: "bespoke", key: "magiBoot" });
    expect(FX_CARD_TABLE.get(WARP)).toEqual({ kind: "bespoke", key: "battleshipWarp" });
  });

  it("R2797000 magiBoot boots each MAGI with a hologram beam from its caster's hero into its zone, the beams of one entry FX_MAGI_STAGGER of D apart", () => {
    const view = fullBoardView();
    const boots = planAll(PLAYS[MAGI]?.("p1", "ex") ?? [], view).filter(({ entry }) => entry.events.some((event) => event.type === "summoned"));
    expect(boots).toHaveLength(3);
    boots.forEach(({ entry, cues }, k) => {
      // Each its own entry: each boots from Travel to Land, rings its zone and shakes the board.
      const t = timesOf(entry.durationMs);
      const zoneId = testid.zone("you", "backrow", 3 + k);
      const zone = tid(zoneId);
      const beam = cues.find((cue) => cue.kind === "shot");
      expect(beam).toEqual(expect.objectContaining({ shot: { motion: "beam" }, from: tid(testid.hero("you")), to: zone, delayMs: t.T, flightMs: t.L - t.T }));
      expect(cues.some((cue) => cue.kind === "ring" && cue.delayMs === t.L && cue.at.kind === "testid" && cue.at.testid === zoneId)).toBe(true);
      expect(cues.some((cue) => cue.kind === "shake" && cue.delayMs === t.L)).toBe(true);
    });
    // One entry holding several: each beam leaves FX_MAGI_STAGGER of D after the one before, the fourth no later than the third.
    const D = 400;
    const t = timesOf(D);
    const events = [0, 1, 2, 3].map((k) => summoned("p1", `m${String(k)}`, "stinkyscape-018-1", "backrow", 1 + k));
    const entry: AnimationEntry = { events, type: "summoned", durationMs: D, frames: new Map(), view };
    const env = envOf();
    env.memory.remember([played("p1", "ex", MAGI), ...events]);
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const leaves = events.map((event) => CARD_RECIPES.magiBoot(event, { entry, view, env, D }, play)?.cues.find((cue) => cue.kind === "shot")?.delayMs);
    const s = frac(FX_MAGI_STAGGER, D);
    expect(leaves).toEqual([0, 1, 2, FX_MAGI_BOOTS - 1].map((k) => t.T + k * s));
    const shakes = events.map((event) => CARD_RECIPES.magiBoot(event, { entry, view, env, D }, play)?.cues.filter((cue) => cue.kind === "shake").length);
    expect(shakes).toEqual([1, 0, 0, 0]);
  });

  it("R2797000 battleshipWarp warps the Battleship in from the board's top edge in starlight, landing in exhaust, once a play", () => {
    const view = fullBoardView();
    const stream = [...(PLAYS[WARP]?.("p1", "ex") ?? []).slice(0, 2), summoned("p1", "di", "stinkyscape-t-drone-interceptor", "units", 4), resolved("p1", "ex", WARP)];
    const results = planAll(stream, view);
    const [first, second] = cuesOn(results, "summoned");
    const t = timesOf(results.find(({ entry }) => entry.events.some((event) => event.type === "summoned"))?.entry.durationMs ?? 0);
    const zoneId = testid.zone("you", "units", 3);
    const zone = tid(zoneId);
    const shot = first?.find((cue) => cue.kind === "shot");
    expect(shot).toEqual(expect.objectContaining({ shot: { motion: "bolt", trail: "star" }, from: tid(testid.board, FX_WARP_FROM), to: zone, delayMs: t.T, flightMs: t.L - t.T }));
    const presets = (first ?? []).flatMap((cue) => (cue.kind === "burst" && cue.at.kind === "testid" && cue.at.testid === zoneId ? [`${cue.preset}@${String(cue.delayMs)}`] : []));
    expect(presets).toEqual(expect.arrayContaining([`star@${String(t.G)}`, `exhaust@${String(t.L)}`, `star@${String(t.Li)}`]));
    expect(first?.some((cue) => cue.kind === "ring" && cue.preset === "star" && cue.delayMs === t.T)).toBe(true);
    expect(first?.some((cue) => cue.kind === "shake" && cue.delayMs === t.L)).toBe(true);
    // The second summon of the play keeps its own row alone.
    expect(second?.some((cue) => cue.kind === "shot")).toBe(false);
  });

  it("R200 R2797000 every cue starts inside its entry and ends within FX_MAX_TAIL_MS of its end, for D in [120, 200, 400, 732, 1400]", () => {
    const view = fullBoardView();
    for (const [defId, stream] of Object.entries(PLAYS)) {
      for (const D of [120, 200, 400, 732, 1400]) {
        const cues = planAll(stream("p1", "ex"), view, envOf(), D).flatMap((result) => result.cues);
        expect(cues.length).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${defId} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "shot" || cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R202 R2797000 a hidden play keys none", () => {
    const view = fullBoardView();
    for (const [defId, stream] of Object.entries(PLAYS)) {
      const veiled = stream("p2", "hidden").map((event) => (isPlay(event) ? { ...event, defId: "hidden" } : event));
      const cues = inside(planAll(veiled, view));
      expect(cues, defId).toEqual(bare(veiled, view));
      expect(cues, defId).not.toEqual(inside(planAll(stream("p2", "ex"), view)));
      expect(JSON.stringify(cues)).not.toContain("stinkyscape-0");
    }
  });

  it("R2797000 intensity 0 draws none", () => {
    const view = fullBoardView();
    for (const [defId, stream] of Object.entries(PLAYS)) {
      expect(planAll(stream("p1", "ex"), view, envOf({ intensity: 0 })).flatMap(({ cues }) => cues), defId).toEqual([]);
    }
  });
});
