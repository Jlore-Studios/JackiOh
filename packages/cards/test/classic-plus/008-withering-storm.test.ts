// C+ #8 Withering Storm — SPEC §8.7 row 8, BUILD M9 Classic+ row C+ 8: "Degrades 4 different random
// cards in the opponent's deck (R60), drawn among the cards a Degrade can change (an Immutable card, or
// one no change reaches, is never picked), one draw from R386's menu each, then you draw 1; a deck with
// 3 or fewer such cards degrades them all, one with none degrades nothing and draws no random number
// (R129), and you still draw; the changes are `tuning` and leave the deck with the card; `degraded`
// events stay unread by both players while the cards are in the deck (R177, R311) and the opponent's
// library list does not change; card count and draw read through `param()`; radiant degrades every card
// in the opponent's deck once".
//
// R569 (with R440): the pick is drawn among the cards a Degrade can change, and the count of `degraded`
// cues never says how many of the deck's cards could — it is padded with `none` cues on cards no change
// reaches, up to {cards} (or the deck's size).

import { reduce, stepParam, type CardInstance, type GameState } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type PileSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/008-withering-storm";

const STORM = "classicplus-008";
const VANILLA = "core-008"; // (1) 4/4: a Degrade can change its cost or its stats.
const IMMUTABLE = { def: "core-019", radiant: true } as const; // Radiant Midrange Menace: Immutable.
const NETHER = "core-088"; // (4) Spell, no keywords, no numbers: no Degrade reaches it.
const HINDER = "core-021"; // Cast on draw: … Discard 1 (a hand prompt during the draw, R431).
const FILLER = "core-010";
const STOCKPILE = "core-005";

type Degraded = Extract<GameEvent, { type: "degraded" }>;

function setup(deck: readonly PileSetup[], radiantFace = false, seed?: string, p1Library: readonly PileSetup[] = [STOCKPILE, STOCKPILE]): Scenario {
  return scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: { hand: [{ def: STORM, radiant: radiantFace }, FILLER], library: p1Library },
    p2: { hand: [FILLER], library: deck },
  });
}

function cues(events: readonly GameEvent[]): Degraded[] {
  return events.filter((event): event is Degraded => event.type === "degraded");
}

function changed(events: readonly GameEvent[]): Degraded[] {
  return cues(events).filter((event) => event.change.kind !== "none");
}

/** Whether a Degrade left anything on the card (its cost or its tuning). */
function wasChanged(card: CardInstance): boolean {
  return card.costMod !== 0 || card.tuning !== undefined;
}

describe("C+ #8 Withering Storm", () => {
  it("is a (2) Spell declaring cards 4 and draw 1 on both faces", () => {
    expect([def.type, def.cost]).toEqual(["Spell", 2]);
    expect(def.params?.map((p) => [p.key, p.base, p.radiant])).toEqual([
      ["cards", 4, 4],
      ["draw", 1, 1],
    ]);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R60 R386 degrades 4 different cards of the opponent's deck, one change each, then you draw 1", () => {
      const s = setup(Array.from({ length: 8 }, () => VANILLA));
      const hand = s.hand("p1").length;

      s.play(STORM);

      const done = changed(s.events);
      expect(done).toHaveLength(4);
      expect(cues(s.events)).toHaveLength(4);
      expect(new Set(done.map((event) => event.instanceId)).size).toBe(4);
      const deck = s.pile("p2", "library");
      expect(deck.filter(wasChanged).map((card) => card.id).sort()).toEqual(done.map((event) => event.instanceId).sort());
      expect(s.hand("p1")).toHaveLength(hand - 1 + 1);
      s.expectEvents("degraded", "drawn");
    });

    it("R569 R60 picks only among cards a Degrade can change: Immutable and unreachable cards are never picked", () => {
      for (let seed = 1; seed <= 10; seed += 1) {
        const s = setup([IMMUTABLE, VANILLA, NETHER, VANILLA, VANILLA, IMMUTABLE, VANILLA, NETHER, VANILLA, VANILLA], false, `storm-${seed}`);

        s.play(STORM);

        const deck = s.pile("p2", "library");
        expect(deck.filter((card) => card.defId !== VANILLA).some(wasChanged)).toBe(false);
        expect(deck.filter(wasChanged)).toHaveLength(4);
        expect(cues(s.events)).toHaveLength(4);
      }
    });

    it("R569 R440 a deck with 3 changeable cards degrades all 3, and a `none` cue keeps the count at 4", () => {
      const s = setup([IMMUTABLE, VANILLA, NETHER, VANILLA, VANILLA]);

      s.play(STORM);

      expect(changed(s.events)).toHaveLength(3);
      expect(cues(s.events)).toHaveLength(4);
      expect(s.pile("p2", "library").filter((card) => card.defId === VANILLA).every(wasChanged)).toBe(true);
    });

    it("R569 R129 a deck no Degrade can change is left alone with no random number drawn, and you still draw", () => {
      const s = setup([IMMUTABLE, NETHER, IMMUTABLE]);
      const cursor = s.state.rngCursor;
      const hand = s.hand("p1").length;

      s.play(STORM);

      expect(changed(s.events)).toHaveLength(0);
      expect(cues(s.events)).toHaveLength(3); // R440: one cue per card it reached, `none` on each
      expect(s.pile("p2", "library").some(wasChanged)).toBe(false);
      expect(s.state.rngCursor).toBe(cursor);
      expect(s.hand("p1")).toHaveLength(hand);
    });

    it("an empty deck: nothing to degrade, and you still draw", () => {
      const s = setup([]);
      const hand = s.hand("p1").length;

      s.play(STORM);

      expect(cues(s.events)).toHaveLength(0);
      expect(s.hand("p1")).toHaveLength(hand);
    });

    it("R386 the changes are `tuning`: they leave the deck with the card, which shows them once its owner draws it", () => {
      const s = setup([VANILLA, VANILLA, VANILLA, VANILLA]);
      s.play(STORM);
      const top = s.pile("p2", "library")[0]!;
      expect(wasChanged(top)).toBe(true);

      s.endTurn(); // p2's start of turn draws the top card

      const drawn = s.card(top.id);
      expect(drawn.zone.z).toBe("hand");
      expect(wasChanged(drawn)).toBe(true);
      const shown = s.view("p2").you.hand;
      if (!Array.isArray(shown)) throw new Error("p2 reads its own hand");
      const card = shown.find((each) => each.instanceId === top.id);
      expect([card?.cost, card?.attack, card?.health]).not.toEqual([1, 4, 4]);
    });

    it("R177 R311 the `degraded` cues stay unread by both players, and the opponent's library list does not change", () => {
      const s = setup([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]);
      const listBefore = JSON.stringify(s.view("p2").you.ownLibrary);

      s.play(STORM);

      for (const viewer of ["p1", "p2"] as const) {
        const seen = cues(s.view(viewer).events);
        expect(seen).toHaveLength(4);
        for (const cue of seen) expect([cue.instanceId, cue.defId, cue.change]).toEqual(["hidden", "hidden", expect.anything()]);
        expect(JSON.stringify(seen)).not.toContain(VANILLA);
      }
      expect(JSON.stringify(s.view("p2").you.ownLibrary)).toBe(listBefore);
    });

    it("R113 a prompt the draw opens survives a JSON round trip, and the Degrades are not made again", () => {
      const s = setup([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], false, undefined, [HINDER, STOCKPILE]);

      s.play(STORM);

      expect(s.state.pending?.kind).toBe("hand");
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      const pick = revived.pending?.options[0]?.selection;
      if (pick === undefined) throw new Error("no option to discard");
      const result = reduce(revived, { type: "answer", playerId: "p1", choiceId: revived.pending?.id ?? "", selection: [pick], nonce: "rt-008" });

      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(cues(result.events)).toHaveLength(0);
      expect(result.state.players.p2.library.filter(wasChanged)).toHaveLength(4);
    });

    it("R386 card count and draw read through param(): an Upgrade of each moves what resolves", () => {
      const s = setup(Array.from({ length: 8 }, () => VANILLA));
      stepParam(s.card(STORM), "cards", 1);
      s.play(STORM);
      expect(changed(s.events)).toHaveLength(5);

      const t = setup([VANILLA], false, undefined, [STOCKPILE, STOCKPILE, STOCKPILE]);
      const hand = t.hand("p1").length;
      stepParam(t.card(STORM), "draw", 1);
      t.play(STORM);
      expect(t.hand("p1")).toHaveLength(hand - 1 + 2);
    });
  });

  describe("radiant", () => {
    it("degrades every card of the opponent's deck once, leaving Immutable and unreachable cards alone", () => {
      const s = setup([VANILLA, IMMUTABLE, VANILLA, NETHER, VANILLA, VANILLA, VANILLA, VANILLA], true);
      const hand = s.hand("p1").length;

      s.play(STORM);

      const deck = s.pile("p2", "library");
      expect(deck.filter((card) => card.defId === VANILLA).every(wasChanged)).toBe(true);
      expect(deck.filter((card) => card.defId !== VANILLA).some(wasChanged)).toBe(false);
      expect(changed(s.events)).toHaveLength(6);
      expect(cues(s.events)).toHaveLength(8);
      expect(s.hand("p1")).toHaveLength(hand);
    });
  });
});
