// R3407000: Stinkyscape part two's signature pass as a whole (#3407): every card of it is a signature of its
// type's shape (a Unit's entrance and its look on its own tile, a Field Spell's entrance and its look on its
// zone, a Spell's cast of its own, a Mythic's own takeover look), each signature look one card's own, each
// permanent wearing its look where it stands; and, R202, from the opponent's seat no beat of a card with a
// Discover, a hand or a deck shows a card: those events redacted as the server redacts them (`view_for.rs`
// `redact_event`: an added, drawn or shuffled card's ids hidden, a prompt never sent), the beat still draws,
// no cue names a card and none reaches into a hand but through its pile.

import { describe, expect, it } from "vitest";

import { faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import { asTheirsAll, CARD_ID, cuesOfRun, planWithOffsets } from "../../../test/signaturePass.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { AURA_SIGNATURE_LOOK_NAMES } from "../../aura/looks.ts";
import { auraZones, planAuras } from "../../aura/plan.ts";
import { galleryBoardOf, galleryCardEvents, galleryPlayOf } from "../../motifs/gallery.ts";
import { TAKEOVER_ART_LOOKS } from "../../takeover.ts";
import { testid } from "../../../game/contract.ts";
import { FX_CARD_TABLE, isSignature } from "../tables.ts";
import { STINKYSCAPE_TWO_SIGNATURES } from "./signature.ts";

const HIDDEN = "hidden";

/** The cards whose signature plays a Discover, a hand or a deck: the R202 pass plays each from the other seat. */
const SECRET_KEEPERS = [
  "stinkyscape-018-2",
  "stinkyscape-018-3",
  "stinkyscape-109",
  "stinkyscape-111",
  "stinkyscape-112",
  "stinkyscape-112-1",
  "stinkyscape-147",
  "stinkyscape-148",
  "stinkyscape-148-1",
  "stinkyscape-149",
  "stinkyscape-150",
  "stinkyscape-153",
  "stinkyscape-t-narcor",
  "stinkyscape-t-red-narcor",
] as const;

const typeOf = (defId: string): string | undefined => CATALOG[defId]?.type;
const rarityOf = (defId: string): string | undefined => CATALOG[defId]?.printedRarity ?? CATALOG[defId]?.rarity;

describe("R3407000 Stinkyscape part two's signatures", () => {
  it("R3407000 every card of the pass is a signature of its type's shape, a Mythic with its own takeover look", () => {
    expect(STINKYSCAPE_TWO_SIGNATURES).toHaveLength(23);
    const looks: string[] = [];
    for (const defId of STINKYSCAPE_TWO_SIGNATURES) {
      const entry = FX_CARD_TABLE.get(defId);
      if (entry === undefined || entry.kind === "none" || entry.kind === "setPiece") throw new Error(`${defId} has no signature entry`);
      expect(isSignature(entry), defId).toBe(true);
      const type = typeOf(defId);
      const aura = entry.aura;
      if (type === "Spell") {
        expect(entry.kind, defId).toBe("bespoke");
        expect(entry.entrance, defId).toBeUndefined();
        expect(aura, defId).toBeUndefined();
      } else {
        expect(entry.entrance, `${defId} enters as its own`).toBeDefined();
        expect(aura?.scope, defId).toBe(type === "Unit" ? "tile" : "zone");
        if (aura !== undefined) looks.push(aura.look);
      }
      if (rarityOf(defId) === "Mythic") expect(TAKEOVER_ART_LOOKS.has(entry.takeover ?? "prismatic"), defId).toBe(true);
      else expect(entry.takeover, defId).toBeUndefined();
    }
    // Each signature look is one card's own.
    expect([...looks].sort()).toEqual([...AURA_SIGNATURE_LOOK_NAMES].sort());
  });

  it("R3407000 every Unit wears its look on its own tile and every Field Spell on its own zone, face up only", () => {
    for (const defId of STINKYSCAPE_TWO_SIGNATURES) {
      const type = typeOf(defId);
      if (type === "Spell") continue;
      const view = fullBoardView();
      const entry = FX_CARD_TABLE.get(defId);
      const aura = entry?.kind === "bespoke" || entry?.kind === "hooks" ? entry.aura : undefined;
      if (type === "Unit") {
        const unit = view.you.units[0];
        if (unit === null || unit === undefined) throw new Error("no unit to stand in");
        view.you.units = [{ ...unit, defId }, ...view.you.units.slice(1)];
        const worn = planAuras(view, new Map()).find((spec) => spec.defId === defId);
        expect(worn, defId).toMatchObject({ key: `unit-${unit.instanceId}`, side: "you", lane: 1, scope: "tile", look: aura?.look });
        if (worn !== undefined) expect(auraZones(worn), defId).toEqual([testid.zone("you", "units", 1)]);
        // Hidden, it wears none (R202).
        view.you.units = [{ ...unit, defId: HIDDEN }, ...view.you.units.slice(1)];
        expect(planAuras(view, new Map()).some((spec) => spec.key === `unit-${unit.instanceId}`), defId).toBe(false);
      } else {
        const field = faceUpBackrow("p1", { defId, type: "Field Spell", instanceId: "f-own" });
        view.you.backrow = [field, ...view.you.backrow.slice(1)];
        const worn = planAuras(view, new Map()).find((spec) => spec.defId === defId);
        expect(worn, defId).toMatchObject({ key: "field-f-own", side: "you", lane: 1, scope: "zone", look: aura?.look });
        if (worn !== undefined) expect(auraZones(worn), defId).toEqual([testid.zone("you", "backrow", 1)]);
      }
    }
  });

  it("R3407000 R202 from the opponent's seat no beat of a card with a Discover, a hand or a deck names a card or reaches into a hand", () => {
    for (const defId of SECRET_KEEPERS) {
      const entry = FX_CARD_TABLE.get(defId);
      const play = entry === undefined ? null : galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      // Its gallery play as the opponent's: they play it, the viewer (p1) reads what the server sends.
      const view = fullBoardView();
      const mine = galleryCardEvents(defId, play, galleryBoardOf(defId, view), 1);
      const theirs = asTheirsAll(mine);
      expect(theirs.some((event) => event.type === "cardPlayed" && event.player === "p2"), defId).toBe(true);
      const cues = cuesOfRun(planWithOffsets(theirs, view));
      expect(cues.length, `${defId} still draws`).toBeGreaterThan(0);
      for (const cue of cues) {
        const json = JSON.stringify(cue);
        expect(json, `${defId} ${cue.kind}`).not.toMatch(CARD_ID);
        expect(json, `${defId} ${cue.kind}`).not.toContain('"handCard"');
      }
    }
  });
});
