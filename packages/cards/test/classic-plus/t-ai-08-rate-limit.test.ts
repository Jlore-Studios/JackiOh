// T-AI-8 Rate Limit — SPEC §8.7 row T-AI-8, BUILD M9 Classic+ row T-AI-8: "Face-down Trap: on the
// opponent's turn it fires as their 3rd play of the turn is played (`cardPlayed`; casts count, R70; a
// countered play is never played), going to your graveyard, and once that play has resolved their turn
// ends as if they had pressed End turn, every end-of-turn step running (`endTurnAfter`, §6.3); the
// opponent learns nothing of it until it fires (R33, R97); radiant after their 2nd play".
//
// Every case sets the trap face-down in p1's backrow and makes p2 the active player.

import { hashState, reduce, type GameState } from "@jackioh/engine";
import type { Action, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-08-rate-limit";

const RATE_LIMIT = "classicplus-t-ai-08";
const REFUSAL = "classicplus-t-ai-09";
const VANILLA = "core-008"; // (1) Unit 4/4
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const REPLENISH = "core-010"; // (0) Spell
const SHREDDER = "core-013"; // (3) Unit: End of turn: deal 2 damage to each enemy Unit and the enemy hero.
const HINDER = "core-021"; // (0) Spell, cast on draw
const SCARAB = "core-007"; // (1) Unit: Cry: Discover a (2) Cost card
const HIT_JOB = "core-016"; // Spell: destroy a target Unit

function trap(radiantFace = false, lane = 2): { def: string; radiant: boolean; faceUp: boolean; lane: number } {
  return { def: RATE_LIMIT, radiant: radiantFace, faceUp: false, lane };
}

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [VANILLA], backrow: [trap(radiantFace)], library: [VANILLA, VANILLA, VANILLA], ...p1 },
    p2: {
      hand: [VANILLA, REPLENISH, STOCKPILE, VANILLA],
      library: [VANILLA, VANILLA, VANILLA, VANILLA],
      mana: 8,
      ...p2,
    },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("T-AI-8 Rate Limit", () => {
  it("is a (1) AI Trap token whose condition lives in `when` (R99)", () => {
    expect(def.type).toBe("Trap");
    expect(def.cost).toBe(1);
    expect(def.tags).toEqual(["AI", "Token"]);
    expect(base.triggers?.[0]?.when).toBeTypeOf("function");
    expect(radiant.triggers?.[0]?.when).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R99 their 1st and 2nd plays leave it armed and face-down", () => {
      const s = setup();
      s.play(VANILLA, { zone: 1 }).play(REPLENISH);
      expect(s.state.active).toBe("p2");
      expect(s.card(RATE_LIMIT).faceUp).toBe(false);
      expect(count(s.events, "trapFired")).toBe(0);
    });

    it("R456 fires as their 3rd play is played; the play resolves first, then their turn ends", () => {
      const s = setup();
      s.play(VANILLA, { zone: 1 }).play(REPLENISH).play(STOCKPILE);

      s.expectInZone(RATE_LIMIT, "graveyard");
      // Stockpile resolved in full (draw 2, heal 2) before the turn ended.
      s.expectHealth("p2", 32);
      s.expectEvents("cardPlayed", "trapFired", "cardResolved", "turnCutShort", "turnEnded", "turnStarted");
      expect(s.state.active).toBe("p1");
    });

    it("R62 R456 every end-of-turn step of theirs runs, as if they had pressed End turn", () => {
      const s = setup({}, { field: [SHREDDER] });
      s.play(VANILLA, { zone: 2 }).play(REPLENISH).play(STOCKPILE);
      expect(s.state.active).toBe("p1");
      // Shredder's end-of-turn hit on p1's hero.
      s.expectHealth("p1", 28);
    });

    it("your own plays never set it off", () => {
      const s = scenario({
        p1: { hand: [VANILLA, REPLENISH, STOCKPILE, VANILLA], backrow: [trap()], library: [VANILLA, VANILLA], mana: 8 },
        p2: { hand: [VANILLA], library: [VANILLA, VANILLA] },
      });
      s.play(VANILLA, { zone: 1 }).play(REPLENISH).play(STOCKPILE);
      expect(s.state.active).toBe("p1");
      expect(s.card(RATE_LIMIT).faceUp).toBe(false);
    });

    it("R70 a cast is a play: a cast-on-draw card their 2nd play draws is their 3rd play", () => {
      const s = setup({}, { library: [{ def: HINDER, radiant: true }, VANILLA, VANILLA, VANILLA] });
      s.play(VANILLA, { zone: 1 }).play(STOCKPILE);
      s.expectInZone(RATE_LIMIT, "graveyard");
      expect(s.state.active).toBe("p1");
      // Stockpile's whole list still resolved: both draws (the cast one and its replacement) and the heal.
      s.expectHealth("p2", 32);
    });

    it("R158 R456 a 3rd play that asks pauses the end: answered after a JSON round trip, the play finishes and then the turn ends", () => {
      // The 3rd play is Scarab, whose Cry Discovers: the question pauses the turn the trap already
      // ended. (Base Hinder used to be the asker here; since R682 its discard is random, so a draw
      // that casts it asks nothing.)
      const s = setup({}, { hand: [VANILLA, REPLENISH, SCARAB, VANILLA] });
      s.play(VANILLA, { zone: 1 }).play(REPLENISH).play(SCARAB, { zone: 2 });
      expect(s.state.pending?.kind).toBe("discover");
      expect(s.state.active).toBe("p2");
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(hashState(thawed)).toBe(hashState(s.state));
      // The offered def ids read out of the view (§10.8), like #7's own test does: the raw state
      // options carry only the key and the selection to send back.
      const seen = s.view("p2").pending;
      if (seen === null || !seen.forYou) throw new Error("no Discover open for p2");
      const offeredId = seen.options[0]?.defId;
      if (offeredId === undefined) throw new Error("no Discover option");
      const selection = s.state.pending?.options[0]?.selection;
      if (selection === undefined) throw new Error("no selection to send back");
      const answer = {
        type: "answer",
        choiceId: s.state.pending?.id ?? "",
        selection: [selection],
        playerId: "p2",
        nonce: "rate-limit-pause",
      } as Action;
      const live = reduce(s.state, answer);
      const frozen = reduce(thawed, answer);
      expect(live.error).toBeUndefined();
      expect(hashState(frozen.state)).toBe(hashState(live.state));
      s.answer([selection]);
      s.expectInZone(RATE_LIMIT, "graveyard");
      // Scarab resolved after the answer: its Discover added the offered (2) Cost card to their hand.
      expect(s.hand("p2").some((card) => card.defId === offeredId)).toBe(true);
      expect(s.state.active).toBe("p1");
      s.expectEvents("cardResolved", "turnCutShort", "turnEnded");
    });

    it("R448 a countered play is never played and doesn't count", () => {
      const s = setup(
        { field: [VANILLA], backrow: [trap(), { def: REFUSAL, faceUp: false, lane: 3 }] },
        { hand: [VANILLA, HIT_JOB, REPLENISH, STOCKPILE] },
      );
      const target = s.unit("p1", 1);
      s.play(VANILLA, { zone: 1 }).play(HIT_JOB, { targets: [{ pick: "instance", instanceId: target?.id ?? "" }] });
      expect(count(s.events, "countered")).toBe(1);
      s.play(REPLENISH);
      // Two plays made: the Hit Job never was one.
      expect(s.state.active).toBe("p2");
      expect(s.card(RATE_LIMIT).faceUp).toBe(false);
      s.play(STOCKPILE);
      expect(s.state.active).toBe("p1");
    });

    it("R33 R97 the opponent learns nothing of it until it fires", () => {
      const s = setup();
      s.play(VANILLA, { zone: 1 }).play(REPLENISH);
      expect(JSON.stringify(s.view("p2"))).not.toContain(RATE_LIMIT);
      s.play(STOCKPILE);
      expect(JSON.stringify(s.view("p2"))).toContain(RATE_LIMIT);
    });

    it("R456 the next turn is theirs again as usual: the cut-short turn costs them only the rest of that turn", () => {
      const s = setup();
      s.play(VANILLA, { zone: 1 }).play(REPLENISH).play(STOCKPILE);
      expect(s.state.active).toBe("p1");
      s.endTurn();
      expect(s.state.active).toBe("p2");
      s.play(VANILLA, { zone: 2 }).play(VANILLA, { zone: 3 }).play(VANILLA, { zone: 4 });
      expect(s.state.active).toBe("p2");
    });
  });

  describe("radiant", () => {
    it("R456 fires on their 2nd play: the play resolves, then their turn ends", () => {
      const s = setup({}, {}, true);
      s.play(VANILLA, { zone: 1 });
      expect(s.card(RATE_LIMIT).faceUp).toBe(false);
      s.play(STOCKPILE);
      s.expectInZone(RATE_LIMIT, "graveyard");
      s.expectHealth("p2", 32);
      expect(s.state.active).toBe("p1");
    });
  });
});
