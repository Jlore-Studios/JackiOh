// #2798: Stinkyscape's three bespoke flourishes, part two (duoBond.ts, flashFreeze.ts, stratosSwarm.ts),
// pinned on their gallery fixtures (motifs/gallery.ts RECIPE_FIXTURES), each held inside its entry (R200),
// keyed only by a readable play and naming no definition (R202), and each named by its card's table
// entry (R502).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { DUO_MARK, MARK_PALETTES } from "../../../cards/marks.ts";
import { testid } from "../../../game/contract.ts";
import { FROZEN_MARK } from "../../../stats/config.ts";
import { baseView, fullBoardView } from "../../../test/fixtures.ts";
import { planCardFx } from "../../cardFx.ts";
import {
  FX_DUO_BOND_TRAUMA,
  FX_FLASH_FREEZE_TRAUMA,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_STRATOS_SWARM_TRAUMA,
} from "../../constants.ts";
import { FLAVOR_SUB_LOOKS } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import { motifScale } from "../../motifs/build.ts";
import { galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES, type CardRecipeKey } from "../bespoke.ts";
import { FX_CARD_TABLE } from "../tables.ts";

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
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

const beats = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);

const FLOURISHES: readonly (readonly [string, CardRecipeKey])[] = [
  ["stinkyscape-110", "duoBond"],
  ["stinkyscape-146", "flashFreeze"],
  ["stinkyscape-158", "stratosSwarm"],
];

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

/**
 * Plays `defId` as `player` (its play's id `playId`) and each of `events` under it, an entry each lasting
 * D, as FxLayer does: remember, then plan. Each entry's cues come from the card's recipe (`direct`) or the
 * dispatch, planCardFx.
 */
function run(
  defId: string,
  events: readonly GameEvent[],
  D: number,
  { player = "p1", view = fullBoardView(), direct, playId = "ex" }: { player?: PlayerId; view?: PlayerView; direct?: CardRecipeKey; playId?: string } = {},
): FxCue[][] {
  const env = envOf();
  env.memory.remember([played(player, playId, defId)]);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const p = { entry, view, env, D };
    const play = env.memory.resolving();
    if (direct === undefined) return planCardFx(event, p)?.cues ?? [];
    if (play === undefined) throw new Error("no play resolving");
    return CARD_RECIPES[direct](event, p, play)?.cues ?? [];
  });
}

function fixtureCues(defId: string, key: CardRecipeKey, D = 400, view: PlayerView = fullBoardView()): FxCue[][] {
  return run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), D, { view, direct: key });
}

const tid = (value: string) => ({ kind: "testid", testid: value });

/** A mark's palette as the flourishes draw it: its core, its rim, its glow. */
const markColors = (color: keyof typeof MARK_PALETTES) => [MARK_PALETTES[color].core, MARK_PALETTES[color].rim, MARK_PALETTES[color].glow];

describe("#2798 Stinkyscape's bespoke flourishes, part two", () => {
  it("R2784000 #2798 iKwzki: duoBond beams from iKwzki's zone to the permanent it Duos with, from the caster's hero when iKwzki is off the board", () => {
    const view = fullBoardView();
    const [mine, second] = view.you.units;
    if (mine === null || mine === undefined || second === null || second === undefined) throw new Error("no units");
    // At D 400: gather 0, travel 80, land 240, linger 300. The gallery's iKwzki is off the board: the hero.
    const [cues = []] = fixtureCues("stinkyscape-110", "duoBond");
    expect(beats(cues)).toEqual(["burst@0", "shot@80", "rays@240", "burst@240", "ring@240", "shake@240"]);
    expect(cues.find((cue) => cue.kind === "shot")).toMatchObject({ from: tid(testid.hero("you")), to: tid(testid.card(mine.instanceId)), flightMs: 160 });
    expect(cues.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "radiant", at: tid(testid.card(mine.instanceId)) });
    expect(cues.find((cue) => cue.kind === "burst")).toMatchObject({ colors: markColors("gold") });
    const scale = motifScale("stinkyscape-110", view, FX_INTENSITY_SCALE.normal);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_DUO_BOND_TRAUMA * scale), delayMs: 240 });

    // iKwzki on the board beams from its own card; a Duo with itself draws no beam.
    const duo = (instanceId: string): GameEvent => ({ type: "marked", instanceId, mark: DUO_MARK, color: "gold", added: true });
    const [fromZone = []] = run("stinkyscape-110", [duo(mine.instanceId)], 400, { direct: "duoBond", playId: second.instanceId });
    expect(fromZone.find((cue) => cue.kind === "shot")).toMatchObject({ from: tid(testid.card(second.instanceId)), to: tid(testid.card(mine.instanceId)) });
    const [self = []] = run("stinkyscape-110", [duo(second.instanceId)], 400, { direct: "duoBond", playId: second.instanceId });
    expect(beats(self)).toEqual(["rays@240", "burst@240", "ring@240", "shake@240"]);

    // Another mark, a Duo coming off and a card the view does not show draw nothing.
    const [frozen = [], off = [], gone = []] = run(
      "stinkyscape-110",
      [
        { type: "marked", instanceId: mine.instanceId, mark: FROZEN_MARK, color: "cyan", added: true },
        { type: "marked", instanceId: mine.instanceId, mark: DUO_MARK, color: "gold", added: false },
        duo("hidden"),
      ],
      400,
      { direct: "duoBond" },
    );
    expect([frozen, off, gone]).toEqual([[], [], []]);
  });

  it("R2784000 #2798 Flash Freeze: flashFreeze rolls one frost wave of 2×5 lane bursts and a board ring at its first freeze, and a sheen and a burst on each Unit it freezes", () => {
    const view = fullBoardView();
    const entries = fixtureCues("stinkyscape-146", "flashFreeze");
    expect(entries).toHaveLength(10);
    const [first = [], ...later] = entries;
    expect(beats(first)).toEqual([
      "burst@80",
      "burst@80",
      "burst@120",
      "burst@120",
      "burst@160",
      "burst@160",
      "burst@200",
      "burst@200",
      "burst@240",
      "burst@240",
      "ring@240",
      "shake@240",
      "sheen@80",
      "burst@240",
    ]);
    expect(first.filter((cue) => cue.kind === "ring").map((cue) => cue.kind === "ring" && cue.at)).toEqual([tid(testid.board)]);
    expect(first.find((cue) => cue.kind === "burst")).toMatchObject({ at: tid(testid.zone("you", "units", 1)), colors: markColors("cyan"), preset: "frost" });
    const scale = motifScale("stinkyscape-146", view, FX_INTENSITY_SCALE.normal);
    expect(first.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_FLASH_FREEZE_TRAUMA * scale), delayMs: 240 });
    for (const cues of later) expect(beats(cues)).toEqual(["sheen@80", "burst@240"]);

    // Two freezes in one entry roll the wave once; a thaw draws nothing.
    const [a, b] = RECIPE_FIXTURES.flashFreeze(view, "ex", "p1");
    if (a === undefined || b === undefined) throw new Error("no freezes");
    const env = envOf();
    env.memory.remember([played("p1", "ex", "stinkyscape-146"), a, b]);
    const entry = { ...galleryEntry([a, b], view), durationMs: 400 };
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const plan = (event: GameEvent) => CARD_RECIPES.flashFreeze(event, { entry, view, env, D: 400 }, play)?.cues ?? [];
    expect(plan(a).filter((cue) => cue.kind === "ring")).toHaveLength(1);
    expect(beats(plan(b))).toEqual(["sheen@80", "burst@240"]);
    if (a.type !== "marked") throw new Error("not a mark");
    expect(run("stinkyscape-146", [{ ...a, added: false }], 400, { direct: "flashFreeze" }).flat()).toEqual([]);
  });

  it("R2784000 #2798 Stratos Swarm: stratosSwarm flies a rocket bolt from the caster's hero into each zone it fills, and shakes once a play", () => {
    const view = baseView();
    const entries = fixtureCues("stinkyscape-158", "stratosSwarm", 400, view);
    // An empty board: five unit zones, then five backrow zones.
    expect(entries).toHaveLength(10);
    const [first = [], ...later] = entries;
    expect(beats(first)).toEqual(["burst@0", "shot@80", "burst@240", "ring@240", "burst@300", "shake@240"]);
    expect(first.find((cue) => cue.kind === "shot")).toMatchObject({ from: tid(testid.hero("you")), to: tid(testid.zone("you", "units", 1)), flightMs: 160 });
    expect(first.find((cue) => cue.kind === "burst")).toMatchObject({ colors: FLAVOR_SUB_LOOKS.Tech?.rocket?.colors, preset: "exhaust" });
    const scale = motifScale("stinkyscape-158", view, FX_INTENSITY_SCALE.normal);
    expect(first.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_STRATOS_SWARM_TRAUMA * scale), delayMs: 240 });
    for (const cues of later) expect(beats(cues)).toEqual(["burst@0", "shot@80", "burst@240", "ring@240", "burst@300"]);
    expect(later.at(-1)?.find((cue) => cue.kind === "shot")).toMatchObject({ to: tid(testid.zone("you", "backrow", 5)) });
    // A full board has no open zone: the fixture's one Stratos lands in lane 1.
    expect(fixtureCues("stinkyscape-158", "stratosSwarm")).toHaveLength(1);
  });

  it("R200 every flourish cue starts inside its entry and ends within FX_MAX_TAIL_MS", () => {
    for (const [defId, key] of FLOURISHES) {
      for (const D of [120, 200, 400, 732, 1400]) {
        const cues = fixtureCues(defId, key, D, key === "stratosSwarm" ? baseView() : fullBoardView()).flat();
        expect(cues.length, `${key} at D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${key} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "sheen") expect(cue.delayMs + cue.durationMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R202 a hidden play keys none, and the other seat's play names no definition", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      const events = RECIPE_FIXTURES[key](view, "ex", "p1");
      expect(run("hidden", events, 400).flat(), key).toEqual([]);
      const cues = run(defId, RECIPE_FIXTURES[key](view, "ex", "p2"), 400, { player: "p2" }).flat();
      expect(cues.length, key).toBeGreaterThan(0);
      expect(JSON.stringify(cues), key).not.toMatch(/hidden|fx-|(core|stinkyscape)-\d{3}/);
    }
  });

  it("R502 stinkyscape-110, stinkyscape-146 and stinkyscape-158 key their recipes", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      // R3407000: iKwzki's entry is her signature too (its frame is ikwzki.test.ts's).
      expect(FX_CARD_TABLE.get(defId), defId).toMatchObject({ kind: "bespoke", key });
      expect(run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), 400).flat(), defId).toEqual(fixtureCues(defId, key).flat());
    }
  });
});
