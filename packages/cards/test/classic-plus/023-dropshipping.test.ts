// C+ #23 Dropshipping — SPEC §8.7 row 23, BUILD M9 Classic+ row C+ 23: "Adds 3 random cards drawn from
// every card and token of every set but Dropshipping (R382, R387; repeats allowed, R60), so a Grape, a
// Loser, an AI generated card or a unit-token card (R11) can arrive, each at its printed cost and
// given Brittle 2 (R385): held in your hand, where it does not tick (R638), and started as the card enters
// the field, where it ticks to 1 at the start of your turn t + 2 and crumbles at t + 4 (an ordinary
// destroy that Indestructible ignores, the count staying 0); the count rides the card from hand to
// field; the owner sees the counts, the opponent sees three cards added under the sentinel (R97); a full hand burns
// the rest; card count and Brittle read through `param()`; radiant they cost (1) (`costOverride` 1)".

import { createRng, GRAPE_ODDS, HAND_CAP, hashState, pickGenerated, query, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, CardView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/023-dropshipping";

const DROP = "classicplus-023";
const FILLER = "core-005";
const DECK = Array.from({ length: 12 }, () => FILLER);

function shop(seed: string, radiantFace = false, hand: string[] = [FILLER]): Scenario {
  return scenario({
    seed,
    p1: { hand: [{ def: DROP, radiant: radiantFace }, ...hand], library: DECK },
    p2: { hand: [FILLER, FILLER], library: DECK },
  });
}

/** The cards Dropshipping put in p1's hand, by the `addedToHand` events of its play. */
function added(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "addedToHand" && event.player === "p1" ? [event.instanceId] : []));
}

function ownHand(s: Scenario): CardView[] {
  const hand = s.view("p1").you.hand;
  if (!Array.isArray(hand)) throw new Error("own hand is a list");
  return hand;
}

/** Two `endTurn`s: the opponent's turn, then p1's next start of turn. */
function nextOwnTurn(s: Scenario): Scenario {
  return s.endTurn().endTurn();
}

describe("C+ #23 Dropshipping", () => {
  it("is a (1) CN Spell; both faces add and give Brittle", () => {
    expect(def.cost).toBe(1);
    expect(def.tags).toContain("CN");
    expect(base.cry).toBeDefined();
    expect(radiant.cry).toBeDefined();
  });

  describe("base", () => {
    it("R382 R387 the pool is every card and token of every set but Dropshipping", () => {
      const pool = query({ withTokens: true, excludeDefId: DROP }).map((card) => card.id);
      expect(pool).not.toContain(DROP);
      for (const id of ["classicplus-065-1", "classicplus-065-5", "classicplus-019-3", "classicplus-t-ai-01", "core-t-rush", "core-t-coin", "core-001", "classic-001"]) {
        expect(pool).toContain(id);
      }
      expect(pool.length).toBe(query({ withTokens: true }).length - 1);
    });

    it("adds three cards to the hand, each at its printed cost, with Brittle 2 the owner sees", () => {
      const s = shop("drop-1");
      s.play(DROP);
      const ids = added(s);
      expect(ids).toHaveLength(3);
      const views = ownHand(s).filter((card) => ids.includes(card.instanceId));
      expect(views).toHaveLength(3);
      for (const id of ids) {
        const card = s.card(id);
        expect(card.zone.z).toBe("hand");
        expect(card.costOverride).toBeUndefined();
        expect(card.radiant).toBe(false);
        expect(card.defId).not.toBe(DROP);
      }
      expect(views.map((card) => card.brittle)).toEqual([2, 2, 2]);
    });

    it("R60 R382 across seeds it never makes itself, may repeat, and hands out tokens", () => {
      let sawToken = false;
      for (let seed = 1; seed <= 40; seed += 1) {
        const s = shop(`drop-seeds-${seed}`);
        s.play(DROP);
        for (const id of added(s)) {
          expect(s.card(id).defId).not.toBe(DROP);
          if (s.card(id).defId.includes("-t-") || /-\d{3}-\d/.test(s.card(id).defId)) sawToken = true;
        }
      }
      expect(sawToken).toBe(true);
    });

    it("R382 a Grape the pool picks is re-rolled on GRAPE_ODDS, not generated as picked", () => {
      // Each seed below picks a Grape mid-pool (read off the probe rng): the card Dropshipping
      // generates for it is the re-roll, so the generated triple equals `pickGenerated` three times
      // over — and differs from the picked triple, which a plain `rng.pick` would generate as is.
      const pool = query({ withTokens: true, excludeDefId: DROP });
      const grapes = new Set(GRAPE_ODDS.map((grape) => grape.defId));
      const table = GRAPE_ODDS.reduce((sum, grape) => sum + grape.percent, 0);
      let differed = 0;
      for (const seed of ["drop-reroll-3", "drop-reroll-36", "drop-reroll-72", "drop-reroll-76"]) {
        const s = shop(seed);
        const rng = createRng(s.state.seed, s.state.rngCursor);
        const probe = createRng(s.state.seed, s.state.rngCursor);
        const expected: (string | undefined)[] = [];
        const firsts: (string | undefined)[] = [];
        for (let at = 0; at < 3; at += 1) {
          const first = probe.pick(pool);
          firsts.push(first?.id);
          if (first !== undefined && grapes.has(first.id)) probe.int(table);
          expected.push(pickGenerated(rng, pool)?.id);
        }
        expect(firsts.some((id) => id !== undefined && grapes.has(id))).toBe(true);
        if (JSON.stringify(firsts) !== JSON.stringify(expected)) differed += 1;
        s.play(DROP);
        expect(added(s).map((id) => s.card(id).defId)).toEqual(expected);
      }
      expect(differed).toBeGreaterThan(0);
    });

    it("§9.3 the three random cards replay from a JSON copy to the same hash", () => {
      const s = shop("drop-replay");
      const action = { type: "play", instanceId: s.card(DROP).id, playerId: "p1", nonce: "drop-replay" } as Action;
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const live = reduce(s.state, action);
      expect(live.error).toBeUndefined();
      expect(hashState(reduce(thawed, action).state)).toBe(hashState(live.state));
    });

    it("R97 the opponent sees three cards added and nothing of which", () => {
      const s = shop("drop-hidden");
      s.play(DROP);
      const ids = added(s);
      const theirs = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(theirs).toHaveLength(3);
      for (const event of theirs) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
      const text = JSON.stringify(s.view("p2"));
      for (const id of ids) expect(text).not.toContain(id);
    });

    it("R385 R638 the cards hold Brittle 2 in the hand: it never ticks there and nothing crumbles, however long they wait", () => {
      const s = shop("drop-hold");
      s.play(DROP);
      const ids = added(s);
      const turn = s.state.turn;
      nextOwnTurn(s);
      nextOwnTurn(s);
      expect(s.state.turn).toBe(turn + 4);
      expect(ownHand(s).filter((card) => ids.includes(card.instanceId)).map((card) => card.brittle)).toEqual([2, 2, 2]);
      expect(s.events.some((event) => event.type === "crumbled" || event.type === "discarded")).toBe(false);
      for (const id of ids) s.expectInZone(id, "hand");
    });

    it("R385 the count rides the card onto the field, and there it is an ordinary destroy Indestructible ignores", () => {
      for (let seed = 1; seed <= 200; seed += 1) {
        const s = shop(`drop-field-${seed}`);
        s.play(DROP);
        const unit = added(s).find((id) => {
          const card = s.card(id);
          const printed = query({ defId: card.defId, withTokens: true })[0];
          return printed?.type === "Unit" && typeof printed.cost === "number" && printed.cost <= 3 && !/Tribute/.test(printed.base.text);
        });
        if (unit === undefined) continue;
        try {
          s.play(unit);
        } catch {
          continue;
        }
        const placed = s.card(unit);
        if (placed.zone.z !== "field") continue;
        // The Brittle count came with it, and shows on the board to both players.
        const lane = placed.zone.z === "field" ? placed.zone.lane : 0;
        expect(s.view("p2").opponent.units[lane - 1]?.brittle).toBe(2);
        s.card(unit).grantedKeywords.push({ kind: "Indestructible" });
        nextOwnTurn(s);
        nextOwnTurn(s);
        // Indestructible ignores the destroy, and the count stays at 0.
        expect(s.card(unit).zone.z).toBe("field");
        expect(s.view("p1").you.units[lane - 1]?.brittle ?? 0).toBe(0);
        return;
      }
      throw new Error("no seed handed out a playable Unit");
    });

    it("§2.4 R317 a full hand burns what doesn't fit, and a burned card takes no count", () => {
      const s = shop("drop-full", false, Array.from({ length: HAND_CAP - 1 }, () => FILLER));
      s.play(DROP);
      // Nine cards in hand after the play: one fits, two burn.
      expect(added(s)).toHaveLength(1);
      const burned = s.events.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(2);
      for (const event of burned) {
        const card = s.card(event.instanceId);
        expect(card.zone.z === "graveyard" || card.zone.z === "gone").toBe(true);
        expect(card.brittle).toBeUndefined();
      }
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
    });

    it("R386 the card count and the Brittle read through param", () => {
      const s = shop("drop-tuned");
      stepParam(s.card(DROP), "cards", 1);
      stepParam(s.card(DROP), "brittle", 1);
      s.play(DROP);
      const ids = added(s);
      expect(ids).toHaveLength(4);
      expect(ownHand(s).filter((card) => ids.includes(card.instanceId)).map((card) => card.brittle)).toEqual([3, 3, 3, 3]);
    });
  });

  describe("radiant", () => {
    it("the three cost (1): `costOverride` 1, still Brittle 2", () => {
      const s = shop("drop-radiant", true);
      s.play(DROP);
      const ids = added(s);
      expect(ids).toHaveLength(3);
      for (const id of ids) expect(s.card(id).costOverride).toBe(1);
      const views = ownHand(s).filter((card) => ids.includes(card.instanceId));
      expect(views.map((card) => card.brittle)).toEqual([2, 2, 2]);
    });
  });
});
