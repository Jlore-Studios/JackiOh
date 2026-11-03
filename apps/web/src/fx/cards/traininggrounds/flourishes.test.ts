// #2799: Training Grounds' three bespoke flourishes (celestialCaller.ts, dinoSize.ts, goetzmannGameDesign.ts)
// pinned on their gallery fixtures (motifs/gallery.ts RECIPE_FIXTURES), each held inside its entry (R200),
// drawing nothing with the effects off, keyed only by a readable play and naming no hidden id (R202), and
// each named by its card's table entry (R502).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { planCardFx } from "../../cardFx.ts";
import { planFx } from "../../cues.ts";
import {
  FX_CELESTIAL_TRAUMA,
  FX_CENTER,
  FX_DINO_STOMP_TRAUMA,
  FX_GAME_DESIGN_TRAUMA,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
} from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import { MOTIF_FOOT, MOTIF_TUNING, motifScale } from "../../motifs/build.ts";
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
  ["traininggrounds-016", "celestialCall"],
  ["traininggrounds-034", "dinoStomp"],
  ["traininggrounds-040", "gameDesign"],
];

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

/**
 * Plays `defId` as `player` and each of `events` under it, an entry each lasting D, as FxLayer does:
 * remember, then plan. Each entry's cues come from the card's recipe (`direct`) or the dispatch, planCardFx.
 */
function run(defId: string, events: readonly GameEvent[], D: number, player: PlayerId = "p1", view: PlayerView = fullBoardView(), direct?: CardRecipeKey): FxCue[][] {
  const env = envOf();
  env.memory.remember([played(player, "ex", defId)]);
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

function fixtureCues(defId: string, key: CardRecipeKey, D = 400): FxCue[][] {
  const view = fullBoardView();
  return run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), D, "p1", view, key);
}

const tid = (value: string, at?: { x: number; y: number }) => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });

describe("#2799 Training Grounds' bespoke flourishes", () => {
  it("R2784000 celestialCall beams the Sun and the Moon in from the hero, each landing in its own sky's look and light", () => {
    const view = fullBoardView();
    const [sun = [], moon = []] = fixtureCues("traininggrounds-016", "celestialCall");
    // At D 400: gather 0, travel 80, land 240, linger 300.
    expect(beats(sun)).toEqual(["burst@0", "shot@80", "rays@240", "burst@240", "ring@240", "burst@300", "shake@240"]);
    expect(beats(moon)).toEqual(beats(sun));
    // The play stands nowhere on the fixture board, so the beams leave from the caster's hero.
    expect(sun.find((cue) => cue.kind === "shot")).toMatchObject({ from: tid(testid.hero("you")), to: tid(testid.zone("you", "units", 2)), flightMs: 160 });
    expect(moon.find((cue) => cue.kind === "shot")).toMatchObject({ to: tid(testid.zone("you", "units", 4)) });
    expect(sun.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "radiant" });
    expect(moon.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "holy" });
    expect(sun.find((cue) => cue.kind === "burst" && cue.delayMs === 240)).toMatchObject({ preset: FLAVOR_LOOKS.Holy.particle, colors: FLAVOR_LOOKS.Holy.colors });
    expect(moon.find((cue) => cue.kind === "burst" && cue.delayMs === 240)).toMatchObject({ preset: FLAVOR_LOOKS.Spirit.particle, colors: FLAVOR_LOOKS.Spirit.colors });
    expect(sun.find((cue) => cue.kind === "burst" && cue.delayMs === 300)).toMatchObject({ preset: "gold" });
    expect(moon.find((cue) => cue.kind === "burst" && cue.delayMs === 300)).toMatchObject({ preset: "star" });
    const scale = motifScale("traininggrounds-016", view, FX_INTENSITY_SCALE.normal);
    expect(sun.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_CELESTIAL_TRAUMA * scale), delayMs: 240 });
    expect(sun.find((cue) => cue.kind === "burst" && cue.delayMs === 0)).toMatchObject({ count: Math.round(MOTIF_TUNING.gather.count * scale) });

    // From the Caller's own zone when the board shows it; another card's summon, or the other seat's, draws nothing.
    const caller = view.you.units[2];
    const sunAt: GameEvent = { type: "summoned", player: "p1", instanceId: "fx-sun", defId: "traininggrounds-006", row: "units", lane: 2 };
    const env = envOf();
    env.memory.remember([played("p1", caller?.instanceId ?? "", "traininggrounds-016")]);
    const entry = { ...galleryEntry([sunAt], view), durationMs: 400 };
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const fromCaller = CARD_RECIPES.celestialCall(sunAt, { entry, view, env, D: 400 }, play)?.cues ?? [];
    expect(fromCaller.find((cue) => cue.kind === "shot")).toMatchObject({ from: tid(`card-${caller?.instanceId ?? ""}`) });
    const [token = [], theirs = []] = run(
      "traininggrounds-016",
      [
        { type: "summoned", player: "p1", instanceId: "fx-token", defId: "core-t-rush", row: "units", lane: 2 },
        { type: "summoned", player: "p2", instanceId: "fx-sun-2", defId: "traininggrounds-006", row: "units", lane: 2 },
      ],
      400,
      "p1",
      view,
      "celestialCall",
    );
    expect(token).toEqual([]);
    expect(theirs).toEqual([]);
  });

  it("R2784000 dinoStomp swells the set unit and lands a stomp of dust at its foot, once a unit", () => {
    const view = fullBoardView();
    const [attack = [], health = []] = fixtureCues("traininggrounds-034", "dinoStomp");
    expect(beats(attack)).toEqual(["arrows@0", "burst@240", "burst@240", "ring@240", "burst@300", "shake@240"]);
    // The set reports attack, then health: the stomp lands on the first alone.
    expect(health).toEqual([]);
    const unit = view.you.units[0]?.instanceId ?? "";
    expect(attack[0]).toMatchObject({ kind: "arrows", direction: "up", at: tid(`card-${unit}`) });
    const scale = motifScale("traininggrounds-034", view, FX_INTENSITY_SCALE.normal);
    expect(attack[1]).toMatchObject({ preset: "dust", spread: "ring", at: tid(`card-${unit}`, MOTIF_FOOT), count: Math.round(MOTIF_TUNING.nova.count * scale) });
    expect(attack.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_DINO_STOMP_TRAUMA * scale), delayMs: 240 });
    // A number the viewer may not read, or on a card the board does not show, draws nothing.
    const [hidden = [], gone = []] = run(
      "traininggrounds-034",
      [
        { type: "numberChanged", instanceId: unit, defId: "core-004", key: "attack", value: 10, hiddenFrom: ["p1"] },
        { type: "numberChanged", instanceId: "fx-gone", defId: "core-004", key: "attack", value: 10 },
      ],
      400,
      "p1",
      view,
      "dinoStomp",
    );
    expect(hidden).toEqual([]);
    expect(gone).toEqual([]);
  });

  it("R2784000 gameDesign drafts the card at the board's centre in Mythic rays and deals it into the caster's hand pile", () => {
    const view = fullBoardView();
    const [cues = []] = fixtureCues("traininggrounds-040", "gameDesign");
    expect(beats(cues)).toEqual(["burst@0", "rays@0", "ring@80", "ghost@80", "burst@240", "burst@300", "shake@240"]);
    const centre = tid(testid.board, FX_CENTER);
    const hand = tid(animTestid.hand("you"));
    expect(cues.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "mythic", at: centre });
    expect(cues.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: centre, to: hand, delayMs: 80, durationMs: 160 });
    expect(cues.find((cue) => cue.kind === "burst" && cue.delayMs === 0)).toMatchObject({ colors: paletteLook("Nerd/pixel")?.colors });
    expect(cues.find((cue) => cue.kind === "burst" && cue.delayMs === 240)).toMatchObject({ preset: "prismatic", at: hand });
    const scale = motifScale("traininggrounds-040", view, FX_INTENSITY_SCALE.normal);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_GAME_DESIGN_TRAUMA * scale), delayMs: 240 });
    const [theirs = []] = run("traininggrounds-040", [{ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" }], 400, "p1", view, "gameDesign");
    expect(theirs).toEqual([]);
  });

  it("R200 every flourish cue starts inside its entry and ends within FX_MAX_TAIL_MS", () => {
    for (const [defId, key] of FLOURISHES) {
      for (const D of [120, 200, 400, 732, 1400]) {
        const cues = fixtureCues(defId, key, D).flat();
        expect(cues.length, `${key} at D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${key} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "ghost") expect(cue.delayMs + cue.durationMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R200 every flourish draws nothing with the effects off, and over its event's own row with them on", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      const events = RECIPE_FIXTURES[key](view, "ex", "p1");
      // The whole effects layer, as FxLayer plans each entry.
      const planned = (intensity: number): FxCue[] => {
        const env = envOf({ intensity });
        env.memory.remember([played("p1", "ex", defId)]);
        return events.flatMap((event) => {
          env.memory.remember([event]);
          return planFx({ ...galleryEntry([event], view), durationMs: 400 }, view, env);
        });
      };
      expect(planned(FX_INTENSITY_SCALE.off), key).toEqual([]);
      const on = planned(FX_INTENSITY_SCALE.normal);
      const own = fixtureCues(defId, key).flat();
      // `keep`: the row's own recipe plays beside the flourish.
      expect(on.length, key).toBeGreaterThan(own.length);
      for (const cue of own) expect(on, key).toContainEqual(cue);
    }
  });

  it("R202 a hidden play keys none, and the other seat's cues name no hidden id", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      const events = RECIPE_FIXTURES[key](view, "ex", "p1");
      expect(run("hidden", events, 400).flat(), key).toEqual([]);
      // The same play by the other seat: its summons on its own side, its card added hidden from the viewer.
      const theirs: GameEvent[] = events.map((event) => {
        if (event.type === "summoned") return { ...event, player: "p2" };
        if (event.type === "addedToHand") return { ...event, player: "p2", instanceId: "hidden", defId: "hidden" };
        return event;
      });
      const cues = run(defId, theirs, 400, "p2", view).flat();
      expect(cues.length, key).toBeGreaterThan(0);
      expect(JSON.stringify(cues), key).not.toMatch(/hidden|fx-|traininggrounds-\d{3}|core-\d{3}|hand-card-/);
    }
  });

  it("R502 traininggrounds-016, -034 and -040 key their recipes", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      expect(FX_CARD_TABLE.get(defId), defId).toEqual({ kind: "bespoke", key });
      expect(run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), 400).flat(), defId).toEqual(fixtureCues(defId, key).flat());
    }
  });
});
