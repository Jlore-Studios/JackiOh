// R3400001: Meditative+'s and Training Grounds' signature sounds. Each card sounds its own effect once, a
// Spell as its play is read and a Unit as it lands face up in a unit zone; a hidden play, a face-down set
// and every other card sound nothing more (R203), and `cuesFor` appends it after the event's row.
// R3407001: Stinkyscape's 23 signature cards join them, a Field Spell sounding as it lands face up in the
// backrow, each with a recipe of its own.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { baseView } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { HIDDEN_DEF_ID } from "./constants.ts";
import { cuesFor, type CueContext } from "./cues.ts";
import { SFX } from "./sfx.ts";
import { SIGNATURE_SFX, signatureCues } from "./signatureCues.ts";
import { FakeAudio } from "./test/fakeAudio.ts";
import type { SfxId, SoundCue } from "./types.ts";

const NO_LINES: CueContext["lines"] = { voices: {}, effects: {}, cards: {}, emotes: {} };
const ctx: CueContext = { view: baseView(), lines: NO_LINES, manaBefore: () => 0 };

const played = (defId: string, over: Partial<Extract<GameEvent, { type: "cardPlayed" }>> = {}): GameEvent => ({
  type: "cardPlayed",
  player: "p1",
  instanceId: "sig",
  defId,
  costPaid: 1,
  ...over,
});
const summoned = (defId: string, over: Partial<Extract<GameEvent, { type: "summoned" }>> = {}): GameEvent => ({
  type: "summoned",
  player: "p1",
  instanceId: "sig",
  defId,
  row: "units",
  lane: 3,
  ...over,
});

const EIGHT = [
  "meditativeplus-001",
  "meditativeplus-024",
  "meditativeplus-026",
  "traininggrounds-037",
  "traininggrounds-038",
  "traininggrounds-039",
  "traininggrounds-039-1",
  "traininggrounds-040",
];

/** R3407001: Stinkyscape's signature cards, in the table's order. */
const STINKYSCAPE = [
  "stinkyscape-018-1",
  "stinkyscape-018-2",
  "stinkyscape-018-3",
  "stinkyscape-109",
  "stinkyscape-110",
  "stinkyscape-111",
  "stinkyscape-112",
  "stinkyscape-112-1",
  "stinkyscape-115",
  "stinkyscape-116",
  "stinkyscape-147",
  "stinkyscape-147-1",
  "stinkyscape-147-2",
  "stinkyscape-148",
  "stinkyscape-148-1",
  "stinkyscape-149",
  "stinkyscape-150",
  "stinkyscape-150-1",
  "stinkyscape-151",
  "stinkyscape-152",
  "stinkyscape-153",
  "stinkyscape-t-narcor",
  "stinkyscape-t-red-narcor",
];

describe("R3400001 the signature sounds", () => {
  it("R3400001 each card sounds its own effect once, a Spell on its play and a Unit on its summon", () => {
    expect(Object.keys(SIGNATURE_SFX).slice(0, EIGHT.length)).toEqual(EIGHT);
    const sounds = EIGHT.map((defId) => SIGNATURE_SFX[defId]?.sfx);
    expect(new Set(sounds).size, "each sound is its own").toBe(EIGHT.length);
    for (const defId of EIGHT) {
      const own = SIGNATURE_SFX[defId];
      const unit = CATALOG[defId]?.type === "Unit";
      expect(own?.on, defId).toBe(unit ? "summoned" : "cardPlayed");
      // Inside the audio budgets: at most 1.6 s, its gain at most 0.8.
      if (own !== undefined) {
        expect(SFX[own.sfx].durationMs, defId).toBeLessThanOrEqual(1600);
        expect(SFX[own.sfx].gain, defId).toBeLessThanOrEqual(0.8);
      }
      const on = unit ? summoned(defId) : played(defId);
      const off = unit ? played(defId) : summoned(defId);
      expect(signatureCues(on), defId).toEqual([{ kind: "sfx", id: own?.sfx, delayMs: 0 }]);
      // The other event of the same card sounds none: once per play.
      expect(signatureCues(off), defId).toEqual([]);
    }
  });

  it("R3400001 R202 hidden or face-down plays and other cards sound none", () => {
    expect(signatureCues(played(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    expect(signatureCues(summoned(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    // A card set face down (R227), or one landing in the backrow, is no signature's moment.
    expect(signatureCues(played("traininggrounds-037", { formerId: "old" }))).toEqual([]);
    expect(signatureCues(summoned("traininggrounds-038", { formerId: "old" }))).toEqual([]);
    expect(signatureCues(summoned("traininggrounds-038", { row: "backrow" }))).toEqual([]);
    // Other cards, and other events of the eight, sound nothing more.
    expect(signatureCues(played("core-002"))).toEqual([]);
    expect(signatureCues(summoned("core-004"))).toEqual([]);
    expect(signatureCues({ type: "addedToHand", player: "p1", instanceId: "x", defId: "traininggrounds-040" })).toEqual([]);
  });

  it("R3400001 cuesFor appends it after the row", () => {
    const withRow = (event: GameEvent): readonly SoundCue[] => cuesFor(event, ctx);
    for (const event of [played("traininggrounds-040"), summoned("traininggrounds-039-1")]) {
      const all = withRow(event);
      const own = signatureCues(event);
      expect(own).toHaveLength(1);
      expect(all.length, event.type).toBeGreaterThan(own.length);
      expect(all.slice(all.length - own.length), event.type).toEqual(own);
    }
    expect(withRow(played("core-002")).some((cue) => cue.kind === "sfx" && Object.values(SIGNATURE_SFX).some((own) => own.sfx === cue.id))).toBe(false);
  });
});

/** The graph a recipe builds on the fake context: its nodes' kinds and types and every automation it schedules. */
function graphOf(id: SfxId): string {
  const audio = new FakeAudio({ state: "running", currentTime: 0 });
  const out = audio.context.createGain();
  const firstNode = audio.nodes.length;
  const firstParam = audio.paramLog.length;
  SFX[id].recipe(audio.context, out, 0, {});
  return JSON.stringify([
    audio.nodes.slice(firstNode).map((node) => [node.kind, node.type]),
    audio.paramLog.slice(firstParam).map(({ event }) => [event.method, "value" in event ? event.value : null]),
  ]);
}

/** Each card's three moments: the play, a landing in a unit zone and a landing in the backrow. */
const moments = (defId: string): { played: GameEvent; units: GameEvent; backrow: GameEvent } => ({
  played: played(defId),
  units: summoned(defId),
  backrow: summoned(defId, { row: "backrow" }),
});

describe("R3407001 Stinkyscape's signature sounds", () => {
  it("R3407001 the table holds the eight and then Stinkyscape's 23, each with a sound of its own", () => {
    expect(Object.keys(SIGNATURE_SFX)).toEqual([...EIGHT, ...STINKYSCAPE]);
    const sounds = Object.values(SIGNATURE_SFX).map((own) => own.sfx);
    expect(new Set(sounds).size, "no two cards share a sound").toBe(EIGHT.length + STINKYSCAPE.length);
  });

  it("R3407001 each of the 23 sounds once on its own moment: a Spell's play, a Unit's landing in a unit zone, a Field Spell's in the backrow", () => {
    for (const defId of STINKYSCAPE) {
      const own = SIGNATURE_SFX[defId];
      const type = CATALOG[defId]?.type;
      expect(type, defId).toBeDefined();
      const moment = type === "Unit" ? "units" : type === "Field Spell" ? "backrow" : "played";
      expect(own, defId).toEqual(
        moment === "played" ? { sfx: own?.sfx, on: "cardPlayed" } : { sfx: own?.sfx, on: "summoned", row: moment },
      );
      const events = moments(defId);
      for (const at of ["played", "units", "backrow"] as const) {
        const expected = at === moment ? [{ kind: "sfx", id: own?.sfx, delayMs: 0 }] : [];
        // On its moment it sounds once; on either other event of the same card, none.
        expect(signatureCues(events[at]), `${defId} ${at}`).toEqual(expected);
      }
    }
  });

  it("R3407001 a Field Spell sounds as it lands face up in the backrow, never on its play or in a unit zone", () => {
    const fields = STINKYSCAPE.filter((defId) => CATALOG[defId]?.type === "Field Spell");
    expect(fields).toEqual([
      "stinkyscape-018-1",
      "stinkyscape-018-2",
      "stinkyscape-018-3",
      "stinkyscape-112",
      "stinkyscape-147-2",
      "stinkyscape-148",
      "stinkyscape-152",
    ]);
    for (const defId of fields) {
      const own = SIGNATURE_SFX[defId];
      expect(signatureCues(summoned(defId, { row: "backrow", lane: 1 })), defId).toEqual([{ kind: "sfx", id: own?.sfx, delayMs: 0 }]);
      expect(signatureCues(played(defId)), defId).toEqual([]);
      expect(signatureCues(summoned(defId)), defId).toEqual([]);
    }
  });

  it("R3407001 R202 a face-down or hidden one never sounds", () => {
    expect(signatureCues(summoned(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID, row: "backrow" }))).toEqual([]);
    for (const defId of STINKYSCAPE) {
      // R227: a card the event set face down, wherever it lands and however it was played.
      for (const event of [
        played(defId, { formerId: "old" }),
        summoned(defId, { formerId: "old" }),
        summoned(defId, { formerId: "old", row: "backrow" }),
      ]) {
        expect(signatureCues(event), `${defId} ${event.type}`).toEqual([]);
      }
    }
    // Other events of the 23 sound nothing more.
    expect(signatureCues({ type: "addedToHand", player: "p1", instanceId: "x", defId: "stinkyscape-153" })).toEqual([]);
  });

  it("R3407001 cuesFor appends it after the row, a Field Spell's landing included", () => {
    for (const event of [played("stinkyscape-149"), summoned("stinkyscape-109"), summoned("stinkyscape-152", { row: "backrow" })]) {
      const all = cuesFor(event, ctx);
      const own = signatureCues(event);
      expect(own, event.type).toHaveLength(1);
      expect(all.length, event.type).toBeGreaterThan(own.length);
      expect(all.slice(all.length - own.length), event.type).toEqual(own);
    }
  });

  it("R3407001 the 23 recipes are distinct from each other and from the eight's, each within the Legendary's 1.6 s and gain 0.8", () => {
    const eight = EIGHT.flatMap((defId) => SIGNATURE_SFX[defId]?.sfx ?? []);
    const stinky = STINKYSCAPE.flatMap((defId) => SIGNATURE_SFX[defId]?.sfx ?? []);
    expect(stinky).toHaveLength(STINKYSCAPE.length);
    for (const id of stinky) {
      expect(eight, id).not.toContain(id);
      expect(SFX[id].durationMs, id).toBeLessThanOrEqual(1600);
      expect(SFX[id].gain, id).toBeLessThanOrEqual(0.8);
    }
    // No two build the same graph, nor share a recipe.
    const all = [...eight, ...stinky];
    expect(new Set(all.map((id) => SFX[id].recipe)).size).toBe(all.length);
    expect(new Set(all.map(graphOf)).size).toBe(all.length);
  });
});
