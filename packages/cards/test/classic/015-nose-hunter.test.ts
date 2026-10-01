// C #15 Nose Hunter — SPEC §8.6 row 15, BUILD M9 Classic row C 15: "Activate, once per turn (R392,
// R384): its cost, discarding a random card of yours, is paid as it activates, so with an empty hand
// it can't activate and `legalActions` doesn't list it; then exile the bottom card of the opponent's
// deck (an empty deck: nothing); usable the turn it is played (no sickness, no exertion), only in your
// main phase; a second activation that turn is refused; not a play (R384); the discard is a discard
// (C #64 sees it); no event carries a deck position; radiant 6/2: also exile a random card from their
// hand (empty: nothing), public once in exile; its tuned number (exiled) reads through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { legalActions, stepParam } from "@jackioh/engine";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/015-nose-hunter";

const NOSE = "classic-015";
const FILLER = "core-005"; // (1) Spell Stockpile: a card to discard, or to keep a hand from auto-ending (§2.5).
const VANILLA = "core-008"; // (1) 4/4 Unit.
const TIMMY = "core-011"; // (1) 3/3.
const MENACE = "core-019"; // (3) 9/9.
const POINTMASTER = "core-020"; // (2) 7/1.

/** p2's deck, top first: the bottom card is Pointmaster, so "the bottom card" is visible by def. */
const DECK = [VANILLA, TIMMY, POINTMASTER] as const;

function setup(p1: SideSetup, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { library: [FILLER, FILLER], ...p1 },
    p2: { hand: [FILLER], library: [...DECK], ...p2 },
  });
}

function activations(s: Scenario, player: PlayerId, instanceId: string): unknown[] {
  return legalActions(s.state, player).filter(
    (action) => (action.type === "activate" || action.type === "activatePower") && action.instanceId === instanceId,
  );
}

function defsOf(cards: readonly { defId: string }[]): string[] {
  return cards.map((card) => card.defId);
}

describe("C #15 Nose Hunter", () => {
  it("declares one Activate ability on each face, once per turn, whose cost is a random discard", () => {
    expect(def.id).toBe(NOSE);
    for (const face of [base, radiant]) {
      expect(face.activations).toHaveLength(1);
      expect(face.activations?.[0]?.uses).toBe(1);
      expect(face.activations?.[0]?.cost).toEqual({ discardRandom: 1 });
    }
  });

  describe("base", () => {
    it("R392 activating discards a random card of yours and exiles the bottom card of the opponent's deck", () => {
      const s = setup({ hand: [FILLER], field: [NOSE] });
      const discarded = s.hand("p1")[0];
      const bottom = s.pile("p2", "library")[2];
      if (discarded === undefined || bottom === undefined) throw new Error("fixture");

      s.activate(NOSE);

      s.expectInZone(discarded, "graveyard");
      s.expectInZone(bottom, "exile");
      expect(defsOf(s.pile("p2", "library"))).toEqual([VANILLA, TIMMY]);
      expect(defsOf(s.pile("p2", "exile"))).toEqual([POINTMASTER]);
      s.expectEvents("activated", "discarded", "exiled");
    });

    it("R392 the discard is its cost: with an empty hand it can't be activated, and legalActions doesn't list it", () => {
      const s = setup({ hand: [], field: [VANILLA, NOSE] });
      const nose = s.card(NOSE);

      expect(activations(s, "p1", nose.id)).toHaveLength(0);
      expect(() => s.activate(nose)).toThrow();
      expect(s.pile("p2", "exile")).toHaveLength(0);
    });

    it("R392 with a card in hand it is listed, and the random discard is drawn from the match rng", () => {
      const s = setup({ hand: [FILLER, VANILLA, TIMMY], field: [NOSE] });
      const nose = s.card(NOSE);
      expect(activations(s, "p1", nose.id).length).toBeGreaterThan(0);

      s.activate(nose);

      expect(s.hand("p1")).toHaveLength(2);
      expect(s.pile("p1", "graveyard")).toHaveLength(1);
      const discards = s.events.filter((event) => event.type === "discarded");
      expect(discards).toHaveLength(1);
    });

    it("an empty deck exiles nothing and deals no fatigue; the discard is still paid", () => {
      const s = setup({ hand: [FILLER], field: [NOSE] }, { library: [] });

      s.activate(NOSE);

      expect(s.pile("p1", "graveyard")).toHaveLength(1);
      expect(s.pile("p2", "exile")).toHaveLength(0);
      s.expectHealth("p2", 30);
      expect(s.events.some((event) => event.type === "fatigue")).toBe(false);
    });

    it("R384 usable the turn it is played: activating needs no readiness and spends no exertion", () => {
      const s = setup({ hand: [NOSE, FILLER, FILLER] });
      s.play(NOSE, { zone: 1 });

      s.activate(NOSE);

      expect(s.pile("p2", "exile")).toHaveLength(1);
      // The same unit may still switch position this turn: activating spent no exertion.
      const nose = s.card(NOSE);
      expect(nose.exertion.attacked).toBe(false);
      expect(nose.exertion.switched).toBe(false);
    });

    it("R384 a unit that activated may still attack that turn", () => {
      const s = setup({ hand: [FILLER, FILLER], field: [NOSE] });

      s.activate(NOSE).attack(NOSE, "hero");

      s.expectHealth("p2", 27);
    });

    it("R384 a second activation that turn is refused and not listed; it is back next turn", () => {
      const s = setup({ hand: [FILLER, FILLER, FILLER], field: [NOSE] });
      const nose = s.card(NOSE);

      s.activate(nose);
      expect(activations(s, "p1", nose.id)).toHaveLength(0);
      expect(() => s.activate(nose)).toThrow();
      expect(s.pile("p2", "exile")).toHaveLength(1);

      s.endTurn().endTurn();
      expect(s.state.active).toBe("p1");
      expect(activations(s, "p1", nose.id).length).toBeGreaterThan(0);
      s.activate(nose);
      expect(s.pile("p2", "exile")).toHaveLength(2);
    });

    it("R384 only its controller, in their own main phase: on the opponent's turn it can't be activated", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [NOSE] },
        p2: { hand: [FILLER], library: [...DECK] },
      });
      const nose = s.card(NOSE);

      expect(activations(s, "p1", nose.id)).toHaveLength(0);
      expect(() => s.activate(nose)).toThrow();
      expect(s.pile("p2", "exile")).toHaveLength(0);
    });

    it("R13 R384 dormant under a Stack pile it does not act, so it can't be activated", () => {
      const s = setup({ hand: [FILLER], field: [{ def: NOSE, lane: 1 }, { def: VANILLA, stack: true }] });
      const nose = s.card(NOSE);

      expect(activations(s, "p1", nose.id)).toHaveLength(0);
      expect(() => s.activate(nose)).toThrow();
      expect(s.hand("p1")).toHaveLength(1);
      expect(s.pile("p2", "exile")).toHaveLength(0);
    });

    it("R384 activating is not a play: no cardPlayed, and the turn's play count does not move", () => {
      const s = setup({ hand: [FILLER, FILLER], field: [NOSE] });
      const played = s.state.players.p1.turnLog.cardsPlayed;

      s.activate(NOSE);

      expect(s.events.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played);
    });

    it("the cost is a discard (§6.3): the card goes hand to graveyard with a discarded event naming it", () => {
      const s = setup({ hand: [VANILLA], field: [NOSE] });
      const card = s.hand("p1")[0];
      if (card === undefined) throw new Error("fixture");

      s.activate(NOSE);

      const discard = s.events.find((event) => event.type === "discarded");
      expect(discard).toMatchObject({ type: "discarded", instanceId: card.id, defId: VANILLA, owner: "p1" });
      expect(s.events.some((event) => event.type === "enteredGraveyard" && event.instanceId === card.id)).toBe(true);
    });

    it("no event carries a deck position; the exiled card is public in both views", () => {
      const s = setup({ hand: [FILLER], field: [NOSE] });

      s.activate(NOSE);

      for (const event of s.lastEvents) {
        expect(Object.keys(event)).not.toContain("position");
        expect(Object.keys(event)).not.toContain("index");
      }
      const seen = (player: PlayerId): string[] => {
        const view = s.view(player);
        const side = player === "p2" ? view.you : view.opponent;
        return (side.exile as { defId?: string }[]).map((card) => card.defId ?? "");
      };
      expect(seen("p1")).toEqual([POINTMASTER]);
      expect(seen("p2")).toEqual([POINTMASTER]);
    });

    it("R386 an Upgrade of its number exiles the bottom 2 cards; a Degrade never takes it below 1", () => {
      const up = setup({ hand: [FILLER], field: [NOSE] });
      stepParam(up.card(NOSE), "exile", 1);
      up.activate(NOSE);
      expect(defsOf(up.pile("p2", "exile"))).toEqual([POINTMASTER, TIMMY]);
      expect(defsOf(up.pile("p2", "library"))).toEqual([VANILLA]);

      const down = setup({ hand: [FILLER], field: [NOSE] });
      stepParam(down.card(NOSE), "exile", -1);
      down.activate(NOSE);
      expect(defsOf(down.pile("p2", "exile"))).toEqual([POINTMASTER]);
    });
  });

  describe("radiant", () => {
    it("R275 the Radiant face is a 6/2", () => {
      const s = setup({ hand: [FILLER], field: [{ def: NOSE, radiant: true }] });
      s.expectStats(NOSE, { attack: 6, health: 2, maxHealth: 2 });
    });

    it("R392 also exiles a random card from the opponent's hand, public once in exile", () => {
      const s = setup({ hand: [FILLER], field: [{ def: NOSE, radiant: true }] }, { hand: [MENACE] });
      const menace = s.card(MENACE);

      s.activate(NOSE);

      s.expectInZone(menace, "exile");
      expect(defsOf(s.pile("p2", "exile")).sort()).toEqual([MENACE, POINTMASTER].sort());
      expect(s.hand("p2")).toHaveLength(0);
      const p1Sees = (s.view("p1").opponent.exile as { defId?: string }[]).map((card) => card.defId);
      expect(p1Sees).toContain(MENACE);
    });

    it("R60 the hand card is picked at random from their hand, one card only", () => {
      const s = setup(
        { hand: [FILLER], field: [{ def: NOSE, radiant: true }] },
        { hand: [MENACE, VANILLA, TIMMY] },
      );

      s.activate(NOSE);

      expect(s.hand("p2")).toHaveLength(2);
      expect(s.pile("p2", "exile")).toHaveLength(2);
    });

    it("an empty opponent's hand: only the deck's bottom card is exiled", () => {
      const s = setup({ hand: [FILLER], field: [{ def: NOSE, radiant: true }] }, { hand: [] });

      s.activate(NOSE);

      expect(defsOf(s.pile("p2", "exile"))).toEqual([POINTMASTER]);
    });

    it("R392 the Radiant face still costs a random discard: an empty hand can't activate it", () => {
      const s = setup({ hand: [], field: [VANILLA, { def: NOSE, radiant: true }] }, { hand: [MENACE] });
      const nose = s.card(NOSE);

      expect(activations(s, "p1", nose.id)).toHaveLength(0);
      expect(() => s.activate(nose)).toThrow();
      expect(s.hand("p2")).toHaveLength(1);
    });

    it("R386 an Upgrade moves the deck exile to 2 and leaves the hand exile at one card", () => {
      const s = setup({ hand: [FILLER], field: [{ def: NOSE, radiant: true }] }, { hand: [MENACE, VANILLA] });
      stepParam(s.card(NOSE), "exile", 1);

      s.activate(NOSE);

      expect(s.pile("p2", "library")).toHaveLength(1);
      expect(s.hand("p2")).toHaveLength(1);
      expect(s.pile("p2", "exile")).toHaveLength(3);
    });
  });
});
