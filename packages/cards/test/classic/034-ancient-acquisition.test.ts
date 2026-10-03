// C #34 Ancient Acquisition — SPEC §8.6 row 34, BUILD M9 Classic row C 34: "Bounce 2 random cards
// from your graveyard, with no prompt (R643; fewer if fewer; an empty graveyard: nothing);
// the resolving Spell is never one of its own returns; a full hand burns the overflow (R317); the
// returned cards follow R97 in the opponent's view once in your hand; cast by C #47 the returns are
// still its caster's; radiant: up to 4 from your graveyard or your exile; its tuned number (cards)
// reads through `param()` (R386)".
//
// The C #47 case casts this card from C #47 Recurring Felinor's Cry (B5 E12).

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/034-ancient-acquisition";

const ACQUIRE = "classic-034";
const RECURRING = "classic-047"; // C #47 Recurring Felinor, (2) Unit: "Cry: Cast Ancient Acquisition."
const FILLER = "core-005"; // (1) Spell, a spare card (§2.5).
// Graveyard and exile cards, one definition each so a def id names one card.
const MENACE = "core-019"; // (3) Unit
const SEVEN = "core-025"; // (4) Unit
const FELINORS = "core-012"; // (2) Unit
const VANILLA = "core-008"; // (1) Unit
const REPLENISH = "core-010"; // (0) Spell
const ECLIPSE = "core-035"; // (1) Spell
const MANA_WELL = "core-006"; // (3) Field Spell

function acquire(
  radiantFace: boolean,
  piles: { graveyard?: readonly string[]; exile?: readonly string[]; hand?: readonly string[] } = {},
): Scenario {
  return scenario({
    p1: {
      hand: [{ def: ACQUIRE, radiant: radiantFace }, ...(piles.hand ?? [FILLER])],
      graveyard: piles.graveyard ?? [MENACE, SEVEN, FELINORS, VANILLA, REPLENISH],
      exile: piles.exile ?? [],
    },
    p2: { hand: [FILLER] },
  });
}

describe("C #34 Ancient Acquisition", () => {
  it("declares its one number, cards (R386): 2, Radiant 4", () => {
    expect(def.params).toEqual([{ key: "cards", base: 2, radiant: 4, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("R643 returns 2 random cards with no prompt, from the graveyard", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(3);
      const grave = s.pile("p1", "graveyard").map((card) => card.defId);
      expect(grave).toHaveLength(4);
      expect(grave).toContain(ACQUIRE);
      // The two in hand came from the graveyard's five.
      const inHand = s.hand("p1").filter((card) => card.defId !== FILLER).map((card) => card.defId);
      expect(inHand).toHaveLength(2);
      for (const defId of inHand) expect([MENACE, SEVEN, FELINORS, VANILLA, REPLENISH]).toContain(defId);
    });

    it("R643 the random returns come from the match rng: the same game returns the same cards", () => {
      const first = acquire(false);
      first.play(ACQUIRE);
      const second = acquire(false);
      second.play(ACQUIRE);
      const ids = (s: Scenario): string[] =>
        s.events.flatMap((event) =>
          event.type === "addedToHand" && event.player === "p1" ? [event.instanceId] : [],
        );
      expect(ids(first)).toEqual(ids(second));
    });

    it("R643 each return is its own addedToHand event, naming the card to you", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      expect(s.events.filter((event) => event.type === "addedToHand" && event.player === "p1")).toHaveLength(2);
    });

    it("fewer if fewer: a graveyard of one card returns it", () => {
      const s = acquire(false, { graveyard: [SEVEN] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      s.expectInZone(SEVEN, "hand");
    });

    it("an empty graveyard asks nothing, and the Spell lands there", () => {
      const s = acquire(false, { graveyard: [] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([ACQUIRE]);
    });

    it("the resolving Spell is never one of its own returns", () => {
      const s = acquire(false, { graveyard: [VANILLA] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      s.expectInZone(VANILLA, "hand");
      s.expectInZone(ACQUIRE, "graveyard");
    });

    it("R317 a full hand burns the overflow back into your graveyard, both players reading which", () => {
      const fillers = Array.from({ length: 9 }, () => FILLER);
      const s = acquire(false, { hand: fillers });
      s.play(ACQUIRE);
      expect(s.hand("p1")).toHaveLength(10);
      const burned = s.events.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(1);
      const burnedId = burned[0]?.type === "burned" ? burned[0].instanceId : "";
      s.expectInZone(burnedId, "graveyard");
      expect(s.view("p2").events.filter((event) => event.type === "burned")).toEqual(burned);
    });

    it("R97 once in your hand, the returned cards are named in no event or pile of the opponent's view", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      const returned = s.hand("p1").filter((card) => card.defId !== FILLER);
      expect(returned).toHaveLength(2);
      const theirs = JSON.stringify(s.view("p2"));
      for (const card of returned) {
        expect(theirs).not.toContain(`"${card.id}"`);
        expect(theirs).not.toContain(card.defId);
      }
      const mine = JSON.stringify(s.view("p1"));
      expect(mine).toContain(`"${returned[0]?.id}"`);
    });

    it("only your own graveyard: the opponent's graveyard and your exile are untouched", () => {
      const s = scenario({
        p1: { hand: [ACQUIRE, FILLER], graveyard: [MENACE], exile: [VANILLA] },
        p2: { hand: [FILLER], graveyard: [SEVEN, FELINORS] },
      });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      // The one own-graveyard card is the only thing that could come: it did.
      s.expectInZone(MENACE, "hand");
      expect(s.pile("p2", "graveyard")).toHaveLength(2);
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([VANILLA]);
    });

    it("R177 R643 no prompt opens, so the opponent reads only the public events", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.view("p2").pending).toBeNull();
    });

    it("R386 an Upgrade of cards lets it return 3", () => {
      const s = acquire(false);
      stepParam(s.card(ACQUIRE), "cards", 1);
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(4);
      expect(s.pile("p1", "graveyard")).toHaveLength(3);
    });

    it("R386 a Degrade of cards lets it return 1", () => {
      const s = acquire(false);
      stepParam(s.card(ACQUIRE), "cards", -1);
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(2);
      expect(s.pile("p1", "graveyard")).toHaveLength(5);
    });

    it("R70 cast by C #47 Recurring Felinor, the returns are still its caster's", () => {
      const s = scenario({
        p1: { hand: [RECURRING, FILLER], graveyard: [MENACE, VANILLA] },
        p2: { hand: [FILLER] },
      });
      s.play(RECURRING);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1").map((card) => card.defId).sort()).toEqual([FILLER, MENACE, VANILLA].sort());
    });
  });

  describe("radiant", () => {
    it("R643 returns up to 4 at random from your graveyard and your exile", () => {
      const s = acquire(true, { graveyard: [MENACE, SEVEN, FELINORS], exile: [VANILLA, ECLIPSE, MANA_WELL] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(5);
      // Six pooled cards, four taken: two stay, plus the spent Spell in the graveyard.
      expect(s.pile("p1", "graveyard").length + s.pile("p1", "exile").length).toBe(3);
    });

    it("returns cards from both piles to your hand", () => {
      const s = acquire(true, { graveyard: [MENACE, SEVEN], exile: [VANILLA, MANA_WELL] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      // All four pooled cards come: both piles empty but for the spent Spell.
      expect(s.hand("p1")).toHaveLength(5);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([ACQUIRE]);
      expect(s.pile("p1", "exile")).toEqual([]);
    });

    it("an exile alone returns from exile", () => {
      const s = acquire(true, { graveyard: [], exile: [ECLIPSE] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      s.expectInZone(ECLIPSE, "hand");
      expect(s.pile("p1", "exile")).toEqual([]);
    });

    it("only your own piles: the opponent's piles are untouched", () => {
      const s = scenario({
        p1: { hand: [{ def: ACQUIRE, radiant: true }, FILLER], graveyard: [MENACE], exile: [VANILLA] },
        p2: { hand: [FILLER], graveyard: [SEVEN], exile: [FELINORS] },
      });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      // Both own cards come (2 of the 4 asked take all there is).
      s.expectInZone(MENACE, "hand");
      s.expectInZone(VANILLA, "hand");
      expect(s.pile("p2", "graveyard")).toHaveLength(1);
      expect(s.pile("p2", "exile")).toHaveLength(1);
    });

    it("both piles empty asks nothing", () => {
      const s = acquire(true, { graveyard: [], exile: [] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      s.expectInZone(ACQUIRE, "graveyard");
    });

    it("R97 a card returned from exile is hidden from the opponent once in your hand", () => {
      const s = acquire(true, { graveyard: [], exile: [MANA_WELL] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      const well = s.card(MANA_WELL);
      s.expectInZone(well, "hand");
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(`"${well.id}"`);
      expect(theirs).not.toContain(MANA_WELL);
    });

    it("R386 a Degrade of cards lets it return 3", () => {
      const s = acquire(true, { graveyard: [MENACE, SEVEN, FELINORS], exile: [VANILLA, ECLIPSE] });
      stepParam(s.card(ACQUIRE), "cards", -1);
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(4);
    });
  });
});
