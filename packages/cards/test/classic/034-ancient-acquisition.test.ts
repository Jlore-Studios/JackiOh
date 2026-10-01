// C #34 Ancient Acquisition — SPEC §8.6 row 34, BUILD M9 Classic row C 34: "Your choice of up to 2
// cards from your graveyard to your hand, a pick from the pile with no three-option limit (fewer if
// fewer; an empty graveyard: nothing); the Spell is resolving and can't pick itself; a full hand burns
// (R317); the returned cards follow R97 in the opponent's view once in your hand; cast by C #47 it is
// your pick too; radiant: up to 4 from your graveyard or your exile; its tuned number (cards) reads
// through `param()` (R386)".
//
// The C #47 case needs C #47 Recurring Felinor's script (cards-classic-b) and the cast verb (B5 E12,
// play pipeline B), neither in this worktree: it waits for integration.

import { describe, expect, it } from "vitest";
import { reduce, stepParam, type GameState, type PendingChoice } from "@jackioh/engine";
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

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the scenario has no ${what}`);
  return value;
}

function open(s: Scenario): PendingChoice {
  return must(s.state.pending, "open prompt");
}

function offeredDefs(s: Scenario): string[] {
  return open(s).options.map((option) =>
    option.selection.pick === "instance" ? s.card(option.selection.instanceId).defId : "?",
  );
}

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
    it("E18 opens your pick over every card in your graveyard, with no three-option limit, up to 2", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      const pending = open(s);
      expect(pending.playerId).toBe("p1");
      expect(pending.kind).toBe("pick");
      expect(pending.min).toBe(0);
      expect(pending.max).toBe(2);
      expect(offeredDefs(s)).toEqual([MENACE, SEVEN, FELINORS, VANILLA, REPLENISH]);
    });

    it("returns the 2 cards you choose to your hand; the rest stay", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      const menace = s.card(MENACE);
      const replenish = s.card(REPLENISH);
      s.answer([menace.id, replenish.id]);
      s.expectInZone(menace, "hand");
      s.expectInZone(replenish, "hand");
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([SEVEN, FELINORS, VANILLA, ACQUIRE]);
      expect(s.state.pending).toBeNull();
    });

    it("up to: one card, or none, is a legal answer", () => {
      const one = acquire(false);
      one.play(ACQUIRE);
      one.answer([one.card(VANILLA).id]);
      one.expectInZone(VANILLA, "hand");

      const none = acquire(false);
      none.play(ACQUIRE);
      none.answer([]);
      expect(none.pile("p1", "graveyard")).toHaveLength(6);
      expect(none.state.pending).toBeNull();
    });

    it("more than 2 is refused", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      expect(() => s.answer([s.card(MENACE).id, s.card(SEVEN).id, s.card(VANILLA).id])).toThrow();
    });

    it("fewer if fewer: a graveyard of one card offers it alone, up to 1", () => {
      const s = acquire(false, { graveyard: [SEVEN] });
      s.play(ACQUIRE);
      expect(open(s).max).toBe(1);
      s.answer([s.card(SEVEN).id]);
      s.expectInZone(SEVEN, "hand");
    });

    it("an empty graveyard asks nothing, and the Spell lands there", () => {
      const s = acquire(false, { graveyard: [] });
      s.play(ACQUIRE);
      expect(s.state.pending).toBeNull();
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([ACQUIRE]);
    });

    it("the Spell is resolving, in no pile, so it can't pick itself", () => {
      const s = acquire(false, { graveyard: [VANILLA] });
      const self = s.card(ACQUIRE);
      s.play(ACQUIRE);
      const ids = open(s).options.map((option) => (option.selection.pick === "instance" ? option.selection.instanceId : ""));
      expect(ids).not.toContain(self.id);
      s.answer([]);
      s.expectInZone(self, "graveyard");
    });

    it("R317 a full hand burns the second card back into your graveyard, both players reading which", () => {
      const fillers = Array.from({ length: 9 }, () => FILLER);
      const s = acquire(false, { hand: fillers });
      s.play(ACQUIRE);
      const menace = s.card(MENACE);
      const seven = s.card(SEVEN);
      s.answer([menace.id, seven.id]);
      expect(s.hand("p1")).toHaveLength(10);
      s.expectInZone(menace, "hand");
      s.expectInZone(seven, "graveyard");
      const burned = s.events.filter((event) => event.type === "burned");
      expect(burned).toEqual([{ type: "burned", instanceId: seven.id, defId: SEVEN, owner: "p1" }]);
      expect(s.view("p2").events.filter((event) => event.type === "burned")).toEqual(burned);
    });

    it("R97 once in your hand, the returned cards are named in no event or pile of the opponent's view", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      const menace = s.card(MENACE);
      const felinors = s.card(FELINORS);
      s.answer([menace.id, felinors.id]);
      const theirs = JSON.stringify(s.view("p2"));
      for (const card of [menace, felinors]) {
        expect(theirs).not.toContain(`"${card.id}"`);
        expect(theirs).not.toContain(card.defId);
      }
      const mine = JSON.stringify(s.view("p1"));
      expect(mine).toContain(`"${menace.id}"`);
    });

    it("only your own graveyard: the opponent's graveyard and your exile are not offered", () => {
      const s = scenario({
        p1: { hand: [ACQUIRE, FILLER], graveyard: [MENACE], exile: [VANILLA] },
        p2: { hand: [FILLER], graveyard: [SEVEN, FELINORS] },
      });
      s.play(ACQUIRE);
      expect(offeredDefs(s)).toEqual([MENACE]);
    });

    it("R177 the opponent reads only that a prompt is open for you", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("§9.3 the open pick survives a JSON round trip and resumes through reduce", () => {
      const s = acquire(false);
      s.play(ACQUIRE);
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const pending = must(revived.pending, "revived prompt");
      const seven = s.card(SEVEN);
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: pending.id,
        selection: [{ pick: "instance", instanceId: seven.id }],
        nonce: "acquisition-round-trip",
      });
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(result.events.some((event) => event.type === "addedToHand" && event.instanceId === seven.id)).toBe(true);
    });

    it("R386 an Upgrade of cards lets it return 3", () => {
      const s = acquire(false);
      stepParam(s.card(ACQUIRE), "cards", 1);
      s.play(ACQUIRE);
      expect(open(s).max).toBe(3);
      s.answer([s.card(MENACE).id, s.card(SEVEN).id, s.card(VANILLA).id]);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER, MENACE, SEVEN, VANILLA]);
    });

    it("R386 a Degrade of cards lets it return 1", () => {
      const s = acquire(false);
      stepParam(s.card(ACQUIRE), "cards", -1);
      s.play(ACQUIRE);
      expect(open(s).max).toBe(1);
    });

    it("R70 cast by C #47 Recurring Felinor, the pick is its caster's", () => {
      const s = scenario({
        p1: { hand: [RECURRING, FILLER], graveyard: [MENACE, VANILLA] },
        p2: { hand: [FILLER] },
      });
      s.play(RECURRING);
      const pending = open(s);
      expect(pending.playerId).toBe("p1");
      expect(pending.kind).toBe("pick");
      expect(pending.max).toBe(2);
      s.answer([s.card(MENACE).id]);
      s.expectInZone(MENACE, "hand");
    });
  });

  describe("radiant", () => {
    it("E18 offers every card of your graveyard and your exile, up to 4", () => {
      const s = acquire(true, { graveyard: [MENACE, SEVEN, FELINORS], exile: [VANILLA, ECLIPSE, MANA_WELL] });
      s.play(ACQUIRE);
      const pending = open(s);
      expect(pending.kind).toBe("pick");
      expect(pending.max).toBe(4);
      expect(offeredDefs(s)).toEqual([MENACE, SEVEN, FELINORS, VANILLA, ECLIPSE, MANA_WELL]);
    });

    it("returns cards from both piles to your hand", () => {
      const s = acquire(true, { graveyard: [MENACE, SEVEN], exile: [VANILLA, MANA_WELL] });
      s.play(ACQUIRE);
      const picks = [s.card(MENACE), s.card(VANILLA), s.card(MANA_WELL), s.card(SEVEN)];
      s.answer(picks.map((card) => card.id));
      for (const card of picks) s.expectInZone(card, "hand");
      expect(s.pile("p1", "exile")).toEqual([]);
    });

    it("an exile alone is enough to pick from", () => {
      const s = acquire(true, { graveyard: [], exile: [ECLIPSE] });
      s.play(ACQUIRE);
      expect(offeredDefs(s)).toEqual([ECLIPSE]);
      s.answer([s.card(ECLIPSE).id]);
      s.expectInZone(ECLIPSE, "hand");
    });

    it("only your own piles: the opponent's graveyard and exile are not offered", () => {
      const s = scenario({
        p1: { hand: [{ def: ACQUIRE, radiant: true }, FILLER], graveyard: [MENACE], exile: [VANILLA] },
        p2: { hand: [FILLER], graveyard: [SEVEN], exile: [FELINORS] },
      });
      s.play(ACQUIRE);
      expect(offeredDefs(s)).toEqual([MENACE, VANILLA]);
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
      const well = s.card(MANA_WELL);
      s.answer([well.id]);
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(`"${well.id}"`);
      expect(theirs).not.toContain(MANA_WELL);
    });

    it("R386 a Degrade of cards lets it return 3", () => {
      const s = acquire(true, { graveyard: [MENACE, SEVEN, FELINORS], exile: [VANILLA, ECLIPSE] });
      stepParam(s.card(ACQUIRE), "cards", -1);
      s.play(ACQUIRE);
      expect(open(s).max).toBe(3);
    });
  });
});
