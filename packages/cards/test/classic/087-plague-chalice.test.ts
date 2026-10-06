// C #87 Plague Chalice — SPEC §8.6 row 87, BUILD M9 Classic row C 87: "X chosen with the play, at least 1
// (R348); it enters with X Plague Counters; Aura: every card either player plays whose cost paid equals its
// current token count is countered in the announce window (§10.5), treated as never played as C #17's
// is; a free cast is countered only at a count of 0 (R70); the count moves (C #78 removes tokens,
// placements add them); it isn't on the field during its own announce and never counters itself; a set
// trap's cost is public (R351), so countering it reveals only what the graveyard then shows; leaving the
// field ends it; radiant: only the opponent's plays; no tuned numbers".

import { legalActions, type CardInstance } from "@jackioh/engine";
import type { ActionBody, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/087-plague-chalice";

const CHALICE = "classic-087";
const BRINGER = "classic-076"; // (2) Unit: Rush; Cry: place 2 Plague Counters (two placements). Draw 1.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const MUTATE = "classic-078"; // (1) Field Spell: Activate ♾️: remove a Plague Counter from a permanent …
const VANILLA = "core-008"; // (1) Unit 4/4.
const POINTMASTER = "core-020"; // (2) Unit 7/1 First Strike.
const HINDER = "core-021"; // (0) Spell: Cast on draw: your opponent has 1 less mana next turn. Discard 1.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const COLLATERAL = "core-034"; // (4) Spell: Exile target permanent and a random card from your opponent's deck.
const ANCHOR = "core-010"; // (0) Spell (§2.5).
const FILLER = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const X = "core-019"; // library filler.

type Play = Extract<ActionBody, { type: "play" }>;

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function countered(events: readonly GameEvent[]): string[] {
  return events.flatMap((event) => (event.type === "countered" ? [event.instanceId] : []));
}

/** A Chalice standing with `tokens` on it (a setup Chalice arrived with no X: R396's 0). */
function standing(tokens: number, radiantFace = false, active: "p1" | "p2" = "p1"): Scenario {
  return scenario({
    active,
    p1: { hand: [VANILLA, POINTMASTER, ANCHOR, FILLER], backrow: [{ def: CHALICE, radiant: radiantFace, counters: tokens > 0 ? { plague: tokens } : {} }], library: lib(4) },
    p2: { hand: [VANILLA, POINTMASTER, ANCHOR, FILLER], library: lib(4) },
  });
}

describe("C #87 Plague Chalice", () => {
  it("is an X-cost Field Spell with no declared numbers; base counters both players, Radiant the opponent's", () => {
    expect(def.id).toBe(CHALICE);
    expect(def.cost).toBe("X");
    expect(def.params).toBeUndefined();
    expect(base.triggers?.map((trigger) => trigger.on)).toEqual([["cardAnnounced"]]);
    expect(radiant.triggers?.map((trigger) => trigger.on)).toEqual([["cardAnnounced"]]);
  });

  describe("base", () => {
    it("R348 X is chosen with the play, at least 1 and at most your mana", () => {
      const s = scenario({ p1: { hand: [CHALICE, ANCHOR] }, p2: { hand: [ANCHOR] } });
      const chalice = s.card(CHALICE);
      const xs = new Set(
        legalActions(s.state, "p1")
          .filter((action): action is Play => action.type === "play" && action.instanceId === chalice.id)
          .map((play) => play.x),
      );

      expect(xs).toEqual(new Set([1, 2, 3, 4]));
      expect(() => s.play(chalice, { x: 0 })).toThrow(/at least 1/);
      expect(() => s.play(chalice)).toThrow();
    });

    it("it enters with X Plague Counters on it, one placement of X", () => {
      const s = scenario({ p1: { hand: [CHALICE, ANCHOR] }, p2: { hand: [ANCHOR] } });

      s.play(CHALICE, { x: 3 });

      const chalice = s.card(CHALICE);
      expect(chalice.counters.plague).toBe(3);
      expect(s.events.filter((event) => event.type === "counterChanged")).toEqual([
        { type: "counterChanged", instanceId: chalice.id, counter: "plague", value: 3, placed: 3 },
      ]);
      s.expectMana("p1", 1);
    });

    it("it never counters itself: it is not on the field during its own announce", () => {
      const s = scenario({ p1: { hand: [CHALICE, ANCHOR] }, p2: { hand: [ANCHOR] } });

      s.play(CHALICE, { x: 1 });

      expect(countered(s.events)).toEqual([]);
      s.expectInZone(CHALICE, "field");
    });

    it("§10.5 your own play whose cost paid equals the count is countered, treated as never played", () => {
      const s = standing(1);
      const vanilla = s.card(VANILLA);
      const playedBefore = s.state.players.p1.turnLog.cardsPlayed;

      s.play(vanilla);

      expect(countered(s.events)).toEqual([vanilla.id]);
      s.expectInZone(vanilla, "graveyard");
      expect(s.events.some((event) => event.type === "cardPlayed" && event.instanceId === vanilla.id)).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(playedBefore);
      // The mana paid stays spent.
      s.expectMana("p1", 3);
    });

    it("the opponent's play at the count is countered too, and a play of another cost resolves", () => {
      const s = standing(1, false, "p2");
      const theirVanilla = s.hand("p2").find((card) => card.defId === VANILLA) as CardInstance;
      const theirPointmaster = s.hand("p2").find((card) => card.defId === POINTMASTER) as CardInstance;

      s.play(theirVanilla);
      s.play(theirPointmaster);

      expect(countered(s.events)).toEqual([theirVanilla.id]);
      s.expectInZone(theirPointmaster, "field");
    });

    it("a second Chalice played for the same X is countered by the first", () => {
      const s = scenario({ p1: { hand: [CHALICE, CHALICE, ANCHOR], mana: 10 }, p2: { hand: [ANCHOR] } });
      const [first, second] = s.hand("p1").filter((card) => card.defId === CHALICE);
      if (first === undefined || second === undefined) throw new Error("two Chalices in hand");

      s.play(first, { x: 2 });
      s.play(second, { x: 2 });

      expect(countered(s.events)).toEqual([second.id]);
      s.expectInZone(second, "graveyard");
    });

    it("R70 a free cast is countered only at a count of 0: a cast-on-draw Hinder at 1 resolves", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], backrow: [{ def: CHALICE, counters: { plague: 1 } }], library: lib(3) },
        p2: { hand: [ANCHOR, FILLER], library: [HINDER, ...lib(3)] },
      });

      s.endTurn(); // p2 draws Hinder and casts it, paying 0; its discard is picked as the cast begins (R70)
      s.answer(s.hand("p2").find((card) => card.defId === FILLER)?.id ?? "");

      expect(countered(s.events)).toEqual([]);
      expect(s.events.some((event) => event.type === "discarded")).toBe(true);
      expect(s.events.some((event) => event.type === "cardPlayed" && event.defId === HINDER)).toBe(true);
    });

    it("R70 at a count of 0 a free cast is countered: Hinder cast on draw does nothing", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], backrow: [CHALICE], library: lib(3) },
        p2: { hand: [ANCHOR, FILLER], library: [HINDER, ...lib(3)] },
      });
      const hinder = s.pile("p2", "library")[0] as CardInstance;

      s.endTurn();
      s.answer(s.hand("p2").find((card) => card.defId === FILLER)?.id ?? "");

      expect(countered(s.events)).toEqual([hinder.id]);
      expect(s.state.pending).toBeNull();
      s.expectInZone(hinder, "graveyard");
      // Countered before it resolved: nothing was discarded and no refresh rider was set.
      expect(s.events.some((event) => event.type === "discarded")).toBe(false);
      expect(s.events.some((event) => event.type === "cardPlayed" && event.instanceId === hinder.id)).toBe(false);
    });

    it("at a count of 0 a (0) Cost play is countered too", () => {
      const s = standing(0);
      const anchor = s.card(ANCHOR);

      s.play(anchor);

      expect(countered(s.events)).toEqual([anchor.id]);
    });

    it("the count moves up: a Plague Bringer's two placements make it 3, and a (3) Cost play is then countered", () => {
      const s = scenario({
        p1: { hand: [BRINGER, MENACE, ANCHOR], backrow: [{ def: CHALICE, counters: { plague: 1 } }], library: lib(2), mana: 10 },
        p2: { hand: [ANCHOR] },
      });
      const chalice = s.card(CHALICE);
      const menace = s.hand("p1").find((card) => card.defId === MENACE) as CardInstance;

      // The (2) Bringer resolves at a count of 1, and both its placements go on the Chalice.
      s.play(BRINGER);
      s.answer(chalice.id).answer(chalice.id);
      expect(s.card(chalice).counters.plague).toBe(3);
      s.play(menace);

      expect(countered(s.events)).toEqual([menace.id]);
    });

    it("the count moves: C #78 Mutate Spell removes a token, and a (1) Cost play then resolves while a (0) is countered", () => {
      const s = scenario({
        p1: { hand: [VANILLA, ANCHOR, FILLER], backrow: [{ def: CHALICE, counters: { plague: 1 } }, { def: MUTATE, lane: 2 }], library: lib(4) },
        p2: { hand: [ANCHOR], health: 30 },
      });
      const vanilla = s.card(VANILLA);
      const anchor = s.card(ANCHOR);

      s.activate(MUTATE, { targets: [{ pick: "instance", instanceId: s.card(CHALICE).id }] });
      expect(s.card(CHALICE).counters.plague).toBeUndefined();
      s.play(vanilla);
      s.play(anchor);

      s.expectInZone(vanilla, "field");
      expect(countered(s.events)).toEqual([anchor.id]);
    });

    it("R351 a set trap's cost is public: one set at the count is countered into the graveyard, which then shows it", () => {
      const t = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], backrow: [{ def: CHALICE, counters: { plague: 1 } }] },
        p2: { hand: [PAWN, ANCHOR] },
      });
      const trap = t.card(PAWN);

      t.play(trap);

      expect(countered(t.events)).toEqual([trap.id]);
      expect(t.pile("p2", "graveyard").map((card) => card.defId)).toEqual([PAWN]);
      // p1 reads it only now that the graveyard shows it.
      expect(JSON.stringify(t.view("p1").opponent)).toContain(PAWN);
    });

    it("leaving the field ends it: exiled, it counters nothing more", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], backrow: [{ def: CHALICE, counters: { plague: 1 } }], library: lib(2) },
        p2: { hand: [COLLATERAL, VANILLA, ANCHOR], library: lib(2), mana: 10 },
      });

      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(CHALICE).id }] });
      s.play(VANILLA);

      expect(countered(s.events)).toEqual([]);
      s.expectInZone(VANILLA, "field");
    });
  });

  describe("radiant", () => {
    it("counters the opponent's play at the count", () => {
      const s = standing(1, true, "p2");
      const theirVanilla = s.hand("p2").find((card) => card.defId === VANILLA) as CardInstance;

      s.play(theirVanilla);

      expect(countered(s.events)).toEqual([theirVanilla.id]);
    });

    it("lets your own play at the count resolve", () => {
      const s = standing(1, true);
      const vanilla = s.card(VANILLA);

      s.play(vanilla);

      expect(countered(s.events)).toEqual([]);
      s.expectInZone(vanilla, "field");
    });

    it("it enters with X tokens too", () => {
      const s = scenario({ p1: { hand: [{ def: CHALICE, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR] } });
      s.play(CHALICE, { x: 2 });
      expect(s.card(CHALICE).counters.plague).toBe(2);
    });
  });
});

describe("C #87 Plague Chalice: R667 the warning on the viewer's hand (patch v0.2.7)", () => {
  /** The hand cards `player`'s own view marks `counteredOnPlay`, by definition. */
  function warned(s: Scenario, player: "p1" | "p2"): string[] {
    const hand = s.view(player).you.hand;
    return Array.isArray(hand) ? hand.filter((card) => card.counteredOnPlay === true).map((card) => card.defId) : [];
  }

  it("R667 base: each player's own hand card whose cost equals the count is marked, and only those", () => {
    const s = standing(2);
    // VANILLA (1), POINTMASTER (2), ANCHOR (0), FILLER (1): only the (2) meets a count of 2, on both seats.
    expect(warned(s, "p1")).toEqual([POINTMASTER]);
    expect(warned(s, "p2")).toEqual([POINTMASTER]);
    expect(warned(standing(0), "p1")).toEqual([ANCHOR]);
    expect(warned(standing(5), "p1")).toEqual([]);
  });

  it("R667 R97 the mark rides only the viewer's own hand: the opponent's is a count", () => {
    const s = standing(2);
    expect(s.view("p1").opponent.hand).toEqual({ count: 4 });
    for (const card of s.view("p1").you.hand as { counteredOnPlay?: true; defId: string }[]) {
      expect(card.counteredOnPlay === true).toBe(card.defId === POINTMASTER);
    }
  });

  it("R667 radiant: only its controller's opponent is warned", () => {
    const s = standing(2, true);
    expect(warned(s, "p1")).toEqual([]);
    expect(warned(s, "p2")).toEqual([POINTMASTER]);
  });

  it("R667 the warning and the counter agree: the marked card is countered, an unmarked one resolves", () => {
    const s = standing(1);
    expect(warned(s, "p1")).toEqual([VANILLA, FILLER]);
    const vanilla = s.card(VANILLA);
    s.play(vanilla);
    expect(countered(s.lastEvents)).toEqual([vanilla.id]);

    const t = standing(1);
    expect(warned(t, "p1")).not.toContain(POINTMASTER);
    const pointmaster = t.card(POINTMASTER);
    t.play(pointmaster);
    expect(countered(t.lastEvents)).toEqual([]);
  });

  it("R667 the mark moves with the count: a token removed takes it off the (2) and puts it on the (1)s", () => {
    const s = standing(2);
    expect(warned(s, "p1")).toEqual([POINTMASTER]);
    s.card(CHALICE).counters.plague = 1;
    expect(warned(s, "p1")).toEqual([VANILLA, FILLER]);
  });

  it("R667 an X card is marked only when every X it could be played for is countered", () => {
    // A second Chalice in hand: X runs 1 to the mana, so one countered X leaves the others to play.
    const s = scenario({
      p1: { hand: [CHALICE], backrow: [{ def: CHALICE, counters: { plague: 1 } }], mana: 3 },
      p2: { hand: [ANCHOR] },
    });
    expect(warned(s, "p1")).toEqual([]);
    // With 1 mana its only X is 1, which the count meets.
    s.state.players.p1.mana.current = 1;
    expect(warned(s, "p1")).toEqual([CHALICE]);
  });

  it("R667 leaving the field ends the warning: exiled, it marks nothing more", () => {
    const s = scenario({
      active: "p2",
      p1: { hand: [ANCHOR], backrow: [{ def: CHALICE, counters: { plague: 1 } }], library: lib(2) },
      p2: { hand: [COLLATERAL, VANILLA, ANCHOR], library: lib(2), mana: 10 },
    });
    expect(warned(s, "p2")).toEqual([VANILLA]);

    s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(CHALICE).id }] });

    expect(warned(s, "p2")).toEqual([]);
  });

  it("R667 wouldCounter is the trigger's own match, on both faces", () => {
    const s = standing(3);
    const self = s.card(CHALICE);
    const ask = (face: typeof base, player: "p1" | "p2", costPaid: number): boolean | undefined =>
      face.wouldCounter?.({ state: s.state, self, controller: "p1", player, costPaid });
    expect([ask(base, "p1", 3), ask(base, "p2", 3), ask(base, "p1", 2)]).toEqual([true, true, false]);
    expect([ask(radiant, "p1", 3), ask(radiant, "p2", 3), ask(radiant, "p2", 2)]).toEqual([false, true, false]);
  });
});
