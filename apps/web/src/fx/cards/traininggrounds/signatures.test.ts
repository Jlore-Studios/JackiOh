// R3400001: Training Grounds' five signatures (seance.ts, tutorialMaster.ts, chaosDice.ts, trainingGolem.ts,
// goetzmannGameDesign.ts), planned as FxLayer plans them, an entry each: remember, then plan. Seance on its
// announced tile and on the centre, Tutorial Master's entrance and his Cry's and Death's cards, Call to
// Chaos's dice, its capped panel and a nested Call's smaller dice, the Golem's lashing; from the caster's
// opponent's seat no beat names a card or flies more than a card back (R202); every cue starts inside its
// entry and is gone within FX_MAX_TAIL_MS of its end, and nothing draws with the effects off (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import {
  FX_CENTER,
  FX_CHAOS_DICE_PANEL_MS,
  FX_CHAOS_DICE_REEL_MS,
  FX_INTENSITY_SCALE,
  FX_LEGENDARY_TRAUMA,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_SLAM_AT,
} from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { ENTRANCES } from "../../entrances.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue, FxMemory, FxPlanEnv } from "../../types.ts";
import type { CardRecipeKey } from "../bespoke.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const SEANCE = "traininggrounds-037";
const TUTOR = "traininggrounds-038";
const CHAOS = "traininggrounds-039";
const GOLEM = "traininggrounds-039-1";
const DESIGN = "traininggrounds-040";

function envOf(intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv {
  return { intensity, card: () => undefined, memory: createFxMemory() };
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
const tid = (value: string, at?: { x: number; y: number }) => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const added = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "addedToHand", player, instanceId, defId });
const announced = (player: PlayerId, instanceId: string, defId: string, targets: string[]): GameEvent => ({
  type: "cardAnnounced",
  player,
  instanceId,
  defId,
  cardType: "Spell",
  costPaid: 2,
  targets,
});

type Planned = { cues: FxCue[]; row: string | undefined };

/** Plans each of `events` as its own entry lasting D, after `before` is remembered: remember, then plan through the dispatch. */
function plan(events: readonly GameEvent[], D: number, before: readonly GameEvent[], view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): Planned[] {
  env.memory.remember(before);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const result = planCardFx(event, { entry, view, env, D });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

/** `defId` played as `player` (instance "ex"), then each of `events` under it. */
function run(defId: string, events: readonly GameEvent[], D: number, player: PlayerId = "p1", view: PlayerView = fullBoardView()): Planned[] {
  return plan(events, D, [played(player, "ex", defId)], view);
}

/** Its own cast's entry: the play remembered, then planned. */
function castOf(defId: string, D: number, before: readonly GameEvent[] = [], player: PlayerId = "p1", view: PlayerView = fullBoardView(), memory?: FxMemory): Planned {
  const env = envOf();
  if (memory !== undefined) env.memory = memory;
  return plan([played(player, "ex", defId)], D, before, view, env)[0] ?? { cues: [], row: undefined };
}

describe("R3400001 Training Grounds' signatures", () => {
  it("R3400001 the five are signatures, Tutorial Master's and the Golem's tiles wearing their own auras", () => {
    expect(FX_CARD_TABLE.get(SEANCE)).toEqual({ kind: "bespoke", key: "seance", signature: true });
    expect(FX_CARD_TABLE.get(TUTOR)).toEqual({ kind: "bespoke", key: "tutorialMaster", entrance: "chalkboardLesson", aura: { look: "chalk", palette: "Nerd/chalk", scope: "tile" }, signature: true });
    expect(FX_CARD_TABLE.get(CHAOS)).toEqual({ kind: "bespoke", key: "chaosDice", signature: true });
    expect(FX_CARD_TABLE.get(GOLEM)).toEqual({ kind: "hooks", hooks: {}, entrance: "golemLashing", aura: { look: "timber", scope: "tile" }, signature: true });
    expect(FX_CARD_TABLE.get(DESIGN)).toEqual({ kind: "bespoke", key: "gameDesign", takeover: "napkin", signature: true });
  });

  it("R3400001 Seance: candles ring the announced tile as the board dims and a spirit rises, its copy drifting from there to the hand; with no tile, the centre", () => {
    const view = fullBoardView();
    const D = 400;
    const foe = view.opponent.units[1]?.instanceId ?? "";
    const tile = tid(testid.card(foe));
    const announce = announced("p1", "ex", SEANCE, [foe]);
    const [cast, copy] = plan([played("p1", "ex", SEANCE), added("p1", "fx-double", "core-013")], D, [announce], view);
    expect(cast?.row).toBe("replace");
    expect(beats(cast?.cues ?? [])).toEqual(["figure@0", "figure@0", "burst@0", "figure@200", "burst@200"]);
    expect(cast?.cues[0]).toMatchObject({ figure: "dusk", at: tid(testid.board), durationMs: D + 400 });
    expect(cast?.cues[1]).toMatchObject({ figure: "candles", at: tile, durationMs: D + 400 });
    expect(cast?.cues[2]).toMatchObject({ preset: "ember", at: tile });
    expect(cast?.cues[3]).toMatchObject({ figure: "spirit", at: tile, durationMs: D - 200 + 400 });
    expect(cast?.cues[4]).toMatchObject({ preset: "sparkle", at: tile, colors: FLAVOR_LOOKS.Spirit.colors });
    expect(copy).toEqual({ cues: [{ kind: "ghost", from: tile, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D }], row: "replace" });
    // No announce (the gallery's), a hidden target or a card the board does not show face up: the centre.
    const centre = tid(testid.board, FX_CENTER);
    for (const before of [[], [announced("p1", "ex", SEANCE, ["hidden"])], [announced("p1", "ex", SEANCE, ["b-facedown"])], [announced("p1", "other", SEANCE, [foe])]]) {
      const [lone, back] = plan([played("p1", "ex", SEANCE), added("p1", "fx-double", "core-013")], D, before, view);
      expect(lone?.cues[1], JSON.stringify(before)).toMatchObject({ figure: "candles", at: centre });
      expect(back?.cues[0], JSON.stringify(before)).toMatchObject({ kind: "ghost", from: centre });
    }
  });

  it("R3400001 Tutorial Master: a chalkboard swings down, his pointer raps twice, a chalk outline draws itself as he lands", () => {
    const view = fullBoardView();
    const D = 400;
    const slam = frac(FX_SLAM_AT, D);
    const tgt = testid.zone("you", "units", 3);
    const landing: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player: "p1", instanceId: "tm", defId: TUTOR, row: "units", lane: 3 };
    const cues = ENTRANCES.chalkboardLesson({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing });
    expect(beats(cues)).toEqual(["figure@0", "burst@80", "burst@140", `figure@${String(slam)}`, `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ figure: "chalkboard", at: tid(tgt), durationMs: D + 300, side: "you" });
    expect(cues[1]).toMatchObject({ at: tid(tgt, { x: 0.5, y: 0.15 }), colors: paletteLook("Nerd/chalk")?.colors });
    expect(cues[3]).toMatchObject({ figure: "chalkOutline", at: tid(tgt), durationMs: D - slam + 400 });
    expect(cues[6]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.chalkboardLesson, delayMs: slam });
    expect(ENTRANCE_TRAUMA.chalkboardLesson).toBe(FX_LEGENDARY_TRAUMA);
  });

  it("R3400001 Tutorial Master: each card his Cry and his Death add slides to the hand as a card back, the first along a chalk arrow, his outline erasing first at his Death", () => {
    const view = fullBoardView();
    const D = 400;
    const hand = tid(animTestid.hand("you"));
    // His Cry, from his card while the board shows it (lane 3's unit standing in for him).
    const him = view.you.units[2]?.instanceId ?? "";
    const cry = plan(RECIPE_FIXTURES.tutorialMaster(view, him, "p1"), D, [played("p1", him, TUTOR)], view);
    expect(cry).toHaveLength(3);
    expect(cry[0]?.cues).toEqual([
      { kind: "figure", figure: "chalkArrow", at: hand, tint: expect.anything() as unknown, delayMs: 0, durationMs: D, side: "you" },
      { kind: "ghost", from: tid(testid.card(him)), to: hand, delayMs: 0, durationMs: D },
    ]);
    for (const later of cry.slice(1)) expect(later).toEqual({ cues: [{ kind: "ghost", from: tid(testid.card(him)), to: hand, delayMs: 0, durationMs: D }], row: "replace" });
    // Played where the board shows no card of his (the gallery's), from his hero.
    const [first] = run(TUTOR, RECIPE_FIXTURES.tutorialMaster(view, "ex", "p1"), D, "p1", view);
    expect(first?.cues[1]).toMatchObject({ kind: "ghost", from: tid(testid.hero("you")) });
    // His Death: its own entry, the outline erasing first and the first card after it.
    const death: GameEvent = { type: "hookResolving", sourceId: "tm-gone", defId: TUTOR, controller: "p1", hook: "death" };
    const dying = plan([death, added("p1", "fx-a", "core-002"), added("p1", "fx-b", "core-003")], D, [], view);
    expect(dying[0]).toEqual({ cues: [], row: undefined });
    expect(beats(dying[1]?.cues ?? [])).toEqual(["figure@0", "figure@200", "ghost@200"]);
    expect(dying[1]?.cues[0]).toMatchObject({ figure: "erase", at: tid(testid.hero("you")), durationMs: 200 });
    expect(dying[1]?.cues[1]).toMatchObject({ figure: "chalkArrow", at: hand, durationMs: 200 });
    expect(dying[2]?.cues).toEqual([{ kind: "ghost", from: tid(testid.hero("you")), to: hand, delayMs: 0, durationMs: D }]);
    // Another of his hooks, and the other player's cards, draw nothing.
    const other: GameEvent = { type: "hookResolving", sourceId: "tm", defId: TUTOR, controller: "p1", hook: "whenever" };
    expect(plan([other, added("p1", "fx-c", "core-002")], D, [], view).map((entry) => entry.cues)).toEqual([[], []]);
    expect(run(TUTOR, [added("p2", "hidden", "hidden")], D).map((entry) => entry.cues)).toEqual([[]]);
  });

  it("R3400001 Call to Chaos (Training-Grounds Edition): two dice tumble in from the caster's side, its roll lights a face as its panel names the clause within the cap, and a Call it casts rolls smaller dice", () => {
    const view = fullBoardView();
    const D = 400;
    const centre = tid(testid.board, FX_CENTER);
    const cast = castOf(CHAOS, D);
    expect(beats(cast.cues)).toEqual(["figure@0", "burst@240"]);
    expect(cast.cues[0]).toMatchObject({ figure: "die", at: centre, durationMs: D + 400, side: "you" });
    expect(cast.cues[0]).not.toHaveProperty("size");
    expect(cast.cues[1]).toMatchObject({ colors: paletteLook("Tech/glitch")?.colors });
    expect(castOf(CHAOS, D, [], "p2").cues[0]).toMatchObject({ side: "opponent" });
    // Its roll: R436's panel, its reels landing within FX_CHAOS_DICE_REEL_MS and the panel gone after FX_CHAOS_DICE_PANEL_MS; no burst.
    const [roll] = run(CHAOS, RECIPE_FIXTURES.chaosDice(view, "ex", "p1"), 900, "p1", view);
    expect(roll?.row).toBe("replace");
    expect(roll?.cues.map((cue) => cue.kind)).toEqual(["chaos", "figure"]);
    const panel = roll?.cues[0];
    if (panel?.kind !== "chaos") throw new Error("no panel");
    expect(panel.durationMs).toBe(FX_CHAOS_DICE_PANEL_MS);
    expect(panel.lines.map((line) => line.text)).toEqual(["Summon a Training-Grounds Golem"]);
    for (const line of panel.lines) expect(line.landMs).toBeLessThanOrEqual(FX_CHAOS_DICE_REEL_MS);
    expect(roll?.cues[1]).toMatchObject({ figure: "dieFace", at: centre, delayMs: 0, durationMs: FX_CHAOS_DICE_PANEL_MS });
    // The outcome's own rows play its beat: the recipe leaves them be.
    expect(run(CHAOS, [{ type: "summoned", player: "p1", instanceId: "g", defId: "core-t-rush", row: "units", lane: 1 }], D)).toEqual([{ cues: [], row: undefined }]);
    // A Call it casts: the inner play rolls a smaller pair, its roll a smaller face.
    const env = envOf();
    env.memory.remember([played("p1", "outer", CHAOS)]);
    const inner = plan([played("p1", "inner", CHAOS), { type: "chaosRolled", player: "p1", instanceId: "inner", defId: CHAOS, effects: ["Gain 6 mana and draw 2 cards"] }], D, [], view, env);
    expect(inner[0]?.cues[0]).toMatchObject({ figure: "die", size: "small" });
    expect(inner[1]?.cues[1]).toMatchObject({ figure: "dieFace", size: "small" });
  });

  it("R3400001 the Training-Grounds Golem: training posts fly in from the yard's four edges and lash together at the slam, then its five keywords lock on one by one", () => {
    const D = 400;
    const slam = frac(FX_SLAM_AT, D);
    const tgt = testid.zone("you", "units", 3);
    const cues = ENTRANCES.golemLashing({ D, tgt, intensity: FX_INTENSITY_SCALE.normal });
    const shots = cues.filter((cue) => cue.kind === "shot");
    expect(shots.map((shot) => (shot.from.kind === "viewport" ? shot.from.at : null))).toEqual([
      { x: 0, y: 0.5 },
      { x: 1, y: 0.5 },
      { x: 0.5, y: 0 },
      { x: 0.5, y: 1 },
    ]);
    for (const shot of shots) {
      expect(shot.delayMs).toBeGreaterThanOrEqual(frac(0.1, D));
      expect(shot.delayMs).toBeLessThanOrEqual(frac(0.25, D));
      expect(shot.delayMs + shot.flightMs).toBe(slam);
      expect(shot).toMatchObject({ to: tid(tgt), preset: "dust", shot: { motion: "bolt", trail: "dust" } });
    }
    const rest = cues.filter((cue) => cue.kind !== "shot");
    expect(beats(rest)).toEqual([`ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`, "figure@248", "shield@280", "figure@312", "callout@344", "figure@376"]);
    expect(rest[1]).toMatchObject({ preset: "dust", at: tid(tgt, FOOT) });
    expect(rest[2]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.golemLashing, delayMs: slam });
    expect(rest.slice(3).map((cue) => (cue.kind === "figure" ? cue.figure : cue.kind === "callout" ? cue.keyword : cue.kind === "shield" ? `shield ${cue.size}` : cue.kind))).toEqual([
      "plank",
      "shield full",
      "palisade",
      "Rush",
      "splinter",
    ]);
    for (const cue of rest.slice(3)) expect(cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0)).toBe(D + 300);
    // Its own landing keys it, the token by the rarity it prints; a hidden summon none (R202).
    const view = fullBoardView();
    const landing: GameEvent = { type: "summoned", player: "p1", instanceId: "golem", defId: GOLEM, row: "units", lane: 3 };
    const env = envOf();
    env.memory.remember([landing]);
    expect(planCardFx(landing, { entry: { ...galleryEntry([landing], view), durationMs: D }, view, env, D })).toEqual({ cues, row: "replace" });
    const hidden: GameEvent = { type: "summoned", player: "p2", instanceId: "hidden", defId: "hidden", row: "units", lane: 3 };
    expect(planCardFx(hidden, { entry: { ...galleryEntry([hidden], view), durationMs: D }, view, env, D })).toBeNull();
  });

  it("R3400001 R202 from the opponent's seat Training Grounds' signatures name no card and fly only card backs", () => {
    // The caster is the other seat: its cards added are hidden from this one; Seance chose its own face-down trap, which the view shows as a back.
    const view = fullBoardView();
    const hidden = added("p2", "hidden", "hidden");
    const plays: readonly [string, GameEvent[], GameEvent[]][] = [
      [SEANCE, [announced("p2", "ex", SEANCE, ["their-trap"])], [played("p2", "ex", SEANCE), hidden]],
      [TUTOR, [played("p2", "ex", TUTOR)], [hidden, hidden, hidden]],
      [CHAOS, [played("p2", "ex", CHAOS)], [{ type: "chaosRolled", player: "p2", instanceId: "ex", defId: CHAOS, effects: ["Add 2 random Radiant Training Grounds cards to your hand, which cost (1), and gain 1 mana"] }, hidden, hidden]],
      [DESIGN, [played("p2", "ex", DESIGN)], [hidden]],
    ];
    for (const [defId, before, events] of plays) {
      const cues = plan(events, 400, before, view).flatMap((entry) => entry.cues);
      expect(cues.length, defId).toBeGreaterThan(0);
      expect(JSON.stringify(cues), defId).not.toMatch(/hidden|their-trap|core-\d{3}|traininggrounds-\d{3}|hand-card-/);
      expect(cues.some((cue) => cue.kind === "sheen"), defId).toBe(false);
      for (const cue of cues) if (cue.kind === "ghost") expect(cue.to, defId).toEqual(tid(animTestid.hand("opponent")));
    }
    // Seance on that face-down trap: its candles stand at the centre, its double a card back from there.
    const [seance, double] = plan([played("p2", "ex", SEANCE), hidden], 400, [announced("p2", "ex", SEANCE, ["their-trap"])], view);
    expect(seance?.cues[1]).toMatchObject({ figure: "candles", at: tid(testid.board, FX_CENTER) });
    expect(double?.cues).toEqual([{ kind: "ghost", from: tid(testid.board, FX_CENTER), to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: 400 }]);
  });

  it("R3400001 R200 each cue starts inside its entry and ends within FX_MAX_TAIL_MS at D 120, 400 and 1400; nothing draws with effects off", () => {
    const view = fullBoardView();
    const keys: readonly [string, CardRecipeKey][] = [
      [SEANCE, "seance"],
      [TUTOR, "tutorialMaster"],
      [CHAOS, "chaosDice"],
      [DESIGN, "gameDesign"],
    ];
    for (const D of [120, 400, 1400]) {
      const all: [string, FxCue[]][] = [
        ...keys.map(([defId, key]): [string, FxCue[]] => [key, [...castOf(defId, D).cues, ...run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), D, "p1", view).flatMap((entry) => entry.cues)]]),
        ["chalkboardLesson", ENTRANCES.chalkboardLesson({ D, tgt: testid.zone("you", "units", 3), intensity: 1 })],
        ["golemLashing", ENTRANCES.golemLashing({ D, tgt: testid.zone("you", "units", 3), intensity: 1 })],
      ];
      for (const [key, cues] of all) {
        expect(cues.length, `${key} at D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${key} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "ghost" || cue.kind === "shot") expect(cue.delayMs + (cue.kind === "shot" ? cue.flightMs : cue.durationMs), label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    }
    // The whole effects layer off draws nothing.
    for (const [defId, key] of keys) {
      const env = envOf(FX_INTENSITY_SCALE.off);
      const events: GameEvent[] = [played("p1", "ex", defId), ...RECIPE_FIXTURES[key](view, "ex", "p1")];
      const cues = events.flatMap((event) => {
        env.memory.remember([event]);
        return planFx({ ...galleryEntry([event], view), durationMs: 400 }, view, env);
      });
      expect(cues, key).toEqual([]);
    }
  });
});
