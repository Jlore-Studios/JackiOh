// R3400001, R3405001: Meditative+'s, Training Grounds' and Meditative's signature sounds. Each card sounds its
// own effect once, a Spell or a Meditative Field Spell as its play is read and a Unit as it lands face up in a
// unit zone; R3405000: The Cane, which Pareto Optimality puts onto the field without a play, as it lands face
// up in the backrow. A hidden play, a face-down set and every other card sound nothing more (R203), and
// `cuesFor` appends it after the event's row.
// R3402001: Core's ten Legendaries and Mythics each sound a signature of their own, once, on the event that
// is the card's first beat (a Trap's firing, a Unit's landing, every other card's play), each sound within the
// card's signature ceiling (fx/constants.ts), and none for a card the viewer cannot read (R203), a face-down
// set or another card.
// R3407001: Stinkyscape's 23 signature cards join them, a Field Spell sounding as it lands face up in the
// backrow, each with a recipe of its own.
// R3395000: Stinkyscape 001 to 090's 24 signatures join them on the same terms, its three Field Spells
// sounding as they land face up in the backrow.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { CATALOG } from "../wire/cards.ts";
import { SIGNATURE_LEGENDARY_MAX_MS, SIGNATURE_MYTHIC_MAX_MS, SIGNATURE_SPELL_SHARE } from "../fx/constants.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import { baseView, emptySide } from "../test/fixtures.ts";
import { HIDDEN_DEF_ID } from "./constants.ts";
import { cuesFor, type CueContext } from "./cues.ts";
import { SFX, SFX_IDS } from "./sfx.ts";
import { SIGNATURE_SFX, signatureCues } from "./signatureCues.ts";
import { FakeAudio } from "./test/fakeAudio.ts";
import type { SfxId, SoundCue } from "./types.ts";
import { cueCard } from "./useGameAudio.ts";
import { CARD_AUDIO } from "./voiceData.ts";

/** R3402001: Core's ten, beside R3400001's eight. */
const SIGNATURES = Object.entries(SIGNATURE_SFX).filter(([defId]) => defId.startsWith("core-"));
const SIGNATURE_IDS: ReadonlySet<SfxId> = new Set(Object.values(SIGNATURE_SFX).map((signature) => signature.sfx));
const LOOKUP = lookupFromDefs(CATALOG);

/** The shipped sound table and catalog, as the board reads them, from either seat. */
function ctxFor(viewer: PlayerId = "p1"): CueContext {
  const other: PlayerId = viewer === "p1" ? "p2" : "p1";
  const view = baseView({ viewer, you: emptySide(viewer), opponent: emptySide(other, { hand: { count: 4 } }) });
  return { view, lines: CARD_AUDIO, manaBefore: () => 0, card: (defId) => cueCard(LOOKUP, defId) };
}

const played = (defId: string, over: Partial<Extract<GameEvent, { type: "cardPlayed" }>> = {}): GameEvent => ({
  type: "cardPlayed",
  player: "p1",
  instanceId: defId === HIDDEN_DEF_ID ? HIDDEN_DEF_ID : "c1",
  defId,
  costPaid: 4,
  ...over,
});
const fired = (defId: string, over: Partial<Extract<GameEvent, { type: "trapFired" }>> = {}): GameEvent => ({
  type: "trapFired",
  instanceId: defId === HIDDEN_DEF_ID ? HIDDEN_DEF_ID : "b1",
  defId,
  controller: "p1",
  row: "backrow",
  lane: 2,
  ...over,
});
const summoned = (defId: string, over: Partial<Extract<GameEvent, { type: "summoned" }>> = {}): GameEvent => ({
  type: "summoned",
  player: "p1",
  instanceId: "c1",
  defId,
  row: "units",
  lane: 2,
  ...over,
});
const beatOf = (defId: string, on: "cardPlayed" | "summoned" | "trapFired"): GameEvent =>
  on === "cardPlayed" ? played(defId) : on === "summoned" ? summoned(defId) : fired(defId);

/** The signature sounds among an event's cues, whatever else it sounds. */
const signaturesIn = (cues: readonly SoundCue[]): SfxId[] =>
  cues.flatMap((cue) => (cue.kind === "sfx" && SIGNATURE_IDS.has(cue.id) ? [cue.id] : []));

/** The card's ceiling: its rarity's signature budget, a Spell's SIGNATURE_SPELL_SHARE of it. */
function ceilingMs(defId: string): number {
  const def = CATALOG[defId];
  const rarity = def?.printedRarity ?? def?.rarity;
  const base = rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : SIGNATURE_LEGENDARY_MAX_MS;
  return def?.type === "Spell" ? base * SIGNATURE_SPELL_SHARE : base;
}

describe("R3402001 ten Core signatures", () => {
  it("R3402001 the ten are ten sounds of their own, each a Legendary's or a Mythic's and within its ceiling", () => {
    expect(SIGNATURES).toHaveLength(10);
    expect(SIGNATURE_IDS.size, "no two cards share a sound").toBe(Object.keys(SIGNATURE_SFX).length);
    for (const [defId, { sfx: id, on }] of SIGNATURES) {
      const def = CATALOG[defId];
      expect(def, defId).toBeDefined();
      expect(["Legendary", "Mythic"], defId).toContain(def?.printedRarity ?? def?.rarity);
      expect(SFX_IDS, defId).toContain(id);
      expect(SFX[id].durationMs, `${defId} ${id}`).toBeLessThanOrEqual(ceilingMs(defId));
      // A Trap's first beat is its firing: its play is a face-down set that every Trap's sounds like. A Unit's
      // is its landing face up, as R3400001's are.
      expect(on, defId).toBe(def?.type === "Trap" ? "trapFired" : def?.type === "Unit" ? "summoned" : "cardPlayed");
    }
  });

  it("R3402001 each signature sounds its own sfx once, on its first beat", () => {
    for (const [defId, { sfx: id, on }] of SIGNATURES) {
      const event = beatOf(defId, on);
      expect(signatureCues(event), defId).toEqual([{ kind: "sfx", id, delayMs: 0 }]);
      // Through the whole table, from either seat: once, and no other card's.
      for (const viewer of ["p1", "p2"] as const) expect(signaturesIn(cuesFor(event, ctxFor(viewer))), `${defId} to ${viewer}`).toEqual([id]);
    }
  });

  it("R3402001 cuesFor sounds the signature after the row's own, unpanned", () => {
    for (const [defId, { sfx: id, on }] of SIGNATURES) {
      const cues = cuesFor(beatOf(defId, on), ctxFor());
      expect(cues.length, defId).toBeGreaterThan(1);
      expect(cues.at(-1), defId).toEqual({ kind: "sfx", id, delayMs: 0 });
    }
  });

  it("R3402001 a card the viewer cannot read sounds no signature", () => {
    for (const event of [played(HIDDEN_DEF_ID), fired(HIDDEN_DEF_ID), fired(HIDDEN_DEF_ID, { controller: "p2" })]) {
      expect(signatureCues(event), JSON.stringify(event)).toEqual([]);
      expect(signaturesIn(cuesFor(event, ctxFor())), JSON.stringify(event)).toEqual([]);
    }
  });

  it("R3402001 a Trap's face-down set, or any card set face down, sounds no signature", () => {
    for (const [defId, { on }] of SIGNATURES) {
      // The owner's own set names the card, but a set sounds as every set does.
      const sets =
        on === "trapFired"
          ? [played(defId)]
          : on === "summoned"
            ? [summoned(defId, { instanceId: "c2", formerId: "c1" }), summoned(defId, { row: "backrow" })]
            : [played(defId, { instanceId: "c2", formerId: "c1" })];
      for (const event of sets) {
        expect(signatureCues(event), defId).toEqual([]);
        expect(signaturesIn(cuesFor(event, ctxFor())), defId).toEqual([]);
      }
    }
    // A Trap's set is the set click alone, as before.
    expect(cuesFor(played("core-096"), ctxFor())).toEqual([{ kind: "sfx", id: "trapSet", delayMs: 0 }]);
  });

  it("R3402001 a different card, or the card on another event, sounds no signature", () => {
    const others: GameEvent[] = [
      played("core-004"),
      played("core-005"),
      fired("core-018"),
      fired("core-018", { again: true }),
      // A signature card on an event that is not its beat: a Spell never fires, a Unit's play is not its landing.
      fired("core-097"),
      played("core-052"),
      { type: "cardResolved", player: "p1", instanceId: "c1", defId: "core-099", permanent: false, costPaid: 4 },
      { type: "destroyed", instanceId: "u1", defId: "core-052", owner: "p1", controller: "p1", attack: 3, maxHealth: 3, killerId: null },
      // A Trap firing again (R2657001) reveals nothing, so its signature has sounded already.
      fired("core-096", { again: true }),
    ];
    for (const event of others) {
      expect(signatureCues(event), JSON.stringify(event)).toEqual([]);
      expect(signaturesIn(cuesFor(event, ctxFor())), JSON.stringify(event)).toEqual([]);
    }
  });
});

const NO_LINES: CueContext["lines"] = { voices: {}, effects: {}, cards: {}, emotes: {} };
const ctx: CueContext = { view: baseView(), lines: NO_LINES, manaBefore: () => 0 };
const landed = (defId: string, over: Partial<Extract<GameEvent, { type: "summoned" }>> = {}): GameEvent => summoned(defId, { row: "backrow", ...over });

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
const expectedMoment = (defId: string): "played" | "units" | "backrow" =>
  defId === THE_CANE ? "backrow" : CATALOG[defId]?.type === "Unit" ? "units" : "played";
/** R3395000: Stinkyscape 001 to 090's Legendaries and Mythics with a signature, in the map's order after Meditative's. */
const STINKYSCAPE_ONE = [
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

describe("R3400001 R3405001 the signature sounds", () => {
  it("R3400001 R3405001 each card sounds its own effect once, a Spell or Field Spell on its play and a Unit on its summon", () => {
    expect(MEDITATIVE).toHaveLength(MEDITATIVE_COUNT);
    // R3402001: Core's ten follow the eight (above); R3395000: Stinkyscape 001 to 090's close the table.
    const core = SIGNATURES.map(([defId]) => defId);
    expect(Object.keys(SIGNATURE_SFX)).toEqual([...EIGHT, ...core, ...STINKYSCAPE, ...MEDITATIVE, ...STINKYSCAPE_ONE]);
    const sounds = Object.values(SIGNATURE_SFX).map((own) => own.sfx);
    expect(new Set(sounds).size, "each sound is its own").toBe(sounds.length);
    const all = [...EIGHT, ...MEDITATIVE];
    for (const defId of all) {
      const own = SIGNATURE_SFX[defId];
      const type = CATALOG[defId]?.type;
      expect(type, `${defId} is in the catalog`).toBeDefined();
      const moment = expectedMoment(defId);
      if (moment === "played") expect(["Spell", "Field Spell"], defId).toContain(type);
      expect(own, defId).toEqual(moment === "played" ? { sfx: own?.sfx, on: "cardPlayed" } : { sfx: own?.sfx, on: "summoned", row: moment });
      // Inside the audio budgets: at most 1.6 s, its gain at most 0.8.
      if (own !== undefined) {
        expect(SFX[own.sfx].durationMs, defId).toBeLessThanOrEqual(1600);
        expect(SFX[own.sfx].gain, defId).toBeLessThanOrEqual(0.8);
      }
      const events = moments(defId);
      for (const at of ["played", "units", "backrow"] as const) {
        // Its own moment sounds it; the card's other events sound none: once per play.
        expect(signatureCues(events[at]), `${defId} on ${at}`).toEqual(at === moment ? [{ kind: "sfx", id: own?.sfx, delayMs: 0 }] : []);
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

  it("R3405000 The Cane sounds only as it lands face up in the backrow", () => {
    const own = SIGNATURE_SFX[THE_CANE];
    expect(own).toEqual({ sfx: own?.sfx, on: "summoned", row: "backrow" });
    expect(CATALOG[THE_CANE]?.type).toBe("Field Spell");
    // Put onto the field face up by Pareto Optimality, readable, with no former id: its moment.
    expect(signatureCues(landed(THE_CANE))).toEqual([{ kind: "sfx", id: own?.sfx, delayMs: 0 }]);
    // Set face down (its fresh id's `formerId`, R227), or hidden from this seat (R202): never.
    expect(signatureCues(landed(THE_CANE, { formerId: "old" }))).toEqual([]);
    expect(signatureCues(landed(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    // In a unit zone, or as a play, it is not a landing.
    expect(signatureCues(summoned(THE_CANE))).toEqual([]);
    expect(signatureCues(played(THE_CANE))).toEqual([]);
    // Every other Meditative signature's backrow summon sounds none: a played Field Spell sounded on its play.
    for (const defId of MEDITATIVE.filter((id) => id !== THE_CANE)) {
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
    // R3402001: Core's ten sit between them; R3405001: Meditative's follow.
    const core = SIGNATURES.map(([defId]) => defId);
    const before = [...EIGHT, ...core, ...STINKYSCAPE];
    expect(Object.keys(SIGNATURE_SFX).slice(0, before.length)).toEqual(before);
    const sounds = before.map((defId) => SIGNATURE_SFX[defId]?.sfx);
    expect(new Set(sounds).size, "no two cards share a sound").toBe(before.length);
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

describe("R3395000 Stinkyscape 001 to 090's signature sounds", () => {
  it("R3395000 each of the 24 sounds its own effect once on its own moment: a Spell's play, a Unit's landing in a unit zone, a Field Spell's in the backrow", () => {
    // R3402001: after Core's ten too.
    expect(Object.keys(SIGNATURE_SFX).slice(EIGHT.length + SIGNATURES.length + STINKYSCAPE.length + MEDITATIVE.length)).toEqual(STINKYSCAPE_ONE);
    // The three Field Spells are among them, so a Field Spell's landing is proven, not only a Spell's play.
    expect(STINKYSCAPE_ONE.filter((defId) => CATALOG[defId]?.type === "Field Spell")).toEqual(["stinkyscape-007", "stinkyscape-016", "stinkyscape-039"]);
    for (const defId of STINKYSCAPE_ONE) {
      const own = SIGNATURE_SFX[defId];
      const type = CATALOG[defId]?.type;
      expect(type, defId).toBeDefined();
      const moment = type === "Unit" ? "units" : type === "Field Spell" ? "backrow" : "played";
      expect(own, defId).toEqual(
        moment === "played" ? { sfx: own?.sfx, on: "cardPlayed" } : { sfx: own?.sfx, on: "summoned", row: moment },
      );
      // Inside the audio budgets: at most 1.6 s, its gain at most 0.8.
      if (own !== undefined) {
        expect(SFX[own.sfx].durationMs, defId).toBeLessThanOrEqual(1600);
        expect(SFX[own.sfx].gain, defId).toBeLessThanOrEqual(0.8);
      }
      const events = moments(defId);
      for (const at of ["played", "units", "backrow"] as const) {
        // On its moment it sounds once; on either other event of the same card, none.
        expect(signatureCues(events[at]), `${defId} ${at}`).toEqual(at === moment ? [{ kind: "sfx", id: own?.sfx, delayMs: 0 }] : []);
      }
    }
  });

  it("R3395000 a hidden play and a face-down set sound none of the 24, and cuesFor appends each after the row", () => {
    expect(signatureCues(played(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    expect(signatureCues(summoned(HIDDEN_DEF_ID, { instanceId: HIDDEN_DEF_ID }))).toEqual([]);
    for (const defId of STINKYSCAPE_ONE) {
      const type = CATALOG[defId]?.type;
      const on = type === "Unit" ? summoned(defId) : type === "Field Spell" ? summoned(defId, { row: "backrow" }) : played(defId);
      // Set face down (R227): the event names its former id, and no signature sounds.
      for (const event of [played(defId, { formerId: "old" }), summoned(defId, { formerId: "old" }), summoned(defId, { formerId: "old", row: "backrow" })]) {
        expect(signatureCues(event), `${defId} ${event.type}`).toEqual([]);
      }
      const all = cuesFor(on, ctx);
      const own = signatureCues(on);
      expect(own, defId).toHaveLength(1);
      expect(all.slice(all.length - own.length), defId).toEqual(own);
    }
  });

  it("R3395000 each of the 24 builds a graph of its own, none the same as another signature's", () => {
    const sounds = Object.values(SIGNATURE_SFX).map((own) => own.sfx);
    expect(new Set(sounds).size, "no two cards share a sound").toBe(Object.keys(SIGNATURE_SFX).length);
    expect(new Set(sounds.map(graphOf)).size).toBe(sounds.length);
  });
});
