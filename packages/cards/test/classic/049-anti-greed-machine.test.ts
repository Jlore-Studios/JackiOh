// C #49 Anti-Greed Machine — SPEC §8.6 row 49, BUILD M9 Classic row C 49: "9/9 Rush; Aura: every
// player's draws beyond 1 in a turn, on either player's turn and the start-of-turn draw included, do
// not happen at all: no card moves, no fatigue, nothing is cast on draw (§2.4); a draw made earlier
// that turn counts; with C #4's limit the lowest holds; lifted when it leaves; radiant 18/18 Rush: the
// opponent only, your draws free; its tuned number (radiant limit, never below 1) reads through
// `param()` (R386)".
//
// The aura is B5 E3's draw limit (B5 E3). C #4 Palantir sets the other limit in the set and is another
// workstream's card, so "the lowest holds" is shown with two Machines whose limits differ (a Radiant
// one tuned to 2 beside a base one). Stockpile (core-005, "Draw 2. Heal your hero 2.") makes the draws.

import { legalActions, stepParam } from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/049-anti-greed-machine";
import { scenario, type Scenario } from "../_harness";

const MACHINE = "classic-049";
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const VANILLA = "core-008"; // 4/4, no text
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const FIENDER = "core-092"; // Stack Unit
const CN_VIRUS = "core-090-1"; // Cast on draw: take 1 damage
const FILLER = "core-010"; // (0) Spell, Combo 3 — a card to keep in hand

function drawn(s: Scenario, player: PlayerId): GameEvent[] {
  return s.lastEvents.filter((event) => event.type === "drawn" && event.player === player);
}

function limited(s: Scenario, player: PlayerId): GameEvent[] {
  return s.lastEvents.filter((event) => event.type === "drawLimited" && event.player === player);
}

describe("C #49 Anti-Greed Machine", () => {
  it("is a (3) 9/9 Rush Unit (18/18 Radiant, Rush), its Radiant limit a declared number", () => {
    expect(def.cost).toBe(3);
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([9, 9, 18, 18]);
    expect(def.base.keywords).toEqual([{ kind: "Rush" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Rush" }]);
    expect(def.params).toEqual([{ key: "limit", base: 1, radiant: 1, better: "down", step: 1, min: 1 }]);
    expect(base.drawLimit).toBeTypeOf("function");
    expect(radiant.drawLimit).toBeTypeOf("function");
  });

  describe("base", () => {
    it("is a 9/9 with Rush on the field: it may attack a Unit the turn it arrives", () => {
      const s = scenario({ p1: { hand: [MACHINE, FILLER] }, p2: { field: [VANILLA], hand: [FILLER] } });
      s.play(MACHINE);
      s.expectStats(MACHINE, { attack: 9, health: 9 });
      s.attack(MACHINE, VANILLA).expectInZone(VANILLA, "graveyard");
    });

    it("§2.4 your second draw in a turn does not happen at all: no card moves, `drawLimited` says so", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MACHINE], library: [VANILLA, VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(1);
      expect(limited(s, "p1")).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(2);
      expect(s.hand("p1")).toHaveLength(2);
    });

    it("§2.4 a draw made earlier that turn counts: after one draw, the next Stockpile draws nothing", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA, VANILLA] },
        p2: { hand: [MACHINE, FILLER] },
      });
      // p1 draws once on its own turn before any Machine is there.
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(2);
      s.endTurn();
      s.play(MACHINE);
      s.endTurn();
      // p1's start-of-turn draw is its first this turn; the Stockpile's two are past the limit.
      expect(drawn(s, "p1")).toHaveLength(1);
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(0);
      expect(limited(s, "p1")).toHaveLength(2);
    });

    it("binds every player, on their own turn: the opponent's start-of-turn draw is their one", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [MACHINE] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA] },
      });
      s.endTurn();
      expect(drawn(s, "p2")).toHaveLength(1);
      s.play(STOCKPILE);
      expect(drawn(s, "p2")).toHaveLength(0);
      expect(limited(s, "p2")).toHaveLength(2);
      expect(s.pile("p2", "library")).toHaveLength(2);
    });

    it("§2.4 a stopped draw takes no fatigue from an empty deck", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MACHINE], library: [] } });
      s.play(STOCKPILE);
      expect(s.state.players.p1.fatigueCount).toBe(1);
      expect(s.lastEvents.filter((event) => event.type === "fatigue")).toHaveLength(1);
      expect(limited(s, "p1")).toHaveLength(1);
    });

    it("§2.4 R58 nothing is cast on a stopped draw: a Cast-on-draw card stays in the deck", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MACHINE], library: [VANILLA, CN_VIRUS] } });
      s.play(STOCKPILE);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([CN_VIRUS]);
      expect(s.lastEvents.filter((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS)).toHaveLength(0);
    });

    it("B5 E3 lifted when it leaves the field: destroyed, the next draw happens", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, STOCKPILE, HIT_JOB, FILLER], field: [MACHINE], library: [VANILLA, VANILLA, VANILLA, VANILLA], mana: 9 },
      });
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(1);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(MACHINE).id }] });
      s.expectInZone(MACHINE, "graveyard");
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(2);
    });

    it("§3.2 R13 dormant under a Stack pile it is not on the field, and limits nothing", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], field: [MACHINE, { def: FIENDER, stack: true }], library: [VANILLA, VANILLA] },
      });
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(2);
      expect(limited(s, "p1")).toHaveLength(0);
    });

    it("B5 E3 with several limits the lowest holds: a Radiant Machine tuned to 2 beside a base one still stops the 2nd draw", () => {
      const both = scenario({
        p1: { hand: [FILLER], field: [{ def: MACHINE, radiant: true }, MACHINE] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA] },
        active: "p2",
      });
      const tuned = both.unit("p1", 1);
      if (tuned === null) throw new Error("no Radiant Machine");
      stepParam(tuned, "limit", 1);
      both.play(STOCKPILE);
      expect(drawn(both, "p2")).toHaveLength(1);
      expect(limited(both, "p2")).toHaveLength(1);
      // Without the base one, the tuned Radiant limit of 2 lets both draws through.
      const alone = scenario({
        p1: { hand: [FILLER], field: [{ def: MACHINE, radiant: true }] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA] },
        active: "p2",
      });
      stepParam(alone.card(MACHINE), "limit", 1);
      alone.play(STOCKPILE);
      expect(drawn(alone, "p2")).toHaveLength(2);
    });

    it("R97 the stopped draw names no card in either view", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MACHINE], library: [VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      for (const viewer of ["p1", "p2"] as const) {
        const events = s.view(viewer).events.filter((event) => event.type === "drawLimited");
        expect(events).toEqual([{ type: "drawLimited", player: "p1" }]);
      }
    });
  });

  describe("radiant", () => {
    it("is an 18/18 with Rush", () => {
      const s = scenario({ p1: { hand: [{ def: MACHINE, radiant: true }, FILLER] } });
      s.play(MACHINE).expectStats(MACHINE, { attack: 18, health: 18 });
      expect(s.stats(MACHINE).keywords.map((keyword) => keyword.kind)).toEqual(["Rush"]);
    });

    it("your draws are free: its controller's Stockpile draws 2", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [{ def: MACHINE, radiant: true }], library: [VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      expect(drawn(s, "p1")).toHaveLength(2);
      expect(limited(s, "p1")).toHaveLength(0);
    });

    it("binds the opponent: their start-of-turn draw is their only one", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: MACHINE, radiant: true }] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA] },
      });
      s.endTurn();
      expect(drawn(s, "p2")).toHaveLength(1);
      s.play(STOCKPILE);
      expect(drawn(s, "p2")).toHaveLength(0);
      expect(limited(s, "p2")).toHaveLength(2);
    });

    it("R386 its limit is the declared number: a Degrade's step (less is better) lets the opponent draw 2", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: MACHINE, radiant: true }] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA, VANILLA] },
      });
      stepParam(s.card(MACHINE), "limit", 1);
      s.endTurn();
      expect(drawn(s, "p2")).toHaveLength(1);
      s.play(STOCKPILE);
      expect(drawn(s, "p2")).toHaveLength(1);
      expect(limited(s, "p2")).toHaveLength(1);
    });

    it("R386 its limit never goes below 1: a step down from 1 holds at 1", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: MACHINE, radiant: true }] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA] },
      });
      stepParam(s.card(MACHINE), "limit", -1);
      expect(s.view("p1").you.units[0]?.params).toEqual({ limit: 1 });
      s.endTurn();
      expect(drawn(s, "p2")).toHaveLength(1);
    });

    it("the opponent's legal plays are not changed by it: a limited draw is not a refusal", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: MACHINE, radiant: true }] },
        p2: { hand: [STOCKPILE, FILLER], library: [VANILLA, VANILLA, VANILLA] },
        active: "p2",
      });
      const stockpile = s.card(STOCKPILE).id;
      expect(legalActions(s.state, "p2").some((action) => action.type === "play" && action.instanceId === stockpile)).toBe(true);
    });
  });
});
