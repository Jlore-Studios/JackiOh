// R3400001: Meditative+'s three signatures (bobsPlan.ts, bobSoldier.ts, scratchPad.ts), planned as FxLayer
// plans them, an entry each: remember, then plan. Bob's Plan's ink and scrolls, Bob's entrance and each of
// his Cry's four modes, Scratch Pad's scaffold and tear; from the caster's opponent's seat no beat names a
// card or flies more than a card back (R202); every cue starts inside its entry and is gone within
// FX_MAX_TAIL_MS of its end, and nothing draws with the effects off (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { FROZEN_MARK, FROZEN_MARK_COLOR } from "../../../stats/config.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_BOBS_PLAN_TRAUMA, FX_CENTER, FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { ENTRANCES } from "../../entrances.ts";
import { paletteLook } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import type { CardRecipeKey } from "../bespoke.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const BOBS_PLAN = "meditativeplus-001";
const BOB = "meditativeplus-024";
const SCRATCH_PAD = "meditativeplus-026";
const HOLD = 1000;

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
const played = (player: PlayerId, instanceId: string, defId: string): Extract<GameEvent, { type: "cardPlayed" }> => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const added = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "addedToHand", player, instanceId, defId });

/** The play's own cast, its entry held `hold` (a Mythic's takeover) and lasting D. */
function castOf(defId: string, D: number, hold = HOLD, player: PlayerId = "p1", view: PlayerView = fullBoardView()): { cues: FxCue[]; row: string | undefined } {
  const env = envOf();
  const cast = played(player, "ex", defId);
  env.memory.remember([cast]);
  const entry = { ...galleryEntry([cast], view), durationMs: D, ...(hold > 0 ? { setPiece: { key: "takeover", holdMs: hold } } : {}) };
  const result = planCardFx(cast, { entry, view, env, D });
  return { cues: result?.cues ?? [], row: result?.row };
}

/**
 * Plays `defId` as `player` and each of `events` under it, an entry each lasting D, as FxLayer does:
 * remember, then plan through the dispatch (planCardFx). Each entry's cues and its row.
 */
function run(defId: string, events: readonly GameEvent[], D: number, player: PlayerId = "p1", view: PlayerView = fullBoardView()): { cues: FxCue[]; row: string | undefined }[] {
  const env = envOf();
  env.memory.remember([played(player, "ex", defId)]);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const result = planCardFx(event, { entry, view, env, D });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

const cuesOf = (results: readonly { cues: FxCue[] }[]): FxCue[][] => results.map((result) => result.cues);

describe("R3400001 Meditative+'s signatures", () => {
  it("R3400001 the three are signatures, each its own recipe, Bob's tile wearing a flurry", () => {
    expect(FX_CARD_TABLE.get(BOBS_PLAN)).toEqual({ kind: "bespoke", key: "bobsPlan", takeover: "summit", signature: true });
    expect(FX_CARD_TABLE.get(BOB)).toEqual({ kind: "bespoke", key: "bobSoldier", entrance: "snowplowHeave", aura: { look: "flurry", palette: "Physical/wind", scope: "tile" }, signature: true });
    expect(FX_CARD_TABLE.get(SCRATCH_PAD)).toEqual({ kind: "bespoke", key: "scratchPad", takeover: "canvas", signature: true });
  });

  it("R3400001 Bob's Plan: ink as its hold ends, then a scroll and a card back for each card it adds, the plan's smaller", () => {
    const cast = castOf(BOBS_PLAN, 1400);
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual([`burst@${String(HOLD)}`, `shake@${String(HOLD)}`]);
    const centre = tid(testid.board, FX_CENTER);
    expect(cast.cues[0]).toMatchObject({ at: centre, spread: "area", colors: paletteLook("Nerd/ink")?.colors });
    expect(cast.cues[1]).toEqual({ kind: "shake", trauma: FX_BOBS_PLAN_TRAUMA, delayMs: HOLD });
    // The pick and its plan, as the gallery adds them.
    const view = fullBoardView();
    const [pick, plan] = run(BOBS_PLAN, RECIPE_FIXTURES.bobsPlan(view, "ex", "p1"), 400, "p1", view);
    const hand = tid(animTestid.hand("you"));
    expect(pick?.row).toBe("replace");
    expect(pick?.cues).toEqual([
      { kind: "figure", figure: "scroll", at: centre, tint: expect.anything() as unknown, delayMs: 0, durationMs: 400 },
      { kind: "ghost", from: centre, to: hand, delayMs: 80, durationMs: 320 },
    ]);
    expect(plan?.cues[0]).toMatchObject({ kind: "figure", figure: "scroll", size: "small" });
    expect(plan?.cues[1]).toEqual(pick?.cues[1]);
    // Its cards are keyed by count, never by what they are: other cards draw the same.
    const [other] = run(BOBS_PLAN, [added("p1", "x", "core-002")], 400, "p1", view);
    expect(other?.cues).toEqual(pick?.cues);
    // The other player's card, or a hidden play, draws nothing.
    expect(cuesOf(run(BOBS_PLAN, [added("p2", "hidden", "hidden")], 400))).toEqual([[]]);
    expect(cuesOf(run("hidden", RECIPE_FIXTURES.bobsPlan(view, "ex", "p1"), 400))).toEqual([[], []]);
  });

  it("R3400001 Bob, Wisconsin's Strongest Soldier: he drops in snow, heaves the plow and slams it, frost cracking into the enemy's zone of his lane", () => {
    const view = fullBoardView();
    const D = 400;
    const slam = frac(FX_SLAM_AT, D);
    const tgt = testid.zone("you", "units", 3);
    const landing: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player: "p1", instanceId: "bob", defId: BOB, row: "units", lane: 3 };
    const cues = ENTRANCES.snowplowHeave({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing });
    expect(beats(cues)).toEqual(["burst@0", "figure@0", `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ preset: "frost", at: tid(tgt, FOOT) });
    expect(cues[1]).toMatchObject({ figure: "plow", at: tid(tgt), durationMs: D + 300, side: "you" });
    expect(cues[3]).toMatchObject({ preset: "frost", at: tid(testid.zone("opponent", "units", 3)) });
    expect(cues[4]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.snowplowHeave, delayMs: slam });
    // Through the dispatch, on his own landing.
    const env = envOf();
    env.memory.remember([played("p1", "bob", BOB), landing]);
    expect(planCardFx(landing, { entry: { ...galleryEntry([landing], view), durationMs: D }, view, env, D })).toEqual({ cues, row: "replace" });
    // The other seat's Bob lands on its side and frosts this one's zone.
    const theirs = ENTRANCES.snowplowHeave({ D, tgt: testid.zone("opponent", "units", 2), intensity: 1, view, landing: { ...landing, player: "p2", lane: 2 } });
    expect(theirs[1]).toMatchObject({ side: "opponent" });
    expect(theirs[3]).toMatchObject({ at: tid(testid.zone("you", "units", 2)) });
  });

  it("R3400001 Bob's Cry: frost sweeps every enemy Unit at the first freeze, a mitten plucks a stolen Unit, Coins fly from his hero, a Discovered card wears its owner's sheen, a drawn card's copies fan in", () => {
    const view = fullBoardView();
    const D = 400;
    // Freeze: the first mark frosts every enemy unit zone the board shows, the rest draw nothing more.
    const freeze = run(BOB, RECIPE_FIXTURES.bobSoldier(view, "ex", "p1"), D, "p1", view);
    expect(freeze).toHaveLength(5);
    expect(freeze[0]?.cues.map((cue) => (cue.kind === "burst" && cue.at.kind === "testid" ? `${cue.preset} ${cue.at.testid}@${String(cue.delayMs)}` : cue.kind))).toEqual(
      [1, 2, 3, 4, 5].map((lane) => `frost zone-opponent-units-${String(lane)}@0`),
    );
    for (const later of freeze.slice(1)) expect(later).toEqual({ cues: [], row: "replace" });
    // A thaw is no freeze.
    const foe = view.opponent.units[0]?.instanceId ?? "";
    expect(run(BOB, [{ type: "marked", instanceId: foe, mark: FROZEN_MARK, color: FROZEN_MARK_COLOR, added: false }], D)).toEqual([{ cues: [], row: undefined }]);

    // Steal: the mitten on the unit and its card back to his hand; its own card added draws nothing more; the Coins fly from his hero.
    const [stolen, own, coin] = run(
      BOB,
      [
        { type: "stolen", instanceId: foe, defId: "core-008", from: "p2", to: "p1", zone: "field" },
        added("p1", foe, "core-008"),
        added("p2", "hidden", "hidden"),
      ],
      D,
      "p1",
      view,
    );
    expect(stolen?.cues).toEqual([
      { kind: "figure", figure: "mitten", at: tid(testid.card(foe)), tint: expect.anything() as unknown, delayMs: 0, durationMs: D, side: "you" },
      { kind: "ghost", from: tid(testid.card(foe)), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D },
    ]);
    expect(own).toEqual({ cues: [], row: "replace" });
    expect(coin?.cues).toEqual([{ kind: "ghost", from: tid(testid.hero("you")), to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: D }]);

    // Discover: the pick flies from his hero, its sheen on the hand pile for its owner alone.
    const discover = run(BOB, [{ type: "promptAnswered", player: "p1" }, added("p1", "fx-pick", "core-004")], D, "p1", view);
    expect(discover[1]?.cues).toEqual([
      { kind: "ghost", from: tid(testid.hero("you")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D },
      { kind: "sheen", at: tid(animTestid.hand("you")), delayMs: 0, durationMs: D },
    ]);
    const watched = run(BOB, [{ type: "promptAnswered", player: "p2" }, added("p2", "hidden", "hidden")], D, "p2", view);
    expect(watched[1]?.cues).toEqual([{ kind: "ghost", from: tid(testid.hero("opponent")), to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: D }]);

    // Draw 1 and fill the hand with copies: each copy's back fans in a step later, at most halfway in.
    const copies = run(BOB, [{ type: "drawn", player: "p1", instanceId: "fx-drawn", defId: "core-004" }, ...[1, 2, 3, 4, 5].map((n) => added("p1", `fx-copy-${String(n)}`, "core-004"))], D, "p1", view);
    expect(copies[0]).toEqual({ cues: [], row: undefined });
    expect(copies.slice(1).map((entry) => entry.cues[0]?.delayMs)).toEqual([0, 60, 120, 180, 200]);
    for (const entry of copies.slice(1)) expect(entry.cues[0]).toMatchObject({ kind: "ghost", from: tid(animTestid.library("you")), to: tid(animTestid.hand("you")) });
  });

  it("R3400001 Scratch Pad: pixels as the canvas's hold ends, the crafted card's scaffold folding into a card back, the paper tearing as it exiles itself", () => {
    const cast = castOf(SCRATCH_PAD, 1400);
    expect(beats(cast.cues)).toEqual([`burst@${String(HOLD)}`]);
    expect(cast.cues[0]).toMatchObject({ colors: paletteLook("Nerd/pixel")?.colors });
    const view = fullBoardView();
    const [craft, tear] = run(SCRATCH_PAD, RECIPE_FIXTURES.scratchPad(view, "ex", "p1"), 400, "p1", view);
    const centre = tid(testid.board, FX_CENTER);
    expect(craft?.cues).toEqual([
      { kind: "figure", figure: "scaffold", at: centre, tint: expect.anything() as unknown, delayMs: 0, durationMs: 240 },
      { kind: "ghost", from: centre, to: tid(animTestid.hand("you")), delayMs: 160, durationMs: 240 },
    ]);
    expect(tear).toEqual({ cues: [{ kind: "figure", figure: "tear", at: centre, tint: expect.anything() as unknown, delayMs: 0, durationMs: 320 }], row: "replace" });
    // Another card's exile is no tear of its own; its prompts draw nothing.
    expect(run(SCRATCH_PAD, [{ type: "exiled", instanceId: "other", defId: "core-002", owner: "p1" }, { type: "promptAnswered", player: "p1" }], 400)).toEqual([
      { cues: [], row: undefined },
      { cues: [], row: undefined },
    ]);
  });

  it("R3400001 R202 from the opponent's seat Meditative+'s signatures name no card and fly only card backs", () => {
    // The caster is the other seat: its cards added are hidden from this one, its prompts bare.
    const view = fullBoardView();
    const bare: GameEvent = { type: "promptAnswered", player: "p2" };
    const hidden = added("p2", "hidden", "hidden");
    const plays: readonly [string, GameEvent[]][] = [
      // Bob's Plan: the pick, its plan's Discover, the plan.
      [BOBS_PLAN, [hidden, { type: "promptOpened", player: "p2" }, bare, hidden]],
      // Bob's Discover and its Radiance (the owner's alone).
      [BOB, [bare, hidden, { type: "radiantSet", instanceId: "hidden", defId: "hidden", zone: { z: "hand", player: "p2" } }]],
      // Scratch Pad's two prompts (its cost and its canvas), the craft and its exile.
      [SCRATCH_PAD, [{ type: "promptOpened", player: "p2" }, bare, bare, hidden, { type: "exiled", instanceId: "ex", defId: SCRATCH_PAD, owner: "p2" }]],
    ];
    for (const [defId, events] of plays) {
      const cues = run(defId, events, 400, "p2", view).flatMap((entry) => entry.cues);
      expect(cues.length, defId).toBeGreaterThan(0);
      const text = JSON.stringify(cues);
      expect(text, defId).not.toMatch(/hidden|core-\d{3}|meditative|craft:|hand-card-/);
      expect(cues.some((cue) => cue.kind === "sheen"), defId).toBe(false);
      for (const cue of cues) if (cue.kind === "ghost") expect(cue.to, defId).toEqual(tid(animTestid.hand("opponent")));
    }
  });

  it("R3400001 R200 each cue starts inside its entry and ends within FX_MAX_TAIL_MS at D 120, 400 and 1400; nothing draws with effects off", () => {
    const view = fullBoardView();
    const keys: readonly [string, CardRecipeKey][] = [
      [BOBS_PLAN, "bobsPlan"],
      [BOB, "bobSoldier"],
      [SCRATCH_PAD, "scratchPad"],
    ];
    for (const D of [120, 400, 1400]) {
      for (const [defId, key] of keys) {
        const cues = [...run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), D, "p1", view).flatMap((entry) => entry.cues), ...castOf(defId, D, Math.min(HOLD, D)).cues];
        expect(cues.length, `${key} at D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${key} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "ghost") expect(cue.delayMs + cue.durationMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
      const entrance = ENTRANCES.snowplowHeave({ D, tgt: testid.zone("you", "units", 3), intensity: 1 });
      for (const cue of entrance) expect(endOf(cue), `snowplowHeave ${cue.kind} at D=${String(D)}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
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
