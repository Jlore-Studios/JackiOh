// #2793: Core's three bespoke flourishes (transmogulate.ts, zephrys.ts, craftACard.ts) pinned on their
// gallery fixtures (motifs/gallery.ts RECIPE_FIXTURES), each held inside its entry (R200), keyed only by
// a readable play and naming no hidden id (R202), and each named by its card's table entry (R502).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_CENTER, FX_FORGE_REACH, FX_FORGE_TRAUMA, FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_TIDE_TRAUMA, FX_WISH_TRAUMA } from "../../constants.ts";
import { createFxMemory } from "../../memory.ts";
import { MOTIF_TUNING, motifScale } from "../../motifs/build.ts";
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
  ["core-083", "legendaryTide"],
  ["core-097", "perfectWish"],
  ["core-099", "fusionForge"],
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

describe("#2793 Core's bespoke flourishes", () => {
  it("R2784000 legendaryTide washes the caster's Deck and hand piles at its first transform, a sheen on each hand card the viewer reads", () => {
    const view = fullBoardView();
    const [cues = []] = fixtureCues("core-083", "legendaryTide");
    // At D 400: gather 0, travel 80, land 240, linger 300.
    expect(beats(cues)).toEqual(["rays@80", "ring@240", "burst@240", "rays@80", "ring@240", "burst@240", "shake@240", "sheen@80", "burst@240"]);
    expect(cues.filter((cue) => cue.kind === "rays").map((cue) => cue.kind === "rays" && [cue.tone, cue.at])).toEqual([
      ["legendary", tid(animTestid.library("you"))],
      ["legendary", tid(animTestid.hand("you"))],
    ]);
    const scale = motifScale("core-083", view, FX_INTENSITY_SCALE.normal);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: FX_TIDE_TRAUMA * scale, delayMs: 240 });
    const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    expect(cues.find((cue) => cue.kind === "sheen")).toEqual({ kind: "sheen", at: tid(`hand-card-${hand?.instanceId ?? ""}`), delayMs: 80, durationMs: 160 });

    // A later transform draws its own sheen only; one the viewer may not read draws nothing.
    const second = Array.isArray(view.you.hand) ? view.you.hand[1] : undefined;
    const turn = (instanceId: string, hiddenFrom?: PlayerId[]): GameEvent => ({
      type: "transformed",
      instanceId,
      fromDefId: "core-019",
      toDefId: "core-052",
      newInstanceId: instanceId,
      ...(hiddenFrom === undefined ? {} : { hiddenFrom }),
    });
    const [first = [], later = [], hidden = []] = run("core-083", [turn(hand?.instanceId ?? ""), turn(second?.instanceId ?? ""), turn("hidden", ["p1"])], 400, "p1", view, "legendaryTide");
    expect(first.some((cue) => cue.kind === "rays")).toBe(true);
    expect(beats(later)).toEqual(["sheen@80", "burst@240"]);
    expect(hidden).toEqual([]);
  });

  it("R2784000 perfectWish blows the caster's card in from the far hero into its hand pile; the other player's card draws nothing", () => {
    const view = fullBoardView();
    const [cues = []] = fixtureCues("core-097", "perfectWish");
    expect(beats(cues)).toEqual(["burst@0", "shot@80", "rays@240", "burst@240", "ring@240", "burst@300", "shake@240"]);
    const shot = cues.find((cue) => cue.kind === "shot");
    expect(shot).toMatchObject({ from: tid(testid.hero("opponent")), to: tid(animTestid.hand("you")), flightMs: 160 });
    expect(cues.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "mythic", at: tid(animTestid.hand("you")) });
    const scale = motifScale("core-097", view, FX_INTENSITY_SCALE.normal);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_WISH_TRAUMA * scale), delayMs: 240 });
    expect(cues.find((cue) => cue.kind === "burst" && cue.delayMs === 0)).toMatchObject({ preset: "star", count: Math.round(MOTIF_TUNING.gather.count * scale) });
    const [theirs = []] = run("core-097", [{ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" }], 400, "p1", view, "perfectWish");
    expect(theirs).toEqual([]);
  });

  it("R2784000 fusionForge forges two halves from the board's edges at the fuse, and flies the crafted card on into the hand", () => {
    const view = fullBoardView();
    const [added = [], cues = []] = fixtureCues("core-099", "fusionForge");
    // The engine adds the crafted card to the hand before it reports the fuse: the forge carries its flight.
    expect(added).toEqual([]);
    expect(beats(cues)).toEqual(["burst@0", "burst@0", "shot@80", "shot@80", "burst@240", "ring@240", "shake@240", "ghost@300", "burst@400"]);
    const centre = tid(testid.board, FX_CENTER);
    expect(cues.filter((cue) => cue.kind === "shot").map((cue) => cue.kind === "shot" && [cue.from, cue.to])).toEqual([
      [tid(testid.board, { x: FX_FORGE_REACH, y: FX_CENTER.y }), centre],
      [tid(testid.board, { x: 1 - FX_FORGE_REACH, y: FX_CENTER.y }), centre],
    ]);
    expect(cues.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: centre, to: tid(animTestid.hand("you")), delayMs: 300, durationMs: 100 });
    const scale = motifScale("core-099", view, FX_INTENSITY_SCALE.normal);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_FORGE_TRAUMA * scale), delayMs: 240 });
    expect(cues.find((cue) => cue.kind === "burst" && cue.delayMs === 240)).toMatchObject({ spread: "area", count: Math.round(MOTIF_TUNING.nova.count * scale) });
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

  it("R202 a hidden play keys none, and the other seat's cues name no hidden id", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      const events = RECIPE_FIXTURES[key](view, "ex", "p1");
      expect(run("hidden", events, 400).flat(), key).toEqual([]);
      // The same play by the other seat, its cards hidden from the viewer as R97 sends them.
      const theirs: GameEvent[] = events.map((event) => {
        if (event.type === "transformed") return { ...event, instanceId: "hidden", fromDefId: "hidden", toDefId: "hidden", newInstanceId: "hidden", hiddenFrom: ["p1"] };
        if (event.type === "addedToHand") return { ...event, player: "p2", instanceId: "hidden", defId: "hidden" };
        if (event.type === "fused") return { ...event, instanceIds: ["hidden", "hidden"], resultInstanceId: "hidden", defId: "hidden" };
        return event;
      });
      const cues = run(defId, theirs, 400, "p2", view).flat();
      expect(cues.length, key).toBeGreaterThan(0);
      const text = JSON.stringify(cues);
      expect(text, key).not.toMatch(/hidden|fx-|core-\d{3}|hand-card-/);
      expect(text, key).toContain(animTestid.hand("opponent"));
    }
  });

  it("R502 core-083, core-097 and core-099 key their recipes", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      // R3332000: Zephrys's entry also names its takeover look.
      expect(FX_CARD_TABLE.get(defId), defId).toEqual({ kind: "bespoke", key, ...(defId === "core-097" ? { takeover: "genie" } : {}) });
      expect(run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), 400).flat(), defId).toEqual(fixtureCues(defId, key).flat());
    }
  });
});
