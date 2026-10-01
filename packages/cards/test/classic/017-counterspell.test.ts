// C #17 Counterspell — SPEC §8.6 row 17, BUILD M9 Classic row C 17: "Face-down (R33); fires on the
// opponent's announce of a Spell (the Spell type; a Field Spell, Trap or Unit leaves it set), a cast by
// an effect included (R70), before the card moves (§10.5); the Spell never resolves and goes to its
// owner's graveyard, treated as never played: no spell script, no `cardPlayed` or `cardResolved`, not
// counted by the turn's or the game's plays, Combo, Quickstriker or Ceaseless Void, and no Echo
// repeats; its mana and Tributes stay spent; with two counters set the first to resolve cancels it and
// the other stays set; your own Spells never fire it; `cardAnnounced` and `countered` name only what
// `cardPlayed` would; radiant: also a fresh copy of the Spell (its Radiant flag kept) in your hand,
// yours, that costs (0) (`costOverride`), burned at a full hand (R317) and never named in the opponent's
// view once there (R97); its tuned number (radiant cost) reads through `param()` (R386)".
//
// Every case sets the trap face-down in p1's backrow and makes p2 the active player: a Trap answers the
// OPPONENT's play. The Spell under test is Stockpile (core-005, "Draw 2. Heal your hero 2."), whose
// resolution would be visible in p2's hand and health.

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/017-counterspell";

const COUNTER = "classic-017";
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const REPLENISH = "core-010"; // (0) Spell, Combo 3.
const HINDER = "core-021"; // (0) Spell, cast on draw.
const TWINSPELL = "core-079"; // (2) Field Spell: your next Spell gains Echo +1.
const BEAR = "core-060"; // (1) Trap.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019";

function armed(radiantFace = false, lane = 2): { def: string; radiant: boolean; faceUp: boolean; lane: number } {
  return { def: COUNTER, radiant: radiantFace, faceUp: false, lane };
}

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [VANILLA], backrow: [armed(radiantFace)], library: [VANILLA, VANILLA], ...p1 },
    p2: { hand: [STOCKPILE, REPLENISH], library: [VANILLA, VANILLA, VANILLA, VANILLA], health: 20, ...p2 },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("C #17 Counterspell", () => {
  it("is a Trap whose condition lives in `when` (R99)", () => {
    expect(def.type).toBe("Trap");
    expect(base.triggers?.[0]?.when).toBeTypeOf("function");
    expect(radiant.triggers?.[0]?.when).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R33 it is face-down, and the opponent's view never names it", () => {
      const s = setup();
      const trap = s.card(COUNTER);
      expect(trap.faceUp).toBe(false);
      expect(JSON.stringify(s.view("p2"))).not.toContain(COUNTER);
    });

    it("R448 counters the opponent's Spell before it moves: it never resolves and goes to its owner's graveyard", () => {
      const s = setup();
      const spell = s.card(STOCKPILE);
      const handBefore = s.hand("p2").length;

      s.play(spell);

      s.expectInZone(spell, "graveyard");
      expect(s.pile("p2", "graveyard").map((card) => card.id)).toContain(spell.id);
      // Stockpile's script never ran: no draws, no heal.
      expect(s.hand("p2")).toHaveLength(handBefore - 1);
      s.expectHealth("p2", 20);
      s.expectEvents("cardAnnounced", "trapFired", "countered");
      expect(count(s.lastEvents, "cardPlayed")).toBe(0);
      expect(count(s.lastEvents, "cardResolved")).toBe(0);
      expect(count(s.lastEvents, "drawn")).toBe(0);
    });

    it("§5.1 the Trap is consumed when it fires", () => {
      const s = setup();
      const trap = s.card(COUNTER);

      s.play(STOCKPILE);

      s.expectInZone(trap, "graveyard");
      expect(s.backrow("p1", 2)).toBeNull();
    });

    it("R448 a countered Spell counts as never played, and its mana stays spent", () => {
      const s = setup();
      const played = s.state.players.p2.turnLog.cardsPlayed;
      const game = s.state.counters.played;

      s.play(STOCKPILE);

      expect(s.state.players.p2.turnLog.cardsPlayed).toBe(played);
      expect(s.state.counters.played).toBe(game);
      s.expectMana("p2", 3);
    });

    it("R448 Combo does not count it: a later Combo reads only the plays that were made", () => {
      const s = setup({}, { hand: [STOCKPILE, REPLENISH, VANILLA] });

      s.play(VANILLA, { zone: 1 });
      s.play(STOCKPILE);
      s.play(REPLENISH);

      // Rapid Replenish's Combo 3 needs three earlier plays; the Vanilla alone was played.
      expect(s.state.players.p2.turnLog.cardsPlayed).toBe(2);
      expect(count(s.lastEvents, "drawn")).toBe(0);
    });

    it("R448 a countered Spell makes no Echo repeats", () => {
      const s = setup({}, { backrow: [{ def: TWINSPELL, faceUp: true }] });

      s.play(STOCKPILE);

      expect(count(s.lastEvents, "drawn")).toBe(0);
      expect(count(s.lastEvents, "cardResolved")).toBe(0);
      s.expectHealth("p2", 20);
    });

    it("R70 a Spell cast by an effect is announced and countered too (a cast-on-draw card)", () => {
      // p2's start-of-turn draw finds a Radiant Hinder, which casts itself on draw (R58): the cast is
      // announced like a play, and the trap counters it before it resolves.
      const s = scenario({
        p1: { hand: [VANILLA], backrow: [armed()], library: [VANILLA, VANILLA] },
        p2: { hand: [VANILLA], library: [{ def: HINDER, radiant: true }, VANILLA, VANILLA] },
      });
      const hinder = s.card(HINDER);

      s.endTurn();

      expect(s.state.active).toBe("p2");
      s.expectInZone(hinder, "graveyard");
      expect(count(s.events, "countered")).toBe(1);
      expect(count(s.events, "cardResolved")).toBe(0);
      // Hinder never resolved, so p1's next refresh is not lowered.
      expect(s.state.players.p1.mana.nextTurnMod).toBe(0);
    });

    it("a Field Spell, a Trap and a Unit leave it set", () => {
      const s = setup({}, { hand: [TWINSPELL, BEAR, VANILLA] });
      s.play(TWINSPELL, { zone: 1 });
      s.play(BEAR, { zone: 3 });
      s.play(VANILLA, { zone: 1 });

      expect(count(s.events, "countered")).toBe(0);
      expect(s.backrow("p1", 2)?.faceUp).toBe(false);
      s.expectInZone(TWINSPELL, "field");
      s.expectInZone(VANILLA, "field");
    });

    it("your own Spells never fire it", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, VANILLA], backrow: [armed()], library: [VANILLA, VANILLA, VANILLA] },
        p2: { hand: [VANILLA] },
      });

      s.play(STOCKPILE);

      expect(count(s.events, "countered")).toBe(0);
      expect(s.backrow("p1", 2)?.faceUp).toBe(false);
      expect(count(s.events, "drawn")).toBe(2);
    });

    it("R448 with two set, the first to resolve cancels the play and the other stays set", () => {
      const s = setup({ backrow: [armed(false, 1), armed(false, 2)] });

      s.play(STOCKPILE);

      expect(count(s.events, "countered")).toBe(1);
      expect(count(s.events, "trapFired")).toBe(1);
      const left = [s.backrow("p1", 1), s.backrow("p1", 2)].filter((card) => card !== null);
      expect(left).toHaveLength(1);
      expect(left[0]?.faceUp).toBe(false);
    });

    it("R97 `cardAnnounced` and `countered` name the Spell, as `cardPlayed` would for a Spell (public)", () => {
      const s = setup();
      const spell = s.card(STOCKPILE);

      s.play(spell);

      const seen = s.view("p1").events ?? [];
      const announced = seen.find((event) => event.type === "cardAnnounced");
      const countered = seen.find((event) => event.type === "countered");
      expect(announced).toMatchObject({ instanceId: spell.id, defId: STOCKPILE, cardType: "Spell" });
      expect(countered).toMatchObject({ instanceId: spell.id, defId: STOCKPILE, to: "graveyard" });
    });

    it("the base face adds no copy to your hand", () => {
      const s = setup();
      const before = s.hand("p1").length;

      s.play(STOCKPILE);

      expect(s.hand("p1")).toHaveLength(before);
    });
  });

  describe("radiant", () => {
    it("counters the Spell and adds a fresh copy of it to your hand, yours, costing (0)", () => {
      const s = setup({}, {}, true);
      const spell = s.card(STOCKPILE);

      s.play(spell);

      s.expectInZone(spell, "graveyard");
      const copy = s.hand("p1").find((card) => card.defId === STOCKPILE);
      expect(copy).toBeDefined();
      expect(copy?.id).not.toBe(spell.id);
      expect(copy?.owner).toBe("p1");
      expect(copy?.costOverride).toBe(0);
      s.expectHealth("p2", 20);
    });

    it("R57 the copy keeps the countered Spell's Radiant flag", () => {
      const s = setup({}, { hand: [{ def: STOCKPILE, radiant: true }, REPLENISH] }, true);

      s.play(STOCKPILE);

      const copy = s.hand("p1").find((card) => card.defId === STOCKPILE);
      expect(copy?.radiant).toBe(true);
    });

    it("R317 a full hand burns the copy into your graveyard", () => {
      const ten = Array.from({ length: 10 }, () => VANILLA);
      const s = setup({ hand: ten }, {}, true);

      s.play(STOCKPILE);

      expect(s.hand("p1")).toHaveLength(10);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(STOCKPILE);
      expect(count(s.events, "burned")).toBe(1);
    });

    it("R97 once in your hand, the opponent's view never names the copy", () => {
      const s = setup({}, {}, true);

      s.play(STOCKPILE);

      const copy = s.hand("p1").find((card) => card.defId === STOCKPILE);
      if (copy === undefined) throw new Error("no copy");
      // The event that put it in p1's hand named it; p2's whole view (its events included) does not.
      expect(s.events.some((event) => JSON.stringify(event).includes(`"${copy.id}"`))).toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(`"${copy.id}"`);
      expect(JSON.stringify(s.view("p1").you.hand)).toContain(`"${copy.id}"`);
    });

    it("R386 a Degrade of its set cost makes the copy cost (1)", () => {
      const s = setup({}, {}, true);
      stepParam(s.card(COUNTER), "setCost", 1);

      s.play(STOCKPILE);

      const copy = s.hand("p1").find((card) => card.defId === STOCKPILE);
      expect(copy?.costOverride).toBe(1);
    });

    it("a Unit leaves the Radiant face set too", () => {
      const s = setup({}, { hand: [VANILLA, MENACE] }, true);

      s.play(VANILLA, { zone: 1 });

      expect(count(s.events, "countered")).toBe(0);
      expect(s.backrow("p1", 2)?.faceUp).toBe(false);
    });
  });
});
