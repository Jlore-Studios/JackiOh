// C #60 Pile On — SPEC §8.6 row 60, BUILD M9 Classic row C 60: "(5) Cost, so it needs mana above the
// cap of 4 (§2.3); Recruits every permanent in your deck, top to bottom, each into its row until that
// row is full (Units to unit zones, the rest to backrow zones, traps face-down and never named in the
// opponent's view, R33), without a Cry; Spells, and permanents that no longer fit, stay in the deck in
// order; no event carries a deck position; base: if this card would go to your graveyard, from
// resolving, a discard or a burn, it goes to the bottom of your deck instead (a replacement); radiant:
// no return clause, so it goes to the graveyard; no tuned numbers".
//
// The deck holds Core cards with their own tests: Units Mr. Vanilla, Gary the Gambler and Bigot (whose
// Cry would destroy an enemy Unit — a Recruit fires none, R1), the Field Spell Mana Well, the Trap
// Sheepish and the Field Trap Bread and Butter, and Spells (Stockpile, Lunar Eclipse); a Rush Token
// card stands for a unit-token card a deck may hold (R11). Zao Gao discards and Stockpile's draw into a
// full hand burns.

import { legalActions, LIBRARY_CAP } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/060-pile-on";

const PILE_ON = "classic-060";
const VANILLA = "core-008"; // Unit 4/4
const GARY = "core-004"; // Unit 1/1, Cry: flip 5 coins
const BIGOT = "core-002"; // Unit 6/1, Cry: destroy target enemy non-Human Unit
const MANA_WELL = "core-006"; // Field Spell
const SHEEPISH = "core-041"; // Trap
const BREAD = "core-018"; // Field Trap
const STOCKPILE = "core-005"; // Spell: draw 2, heal 2
const LUNAR = "core-035"; // Spell
const ZAO_GAO = "core-080"; // Spell: discard 2 random cards; summon 2 Rush Tokens
const RUSH_TOKEN = "core-t-rush";
const FILLER = "core-005";

function libraryIds(s: Scenario): string[] {
  return s.pile("p1", "library").map((card) => card.defId);
}

function unitRow(s: Scenario): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null);
}

function backRow(s: Scenario): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.backrow("p1", lane)?.defId ?? null);
}

const MIXED_DECK = [STOCKPILE, VANILLA, MANA_WELL, LUNAR, SHEEPISH, GARY, BREAD, BIGOT] as const;

describe("C #60 Pile On", () => {
  it("costs (5) and declares no numbers; the Radiant face drops the replacement", () => {
    expect(def.id).toBe(PILE_ON);
    expect(def.cost).toBe(5);
    expect(def.params).toBeUndefined();
    expect(base.replacements).toEqual([
      { id: "pile-on", on: "toGraveyard", where: "self", instead: { to: "bottomOfLibrary" } },
    ]);
    expect(radiant.replacements).toBeUndefined();
    expect(radiant.cry).toBe(base.cry);
  });

  describe("base", () => {
    it("§2.3 at the mana cap of 4 it cannot be played: refused, and legalActions offers no play of it", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: [VANILLA] }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      expect(
        legalActions(s.state, "p1").some((action) => action.type === "play" && action.instanceId === pileOn.id),
      ).toBe(false);
      expect(() => s.play(PILE_ON)).toThrow();
      s.expectInZone(pileOn, "hand");
    });

    it("§2.3 with 5 mana it is offered and plays", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: [VANILLA], mana: 5 }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      expect(
        legalActions(s.state, "p1").some((action) => action.type === "play" && action.instanceId === pileOn.id),
      ).toBe(true);
      s.play(PILE_ON).expectMana("p1", 0);
    });

    it("recruits every permanent of the deck top to bottom, each into its row; Spells stay, in order", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: MIXED_DECK, mana: 5 }, p2: { hand: [FILLER] } });
      s.play(PILE_ON);
      expect(unitRow(s)).toEqual([VANILLA, GARY, BIGOT, null, null]);
      expect(backRow(s)).toEqual([MANA_WELL, SHEEPISH, BREAD, null, null]);
      // The Spells keep their order, and Pile On itself goes under them (the base clause).
      expect(libraryIds(s)).toEqual([STOCKPILE, LUNAR, PILE_ON]);
    });

    it("R1 no recruited card's Cry fires: Bigot destroys nothing, Gary flips no coins", () => {
      const s = scenario({
        p1: { hand: [PILE_ON, FILLER], library: [BIGOT, GARY], mana: 5 },
        p2: { hand: [FILLER], field: [VANILLA] },
      });
      s.play(PILE_ON);
      s.expectInZone(VANILLA, "field");
      s.expectStats(GARY, { attack: 1, health: 1 });
      expect(s.events.some((event) => event.type === "destroyed" || event.type === "buffed")).toBe(false);
    });

    it("R33 a recruited Trap or Field Trap is set face-down and never named in the opponent's view; a Field Spell is face-up", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: [SHEEPISH, MANA_WELL, BREAD], mana: 5 }, p2: { hand: [FILLER] } });
      s.play(PILE_ON);
      const [trap, field, fieldTrap] = [s.backrow("p1", 1), s.backrow("p1", 2), s.backrow("p1", 3)];
      expect([trap?.defId, field?.defId, fieldTrap?.defId]).toEqual([SHEEPISH, MANA_WELL, BREAD]);
      expect(trap?.faceUp).not.toBe(true);
      expect(fieldTrap?.faceUp).not.toBe(true);
      expect(field?.faceUp).toBe(true);
      const theirs = s.view("p2");
      expect(theirs.opponent.backrow[0]).toMatchObject({ faceDown: true });
      expect(theirs.opponent.backrow[2]).toMatchObject({ faceDown: true });
      expect(JSON.stringify(theirs)).not.toContain(SHEEPISH);
      expect(JSON.stringify(theirs)).not.toContain(BREAD);
      expect(JSON.stringify(theirs)).toContain(MANA_WELL);
    });

    it("stops filling a row once it is full: the permanents that no longer fit stay in the deck, in order", () => {
      const s = scenario({
        p1: { hand: [PILE_ON, FILLER], field: [VANILLA, VANILLA, VANILLA, VANILLA], library: [GARY, LUNAR, BIGOT, VANILLA, MANA_WELL], mana: 5 },
        p2: { hand: [FILLER] },
      });
      s.play(PILE_ON);
      expect(unitRow(s)).toEqual([VANILLA, VANILLA, VANILLA, VANILLA, GARY]);
      expect(backRow(s)).toEqual([MANA_WELL, null, null, null, null]);
      expect(libraryIds(s)).toEqual([LUNAR, BIGOT, VANILLA, PILE_ON]);
    });

    it("R11 a unit-token card in the deck is no card a Recruit takes: it stays", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: [RUSH_TOKEN, VANILLA], mana: 5 }, p2: { hand: [FILLER] } });
      s.play(PILE_ON);
      expect(unitRow(s)).toEqual([VANILLA, null, null, null, null]);
      expect(libraryIds(s)).toEqual([RUSH_TOKEN, PILE_ON]);
    });

    it("with an empty deck it recruits nothing, and still goes to the bottom of the deck", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], mana: 5 }, p2: { hand: [FILLER] } });
      s.play(PILE_ON);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      expect(libraryIds(s)).toEqual([PILE_ON]);
    });

    it("no event carries a deck position: the summons name only the zone each lands in", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: MIXED_DECK, mana: 5 }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      s.play(pileOn);
      expect(s.lastEvents.filter((event) => event.type === "summoned")).toHaveLength(6);
      // The one event that names a deck slot is Pile On's own public move to the bottom of the deck.
      const positioned = s.lastEvents.filter((event) => "position" in event);
      expect(positioned).toEqual([expect.objectContaining({ type: "shuffledIn", instanceId: pileOn.id })]);
      const radiantPlay = scenario({
        p1: { hand: [{ def: PILE_ON, radiant: true }, FILLER], library: MIXED_DECK, mana: 5 },
        p2: { hand: [FILLER] },
      }).play(PILE_ON);
      expect(radiantPlay.lastEvents.filter((event) => "position" in event)).toEqual([]);
    });

    it("its return is a replacement: after it resolves it goes to the bottom of your deck, never the graveyard", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: [LUNAR, STOCKPILE], mana: 5 }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      s.play(pileOn);
      s.expectInZone(pileOn, "library");
      expect(libraryIds(s)).toEqual([LUNAR, STOCKPILE, PILE_ON]);
      expect(s.pile("p1", "graveyard")).toEqual([]);
      expect(s.events.some((event) => event.type === "enteredGraveyard")).toBe(false);
    });

    it("discarded, it goes to the bottom of your deck instead", () => {
      const s = scenario({ p1: { hand: [ZAO_GAO, PILE_ON, LUNAR], library: [STOCKPILE] }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      s.play(ZAO_GAO);
      s.expectInZone(pileOn, "library").expectInZone(LUNAR, "graveyard");
      expect(libraryIds(s)).toEqual([STOCKPILE, PILE_ON]);
    });

    it("burned at the hand cap, it goes to the bottom of your deck instead", () => {
      const nine = Array.from({ length: 9 }, () => FILLER);
      const s = scenario({ p1: { hand: [STOCKPILE, ...nine], library: [LUNAR, PILE_ON, GARY] }, p2: { hand: [FILLER] } });
      s.play(STOCKPILE);
      expect(s.events.some((event) => event.type === "burned")).toBe(true);
      expect(libraryIds(s)).toEqual([GARY, PILE_ON]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([STOCKPILE]);
    });

    it("R80 a full deck turns it away, so it reaches the graveyard after all", () => {
      const full = Array.from({ length: LIBRARY_CAP }, () => FILLER);
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: full, mana: 5 }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      s.play(pileOn);
      s.expectInZone(pileOn, "graveyard");
      expect(s.pile("p1", "library")).toHaveLength(LIBRARY_CAP);
    });

    it("counts as a played Spell and the recruited cards are summoned, summoning sick", () => {
      const s = scenario({ p1: { hand: [PILE_ON, FILLER], library: [VANILLA], mana: 5 }, p2: { hand: [FILLER] } });
      s.play(PILE_ON);
      s.expectEvents("cardPlayed", "summoned", "cardResolved");
      expect(() => s.attack(VANILLA, "hero")).toThrow();
    });
  });

  describe("radiant", () => {
    it("recruits every permanent of the deck the same way", () => {
      const s = scenario({
        p1: { hand: [{ def: PILE_ON, radiant: true }, FILLER], library: MIXED_DECK, mana: 5 },
        p2: { hand: [FILLER] },
      });
      s.play(PILE_ON);
      expect(unitRow(s)).toEqual([VANILLA, GARY, BIGOT, null, null]);
      expect(backRow(s)).toEqual([MANA_WELL, SHEEPISH, BREAD, null, null]);
      expect(libraryIds(s)).toEqual([STOCKPILE, LUNAR]);
    });

    it("has no return clause: after it resolves it goes to the graveyard as any Spell does", () => {
      const s = scenario({ p1: { hand: [{ def: PILE_ON, radiant: true }, FILLER], library: [VANILLA], mana: 5 }, p2: { hand: [FILLER] } });
      const pileOn = s.card(PILE_ON);
      s.play(pileOn);
      s.expectInZone(pileOn, "graveyard");
      expect(libraryIds(s)).toEqual([]);
    });

    it("discarded, it goes to the graveyard", () => {
      const s = scenario({ p1: { hand: [ZAO_GAO, { def: PILE_ON, radiant: true }, LUNAR] }, p2: { hand: [FILLER] } });
      s.play(ZAO_GAO);
      s.expectInZone(PILE_ON, "graveyard");
    });

    it("R33 a recruited Trap is face-down and hidden from the opponent", () => {
      const s = scenario({
        p1: { hand: [{ def: PILE_ON, radiant: true }, FILLER], library: [SHEEPISH], mana: 5 },
        p2: { hand: [FILLER] },
      });
      s.play(PILE_ON);
      expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
    });

    it("§2.3 at the mana cap of 4 it cannot be played", () => {
      const s = scenario({ p1: { hand: [{ def: PILE_ON, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      expect(() => s.play(PILE_ON)).toThrow();
    });
  });
});
