// R2797000: Stinkyscape's Flourishes (fx/cards/stinkyscape/): Imitated Instrumentality's boots (R3395000: its
// signature now, the cores behind its caster's hero, then each boot in its own hue) and Drone
// Core X's warp, planned the way FxLayer plans them (remember, then plan), each held inside its entry and
// its tail (R200), keyed by readable plays only (R202) and drawing nothing with the effects off. Olivian
// Expanse's rift became its set piece (R3338000, fx/terminus/terminus.test.ts).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { SET_PIECE_HOLD_CAP_MS, planEntries, type AnimationEntry } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_SHIP_ARRIVAL_BEATS,
  FX_SHIP_DOME_TRAUMA,
  FX_SHIP_LOCK_TRAUMA,
  FX_WARP_FROM,
} from "../../constants.ts";
import { planCardFx } from "../../cardFx.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { motifScale, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { STINKYSCAPE_SIGNATURE_BOARDS } from "./signatureFixtures.ts";

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
  // R3336000: the Battleship's three layers, bottom first, into one zone (R1842000).
  [WARP]: (player, id) => [
    played(player, id, WARP),
    summoned(player, "bs1", "stinkyscape-t-meltdown-layer", "units", 3),
    summoned(player, "bs2", "stinkyscape-t-propagation-layer", "units", 3),
    summoned(player, "bs3", "stinkyscape-t-shield-layer", "units", 3),
    resolved(player, id, WARP),
  ],
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
    expect(FX_CARD_TABLE.get(MAGI)).toEqual({ kind: "bespoke", key: "magiBoot", signature: true });
    expect(FX_CARD_TABLE.get(WARP)).toEqual({ kind: "bespoke", key: "battleshipWarp" });
  });

  it("R3395000 magiBoot raises three cores behind the caster's hero as it is cast, then boots each MAGI in its zone in its own hue, replacing its summon's row", () => {
    // Over the gallery's board, its three backrow zones open (a summon lands in an empty zone, §3.2).
    const view = STINKYSCAPE_SIGNATURE_BOARDS.magiBoot(fullBoardView());
    const results = planAll(PLAYS[MAGI]?.("p1", "ex") ?? [], view);
    const hero = tid(testid.hero("you"));
    // The cast: the cores' figure, a hologram burst and a ring behind the hero.
    const [cast] = cuesOn(results, "cardPlayed");
    expect(cast?.map((cue) => cue.kind)).toEqual(["figure", "burst", "ring"]);
    expect(cast?.[0]).toMatchObject({ kind: "figure", figure: "magiCores", at: hero, delayMs: 0 });
    expect(cast?.[1]).toMatchObject({ kind: "burst", at: hero, delayMs: 0, spread: "area" });
    const boots = results.filter(({ entry }) => entry.events.some((event) => event.type === "summoned"));
    expect(boots).toHaveLength(3);
    const tints = new Set<string>();
    boots.forEach(({ entry, cues: drawn }, k) => {
      const zone = tid(testid.zone("you", "backrow", 3 + k));
      // R3407000: each MAGI's own entrance draws beside the boot (its card's entry alone plans it), so the boot is the rest.
      const landing = entry.events.find((event) => event.type === "summoned");
      const magi = landing?.type === "summoned" ? FX_CARD_TABLE.get(landing.defId) : undefined;
      if (landing?.type !== "summoned" || magi === undefined) throw new Error(`boot ${String(k)} has no MAGI`);
      const entrance = (planCardFx(landing, { entry, view: entry.view, env: envOf(), D: entry.durationMs }, new Map([[landing.defId, magi]]))?.cues ?? []).map((cue) => JSON.stringify(cue));
      expect(entrance.length, `${landing.defId}'s own entrance`).toBeGreaterThan(0);
      for (const cue of entrance) expect(drawn.map((own) => JSON.stringify(own)), landing.defId).toContain(cue);
      // Each of the entrance's cues once: the boot's own sheen may match one of the entrance's.
      const left = [...entrance];
      const cues = drawn.filter((cue) => {
        const at = left.indexOf(JSON.stringify(cue));
        if (at >= 0) left.splice(at, 1);
        return at < 0;
      });
      // Its own hue's brand and a sheen, a ring for the first two, the first's shake; no particles after the cast.
      const brand = cues.find((cue) => cue.kind === "brand");
      expect(brand).toMatchObject({ at: zone, delayMs: 0, durationMs: entry.durationMs });
      if (brand?.kind === "brand") tints.add(JSON.stringify(brand.tint));
      expect(cues.some((cue) => cue.kind === "sheen" && JSON.stringify(cue.at) === JSON.stringify(zone))).toBe(true);
      expect(cues.some((cue) => cue.kind === "ring"), `boot ${String(k)} ring`).toBe(k < 2);
      expect(cues.filter((cue) => cue.kind === "shake")).toHaveLength(k === 0 ? 1 : 0);
      expect(cues.some((cue) => cue.kind === "burst" || cue.kind === "shot" || cue.kind === "projectile")).toBe(false);
    });
    expect(tints.size).toBe(3);
    // Each summon's own row is replaced: the boot alone draws it.
    const env = envOf();
    const event = summoned("p1", "m1", "stinkyscape-018-1", "backrow", 3);
    env.memory.remember([played("p1", "ex", MAGI), event]);
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const entry: AnimationEntry = { events: [event], type: "summoned", durationMs: 250, frames: new Map(), view };
    expect(CARD_RECIPES.magiBoot(event, { entry, view, env, D: 250 }, play)?.row).toBe("replace");
  });

  it("R2797000 R3336000 battleshipWarp warps the Battleship in as three beats, one per layer: the keel in starlight, the hangar locking on, the superstructure's dome", () => {
    const view = fullBoardView();
    const stream = [...(PLAYS[WARP]?.("p1", "ex") ?? []).slice(0, 4), summoned("p1", "di", "stinkyscape-t-drone-interceptor", "units", 4), resolved("p1", "ex", WARP)];
    const results = planAll(stream, view);
    const [keel, hangar, bridge, drone] = cuesOn(results, "summoned");
    const D = results.find(({ entry }) => entry.events.some((event) => event.type === "summoned"))?.entry.durationMs ?? 0;
    const t = timesOf(D);
    // The card's motif scale (its rarity and cost) multiplies each shake, as it does every beat of the warp.
    const scale = motifScale(WARP, view, FX_INTENSITY_SCALE.normal);
    const zoneId = testid.zone("you", "units", 3);
    const zone = tid(zoneId);
    // Beat 1: today's warp, from the top edge, landing in exhaust.
    const shot = keel?.find((cue) => cue.kind === "shot");
    expect(shot).toEqual(expect.objectContaining({ shot: { motion: "bolt", trail: "star" }, from: tid(testid.board, FX_WARP_FROM), to: zone, delayMs: t.T, flightMs: t.L - t.T }));
    const presets = (keel ?? []).flatMap((cue) => (cue.kind === "burst" && cue.at.kind === "testid" && cue.at.testid === zoneId ? [`${cue.preset}@${String(cue.delayMs)}`] : []));
    expect(presets).toEqual(expect.arrayContaining([`star@${String(t.G)}`, `exhaust@${String(t.L)}`, `star@${String(t.Li)}`]));
    expect(keel?.some((cue) => cue.kind === "ring" && cue.preset === "star" && cue.delayMs === t.T)).toBe(true);
    expect(keel?.some((cue) => cue.kind === "shake" && cue.delayMs === t.L)).toBe(true);
    // Beat 2: two bolts from above the zone's corners lock onto it, with a ring, a spark burst and the lock's shake.
    const locks = (hangar ?? []).filter((cue) => cue.kind === "shot");
    expect(locks.map((cue) => (cue.kind === "shot" ? cue.from : null))).toEqual([tid(zoneId, { x: 0.1, y: -0.4 }), tid(zoneId, { x: 0.9, y: -0.4 })]);
    for (const cue of locks) expect(cue).toEqual(expect.objectContaining({ to: zone, delayMs: t.T, flightMs: t.L - t.T }));
    expect(hangar?.some((cue) => cue.kind === "ring" && cue.delayMs === t.L)).toBe(true);
    expect(hangar?.some((cue) => cue.kind === "burst" && cue.preset === "spark" && cue.delayMs === t.L)).toBe(true);
    expect(hangar?.some((cue) => cue.kind === "shake" && cue.delayMs === t.L && cue.trauma === Math.min(1, FX_SHIP_LOCK_TRAUMA * scale))).toBe(true);
    expect(hangar?.some((cue) => cue.kind === "burst" && cue.preset === "exhaust")).toBe(false);
    // Beat 3: the superstructure drops in and its dome flares over the zone.
    expect(bridge?.some((cue) => cue.kind === "shot" && cue.from.kind === "testid" && cue.from.testid === zoneId && cue.to.kind === "testid" && cue.to.testid === zoneId)).toBe(true);
    expect(bridge).toEqual(expect.arrayContaining([expect.objectContaining({ kind: "shield", size: "full", at: zone, delayMs: t.L })]));
    expect(bridge?.some((cue) => cue.kind === "shake" && cue.delayMs === t.L && cue.trauma === Math.min(1, FX_SHIP_DOME_TRAUMA * scale))).toBe(true);
    // The three beats differ, and a fourth summon of the play keeps its own row alone.
    expect(new Set([keel, hangar, bridge].map((cues) => JSON.stringify(cues))).size).toBe(FX_SHIP_ARRIVAL_BEATS);
    expect(drone?.some((cue) => cue.kind === "shot")).toBe(false);
  });

  it("R3336000 the arrival's three beats, each in its own entry, fit one set piece's hold (SET_PIECE_HOLD_CAP_MS)", () => {
    const results = planAll(PLAYS[WARP]?.("p1", "ex") ?? [], fullBoardView());
    const beats = results.filter(({ entry }) => entry.events.some((event) => event.type === "summoned"));
    expect(beats).toHaveLength(FX_SHIP_ARRIVAL_BEATS);
    // The beats run one after the next: the last cue of the last ends this long after the first begins.
    let start = 0;
    let end = 0;
    for (const { entry, cues } of beats) {
      end = Math.max(end, start + Math.max(...cues.map((cue) => endOf(cue))));
      start += entry.durationMs;
    }
    expect(end).toBeGreaterThan(0);
    expect(end).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
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
