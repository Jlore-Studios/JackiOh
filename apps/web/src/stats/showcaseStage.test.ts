// R1584000: the stat showcase's one-card stage plays the board's own entries and effects. R2649000: the
// fifteen animations of the game's own mechanics, and the aura that leads three of them.

import { describe, expect, it } from "vitest";

import { BRITTLE_LAST_COLOR, TEMPORARY_LAST_COLOR } from "../cards/marks.ts";
import { createFxMemory } from "../fx/memory.ts";
import { animationRow } from "../game/animations.ts";
import { testid, type Side } from "../game/contract.ts";
import {
  DESTROY_MARK,
  DESTROY_MARK_COLOR,
  FROZEN_MARK,
  FROZEN_MARK_COLOR,
  SHOWCASE_BEAT_GAP_MS,
  SHOWCASE_INTERVAL_MS,
  SHOWCASE_LEAD_MS,
} from "./config.ts";
import {
  STAGE_ANIMATIONS,
  STAGE_INSTANCE_ID,
  STAGE_LANE,
  firstBeatAt,
  stageCues,
  stageScript,
  stageView,
  type StageAnimation,
} from "./showcaseStage.ts";

const CARD = "core-004";
const SIDES: readonly Side[] = ["you", "opponent"];

/** R2649000: the animations whose card leaves the stage and comes back once they have played. */
const COMES_BACK: readonly StageAnimation[] = ["death", "crumble", "expire", "doom", "exile", "bounce"];

function kinds(animation: StageAnimation, side: Side, beat: number, intensity = 1): string[] {
  const script = stageScript(animation, CARD, side);
  return stageCues(script.beats[beat]!, intensity, createFxMemory()).map((cue) => cue.kind);
}

describe("R1584000 the stage's view", () => {
  it("R1584000 holds the card as a unit on its side's one lane and nothing else", () => {
    for (const side of SIDES) {
      const view = stageView(CARD, side);
      const other = side === "you" ? "opponent" : "you";
      const unit = view[side].units[STAGE_LANE];
      expect(unit?.instanceId).toBe(STAGE_INSTANCE_ID);
      expect(unit?.defId).toBe(CARD);
      expect(view[side].units).toHaveLength(1);
      expect(view[other].units).toEqual([null]);
      expect(view.pending).toBeNull();
      expect(view.result).toBeNull();
    }
  });
});

describe("R1584000 the stage's beats", () => {
  it("R1584000 each animation's beats are the board's own entries on the stage's card or zone", () => {
    for (const animation of STAGE_ANIMATIONS) {
      for (const side of SIDES) {
        const script = stageScript(animation, CARD, side);
        expect(script.beats.length).toBeGreaterThan(0);
        for (const beat of script.beats) {
          expect(beat.entry.durationMs).toBe(animationRow(beat.entry.type)?.durationMs);
          const keys = [...beat.entry.frames.keys()];
          const card = testid.card(STAGE_INSTANCE_ID);
          const zone = testid.zone(beat.side, "units", STAGE_LANE);
          expect(keys.length === 1 && (keys[0] === card || keys[0] === zone), `${animation} on ${side}: ${keys.join()}`).toBe(true);
        }
        expect(script.side).toBe(side);
        expect([animation, script.returns]).toEqual([animation, COMES_BACK.includes(animation)]);
      }
    }
  });

  it("R1584000 the Allure stat marks the card, then takes it to the other side", () => {
    for (const side of SIDES) {
      const script = stageScript("allure", CARD, side);
      const other = side === "you" ? "opponent" : "you";
      expect(script.beats.map((beat) => beat.entry.type)).toEqual(["marked", "controlChanged"]);
      expect(script.beats.map((beat) => beat.side)).toEqual([side, other]);
    }
  });

  it("R2648000 cast plays the board's cardPlayed entry on the stage card, on either side", () => {
    for (const side of SIDES) {
      const script = stageScript("cast", CARD, side);
      expect(script.beats.map((beat) => beat.entry.type)).toEqual(["cardPlayed"]);
      expect([...script.beats[0]!.entry.frames]).toEqual([[testid.card(STAGE_INSTANCE_ID), "cardPlayed"]]);
      expect(script.beats[0]!.side).toBe(side);
      expect(script.returns).toBe(false);
      expect(kinds("cast", side, 0)).toEqual(expect.arrayContaining(["burst", "ring"]));
    }
  });

  it("R1584000 the stage plays the board's own effects for each beat", () => {
    expect(kinds("death", "opponent", 0)).toContain("crack");
    expect(kinds("allure", "you", 0)).toContain("brand");
    expect(kinds("trap", "you", 0)).toContain("ring");
    expect(kinds("radiant", "you", 0)).toContain("sheen");
    expect(kinds("shield", "you", 0)).toContain("ring");
    for (const animation of STAGE_ANIMATIONS) expect(kinds(animation, "you", 0, 0)).toEqual([]);
  });

  it("R1584000 the way in, every beat and the way back fit in one interval", () => {
    for (const animation of STAGE_ANIMATIONS) {
      for (const side of SIDES) {
        const script = stageScript(animation, CARD, side);
        const beats = script.beats.reduce((sum, beat) => sum + beat.entry.durationMs + SHOWCASE_BEAT_GAP_MS, 0);
        const total = firstBeatAt(script) + beats + (script.returns ? SHOWCASE_LEAD_MS : 0);
        expect(total, `${animation} takes ${String(total)} ms`).toBeLessThan(SHOWCASE_INTERVAL_MS);
      }
    }
  });
});

describe("R2649000 the game's own mechanics on the stage", () => {
  /** Each new animation's beats: the board event, its keyframe and its effects recipe. */
  const BEATS: Readonly<Record<string, readonly (readonly [string, string, string])[]>> = {
    crumble: [["crumbled", "jk-dissolve", "death"]],
    expire: [["exiled", "jk-exile-fade", "void"]],
    inTime: [["cardPlayed", "jk-card-played", "cast"]],
    doom: [
      ["marked", "jk-radiant-pulse", "brand"],
      ["destroyed", "jk-dissolve", "death"],
    ],
    stamp: [
      ["summoned", "jk-summon-scale", "summon"],
      ["summoned", "jk-summon-scale", "summon"],
      ["summoned", "jk-summon-scale", "summon"],
    ],
    exile: [["exiled", "jk-exile-fade", "void"]],
    bounce: [["bounced", "jk-bounce-to-hand", "bounce"]],
    hit: [["damage", "jk-damage-shake", "impact"]],
    heal: [["healed", "jk-heal-pop", "heal"]],
    armor: [["damageAbsorbed", "jk-armor-absorb", "armor"]],
    reborn: [
      ["destroyed", "jk-dissolve", "death"],
      ["summoned", "jk-summon-scale", "summon"],
    ],
    windfury: [
      ["attackDeclared", "jk-lunge", "lunge"],
      ["attackDeclared", "jk-lunge", "lunge"],
    ],
    freeze: [["marked", "jk-radiant-pulse", "brand"]],
    thaw: [
      ["marked", "jk-radiant-pulse", "brand"],
      ["marked", "jk-radiant-pulse", "brand"],
    ],
    launch: [["starshipLaunched", "jk-radiant-pulse", "brand"]],
  };

  it("R2649000 each new animation plays the board's entries for its events on the stage card or its zone", () => {
    expect(STAGE_ANIMATIONS.slice(8)).toEqual(Object.keys(BEATS));
    for (const [animation, expected] of Object.entries(BEATS) as [StageAnimation, (typeof BEATS)[string]][]) {
      for (const side of SIDES) {
        const script = stageScript(animation, CARD, side);
        expect([animation, script.beats.map((beat) => beat.entry.type)]).toEqual([animation, expected.map(([type]) => type)]);
        script.beats.forEach((beat, index) => {
          const row = animationRow(beat.entry.type);
          expect([animation, row?.animation, row?.fx?.recipe]).toEqual([animation, expected[index]?.[1], expected[index]?.[2]]);
          expect(beat.side).toBe(side);
          // The card's own frame, but for a summon's, which the board plays on the card's zone.
          const at = beat.entry.type === "summoned" ? testid.zone(side, "units", STAGE_LANE) : testid.card(STAGE_INSTANCE_ID);
          expect([animation, [...beat.entry.frames]]).toEqual([animation, [[at, beat.entry.type]]]);
        });
      }
    }
  });

  it("R2649000 crumble wears BRITTLE_LAST_COLOR, expire and inTime TEMPORARY_LAST_COLOR, the rest none, and an aura holds the first beat a gap longer", () => {
    const auras: Partial<Record<StageAnimation, string>> = { crumble: BRITTLE_LAST_COLOR, expire: TEMPORARY_LAST_COLOR, inTime: TEMPORARY_LAST_COLOR };
    expect([BRITTLE_LAST_COLOR, TEMPORARY_LAST_COLOR]).toEqual(["gray", "fuchsia"]);
    for (const animation of STAGE_ANIMATIONS) {
      for (const side of SIDES) {
        const script = stageScript(animation, CARD, side);
        expect([animation, script.aura]).toEqual([animation, auras[animation]]);
        expect(firstBeatAt(script)).toBe(SHOWCASE_LEAD_MS + (auras[animation] === undefined ? 0 : SHOWCASE_BEAT_GAP_MS));
      }
    }
  });

  it("R2649000 doom's first beat is the red destroy mark, freeze's the frost, and thaw adds the frost, then lifts it", () => {
    for (const side of SIDES) {
      const doom = stageScript("doom", CARD, side).beats.map((beat) => beat.entry.events[0]);
      expect(doom[0]).toEqual({ type: "marked", instanceId: STAGE_INSTANCE_ID, mark: DESTROY_MARK, color: DESTROY_MARK_COLOR, added: true });
      expect([DESTROY_MARK, DESTROY_MARK_COLOR]).toEqual(["destroy", "red"]);
      const frost = { type: "marked", instanceId: STAGE_INSTANCE_ID, mark: FROZEN_MARK, color: FROZEN_MARK_COLOR };
      expect(stageScript("freeze", CARD, side).beats.map((beat) => beat.entry.events[0])).toEqual([{ ...frost, added: true }]);
      expect(stageScript("thaw", CARD, side).beats.map((beat) => beat.entry.events[0])).toEqual([
        { ...frost, added: true },
        { ...frost, added: false },
      ]);
      expect([FROZEN_MARK, FROZEN_MARK_COLOR]).toEqual(["frozen", "cyan"]);
    }
  });

  it("R2649000 stamp is three summoned beats, windfury two lunges, and reborn a death then a summon", () => {
    for (const side of SIDES) {
      expect(stageScript("stamp", CARD, side).beats.map((beat) => beat.entry.type)).toEqual(["summoned", "summoned", "summoned"]);
      expect(stageScript("windfury", CARD, side).beats.map((beat) => beat.entry.type)).toEqual(["attackDeclared", "attackDeclared"]);
      const reborn = stageScript("reborn", CARD, side);
      expect(reborn.beats.map((beat) => beat.entry.type)).toEqual(["destroyed", "summoned"]);
      // The summon brings the card back, so it needs no return of its own.
      expect(reborn.returns).toBe(false);
    }
  });

  it("R2649000 the new animations play the board's own effects", () => {
    expect(kinds("crumble", "you", 0)).toContain("crack");
    expect(kinds("doom", "you", 0)).toContain("brand");
    expect(kinds("freeze", "opponent", 0)).toContain("brand");
    // The board's brand recipe draws a mark's sigil, and a launch carries no mark: the board plays its keyframe alone, and so does the stage.
    expect(kinds("launch", "you", 0)).toEqual([]);
    expect(kinds("inTime", "you", 0)).toEqual(expect.arrayContaining(["burst", "ring"]));
  });
});
