// C+ #52 Jlockheed's Permanent Defense Contract — SPEC §8.7 row 52, BUILD M9 Classic+ row C+ 52: "Leaves
// a player modifier for the rest of the game (nothing on the field to remove) that adds, at each start
// of your turn, a random non-token Jlockeed card to your hand: Core #13, #14, C+ #48 or #51, never this
// card (R387); two Contracts add two; a full hand burns; the cards are hidden from the opponent (R97);
// count and discount read through `param()`; radiant the card is Radiant and costs (1) less (`costMod`
// −1, floor 0)".

import { effectiveCost, hashState, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/052-jlockheeds-permanent-defense-contract";

const CONTRACT = "classicplus-052";
const NETHER = "core-088"; // (4) Spell: destroy all permanents
const FILLER = "core-005";
const POOL = ["classicplus-048", "classicplus-051", "core-013", "core-014"];

function signed(opts: { radiant?: boolean; seed?: string; contracts?: number; fillers?: number } = {}): Scenario {
  const contract = { def: CONTRACT, ...(opts.radiant === true ? { radiant: true } : {}) };
  return scenario({
    seed: opts.seed ?? "contract",
    p1: {
      hand: [...Array.from({ length: opts.contracts ?? 1 }, () => contract), ...Array.from({ length: opts.fillers ?? 1 }, () => FILLER)],
      library: [FILLER, FILLER, FILLER, FILLER],
      mana: 10,
    },
    p2: { hand: [FILLER, NETHER], library: [FILLER, FILLER, FILLER], mana: 10 },
  });
}

function playAll(s: Scenario): void {
  while (s.hand("p1").some((card) => card.defId === CONTRACT)) s.play(s.hand("p1").find((card) => card.defId === CONTRACT) ?? "");
}

/** The Jlockeed cards p1's turn start added in the last step (before the turn's draw). */
function delivered(s: Scenario): Extract<GameEvent, { type: "addedToHand" | "burned" }>[] {
  return s.lastEvents.filter(
    (event): event is Extract<GameEvent, { type: "addedToHand" | "burned" }> =>
      (event.type === "addedToHand" && event.player === "p1" && POOL.includes(event.defId)) ||
      (event.type === "burned" && POOL.includes(event.defId)),
  );
}

/** p1 ends their turn and p2 theirs: p1's next turn starts. */
function nextTurn(s: Scenario): void {
  s.endTurn().endTurn();
}

describe("C+ #52 Jlockheed's Permanent Defense Contract", () => {
  it("is a (2) Spell, Jlockeed", () => {
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["Jlockeed"]);
  });

  describe("base", () => {
    it("R458 it leaves a rest-of-game modifier and nothing on the field", () => {
      const s = signed();
      playAll(s);
      s.expectInZone(CONTRACT, "graveyard");
      const mods = s.state.players.p1.mods.filter((mod) => mod.kind === "startOfTurnEffect");
      expect(mods).toHaveLength(1);
      expect(mods[0]?.expiry).toEqual({ until: "never" });
      // R169: its badge, public to both seats, in the card's own words.
      const label = "For the rest of the game: At the start of your turn, add 1 random Jlockheed card to your hand.";
      expect(s.view("p2").opponent.modifiers.map((mod) => mod.label)).toEqual([label]);
      expect(s.view("p1").you.modifiers.map((mod) => mod.label)).toEqual([label]);
    });

    it("R62 at each start of your turn it adds a random Jlockeed card, before the draw", () => {
      const s = signed();
      playAll(s);
      for (let turn = 0; turn < 3; turn += 1) {
        nextTurn(s);
        const cards = delivered(s);
        expect(cards).toHaveLength(1);
        expect(s.card(cards[0]?.instanceId ?? "").zone.z).toBe("hand");
        const order = s.lastEvents.map((event) => (event === cards[0] ? "contract" : event.type));
        expect(order.indexOf("contract")).toBeLessThan(order.lastIndexOf("drawn"));
      }
    });

    it("nothing at the opponent's start of turn", () => {
      const s = signed();
      playAll(s);
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(delivered(s)).toEqual([]);
    });

    it("R278 R387 the pool is exactly Core #13, #14 and C+ #48, #51 — never this card", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 60; i += 1) {
        const s = signed({ seed: `contract-${i}` });
        playAll(s);
        nextTurn(s);
        for (const event of delivered(s)) seen.add(event.defId);
      }
      expect([...seen].sort()).toEqual(POOL);
      const all = new Set<string>();
      for (let i = 0; i < 30; i += 1) {
        const s = signed({ seed: `contract-all-${i}` });
        playAll(s);
        nextTurn(s);
        for (const event of s.lastEvents) if (event.type === "addedToHand" && event.player === "p1") all.add(event.defId);
      }
      expect(all.has(CONTRACT)).toBe(false);
    });

    it("§10.1 destroying every permanent does not end it", () => {
      const s = signed();
      playAll(s);
      s.endTurn();
      const from = s.events.length;
      s.play(NETHER);
      // Nether spends p2's last mana, so their turn may end by itself (§2.5).
      if (s.state.active === "p2") s.endTurn();
      expect(s.state.active).toBe("p1");
      const added = s.events.slice(from).filter((event) => event.type === "addedToHand" && event.player === "p1" && POOL.includes(event.defId));
      expect(added).toHaveLength(1);
    });

    it("R458 two Contracts add two cards", () => {
      const s = signed({ contracts: 2 });
      playAll(s);
      nextTurn(s);
      expect(delivered(s)).toHaveLength(2);
    });

    it("§2.4 R4 a full hand burns the card", () => {
      const s = signed({ fillers: 10 });
      playAll(s);
      nextTurn(s);
      const burned = delivered(s).filter((event) => event.type === "burned");
      expect(burned).toHaveLength(1);
    });

    it("R97 the opponent sees the add under the sentinel", () => {
      const s = signed();
      playAll(s);
      nextTurn(s);
      const id = delivered(s)[0]?.instanceId ?? "?";
      const theirs = s.view("p2");
      expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
      const events = theirs.events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(events.length).toBeGreaterThan(0);
      for (const event of events) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
    });

    it("R594 R386 an Upgrade before the cast adds 2 each turn: the count is read as it resolves and carried", () => {
      const s = signed();
      stepParam(s.card(CONTRACT), "cards", 1);
      playAll(s);
      nextTurn(s);
      expect(delivered(s)).toHaveLength(2);
      nextTurn(s);
      expect(delivered(s)).toHaveLength(2);
    });

    it("R113 the state survives a JSON round trip and the next turn start replays the same", () => {
      const s = signed();
      playAll(s);
      s.endTurn();
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const replayed = reduce(revived, { type: "endTurn", playerId: "p2", nonce: "contract-replay" });
      expect(replayed.error).toBeUndefined();
      s.endTurn();
      expect(delivered(s)).toHaveLength(1);
      expect(hashState(replayed.state)).toBe(hashState(s.state));
    });
  });

  describe("radiant", () => {
    it("R74 the card is Radiant and costs (1) less", () => {
      for (let i = 0; i < 12; i += 1) {
        const s = signed({ radiant: true, seed: `rcontract-${i}` });
        playAll(s);
        nextTurn(s);
        const card = s.card(delivered(s)[0]?.instanceId ?? "");
        expect(card.radiant).toBe(true);
        expect(card.costMod).toBe(-1);
        expect(card.defId).not.toBe(CONTRACT);
      }
    });

    it("R97 R177 the opponent sees neither the Radiant card nor its price change", () => {
      const s = signed({ radiant: true });
      playAll(s);
      nextTurn(s);
      const id = delivered(s)[0]?.instanceId ?? "?";
      expect(s.card(id).zone.z).toBe("hand");
      const theirs = s.view("p2");
      expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
      const seen = theirs.events.filter((event) => (event.type === "addedToHand" && event.player === "p1") || event.type === "costChanged");
      expect(seen.length).toBeGreaterThan(0);
      for (const event of seen) expect(event).toMatchObject({ instanceId: "hidden" });
    });

    it("§2.3 the discount floors at (0): a Lobbyist (1) costs (0)", () => {
      let checked = false;
      for (let i = 0; i < 40 && !checked; i += 1) {
        const s = signed({ radiant: true, seed: `rfloor-${i}` });
        playAll(s);
        nextTurn(s);
        const card = s.card(delivered(s)[0]?.instanceId ?? "");
        if (card.defId !== "classicplus-048") continue;
        expect(effectiveCost(s.state, card)).toBe(0);
        checked = true;
      }
      expect(checked).toBe(true);
    });

    it("R594 R386 count and discount read through `param()` as it resolves", () => {
      const s = signed({ radiant: true });
      stepParam(s.card(CONTRACT), "cards", 1);
      stepParam(s.card(CONTRACT), "discount", 1);
      playAll(s);
      nextTurn(s);
      const cards = delivered(s).map((event) => s.card(event.instanceId));
      expect(cards).toHaveLength(2);
      expect(cards.every((card) => card.costMod === -2 && card.radiant)).toBe(true);
    });
  });
});
