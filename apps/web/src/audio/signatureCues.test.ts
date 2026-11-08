// R3400001: Meditative+'s and Training Grounds' signature sounds. Each card sounds its own effect once, a
// Spell as its play is read and a Unit as it lands face up in a unit zone; a hidden play, a face-down set
// and every other card sound nothing more (R203), and `cuesFor` appends it after the event's row.
// R3395000: Stinkyscape 001 to 090's signatures join them on the same terms, a Field Spell's on its play.

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

/** R3395000: Stinkyscape 001 to 090's Legendaries and Mythics with a signature, in the map's order. */
const STINKYSCAPE = [
  "stinkyscape-006",
  "stinkyscape-007",
  "stinkyscape-016",
  "stinkyscape-018",
  "stinkyscape-026",
  "stinkyscape-034",
  "stinkyscape-035",
  "stinkyscape-037",
  "stinkyscape-039",
  "stinkyscape-044",
  "stinkyscape-058",
  "stinkyscape-059",
  "stinkyscape-060",
  "stinkyscape-061",
  "stinkyscape-066",
  "stinkyscape-070",
  "stinkyscape-071",
  "stinkyscape-073",
  "stinkyscape-074",
  "stinkyscape-080",
  "stinkyscape-085",
  "stinkyscape-086",
  "stinkyscape-087",
  "stinkyscape-088",
];

/** Each of `ids` sounds an effect no other of them does, once, a Unit's on its summon and any other card's on its play. */
function expectOwnSounds(ids: readonly string[]): void {
  const sounds = ids.map((defId) => SIGNATURE_SFX[defId]?.sfx);
  expect(new Set(sounds).size, "each sound is its own").toBe(ids.length);
  for (const defId of ids) {
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
}

/** `id`'s recipe on the fake context: the node kinds it builds and every automation it schedules. */
function graphOf(id: SfxId): string {
  const audio = new FakeAudio({ state: "running", currentTime: 1 });
  const out = audio.context.createGain();
  const firstNode = audio.nodes.length;
  const firstParam = audio.paramLog.length;
  SFX[id].recipe(audio.context, out, 1.5, {});
  return JSON.stringify([
    audio.nodes.slice(firstNode).map((node) => [node.kind, node.type]),
    audio.paramLog.slice(firstParam).map(({ event }) => [event.method, "value" in event ? event.value : null]),
  ]);
}

describe("R3400001 the signature sounds", () => {
  it("R3400001 each card sounds its own effect once, a Spell on its play and a Unit on its summon", () => {
    expect(Object.keys(SIGNATURE_SFX).slice(0, EIGHT.length)).toEqual(EIGHT);
    expectOwnSounds(EIGHT);
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

describe("R3395000 Stinkyscape 001 to 090's signature sounds", () => {
  it("R3395000 each of the 24 sounds its own effect once, a Unit on its summon and a Spell or Field Spell on its play", () => {
    expect(Object.keys(SIGNATURE_SFX).slice(EIGHT.length)).toEqual(STINKYSCAPE);
    expectOwnSounds(STINKYSCAPE);
    // No sound is shared across the whole map, the eight's included.
    expect(new Set(Object.values(SIGNATURE_SFX).map((own) => own.sfx)).size).toBe(Object.keys(SIGNATURE_SFX).length);
    // The three Field Spells are among them, so a Field Spell's play is proven, not only a Spell's.
    expect(STINKYSCAPE.filter((defId) => CATALOG[defId]?.type === "Field Spell")).toEqual(["stinkyscape-007", "stinkyscape-016", "stinkyscape-039"]);
  });

  it("R3395000 a hidden play, a face-down set and a backrow landing sound none of the 24, and cuesFor appends each after the row", () => {
    expect(signatureCues(played(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    expect(signatureCues(summoned(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    for (const defId of STINKYSCAPE) {
      const unit = CATALOG[defId]?.type === "Unit";
      const on = unit ? summoned(defId) : played(defId);
      // Set face down (R227): the event names its former id, and no signature sounds.
      expect(signatureCues(unit ? summoned(defId, { formerId: "old" }) : played(defId, { formerId: "old" })), defId).toEqual([]);
      if (unit) expect(signatureCues(summoned(defId, { row: "backrow" })), defId).toEqual([]);
      const all = cuesFor(on, ctx);
      const own = signatureCues(on);
      expect(own, defId).toHaveLength(1);
      expect(all.slice(all.length - own.length), defId).toEqual(own);
    }
  });

  it("R3395000 each of the 24 builds a graph of its own, none the same as another signature's", () => {
    const sounds = Object.values(SIGNATURE_SFX).map((own) => own.sfx);
    const graphs = sounds.map(graphOf);
    expect(new Set(graphs).size).toBe(sounds.length);
  });
});
