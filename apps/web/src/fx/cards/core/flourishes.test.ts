// #2793: Core's three bespoke flourishes (transmogulate.ts, zephrys.ts, craftACard.ts) pinned on their
// gallery fixtures (motifs/gallery.ts RECIPE_FIXTURES), each held inside its entry (R200), keyed only by
// a readable play and naming no hidden id (R202), and each named by its card's table entry (R502).
// R3402001: each grown into its card's signature, its cast opening it.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { planCardFx } from "../../cardFx.ts";
import {
  FX_CENTER,
  FX_CRUCIBLE_TRAUMA,
  FX_FORGE_REACH,
  FX_FORGE_TRAUMA,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_TIDE_TRAUMA,
  FX_WISH_TRAUMA,
} from "../../constants.ts";
import { createFxMemory } from "../../memory.ts";
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

/** R3402001: the cues `key` plans for its own play's `cardPlayed`, the signature's first beat, over an entry of D with no hold. */
function castCues(defId: string, key: CardRecipeKey, D = 400): FxCue[] {
  const env = envOf();
  const view = fullBoardView();
  const event = played("p1", "ex", defId);
  env.memory.remember([event]);
  const play = env.memory.resolving();
  if (play === undefined) throw new Error("no play resolving");
  const entry = { ...galleryEntry([event], view), durationMs: D };
  return CARD_RECIPES[key](event, { entry, view, env, D }, play)?.cues ?? [];
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
    // At D 400: gather 0, travel 80, land 240, linger 300. R3402001: the wave crests over both piles in a ring and a sheen.
    expect(beats(cues)).toEqual(["ring@0", "sheen@0", "ring@0", "sheen@0", "shake@80", "sheen@80"]);
    expect(cues.filter((cue) => cue.kind === "ring").map((cue) => cue.kind === "ring" && cue.at)).toEqual([tid(animTestid.library("you")), tid(animTestid.hand("you"))]);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_TIDE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 80 });
    const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    expect(cues.filter((cue) => cue.kind === "sheen").at(-1)).toEqual({ kind: "sheen", at: tid(`hand-card-${hand?.instanceId ?? ""}`), delayMs: 80, durationMs: 160 });

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
    expect(first.some((cue) => cue.kind === "ring")).toBe(true);
    expect(beats(later)).toEqual(["sheen@80"]);
    expect(hidden).toEqual([]);
  });

  it("R2784000 perfectWish blows the caster's card in from the far hero into its hand pile; the other player's card draws nothing", () => {
    const view = fullBoardView();
    const [cues = []] = fixtureCues("core-097", "perfectWish");
    // R3402001: one card back on a gust from the far hero into the hand pile, which rings and takes a sheen.
    expect(beats(cues)).toEqual(["ghost@0", "ring@0", "sheen@0", "shake@240"]);
    expect(cues.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: tid(testid.hero("opponent")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: 400 });
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_WISH_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 240 });
    const [theirs = []] = run("core-097", [{ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" }], 400, "p1", view, "perfectWish");
    expect(theirs).toEqual([]);
  });

  it("R2784000 fusionForge forges two halves from the board's edges at the fuse, and flies the crafted card on into the hand", () => {
    const [added = [], cues = []] = fixtureCues("core-099", "fusionForge");
    // The engine adds the crafted card to the hand before it reports the fuse: R3402001 the hand pile rings
    // as it does, and the forge carries its flight in place of the rows.
    expect(added).toEqual([{ kind: "ring", preset: "frost", at: tid(animTestid.hand("you")), delayMs: 0, durationMs: expect.any(Number) as number }]);
    expect(beats(cues)).toEqual(["ghost@0", "shake@0"]);
    const centre = tid(testid.board, FX_CENTER);
    expect(cues.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: centre, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: 240 });
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_FORGE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 0 });
  });

  it("R3402001 each flourish's cast opens its signature: the crucible at the hero, the silk and the forge's halves at the centre", () => {
    const hero = tid(testid.hero("you"));
    const centre = tid(testid.board, FX_CENTER);
    const tide = castCues("core-083", "legendaryTide");
    expect(beats(tide)).toEqual(["burst@0", "rays@0", "ring@80", "burst@240", "burst@240", "shake@240"]);
    for (const cue of tide) if (cue.kind !== "shake") expect("at" in cue ? cue.at : undefined, cue.kind).toEqual(hero);
    expect(tide.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "legendary" });
    expect(tide.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_CRUCIBLE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 240 });
    // With no takeover hold (Big moments off, or a bare entry) the silk settles and the halves meet as the entry lands.
    const wish = castCues("core-097", "perfectWish");
    expect(beats(wish)).toEqual(["burst@0", "ring@120", "ring@240", "burst@240"]);
    for (const cue of wish) expect("at" in cue ? cue.at : undefined, cue.kind).toEqual(centre);
    const forge = castCues("core-099", "fusionForge");
    expect(beats(forge)).toEqual(["burst@0", "burst@0", "ghost@120", "ghost@120", "burst@240", "ring@240"]);
    expect(forge.filter((cue) => cue.kind === "ghost").map((cue) => cue.kind === "ghost" && [cue.from, cue.to, cue.durationMs])).toEqual([
      [tid(testid.board, { x: FX_FORGE_REACH, y: FX_CENTER.y }), centre, 120],
      [tid(testid.board, { x: 1 - FX_FORGE_REACH, y: FX_CENTER.y }), centre, 120],
    ]);
    // Not cast by the play itself, it draws no first beat: another card's play draws none of it.
    for (const [defId, key] of FLOURISHES) {
      const env = envOf();
      env.memory.remember([played("p1", "ex", defId)]);
      const other = played("p1", "other", "core-003");
      const view = fullBoardView();
      const entry = { ...galleryEntry([other], view), durationMs: 400 };
      const play = env.memory.resolving();
      if (play === undefined) throw new Error("no play resolving");
      expect(CARD_RECIPES[key](other, { entry, view, env, D: 400 }, play), key).toBeNull();
    }
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
      // R3332000: Zephrys's entry also names its takeover look; R3402001 each is its card's signature, Craft a Card's with its own look too.
      const takeover = defId === "core-097" ? { takeover: "genie" } : defId === "core-099" ? { takeover: "crystalForge" } : {};
      expect(FX_CARD_TABLE.get(defId), defId).toEqual({ kind: "bespoke", key, ...takeover, signature: true });
      expect(run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), 400).flat(), defId).toEqual(fixtureCues(defId, key).flat());
    }
  });
});
