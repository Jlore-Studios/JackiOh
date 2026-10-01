// C #72 Grand Counterspell — SPEC §8.6 row 72, BUILD M9 Classic row C 72: "Renamed from the designer's
// second Counterspell (R381); face-down (R33); fires on the opponent's announce of any non-Unit (Spell,
// Field Spell, Trap, Field Trap), a cast included (R70), and counters it before it reaches the backrow
// (a face-down set is announced by its zone only while the engine knows what it is), treated as never
// played as C #17's is, mana and Tributes spent; a Unit play leaves it set; the countered card goes to
// its owner's graveyard, where a countered trap is public; radiant: steals it instead: countered and
// moved to your hand as yours (its owner changes, R12), burned at a full hand (R317), and hidden in
// the opponent's view once there (R97); no tuned numbers".

import { cardsPlayedThisTurn, lastSpellPlayed } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { base, def, radiant } from "../../src/scripts/classic/072-grand-counterspell";
import { scenario, type Scenario } from "../_harness";

const GRAND = "classic-072";
const STOCKPILE = "core-005"; // (1) Spell
const ARMOR = "core-073"; // (2) Field Spell
const HONEYPOT = "core-060"; // (1) Trap
const BREAD = "core-018"; // (1) Field Trap
const VANILLA = "core-008"; // (1) Unit 4/4
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw
const FILLER = "core-010"; // (0) Spell

function countered(s: Scenario): Extract<GameEvent, { type: "countered" }>[] {
  return s.lastEvents.filter((event): event is Extract<GameEvent, { type: "countered" }> => event.type === "countered");
}

/** p1 holds a Grand Counterspell face-down; p2 is to play. */
function armed(opts: { radiant?: boolean; p2Hand: readonly string[]; p1Hand?: readonly string[]; p2Library?: readonly string[] }): Scenario {
  return scenario({
    p1: { hand: opts.p1Hand ?? [FILLER], backrow: [{ def: GRAND, radiant: opts.radiant === true }] },
    p2: { hand: [...opts.p2Hand, FILLER], library: [...(opts.p2Library ?? [])] },
    active: "p2",
  });
}

describe("C #72 Grand Counterspell", () => {
  it("R381 is Grand Counterspell, a (2) Trap, renamed from the designer's second Counterspell; no card names it", () => {
    expect(def.name).toBe("Grand Counterspell");
    expect(cardDef("classic-017").name).toBe("Counterspell");
    expect(def.type).toBe("Trap");
    expect(def.cost).toBe(2);
    expect(def.refs).toBeUndefined();
    expect(base.triggers?.[0]?.on).toEqual(["cardAnnounced"]);
    expect(radiant.triggers?.[0]?.on).toEqual(["cardAnnounced"]);
  });

  describe("base", () => {
    it("R33 it is set face-down: its controller reads it, the opponent sees a back and its cost", () => {
      const s = scenario({ p1: { hand: [GRAND, FILLER] } });
      s.play(GRAND);
      const own = s.view("p1").you.backrow[0];
      expect(own).toMatchObject({ defId: GRAND, unrevealed: true });
      const theirs = s.view("p2").opponent.backrow[0];
      expect(JSON.stringify(theirs)).not.toContain(GRAND);
      expect(theirs).toMatchObject({ cost: 2 });
    });

    it("§6.3 counters the opponent's Spell: no resolution, no cardPlayed, to its owner's graveyard, mana spent", () => {
      const s = armed({ p2Hand: [STOCKPILE], p2Library: [VANILLA, VANILLA] });
      s.play(STOCKPILE);
      expect(countered(s)).toHaveLength(1);
      expect(s.lastEvents.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.lastEvents.some((event) => event.type === "drawn")).toBe(false);
      s.expectInZone(STOCKPILE, "graveyard").expectMana("p2", 3);
      expect(cardsPlayedThisTurn(s.state, "p2")).toBe(0);
      expect(lastSpellPlayed(s.state)).toBeNull();
      // A Trap fires once and is spent (§5.1).
      s.expectInZone(GRAND, "graveyard");
    });

    it("§6.3 counters a Field Spell before it reaches the backrow: it is never placed, its Cry never fires", () => {
      const s = armed({ p2Hand: [ARMOR], p2Library: [VANILLA] });
      s.play(ARMOR);
      expect(countered(s)).toHaveLength(1);
      expect(s.backrow("p2", 1)).toBeNull();
      s.expectInZone(ARMOR, "graveyard");
      expect(s.lastEvents.some((event) => event.type === "drawn")).toBe(false);
    });

    it("§6.3 R33 counters a Trap set face-down, announced by its zone only; once countered it is public in the graveyard", () => {
      const s = armed({ p2Hand: [HONEYPOT] });
      s.play(HONEYPOT);
      expect(countered(s)).toHaveLength(1);
      expect(s.backrow("p2", 1)).toBeNull();
      s.expectInZone(HONEYPOT, "graveyard");
      const p1Graveyard = s.view("p1").opponent.graveyard.map((card) => card.defId);
      expect(p1Graveyard).toContain(HONEYPOT);
    });

    it("§6.3 counters a Field Trap too", () => {
      const s = armed({ p2Hand: [BREAD] });
      s.play(BREAD);
      expect(countered(s)).toHaveLength(1);
      s.expectInZone(BREAD, "graveyard");
    });

    it("a Unit play leaves it set", () => {
      const s = armed({ p2Hand: [VANILLA] });
      s.play(VANILLA);
      expect(countered(s)).toHaveLength(0);
      s.expectInZone(VANILLA, "field");
      expect(s.backrow("p1", 1)?.defId).toBe(GRAND);
    });

    it("its controller's own plays leave it set", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], backrow: [GRAND], library: [VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      expect(countered(s)).toHaveLength(0);
      expect(s.backrow("p1", 1)?.defId).toBe(GRAND);
    });

    it("R70 a cast is a play: the opponent's Spell cast on draw is countered", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [GRAND] },
        p2: { hand: [FILLER], library: [CN_VIRUS, VANILLA] },
      });
      s.endTurn();
      expect(s.events.some((event) => event.type === "countered" && event.defId === CN_VIRUS)).toBe(true);
      expect(s.events.some((event) => event.type === "damage" && event.targetId === "hero-p2")).toBe(false);
      s.expectInZone(CN_VIRUS, "graveyard");
    });

    it("B5 E1 the first counter cancels the play; a second Grand Counterspell finds no card and stays set", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [GRAND, GRAND] },
        p2: { hand: [STOCKPILE, FILLER] },
        active: "p2",
      });
      s.play(STOCKPILE);
      expect(countered(s)).toHaveLength(1);
      const set = [s.backrow("p1", 1), s.backrow("p1", 2)].filter((card) => card !== null);
      expect(set).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("E2 R12 steals the opponent's Spell: countered, and in your hand as yours", () => {
      const s = armed({ radiant: true, p2Hand: [STOCKPILE] });
      s.play(STOCKPILE);
      expect(countered(s)).toEqual([expect.objectContaining({ to: "hand", defId: STOCKPILE })]);
      const stolen = s.card(STOCKPILE);
      s.expectInZone(stolen, "hand");
      expect(stolen.owner).toBe("p1");
      expect(s.hand("p1").map((card) => card.id)).toContain(stolen.id);
      expect(s.lastEvents.some((event) => event.type === "cardPlayed")).toBe(false);
    });

    it("steals a Field Spell before it reaches the backrow", () => {
      const s = armed({ radiant: true, p2Hand: [ARMOR] });
      s.play(ARMOR);
      expect(s.backrow("p2", 1)).toBeNull();
      expect(s.card(ARMOR).owner).toBe("p1");
      s.expectInZone(ARMOR, "hand");
    });

    it("R97 once in your hand the stolen card is hidden in the opponent's view: a count, and its later moves unnamed", () => {
      const s = armed({ radiant: true, p2Hand: [STOCKPILE] });
      s.play(STOCKPILE);
      const id = s.card(STOCKPILE).id;
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: 2 });
      const own = s.view("p1");
      expect(Array.isArray(own.you.hand) && own.you.hand.some((card) => card.instanceId === id)).toBe(true);
      // The steal itself was public — the opponent watched the card announced (B5 E16's "readable where
      // stolen") — but the card is p1's hidden hand card now: the turn that follows names it to no one
      // but p1.
      s.endTurn();
      const later = s.view("p2").events.slice(-5);
      expect(JSON.stringify(s.view("p2").opponent.hand)).not.toContain(id);
      expect(later.every((event) => !JSON.stringify(event).includes(`"instanceId":"${id}"`) || event.type === "stolen" || event.type === "countered" || event.type === "cardAnnounced")).toBe(true);
    });

    it("R317 burned at your full hand: it goes to your graveyard, yours", () => {
      const fullHand = Array.from({ length: 10 }, () => FILLER);
      const s = armed({ radiant: true, p2Hand: [STOCKPILE], p1Hand: fullHand });
      s.play(STOCKPILE);
      const stolen = s.card(STOCKPILE);
      s.expectInZone(stolen, "graveyard");
      expect(stolen.owner).toBe("p1");
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toContain(stolen.id);
      expect(s.lastEvents.some((event) => event.type === "burned")).toBe(true);
    });

    it("a Unit play leaves it set, as on the base face", () => {
      const s = armed({ radiant: true, p2Hand: [VANILLA] });
      s.play(VANILLA);
      expect(countered(s)).toHaveLength(0);
      expect(s.backrow("p1", 1)?.defId).toBe(GRAND);
    });

    it("the stolen card is playable by its thief as their own", () => {
      const s = armed({ radiant: true, p2Hand: [STOCKPILE] });
      s.play(STOCKPILE);
      s.endTurn();
      s.play(STOCKPILE);
      expect(s.lastEvents.some((event) => event.type === "cardPlayed" && event.player === "p1" && event.defId === STOCKPILE)).toBe(true);
    });
  });
});
