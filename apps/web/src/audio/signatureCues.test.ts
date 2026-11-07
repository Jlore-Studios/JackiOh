// R3402001: ten Core Legendaries and Mythics each sound a signature of their own, once, on the event that is
// the card's first beat (a Trap's firing, every other card's play), each sound within the card's signature
// ceiling (fx/constants.ts), and none for a card the viewer cannot read (R203), a face-down set or another card.

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

const SIGNATURES = Object.entries(SIGNATURE_SFX);
const SIGNATURE_IDS: ReadonlySet<SfxId> = new Set(SIGNATURES.map(([, signature]) => signature.id));
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
const beatOf = (defId: string, on: "cardPlayed" | "trapFired"): GameEvent => (on === "cardPlayed" ? played(defId) : fired(defId));

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
    expect(SIGNATURE_IDS.size, "no two cards share a sound").toBe(SIGNATURES.length);
    for (const [defId, { id, on }] of SIGNATURES) {
      const def = CATALOG[defId];
      expect(def, defId).toBeDefined();
      expect(["Legendary", "Mythic"], defId).toContain(def?.printedRarity ?? def?.rarity);
      expect(SFX_IDS, defId).toContain(id);
      expect(SFX[id].durationMs, `${defId} ${id}`).toBeLessThanOrEqual(ceilingMs(defId));
      // A Trap's first beat is its firing: its play is a face-down set that every Trap's sounds like.
      expect(on, defId).toBe(def?.type === "Trap" ? "trapFired" : "cardPlayed");
    }
  });

  it("R3402001 each signature sounds its own sfx once, on its first beat", () => {
    for (const [defId, { id, on }] of SIGNATURES) {
      const event = beatOf(defId, on);
      expect(signatureCues(event, ctxFor()), defId).toEqual([{ kind: "sfx", id, delayMs: 0 }]);
      // Through the whole table, from either seat: once, and no other card's.
      for (const viewer of ["p1", "p2"] as const) expect(signaturesIn(cuesFor(event, ctxFor(viewer))), `${defId} to ${viewer}`).toEqual([id]);
    }
  });

  it("R3402001 cuesFor sounds the signature after the row's own, unpanned", () => {
    for (const [defId, { id, on }] of SIGNATURES) {
      const cues = cuesFor(beatOf(defId, on), ctxFor());
      expect(cues.length, defId).toBeGreaterThan(1);
      expect(cues.at(-1), defId).toEqual({ kind: "sfx", id, delayMs: 0 });
    }
  });

  it("R3402001 a card the viewer cannot read sounds no signature", () => {
    for (const event of [played(HIDDEN_DEF_ID), fired(HIDDEN_DEF_ID), fired(HIDDEN_DEF_ID, { controller: "p2" })]) {
      expect(signatureCues(event, ctxFor()), JSON.stringify(event)).toEqual([]);
      expect(signaturesIn(cuesFor(event, ctxFor())), JSON.stringify(event)).toEqual([]);
    }
  });

  it("R3402001 a Trap's face-down set, or any card set face down, sounds no signature", () => {
    for (const [defId, { on }] of SIGNATURES) {
      // The owner's own set names the card, but a set sounds as every set does.
      const sets = on === "trapFired" ? [played(defId)] : [played(defId, { instanceId: "c2", formerId: "c1" })];
      for (const event of sets) {
        expect(signatureCues(event, ctxFor()), defId).toEqual([]);
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
      // A signature card on an event that is not its beat: a Spell never fires, a Unit's arrival is not its play.
      fired("core-097"),
      { type: "summoned", player: "p1", instanceId: "u1", defId: "core-052", row: "units", lane: 2 },
      { type: "cardResolved", player: "p1", instanceId: "c1", defId: "core-099", permanent: false, costPaid: 4 },
      { type: "destroyed", instanceId: "u1", defId: "core-052", owner: "p1", controller: "p1", attack: 3, maxHealth: 3, killerId: null },
      // A Trap firing again (R2657001) reveals nothing, so its signature has sounded already.
      fired("core-096", { again: true }),
    ];
    for (const event of others) {
      expect(signatureCues(event, ctxFor()), JSON.stringify(event)).toEqual([]);
      expect(signaturesIn(cuesFor(event, ctxFor())), JSON.stringify(event)).toEqual([]);
    }
  });
});
