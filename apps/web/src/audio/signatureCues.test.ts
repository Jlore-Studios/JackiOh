// R3400001, R3405001: Meditative+'s, Training Grounds' and Meditative's signature sounds. Each card sounds its
// own effect once, a Spell or a Field Spell as its play is read and a Unit as it lands face up in a unit zone;
// R3405000: The Cane, which Pareto Optimality puts onto the field without a play, as it lands face up in the
// backrow (`landed`). A hidden play, a face-down set and every other card sound nothing more (R203), and
// `cuesFor` appends it after the event's row.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { baseView } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { HIDDEN_DEF_ID } from "./constants.ts";
import { cuesFor, type CueContext } from "./cues.ts";
import { SFX } from "./sfx.ts";
import { SIGNATURE_SFX, signatureCues, type SignatureMoment } from "./signatureCues.ts";
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
const landed = (defId: string, over: Partial<Extract<GameEvent, { type: "summoned" }>> = {}): GameEvent => summoned(defId, { row: "backrow", ...over });

/** Each moment's event for `defId`. */
const MOMENTS: Readonly<Record<SignatureMoment, (defId: string) => GameEvent>> = { cardPlayed: played, summoned, landed };

/** R3400001's eight. */
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

/** R3405001: Meditative's 32, its tokens among them. */
const MEDITATIVE = [
  "meditative-007",
  "meditative-008",
  "meditative-011",
  "meditative-017",
  "meditative-020",
  "meditative-026",
  "meditative-039-4",
  "meditative-039-5",
  "meditative-040",
  "meditative-042",
  "meditative-045",
  "meditative-045-1",
  "meditative-047",
  "meditative-049",
  "meditative-049-1",
  "meditative-049-2",
  "meditative-049-3",
  "meditative-051",
  "meditative-063",
  "meditative-065",
  "meditative-071",
  "meditative-071-1",
  "meditative-075",
  "meditative-079",
  "meditative-080",
  "meditative-086",
  "meditative-087",
  "meditative-095",
  "meditative-095-1",
  "meditative-097",
  "meditative-097-8",
  "meditative-097-9",
];
const MEDITATIVE_COUNT = 32;

/** R3405000: the one card whose signature sounds as it lands in the backrow, since it reaches the field without a play. */
const THE_CANE = "meditative-071-1";

/** The moment a card's signature sounds on, from its catalog type: a Unit's landing, The Cane's, or a play. */
const expectedMoment = (defId: string): SignatureMoment =>
  defId === THE_CANE ? "landed" : CATALOG[defId]?.type === "Unit" ? "summoned" : "cardPlayed";

describe("R3400001 R3405001 the signature sounds", () => {
  it("R3400001 R3405001 each card sounds its own effect once, a Spell or Field Spell on its play and a Unit on its summon", () => {
    expect(MEDITATIVE).toHaveLength(MEDITATIVE_COUNT);
    const all = [...EIGHT, ...MEDITATIVE];
    expect(Object.keys(SIGNATURE_SFX)).toEqual(all);
    const sounds = Object.values(SIGNATURE_SFX).map((own) => own.sfx);
    expect(new Set(sounds).size, "each sound is its own").toBe(all.length);
    for (const defId of all) {
      const own = SIGNATURE_SFX[defId];
      const type = CATALOG[defId]?.type;
      expect(type, `${defId} is in the catalog`).toBeDefined();
      const moment = expectedMoment(defId);
      if (moment === "cardPlayed") expect(["Spell", "Field Spell"], defId).toContain(type);
      expect(own?.on, defId).toBe(moment);
      // Inside the audio budgets: at most 1.6 s, its gain at most 0.8.
      if (own !== undefined) {
        expect(SFX[own.sfx].durationMs, defId).toBeLessThanOrEqual(1600);
        expect(SFX[own.sfx].gain, defId).toBeLessThanOrEqual(0.8);
      }
      for (const [at, event] of Object.entries(MOMENTS) as [SignatureMoment, (id: string) => GameEvent][]) {
        // Its own moment sounds it; the card's other events sound none: once per play.
        expect(signatureCues(event(defId)), `${defId} on ${at}`).toEqual(at === moment ? [{ kind: "sfx", id: own?.sfx, delayMs: 0 }] : []);
      }
    }
  });

  it("R3400001 R202 hidden or face-down plays and other cards sound none", () => {
    expect(signatureCues(played(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    expect(signatureCues(summoned(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    // A card set face down (R227), or one landing in the backrow, is no signature's moment.
    expect(signatureCues(played("traininggrounds-037", { formerId: "old" }))).toEqual([]);
    expect(signatureCues(summoned("traininggrounds-038", { formerId: "old" }))).toEqual([]);
    expect(signatureCues(summoned("traininggrounds-038", { row: "backrow" }))).toEqual([]);
    // Other cards, and other events of the forty, sound nothing more.
    expect(signatureCues(played("core-002"))).toEqual([]);
    expect(signatureCues(summoned("core-004"))).toEqual([]);
    expect(signatureCues({ type: "addedToHand", player: "p1", instanceId: "x", defId: "traininggrounds-040" })).toEqual([]);
    expect(signatureCues({ type: "addedToHand", player: "p1", instanceId: "x", defId: "meditative-017" })).toEqual([]);
  });

  it("R3405000 landed sounds only for a face-up backrow summon", () => {
    const own = SIGNATURE_SFX[THE_CANE];
    expect(own?.on).toBe("landed");
    expect(CATALOG[THE_CANE]?.type).toBe("Field Spell");
    // Put onto the field face up by Pareto Optimality, readable, with no former id: its moment.
    expect(signatureCues(landed(THE_CANE))).toEqual([{ kind: "sfx", id: own?.sfx, delayMs: 0 }]);
    // Set face down (its fresh id's `formerId`, R227), or hidden from this seat (R202): never.
    expect(signatureCues(landed(THE_CANE, { formerId: "old" }))).toEqual([]);
    expect(signatureCues(landed(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    // In a unit zone, or as a play, it is not a landing.
    expect(signatureCues(summoned(THE_CANE))).toEqual([]);
    expect(signatureCues(played(THE_CANE))).toEqual([]);
    // Every other signature's backrow summon sounds none: a played Field Spell sounded on its play.
    for (const defId of Object.keys(SIGNATURE_SFX).filter((id) => id !== THE_CANE)) {
      expect(signatureCues(landed(defId)), defId).toEqual([]);
    }
    // The landing follows the row's cues, as every signature does.
    const all = cuesFor(landed(THE_CANE), ctx);
    expect(all.at(-1)).toEqual({ kind: "sfx", id: own?.sfx, delayMs: 0 });
  });

  it("R3400001 cuesFor appends it after the row", () => {
    const withRow = (event: GameEvent): readonly SoundCue[] => cuesFor(event, ctx);
    for (const event of [played("traininggrounds-040"), summoned("traininggrounds-039-1"), played("meditative-097"), summoned("meditative-049-3")]) {
      const all = withRow(event);
      const own = signatureCues(event);
      expect(own).toHaveLength(1);
      expect(all.length, event.type).toBeGreaterThan(own.length);
      expect(all.slice(all.length - own.length), event.type).toEqual(own);
    }
    expect(withRow(played("core-002")).some((cue) => cue.kind === "sfx" && Object.values(SIGNATURE_SFX).some((own) => own.sfx === cue.id))).toBe(false);
  });
});
