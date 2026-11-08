// R3400001: Meditative+'s and Training Grounds' signature sounds. Each card sounds its own effect once, a
// Spell as its play is read and a Unit as it lands face up in a unit zone; a hidden play, a face-down set
// and every other card sound nothing more (R203), and `cuesFor` appends it after the event's row.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { baseView } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { HIDDEN_DEF_ID } from "./constants.ts";
import { cuesFor, type CueContext } from "./cues.ts";
import { SFX } from "./sfx.ts";
import { SIGNATURE_SFX, signatureCues } from "./signatureCues.ts";
import type { SoundCue } from "./types.ts";

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

describe("R3400001 the signature sounds", () => {
  it("R3400001 each card sounds its own effect once, a Spell on its play and a Unit on its summon", () => {
    expect(Object.keys(SIGNATURE_SFX)).toEqual(EIGHT);
    const sounds = Object.values(SIGNATURE_SFX).map((own) => own.sfx);
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
