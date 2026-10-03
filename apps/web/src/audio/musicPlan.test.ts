// R631 (SPEC §10.11 "Music"): the priority stack, and dynamic music off.

import { describe, expect, it } from "vitest";

import { chooseMusic, defaultTrack, type MusicMoment } from "./musicPlan.ts";
import type { MusicManifest, MusicTrack } from "./types.ts";

const LOOP: MusicTrack = { hash: "x", bytes: 1, bpm: 120, beatsPerBar: 4, loop: true, intro: 0, duration: 20, loopStart: 3, loopEnd: 19, handoff: null };

/** Two in-game tracks for Tavern, one for EDM, and none for Lo-fi. */
const MANIFEST: MusicManifest = {
  version: 1,
  format: "test",
  files: { "tavern-1": LOOP, "tavern-2": LOOP, "tavern-danger": LOOP, "edm-1": LOOP, menu: LOOP },
};

const CALM: MusicMoment = { station: "tavern", rotation: 0, dynamic: true, result: null, theme: null, lowHealth: false, opponentTurn: false };

describe("R631 the music's priority stack", () => {
  it("R631 plays the station's in-game track when nothing outranks it", () => {
    expect(chooseMusic(CALM, MANIFEST)).toEqual({ track: "tavern-1", opponentTurn: false });
  });

  it("R631 puts the opponent's-turn mix on the in-game track only", () => {
    expect(chooseMusic({ ...CALM, opponentTurn: true }, MANIFEST)).toEqual({ track: "tavern-1", opponentTurn: true });
    expect(chooseMusic({ ...CALM, opponentTurn: true, lowHealth: true }, MANIFEST)).toEqual({ track: "tavern-danger", opponentTurn: false });
    expect(chooseMusic({ ...CALM, opponentTurn: true, theme: "mythic-zephyrs" }, MANIFEST)).toEqual({ track: "mythic-zephyrs", opponentTurn: false });
  });

  it("R631 ranks the result over a Mythic theme, a theme over low health, and low health over the in-game track", () => {
    const all: MusicMoment = { ...CALM, result: "defeat", theme: "mythic-zephyrs", lowHealth: true, opponentTurn: true };
    expect(chooseMusic(all, MANIFEST).track).toBe("defeat");
    expect(chooseMusic({ ...all, result: null }, MANIFEST).track).toBe("mythic-zephyrs");
    expect(chooseMusic({ ...all, result: null, theme: null }, MANIFEST).track).toBe("tavern-danger");
    expect(chooseMusic({ ...all, result: null, theme: null, lowHealth: false }, MANIFEST).track).toBe("tavern-1");
  });

  it("R631 names each result's own sting", () => {
    expect(chooseMusic({ ...CALM, result: "victory" }, MANIFEST).track).toBe("victory");
    expect(chooseMusic({ ...CALM, result: "defeat" }, MANIFEST).track).toBe("defeat");
    expect(chooseMusic({ ...CALM, result: "draw" }, MANIFEST).track).toBe("draw");
  });

  it("R631 with dynamic music off plays the station's in-game track and nothing else", () => {
    const busy: MusicMoment = { ...CALM, dynamic: false, result: "victory", theme: "mythic-zephyrs", lowHealth: true, opponentTurn: true };
    expect(chooseMusic(busy, MANIFEST)).toEqual({ track: "tavern-1", opponentTurn: false });
  });

  it("R631 rotates a station's in-game tracks from match to match", () => {
    expect([0, 1, 2, 3].map((rotation) => defaultTrack("tavern", rotation, MANIFEST))).toEqual(["tavern-1", "tavern-2", "tavern-1", "tavern-2"]);
    expect(defaultTrack("edm", 5, MANIFEST)).toBe("edm-1");
    // A station with no in-game track at all falls back to the menu theme rather than silence.
    expect(defaultTrack("lofi", 0, MANIFEST)).toBe("menu");
  });

  it("R631 the shipped manifest gives every station two in-game tracks to rotate", () => {
    for (const station of ["tavern", "edm", "lofi", "epic"] as const) {
      expect(defaultTrack(station, 0)).toBe(`${station}-1`);
      expect(defaultTrack(station, 1)).toBe(`${station}-2`);
    }
  });
});
