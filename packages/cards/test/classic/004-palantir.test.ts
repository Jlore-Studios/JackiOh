// C #4 Palantir — SPEC §8.6 row 4, BUILD M9 Classic row C 4: "Aura: the opponent's draws beyond 1 in a
// turn, on either player's turn and the start-of-turn draw included, do not happen at all: no card
// moves, no fatigue, nothing is cast on draw (§2.4); a draw made earlier that turn counts; with another
// limit the lowest holds; lifted when Palantir leaves; the opponent playing a Book (a Spell tagged
// Book, a cast included, R70) opens the announce window (§10.5), where the base face steals with no
// "you may" (balance patch 1, mandatory): Palantir is Tributed and the Book is countered (never
// played: no spell script, not counted as played, its mana spent) and moves to your hand as your
// card (its owner changes, R12), burned into your graveyard at a full hand (R317); there is no pass,
// so no prompt opens; a non-Book Spell opens no prompt either; every event naming the stolen Book
// follows R97, judged where it sits now; the resolved steal survives a JSON round trip and replays;
// radiant: every opponent Spell is countered and stolen with no prompt, its own cost (R65 out of
// play: an X Spell 0) added to `memory.stolenCost`, and Palantir Tributes itself once that total
// reaches 2, not before; its tuned numbers (draw limit, never below 1; radiant threshold) read
// through `param()` (R386)".
//
// Palantir sits face-up in p1's backrow; p2, the opponent, is active and plays or draws. The Book is
// C #24 Book of Knowledge ("Draw 3"), whose resolution shows in p2's draws.

import { describe, expect, it } from "vitest";
import { hashState, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/004-palantir";

const PALANTIR = "classic-004";
const BOOK = "classic-024"; // (1) Spell, Book: Draw 3.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const REPLENISH = "core-010"; // (0) Spell.
const DIVIDEND = "core-024"; // (X) Spell.
const HINDER = "core-021"; // (0) Spell, cast on draw.
const VANILLA = "core-008";
const TIMMY = "core-011";
const RECYCLE = "core-039"; // (0) Spell: Exile this.
const TWINSPELL = "core-079"; // (2) Field Spell.
const CALL = "core-069"; // (2) Spell: Recruit 3.
const COUNTERSPELL = "classic-017"; // Trap: counters the opponent's Spell.
const AUCTIONEER = "classic-038"; // Field Trap: from its activation on, each play draws its controller 1.

const DECK = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] as const;

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [TIMMY], backrow: [{ def: PALANTIR, radiant: radiantFace, faceUp: true, lane: 1 }], library: [VANILLA, VANILLA], ...p1 },
    p2: { hand: [BOOK, STOCKPILE, VANILLA], library: [...DECK], ...p2 },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

function drawsBy(events: readonly GameEvent[], player: "p1" | "p2"): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

describe("C #4 Palantir", () => {
  it("both faces set a draw limit on the opponent", () => {
    expect(def.id).toBe(PALANTIR);
    expect(base.drawLimit).toBeTypeOf("function");
    expect(radiant.drawLimit).toBeTypeOf("function");
  });

  describe("the Aura (both faces)", () => {
    it("§2.4 the opponent's draws beyond 1 in a turn do not happen at all", () => {
      const s = setup();

      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(1);
      expect(s.pile("p2", "library")).toHaveLength(DECK.length - 1);
      expect(count(s.lastEvents, "drawLimited")).toBe(1);
      expect(count(s.lastEvents, "fatigue")).toBe(0);
    });

    it("§2.4 the start-of-turn draw counts: after it, every other draw that turn is stopped", () => {
      const s = scenario({
        p1: { hand: [TIMMY], backrow: [{ def: PALANTIR, faceUp: true }], library: [VANILLA, VANILLA] },
        p2: { hand: [STOCKPILE, VANILLA], library: [...DECK] },
      });

      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(drawsBy(s.lastEvents, "p2")).toBe(1);
      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(0);
      expect(count(s.lastEvents, "drawLimited")).toBe(2);
    });

    it("§2.4 a stopped draw casts nothing: a cast-on-draw card stays in the deck", () => {
      const s = setup({}, { hand: [STOCKPILE, VANILLA], library: [VANILLA, { def: HINDER, radiant: true }, VANILLA] });
      const hinder = s.card(HINDER);

      s.play(STOCKPILE);

      s.expectInZone(hinder, "library");
      expect(s.state.players.p1.mana.nextTurnMod).toBe(0);
    });

    it("§2.4 a stopped draw from an empty deck deals no fatigue", () => {
      const s = setup({}, { library: [VANILLA] });

      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(1);
      expect(count(s.lastEvents, "fatigue")).toBe(0);
      s.expectHealth("p2", 32);
    });

    it("§2.4 it holds on its controller's turn too: the opponent's off-turn draw counts, and a second is stopped", () => {
      // p2's C #38 Jackiestan Auctioneer activates on p1's 3rd play; from then on each play makes p2
      // draw 1, on p1's own turn. The first such draw happens, the second is past the limit.
      const s = scenario({
        p1: { hand: [RECYCLE, RECYCLE, RECYCLE, RECYCLE, RECYCLE, TIMMY], backrow: [{ def: PALANTIR, faceUp: true, lane: 1 }], library: [VANILLA] },
        p2: { hand: [VANILLA], backrow: [{ def: AUCTIONEER, faceUp: false, lane: 2 }], library: [VANILLA, VANILLA, VANILLA] },
      });
      const recycles = (): string[] => s.hand("p1").filter((card) => card.defId === RECYCLE).map((card) => card.id);

      for (const id of recycles().slice(0, 4)) s.play(id);
      expect(drawsBy(s.events, "p2")).toBe(1);

      s.play(recycles()[0] ?? RECYCLE);

      expect(drawsBy(s.events, "p2")).toBe(1);
      expect(count(s.lastEvents, "drawLimited")).toBe(1);
      expect(s.pile("p2", "library")).toHaveLength(2);
    });

    it("its controller's own draws are not limited", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, TIMMY], backrow: [{ def: PALANTIR, faceUp: true }], library: [VANILLA, VANILLA, VANILLA] },
        p2: { hand: [VANILLA] },
      });

      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });

    it("§2.4 with another limit the lowest holds", () => {
      const s = setup({ backrow: [{ def: PALANTIR, faceUp: true, lane: 1 }, { def: PALANTIR, faceUp: true, lane: 2 }] });
      // A Degrade moves a limit toward more draws: this one allows 2, the other still 1.
      const lenient = s.backrow("p1", 2);
      if (lenient === null) throw new Error("fixture");
      stepParam(lenient, "drawLimit", 1);

      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(1);
    });

    it("R386 a Degrade of the limit lets the opponent draw 2 a turn", () => {
      const s = setup();
      stepParam(s.card(PALANTIR), "drawLimit", 1);

      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(2);
    });

    it("R386 an Upgrade never takes the limit below 1", () => {
      const s = setup();
      stepParam(s.card(PALANTIR), "drawLimit", -1);

      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(1);
    });

    it("the limit is lifted when Palantir leaves the field", () => {
      const s = setup({}, {}, false);
      // p2 plays the Book and p1 takes it, Tributing Palantir: the limit goes with it.
      s.play(BOOK);
      s.play(STOCKPILE);

      expect(drawsBy(s.lastEvents, "p2")).toBe(2);
    });
  });

  describe("base: the Book steal", () => {
    it("R448 the opponent's Book opens no prompt: the steal resolves during the opponent's turn", () => {
      const s = setup();

      s.play(BOOK);

      expect(s.state.pending).toBeNull();
      expect(count(s.events, "promptOpened")).toBe(0);
      s.expectEvents("cardAnnounced", "countered");
      expect(count(s.events, "cardPlayed")).toBe(0);
    });

    it("R12 Palantir is Tributed and the Book is countered into your hand as your card", () => {
      const s = setup();
      const book = s.card(BOOK);
      const palantir = s.card(PALANTIR);

      s.play(book);

      s.expectInZone(palantir, "graveyard");
      const taken = s.card(book.id);
      expect(taken.zone).toMatchObject({ z: "hand", player: "p1" });
      expect(taken.owner).toBe("p1");
      // Never played: its script did not run, it is not counted, and its mana stays spent.
      expect(drawsBy(s.events, "p2")).toBe(0);
      expect(count(s.events, "cardPlayed")).toBe(0);
      expect(s.state.players.p2.turnLog.cardsPlayed).toBe(0);
      s.expectMana("p2", 3);
      s.expectEvents("countered");
    });

    it("R317 with a full hand: the stolen Book burns into your graveyard", () => {
      const ten = Array.from({ length: 10 }, () => VANILLA);
      const s = setup({ hand: ten });
      const book = s.card(BOOK);

      s.play(book);

      expect(s.hand("p1")).toHaveLength(10);
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toContain(book.id);
      expect(s.card(book.id).owner).toBe("p1");
    });

    it("no 'you may': there is no pass, so the Book never resolves while Palantir stands", () => {
      const s = setup();
      const palantir = s.card(PALANTIR);
      const book = s.card(BOOK);

      s.play(BOOK);

      s.expectInZone(palantir, "graveyard");
      expect(count(s.events, "cardPlayed")).toBe(0);
      const taken = s.card(book.id);
      expect(taken.zone).toMatchObject({ z: "hand", player: "p1" });
    });

    it("a non-Book Spell opens no prompt", () => {
      const s = setup();

      s.play(STOCKPILE);

      expect(s.state.pending).toBeNull();
      expect(count(s.events, "promptOpened")).toBe(0);
    });

    it("R97 every event naming the stolen Book is judged where it sits now: your hand, hidden from the opponent", () => {
      const s = setup();
      const book = s.card(BOOK);

      s.play(book);

      // The announce and the counter named the Book; now it is in p1's hand, so p2's view names it
      // nowhere, while p1 reads it. The `stolen` event is left out: the engine shows it to whoever
      // could read the card where it was taken from (the resolving zone, public), which is its own
      // reading of hidden information for steals and differs from this row's "judged where it sits
      // now" (reported to the lead).
      const named = (text: string): boolean => text.includes(`"${book.id}"`) || text.includes(`"${BOOK}"`);
      expect(s.events.filter((event) => event.type !== "stolen").some((event) => named(JSON.stringify(event)))).toBe(true);
      const { events, ...board } = s.view("p2");
      expect(named(JSON.stringify(board))).toBe(false);
      expect(events.filter((event) => event.type !== "stolen").some((event) => named(JSON.stringify(event)))).toBe(false);
      expect(named(JSON.stringify(s.view("p1").you.hand))).toBe(true);
    });

    it("§9.3 the stolen play replays from the log: the same actions on the starting state reach the same state", () => {
      const s = setup();
      const start = JSON.parse(JSON.stringify(s.state)) as GameState;
      const book = s.card(BOOK);

      s.play(book);

      const played = reduce(start, { type: "play", playerId: "p2", instanceId: book.id, nonce: "replay-1" });
      expect(played.error).toBeUndefined();
      expect(played.state.pending).toBeNull();
      expect(hashState(played.state)).toBe(hashState(s.state));
    });

    it("§9.3 the resolved steal survives a JSON round trip with the Book in your hand", () => {
      const s = setup();
      const book = s.card(BOOK);
      s.play(book);
      const done = s.state;
      const revived = JSON.parse(JSON.stringify(done)) as GameState;
      expect(revived).toEqual(done);
      expect(revived.pending).toBeNull();

      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: "",
        selection: [],
        nonce: "palantir-round-trip",
      });

      expect(result.error).toBe("no prompt is open");
      expect(result.state.players.p1.hand.map((card) => card.id)).toContain(book.id);
    });
  });

  describe("radiant: every Spell", () => {
    it("steals every opponent Spell with no prompt", () => {
      const s = setup({}, {}, true);
      const spell = s.card(STOCKPILE);

      s.play(spell);

      expect(s.state.pending).toBeNull();
      expect(s.card(spell.id).zone).toMatchObject({ z: "hand", player: "p1" });
      expect(s.card(spell.id).owner).toBe("p1");
      expect(drawsBy(s.events, "p2")).toBe(0);
    });

    it("counts what it stole and Tributes itself once the total reaches 2, not before", () => {
      const s = setup({}, { hand: [STOCKPILE, BOOK, VANILLA] }, true);
      const palantir = s.card(PALANTIR);

      s.play(STOCKPILE);
      s.expectInZone(palantir, "field");
      expect(s.card(palantir).memory.stolenCost).toBe(1);

      s.play(BOOK);
      s.expectInZone(palantir, "graveyard");
    });

    it("R65 R396 a (0) Cost Spell and an X Spell add nothing to the total", () => {
      const s = setup({}, { hand: [REPLENISH, DIVIDEND, VANILLA] }, true);
      const palantir = s.card(PALANTIR);

      s.play(REPLENISH);
      s.play(DIVIDEND, { x: 1, modes: ["mana"] });

      s.expectInZone(palantir, "field");
      expect(s.card(palantir).memory.stolenCost ?? 0).toBe(0);
    });

    it("R386 an Upgrade of its threshold lets it steal (3) before it goes", () => {
      const s = setup({}, { hand: [STOCKPILE, BOOK, VANILLA] }, true);
      const palantir = s.card(PALANTIR);
      stepParam(palantir, "threshold", 1);

      s.play(STOCKPILE);
      s.play(BOOK);

      s.expectInZone(palantir, "field");
      expect(s.card(palantir).memory.stolenCost).toBe(2);
    });

    it("only the Spell type: a Field Spell and a Unit are played as usual and add nothing", () => {
      const s = setup({}, { hand: [TWINSPELL, VANILLA, STOCKPILE] }, true);
      const palantir = s.card(PALANTIR);

      s.play(TWINSPELL, { zone: 2 });
      s.play(VANILLA, { zone: 1 });

      s.expectInZone(TWINSPELL, "field");
      s.expectInZone(VANILLA, "field");
      expect(count(s.events, "countered")).toBe(0);
      expect(s.card(palantir).memory.stolenCost).toBeUndefined();
    });

    it("R70 a Spell cast by an effect is stolen too: a cast-on-draw card goes to your hand as yours", () => {
      const s = scenario({
        p1: { hand: [TIMMY], backrow: [{ def: PALANTIR, radiant: true, faceUp: true }], library: [VANILLA, VANILLA] },
        p2: { hand: [VANILLA], library: [{ def: HINDER, radiant: true }, VANILLA] },
      });
      const hinder = s.card(HINDER);

      s.endTurn();

      expect(s.state.active).toBe("p2");
      expect(s.card(hinder).zone).toMatchObject({ z: "hand", player: "p1" });
      expect(s.card(hinder).owner).toBe("p1");
      expect(count(s.events, "cardResolved")).toBe(0);
      expect(s.state.players.p1.mana.nextTurnMod).toBe(0);
    });

    it("R448 a Spell a trap has already countered is not stolen and adds nothing to the total", () => {
      const s = setup(
        { backrow: [{ def: PALANTIR, radiant: true, faceUp: true, lane: 1 }, { def: COUNTERSPELL, faceUp: false, lane: 2 }] },
        { hand: [CALL, VANILLA] },
      );
      const palantir = s.card(PALANTIR);
      const call = s.card(CALL);

      s.play(call);

      expect(s.card(call).zone).toMatchObject({ z: "graveyard", player: "p2" });
      expect(s.card(call).owner).toBe("p2");
      s.expectInZone(palantir, "field");
      expect(s.card(palantir).memory.stolenCost).toBeUndefined();
    });

    it("R386 a Degrade of its threshold makes it Tribute itself after (1)", () => {
      const s = setup({}, {}, true);
      const palantir = s.card(PALANTIR);
      stepParam(palantir, "threshold", -1);

      s.play(STOCKPILE);

      s.expectInZone(palantir, "graveyard");
      expect(s.card(STOCKPILE).zone).toMatchObject({ z: "hand", player: "p1" });
    });

    it("its own controller's Spells are never stolen", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, TIMMY], backrow: [{ def: PALANTIR, radiant: true, faceUp: true }], library: [VANILLA, VANILLA] },
        p2: { hand: [VANILLA] },
      });

      s.play(STOCKPILE);

      expect(count(s.events, "countered")).toBe(0);
      expect(drawsBy(s.events, "p1")).toBe(2);
    });
  });
});
