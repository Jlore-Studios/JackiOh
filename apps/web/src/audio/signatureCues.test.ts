// R3400001: Meditative+'s and Training Grounds' signature sounds. Each card sounds its own effect once, a
// Spell as its play is read and a Unit as it lands face up in a unit zone; a hidden play, a face-down set
// and every other card sound nothing more (R203), and `cuesFor` appends it after the event's row.
// R3402001: Core's ten Legendaries and Mythics each sound a signature of their own, once, on the event that
// is the card's first beat (a Trap's firing, a Unit's landing, every other card's play), each sound within the
// card's signature ceiling (fx/constants.ts), and none for a card the viewer cannot read (R203), a face-down
// set or another card.

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
    // R3402001: Core's ten follow them (above).
    expect(Object.keys(SIGNATURE_SFX).filter((defId) => !defId.startsWith("core-"))).toEqual(EIGHT);
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
