// C #38 Jackiestan Auctioneer — SPEC §8.6 row 38, BUILD M9 Classic row C 38: "Face-down, only its
// activation condition is live (R395): it fires when any player plays their 3rd card in a turn
// (counted per player per turn; casts count, R70; a countered card was never played); then it animates
// (R383) in Attack Position into its lane's unit zone, else the leftmost open one, summoning sick; with
// no open zone it stays face-up in the backrow; from the next play on, never the play that set it off
// (R395), whenever either player plays a card you draw 1 and deal one hit of 2 to the enemy hero,
// animated or stuck in the backrow; animated it is a Unit for every rule and keeps that trigger; the
// opponent's view never names it while face-down (R33); radiant 8/8: the 2nd card, 4 damage; its tuned
// numbers (trigger play, never below 2; draw; damage) read through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/038-jackiestan-auctioneer";

const AUCTION = "classic-038";
const COUNTER = "classic-017"; // Counterspell: counters the opponent's Spell.
const FILLER = "core-039"; // (0) Spell Recycling Initiative: exiles itself; its effect waits for the turn's end.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2, heal 2.
const VANILLA = "core-008"; // (1) 4/4.
const TIMMY = "core-011"; // (1) 3/3.
const MENACE = "core-019"; // 9/9.
const HINDER = "core-021"; // (0) cast on draw.
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.

function setAuction(radiantFace = false, lane = 2): { def: string; radiant: boolean; faceUp: boolean; lane: number } {
  return { def: AUCTION, radiant: radiantFace, faceUp: false, lane };
}

/** p1 sets the Auctioneer; p2 is active with four (0) Cost Spells to play. */
function setup(p2: SideSetup = {}, p1: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [VANILLA], backrow: [setAuction(radiantFace)], library: [TIMMY, TIMMY, TIMMY, TIMMY], ...p1 },
    p2: { hand: [FILLER, FILLER, FILLER, FILLER, VANILLA], library: [VANILLA, VANILLA], ...p2 },
  });
}

function fillers(s: Scenario, player: "p1" | "p2" = "p2") {
  return s.hand(player).filter((card) => card.defId === FILLER);
}

function playFillers(s: Scenario, n: number, player: "p1" | "p2" = "p2"): Scenario {
  for (let i = 0; i < n; i += 1) {
    const next = fillers(s, player)[0];
    if (next === undefined) throw new Error("no filler left");
    s.play(next);
  }
  return s;
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

function drawsBy(events: readonly GameEvent[], player: "p1" | "p2"): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

describe("C #38 Jackiestan Auctioneer", () => {
  it("is an Animated Field Trap; both faces run one script", () => {
    expect(def.type).toBe("Field Trap");
    expect(def.base.keywords.map((keyword) => keyword.kind)).toEqual(["Animated"]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R395 face-down, only the activation condition is live: a player's 1st and 2nd plays do nothing", () => {
      const s = setup();

      playFillers(s, 2);

      expect(count(s.events, "trapFired")).toBe(0);
      expect(s.card(AUCTION).faceUp).toBe(false);
      expect(drawsBy(s.events, "p1")).toBe(0);
    });

    it("R395 the 3rd play of a turn sets it off: it animates in Attack Position into its own lane, summoning sick", () => {
      const s = setup();

      playFillers(s, 3);

      const auction = s.card(AUCTION);
      expect(auction.zone).toMatchObject({ z: "field", row: "units", lane: 2 });
      expect(s.stats(auction).position).toBe("ATK");
      expect(auction.summonedTurn).toBe(s.state.turn);
      s.expectEvents("trapFired", "animated");
    });

    it("R395 the play that set it off draws nothing and deals nothing", () => {
      const s = setup();

      playFillers(s, 3);

      expect(drawsBy(s.events, "p1")).toBe(0);
      s.expectHealth("p2", 30);
    });

    it("R395 from the next play on, any player's play: you draw 1 and deal 2 to the enemy hero", () => {
      const s = setup();

      playFillers(s, 4);

      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
      s.expectHealth("p2", 28);
      const hit = s.lastEvents.find((event) => event.type === "damage");
      expect(hit).toMatchObject({ sourceId: s.card(AUCTION).id, amount: 2 });
    });

    it("its own controller's plays count and answer too", () => {
      const s = scenario({
        p1: { hand: [FILLER, FILLER, FILLER, FILLER, VANILLA], backrow: [setAuction()], library: [TIMMY, TIMMY] },
        p2: { hand: [VANILLA] },
      });

      playFillers(s, 3, "p1");
      expect(s.card(AUCTION).zone).toMatchObject({ row: "units" });
      playFillers(s, 1, "p1");

      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
      s.expectHealth("p2", 28);
    });

    it("R70 a cast counts as a play: a cast-on-draw card can be the 3rd", () => {
      const s = setup({ hand: [FILLER, STOCKPILE, VANILLA], library: [{ def: HINDER, radiant: true }, VANILLA] });

      s.play(fillers(s)[0] ?? FILLER);
      // Stockpile is the 2nd play; its first draw casts Hinder, the 3rd.
      s.play(STOCKPILE);

      expect(count(s.events, "trapFired")).toBeGreaterThanOrEqual(1);
      expect(s.card(AUCTION).zone).toMatchObject({ row: "units" });
    });

    it("R448 a countered card was never played: it does not count", () => {
      const s = setup({ hand: [FILLER, FILLER, STOCKPILE, VANILLA] }, { backrow: [setAuction(), { def: COUNTER, faceUp: false, lane: 4 }] });

      playFillers(s, 2);
      s.play(STOCKPILE);

      expect(count(s.events, "countered")).toBe(1);
      expect(s.card(AUCTION).faceUp).toBe(false);
    });

    it("the count is per player per turn: plays across two turns never add up", () => {
      const s = scenario({
        p1: { hand: [FILLER, FILLER, VANILLA], backrow: [setAuction()], library: [TIMMY, TIMMY] },
        p2: { hand: [FILLER, VANILLA], library: [VANILLA] },
      });

      playFillers(s, 2, "p1");
      s.endTurn();
      playFillers(s, 1, "p2");

      expect(count(s.events, "trapFired")).toBe(0);
    });

    it("R383 no open unit zone: it stays face-up in the backrow, and still sells", () => {
      const full = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];
      const s = setup({}, { field: full });

      playFillers(s, 4);

      const auction = s.card(AUCTION);
      expect(auction.zone).toMatchObject({ z: "field", row: "backrow", lane: 2 });
      expect(auction.faceUp).toBe(true);
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
      s.expectHealth("p2", 28);
    });

    it("R383 its own lane's unit zone taken: it animates into the leftmost open one", () => {
      const s = setup({}, { field: [{ def: VANILLA, lane: 2 }, { def: VANILLA, lane: 1 }] });

      playFillers(s, 3);

      expect(s.card(AUCTION).zone).toMatchObject({ z: "field", row: "units", lane: 3 });
    });

    it("R383 stuck in the backrow, a later firing animates it once a unit zone has opened", () => {
      // p1's row is full, so the 3rd play leaves it face-up in the backrow. Hit Job then empties
      // p1's lane 2 (its own firing on that play finds the row still full); the next play's firing
      // animates it into that zone.
      const full = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];
      const s = setup({ hand: [FILLER, FILLER, FILLER, FILLER, HIT_JOB] }, { field: full });
      playFillers(s, 3);
      expect(s.card(AUCTION).zone).toMatchObject({ row: "backrow", lane: 2 });
      const doomed = s.unit("p1", 2);
      if (doomed === null) throw new Error("fixture");

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: doomed.id }] });
      s.expectInZone(doomed, "graveyard");
      expect(s.card(AUCTION).zone).toMatchObject({ row: "backrow", lane: 2 });
      playFillers(s, 1);

      expect(s.card(AUCTION).zone).toMatchObject({ z: "field", row: "units", lane: 2 });
      expect(count(s.events, "animated")).toBe(1);
    });

    it("R383 animated it is a Unit for every rule and keeps its trigger", () => {
      const s = setup({ field: [MENACE] });
      playFillers(s, 3);
      const auction = s.card(AUCTION);

      s.attack(MENACE, auction);

      s.expectInZone(auction, "graveyard");
    });

    it("R33 the opponent's view never names it while face-down", () => {
      const s = setup();
      playFillers(s, 2);
      expect(JSON.stringify(s.view("p2"))).not.toContain(AUCTION);
    });

    it("R386 a Degrade of its trigger play makes it wait for the 4th", () => {
      const s = setup();
      stepParam(s.card(AUCTION), "plays", 1);

      playFillers(s, 3);
      expect(s.card(AUCTION).faceUp).toBe(false);
      playFillers(s, 1);

      expect(s.card(AUCTION).zone).toMatchObject({ row: "units" });
    });

    it("R386 an Upgrade of its draw and damage: 2 cards and 3 damage per play", () => {
      const s = setup();
      stepParam(s.card(AUCTION), "draw", 1);
      stepParam(s.card(AUCTION), "damage", 1);

      playFillers(s, 4);

      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
      s.expectHealth("p2", 27);
    });
  });

  describe("radiant", () => {
    it("R275 its unit face is an 8/8, and it activates on the 2nd play", () => {
      const s = setup({}, {}, true);

      playFillers(s, 2);

      s.expectStats(AUCTION, { attack: 8, health: 8, maxHealth: 8 });
      expect(s.card(AUCTION).zone).toMatchObject({ row: "units" });
      s.expectHealth("p2", 30);
    });

    it("then each play draws 1 and deals 4", () => {
      const s = setup({}, {}, true);

      playFillers(s, 3);

      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
      s.expectHealth("p2", 26);
    });

    it("R386 an Upgrade never takes its trigger play below 2", () => {
      const s = setup({}, {}, true);
      stepParam(s.card(AUCTION), "plays", -1);

      playFillers(s, 1);
      expect(s.card(AUCTION).faceUp).toBe(false);
      playFillers(s, 1);

      expect(s.card(AUCTION).zone).toMatchObject({ row: "units" });
    });
  });
});
