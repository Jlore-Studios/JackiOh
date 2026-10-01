// C+ #37 Wardrum — SPEC §8.7 row 37, BUILD M9 Classic+ row C+ 37: "Quickdraw; while in your hand or
// deck, once your 3rd Spell, Field Spell or Trap play of a turn has resolved (casts count, R70; Units
// don't; the 4th doesn't), it is summoned into your leftmost open unit zone with no Cry (R1), from the
// deck too after a mulligan returned it; with no open zone it stays where it is; on the field, at the
// end of your turn, it casts a copy (R70) of one random Spell, Field Spell or Trap you played this
// turn, by definition and face, a Trap copy set face-down and fizzling to the graveyard with no open
// backrow zone; none played, nothing; a Trap copy stays hidden from the opponent (R33, R97), and a
// hand or deck trigger that does not fire shows the opponent nothing, the summon being the first they
// see of it; `conditionMet` in hand answers whether your next such play would summon it (R195); the
// threshold reads through `param()` and never drops below 2; radiant casts a copy of each".
//
// The R195 proofs live in this file (both answers, against the branch the play then takes).

import { DECK_SIZE, beginGame, createGame, query, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, CardView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";

const WARDRUM = "classicplus-037";
const REPLENISH = "core-010"; // (0) Spell: Combo 3: draw 3.
const STOCKPILE = "core-005"; // (1) Spell: draw 2, heal your hero 2.
const LUNAR = "core-035"; // (1) Spell: 3 damage to a target.
const BONE_STORM = "classicplus-036-1"; // (1) Spell: 1 damage to each enemy.
const SHEEPISH = "core-041"; // (1) Trap.
const BREAD = "core-018"; // (1) Field Trap.
const MANA_WELL = "core-006"; // (3) Field Spell.
const VANILLA = "core-008"; // (1) Unit, no text.
const MENACE = "core-019";
const POINTMASTER = "core-020"; // (2) 7/1.
const FILLER = STOCKPILE;

const LIBRARY = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

function hand(s: Scenario, defId: string): string {
  const card = s.hand("p1").find((held) => held.defId === defId);
  if (card === undefined) throw new Error(`no ${defId} in p1's hand`);
  return card.id;
}

function playThree(s: Scenario): Scenario {
  s.play(hand(s, REPLENISH));
  s.play(hand(s, REPLENISH));
  s.play(hand(s, REPLENISH));
  return s;
}

function wardrumIn(s: Scenario): string {
  return s.card(WARDRUM).zone.z;
}

function glows(s: Scenario): boolean {
  const own = s.view("p1").you.hand;
  if (!Array.isArray(own)) throw new Error("own hand in full");
  const card = own.find((view: CardView) => view.defId === WARDRUM);
  return card?.conditionActive === true;
}

describe("C+ #37 Wardrum", () => {
  describe("base", () => {
    it("Quickdraw: dealt a deck holding it, it starts in the opening hand (§2.1)", () => {
      const others = query({})
        .map((def) => def.id)
        .filter((id) => id !== WARDRUM)
        .slice(0, DECK_SIZE - 1);
      const deck = [...others, WARDRUM];
      for (const seed of ["wd-q1", "wd-q2", "wd-q3"]) {
        const begun = beginGame(createGame({ seed, decks: [deck, deck] })).state;
        expect(begun.players.p1.hand.some((card) => card.defId === WARDRUM), seed).toBe(true);
        expect(begun.players.p2.hand.some((card) => card.defId === WARDRUM), seed).toBe(true);
      }
    });

    it("in hand: once the 3rd Spell of the turn has resolved, it is summoned into the leftmost open zone", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH], field: [{ def: MENACE, lane: 1 }], library: LIBRARY },
        p2: { hand: [FILLER] },
      });
      s.play(hand(s, REPLENISH));
      s.play(hand(s, REPLENISH));
      expect(wardrumIn(s)).toBe("hand");
      s.play(hand(s, REPLENISH));
      expect(s.unit("p1", 2)?.defId).toBe(WARDRUM);
      // Summoned after the 3rd resolved: its draw-3 came first.
      const resolved = s.events.map((event) => event.type).lastIndexOf("cardResolved");
      const summoned = s.events.findIndex((event) => event.type === "summoned" && event.defId === WARDRUM);
      expect(summoned).toBeGreaterThan(resolved);
      expect(s.events.some((event) => event.type === "cardPlayed" && event.defId === WARDRUM)).toBe(false);
    });

    it("Field Spells, Traps and Field Traps count; Units do not", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, VANILLA, VANILLA, SHEEPISH, BREAD, MANA_WELL, FILLER], library: LIBRARY, mana: 10 },
        p2: { hand: [FILLER] },
      });
      s.play(hand(s, VANILLA));
      s.play(hand(s, VANILLA));
      s.play(hand(s, SHEEPISH));
      s.play(hand(s, BREAD));
      expect(wardrumIn(s)).toBe("hand");
      s.play(hand(s, MANA_WELL));
      expect(wardrumIn(s)).toBe("field");
    });

    it("R70 casts count: a Bone Storm cast on draw is one of the three", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, STOCKPILE], library: [BONE_STORM, FILLER, FILLER, FILLER], mana: 4 },
        p2: { hand: [FILLER] },
      });
      s.play(hand(s, REPLENISH));
      s.play(hand(s, STOCKPILE)); // draws the Bone Storm: it casts itself (the 3rd), then draws again.
      expect(wardrumIn(s)).toBe("field");
    });

    it("R578 a cast inside the 3rd play is the 4th: it fires on the 3rd's resolution, after the cast's", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, STOCKPILE], library: [BONE_STORM, FILLER, FILLER, FILLER], mana: 4 },
        p2: { hand: [FILLER] },
      });
      s.play(hand(s, REPLENISH));
      s.play(hand(s, REPLENISH));
      const stockpile = hand(s, STOCKPILE);
      s.play(stockpile); // its draw casts the Bone Storm (the 4th) before the Stockpile (the 3rd) resolves
      expect(wardrumIn(s)).toBe("field");
      const order = s.events.flatMap((event) =>
        event.type === "cardResolved" ? [event.instanceId] : event.type === "summoned" && event.defId === WARDRUM ? ["wardrum"] : [],
      );
      expect(order.indexOf("wardrum")).toBeGreaterThan(order.indexOf(stockpile));
    });

    it("the 4th doesn't: a zone that opens only for the 4th play leaves it in hand", () => {
      const s = scenario({
        p1: {
          hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH, LUNAR],
          field: [POINTMASTER, MENACE, MENACE, MENACE, MENACE],
          library: LIBRARY,
          mana: 4,
        },
        p2: { hand: [FILLER] },
      });
      playThree(s);
      expect(wardrumIn(s)).toBe("hand"); // the 3rd found the board full
      s.play(hand(s, LUNAR), { targets: [{ pick: "instance", instanceId: s.card(POINTMASTER).id }] });
      expect(s.unit("p1", 1)).toBeNull(); // the 4th opened lane 1
      expect(wardrumIn(s)).toBe("hand");
    });

    it("from the deck too: summoned out of the library", () => {
      const s = scenario({
        p1: { hand: [LUNAR, LUNAR, LUNAR, FILLER], library: [FILLER, WARDRUM, FILLER], mana: 10 },
        p2: { hand: [FILLER] },
      });
      for (let i = 0; i < 3; i += 1) s.play(hand(s, LUNAR), { targets: [{ pick: "hero", player: "p2" }] });
      expect(s.unit("p1", 1)?.defId).toBe(WARDRUM);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("with no open unit zone it stays where it is", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH], field: [MENACE, MENACE, MENACE, MENACE, MENACE], library: LIBRARY },
        p2: { hand: [FILLER] },
      });
      playThree(s);
      expect(wardrumIn(s)).toBe("hand");
    });

    it("R97 a trigger that does not fire shows the opponent nothing; the summon is the first they see", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH], library: LIBRARY },
        p2: { hand: [FILLER] },
      });
      s.play(hand(s, REPLENISH));
      s.play(hand(s, REPLENISH));
      const id = s.card(WARDRUM).id;
      expect(JSON.stringify(s.view("p2"))).not.toContain(WARDRUM);
      expect(JSON.stringify(s.view("p2").events)).not.toContain(id);
      s.play(hand(s, REPLENISH));
      expect(s.view("p2").opponent.units[0]?.defId).toBe(WARDRUM);
      expect(s.view("p2").events.some((event) => event.type === "summoned" && event.defId === WARDRUM)).toBe(true);
    });

    it("R195 in hand it glows when the next such play would summon it, and the play then does", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH], library: LIBRARY },
        p2: { hand: [FILLER] },
      });
      expect(glows(s)).toBe(false);
      s.play(hand(s, REPLENISH));
      expect(glows(s)).toBe(false);
      s.play(hand(s, REPLENISH));
      expect(glows(s)).toBe(true);
      s.play(hand(s, REPLENISH));
      expect(wardrumIn(s)).toBe("field");
    });

    it("R195 it does not glow when the board is full, and the play then summons nothing", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH], field: [MENACE, MENACE, MENACE, MENACE, MENACE], library: LIBRARY },
        p2: { hand: [FILLER] },
      });
      s.play(hand(s, REPLENISH));
      s.play(hand(s, REPLENISH));
      expect(glows(s)).toBe(false);
      s.play(hand(s, REPLENISH));
      expect(wardrumIn(s)).toBe("hand");
      // The opponent never sees the flag.
      expect(JSON.stringify(s.view("p2"))).not.toContain("conditionActive");
    });

    it("R386 the threshold reads through param(): an Upgrade makes it 2, and never below 2", () => {
      const s = scenario({
        p1: { hand: [WARDRUM, REPLENISH, REPLENISH, REPLENISH], library: LIBRARY },
        p2: { hand: [FILLER] },
      });
      stepParam(s.card(WARDRUM), "threshold", -1);
      expect(glows(s)).toBe(false);
      s.play(hand(s, REPLENISH));
      expect(glows(s)).toBe(true);
      s.play(hand(s, REPLENISH));
      expect(wardrumIn(s)).toBe("field");

      const floor = scenario({ p1: { hand: [WARDRUM, REPLENISH, REPLENISH], library: LIBRARY }, p2: { hand: [FILLER] } });
      stepParam(floor.card(WARDRUM), "threshold", -5);
      floor.play(hand(floor, REPLENISH));
      expect(wardrumIn(floor)).toBe("hand");
      floor.play(hand(floor, REPLENISH));
      expect(wardrumIn(floor)).toBe("field");
    });

    it("end of turn: casts a copy of the one Spell played this turn, by definition and face", () => {
      const s = scenario({
        p1: { hand: [{ def: BONE_STORM, radiant: true }, FILLER], field: [WARDRUM], library: LIBRARY },
        p2: { hand: [FILLER], field: [MENACE], library: LIBRARY },
      });
      s.play(BONE_STORM); // Radiant: Echo, 2 hits on each enemy.
      s.expectStats(MENACE, { health: 7 });
      s.endTurn(); // the copy: Radiant too, 2 more hits.
      s.expectStats(MENACE, { health: 5 });
      const copies = s.pile("p1", "graveyard").filter((card) => card.defId === BONE_STORM);
      expect(copies).toHaveLength(2);
      expect(copies.every((card) => card.radiant)).toBe(true);
    });

    it("end of turn: one random card among several, each possible", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 12; i += 1) {
        const s = scenario({
          seed: `wd-r${i}`,
          p1: { hand: [BONE_STORM, MANA_WELL, FILLER], field: [WARDRUM], library: LIBRARY, mana: 6 },
          p2: { hand: [FILLER], library: LIBRARY },
        });
        s.play(BONE_STORM);
        s.play(MANA_WELL);
        const from = s.events.length;
        s.endTurn();
        const casts = s.events.slice(from).filter((event) => event.type === "cardAnnounced" && event.player === "p1");
        expect(casts).toHaveLength(1);
        const cast = casts[0];
        if (cast?.type === "cardAnnounced") seen.add(cast.defId);
      }
      expect([...seen].sort()).toEqual([MANA_WELL, BONE_STORM].sort());
    });

    it("none played: nothing; a Unit played is not a candidate", () => {
      const s = scenario({
        p1: { hand: [VANILLA, FILLER], field: [WARDRUM], library: LIBRARY },
        p2: { hand: [FILLER], library: LIBRARY },
      });
      s.play(VANILLA);
      const from = s.events.length;
      s.endTurn();
      expect(s.events.slice(from).some((event) => event.type === "cardAnnounced" && event.player === "p1")).toBe(false);
    });

    it("R33 R97 a Trap copy is set face-down, hidden from the opponent", () => {
      const s = scenario({
        p1: { hand: [SHEEPISH, FILLER], field: [WARDRUM], library: LIBRARY },
        p2: { hand: [FILLER], library: LIBRARY },
      });
      s.play(SHEEPISH, { zone: 1 });
      s.endTurn();
      const traps = [1, 2, 3, 4, 5].filter((lane) => s.backrow("p1", lane)?.defId === SHEEPISH);
      expect(traps).toHaveLength(2);
      for (const lane of traps) expect(s.backrow("p1", lane)?.faceUp === true).toBe(false);
      const theirs = s.view("p2");
      for (const lane of traps) expect(theirs.opponent.backrow[lane - 1]).toMatchObject({ faceDown: true });
      expect(JSON.stringify(theirs)).not.toContain(SHEEPISH);
    });

    it("a Trap copy with no open backrow zone fizzles to the graveyard", () => {
      const s = scenario({
        p1: {
          hand: [SHEEPISH, FILLER],
          field: [WARDRUM],
          backrow: [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
          library: LIBRARY,
        },
        p2: { hand: [FILLER], library: LIBRARY },
      });
      s.play(SHEEPISH, { zone: 5 });
      s.endTurn();
      const inGraveyard = s.pile("p1", "graveyard").filter((card) => card.defId === SHEEPISH);
      expect(inGraveyard).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("casts a copy of each Spell, Field Spell and Trap played this turn, in play order", () => {
      const s = scenario({
        p1: { hand: [BONE_STORM, MANA_WELL, SHEEPISH, VANILLA, FILLER], field: [{ def: WARDRUM, radiant: true }], library: LIBRARY, mana: 8 },
        p2: { hand: [FILLER], field: [MENACE], library: LIBRARY },
      });
      s.play(BONE_STORM);
      s.play(MANA_WELL);
      s.play(SHEEPISH);
      s.play(VANILLA);
      const from = s.events.length;
      s.endTurn();
      const casts = s.events
        .slice(from)
        .flatMap((event) => (event.type === "cardAnnounced" && event.player === "p1" ? [event.defId] : []));
      expect(casts).toEqual([BONE_STORM, MANA_WELL, SHEEPISH]);
      s.expectStats(MENACE, { health: 7 });
    });

    it("a copy that needs a target asks its caster; the paused sequence survives JSON and goes on", () => {
      const s = scenario({
        p1: { hand: [LUNAR, BONE_STORM, FILLER], field: [{ def: WARDRUM, radiant: true }], library: LIBRARY, mana: 8 },
        p2: { hand: [FILLER], field: [MENACE], library: LIBRARY },
      });
      s.play(LUNAR, { targets: [{ pick: "hero", player: "p2" }] });
      s.play(BONE_STORM);
      s.endTurn();
      const pending = s.state.pending;
      expect(pending?.playerId).toBe("p1");
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(thawed).toEqual(s.state);
      const action: Action = {
        type: "answer",
        choiceId: pending?.id ?? "",
        selection: [{ pick: "hero", player: "p2" }],
        playerId: "p1",
        nonce: "wardrum-answer",
      } as Action;
      const live = reduce(s.state, action);
      const frozen = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      // Lunar Eclipse 3 twice, Bone Storm 1 twice: 8 in all on p2's hero.
      expect(live.state.players.p2.hero.health).toBe(30 - 8);
    });
  });
});
