// C #9 Income Tax — SPEC §8.6 row 9, BUILD M9 Classic row C 9: "Face-down (R33); fires when the
// opponent's second draw of a turn is complete, on either player's turn (on theirs the start-of-turn
// draw is the first), after any cast-on-draw card that draw found is cast (R58); a draw a draw limit
// stops does not happen and does not count (§2.4), so under a limit of 1 it never fires; the opponent
// keeps one hand card of their choice (their prompt, its options their own hand, none named in your
// view) and every other card moves to your hand as yours (its owner changes, R12); your hand cap burns
// the overflow into your graveyard, both players seeing which (R317); an opponent holding one card
// keeps it and you get nothing; the moved cards follow R97 once in your hand; radiant: the cards you get
// cost (1) less (`costMod`, kept in every zone, R78); its tuned numbers (trigger draw, never below 2;
// radiant discount) read through `param()` (R386)".
//
// Here p2 sets the trap and p1, the active player, is "the opponent" who draws.

import { describe, expect, it } from "vitest";
import { effectiveCost, reduce, stepParam, type CardInstance, type GameState, type PendingChoice } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/009-income-tax";

const TAX = "classic-009";
const PALANTIR = "classic-004"; // (1) Field Spell: "Aura: Your opponent can't draw more than 1 card each turn."
const STOCKPILE = "core-005"; // (1) Spell: "Draw 2. Heal your hero 2."
const JELLY_BEAN = "core-027"; // (1) Spell: "Cast on draw: Make a random card in your hand Radiant. Lose 5 health."
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const SEVEN = "core-025"; // (4) Unit 7/7
const FELINORS = "core-012"; // (2) Unit 3/4
const TIMMY = "core-011"; // (1) Unit 3/3

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the scenario has no ${what}`);
  return value;
}

function open(s: Scenario): PendingChoice {
  return must(s.state.pending, "open prompt");
}

function taxBoard(
  opts: {
    radiantFace?: boolean;
    p1Hand?: readonly string[];
    p1Library?: readonly string[];
    p2Hand?: readonly string[];
    p2Backrow?: readonly (string | { def: string; faceUp?: boolean; radiant?: boolean })[];
  } = {},
): Scenario {
  return scenario({
    p1: {
      hand: [...(opts.p1Hand ?? [STOCKPILE, MENACE])],
      library: [...(opts.p1Library ?? [FELINORS, SEVEN, TIMMY])],
    },
    p2: {
      hand: [...(opts.p2Hand ?? [VANILLA])],
      backrow: [{ def: TAX, faceUp: false, radiant: opts.radiantFace === true }, ...(opts.p2Backrow ?? [])],
      library: [VANILLA, VANILLA],
    },
  });
}

function fired(s: Scenario): boolean {
  return s.events.some((event) => event.type === "trapFired");
}

function handDefs(s: Scenario, player: "p1" | "p2"): string[] {
  return s.hand(player).map((card) => card.defId);
}

describe("C #9 Income Tax", () => {
  it("declares its two numbers (R386): trigger draw 2 (never below 2), Radiant discount 1", () => {
    expect(def.params).toEqual([
      { key: "draws", base: 2, radiant: 2, better: "down", step: 1, min: 2 },
      { key: "discount", base: 1, radiant: 1, better: "up", step: 1, min: 1 },
    ]);
    expect(base.triggers?.map((trigger) => trigger.on)).toEqual([["drawn"]]);
    expect(radiant.triggers?.map((trigger) => trigger.on)).toEqual([["drawn"]]);
  });

  describe("base", () => {
    it("R33 it sits face-down: the opponent reads a face-down card and nothing more", () => {
      const s = taxBoard();
      expect(s.view("p1").opponent.backrow[0]).toEqual({ faceDown: true, cost: 2 });
    });

    it("fires when the opponent's second draw of a turn is complete: they pick one card to keep", () => {
      const s = taxBoard();
      s.play(STOCKPILE);
      expect(fired(s)).toBe(true);
      const keep = open(s);
      expect(keep.playerId).toBe("p1");
      expect(keep.kind).toBe("hand");
      expect(keep.min).toBe(1);
      expect(keep.max).toBe(1);
      const offered = keep.options.map((option) => (option.selection.pick === "instance" ? s.card(option.selection.instanceId).defId : "?"));
      expect(offered).toEqual([MENACE, FELINORS, SEVEN]);
    });

    it("R12 the opponent keeps one card and every other card moves to your hand, as yours", () => {
      const s = taxBoard();
      s.play(STOCKPILE);
      const menace = s.card(MENACE);
      const felinors = s.hand("p1").find((card) => card.defId === FELINORS);
      const seven = s.hand("p1").find((card) => card.defId === SEVEN);
      s.answer(menace.id);
      expect(handDefs(s, "p1")).toEqual([MENACE]);
      expect(handDefs(s, "p2")).toEqual([VANILLA, FELINORS, SEVEN]);
      for (const card of [felinors, seven]) {
        const moved = s.card(must(card, "a moved card"));
        expect(moved.owner).toBe("p2");
        expect(moved.controller).toBe("p2");
      }
      s.expectInZone(TAX, "graveyard");
    });

    it("R521 one draw is not two: a single draw leaves it set", () => {
      const s = taxBoard({ p1Library: [FELINORS] });
      s.startTurn();
      expect(fired(s)).toBe(false);
      expect(s.backrow("p2", 1)?.defId).toBe(TAX);
    });

    it("on their own turn the start-of-turn draw is the first, so any extra draw sets it off", () => {
      const s = taxBoard({ p1Library: [TIMMY, FELINORS, SEVEN] });
      s.startTurn();
      expect(fired(s)).toBe(false);
      s.play(STOCKPILE);
      expect(fired(s)).toBe(true);
      expect(open(s).playerId).toBe("p1");
    });

    it("a count is per turn: a draw last turn does not add to this turn's", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, MENACE], library: [FELINORS, SEVEN, TIMMY, VANILLA] },
        p2: { hand: [VANILLA, VANILLA], backrow: [{ def: TAX, faceUp: false }], library: [VANILLA, VANILLA] },
        active: "p2",
      });
      s.endTurn(); // p1's turn starts: one draw (their first this turn)
      expect(s.state.active).toBe("p1");
      expect(fired(s)).toBe(false);
    });

    it("R58 a cast-on-draw card that draw finds is cast first, then the trap fires", () => {
      const s = taxBoard({ p1Library: [FELINORS, JELLY_BEAN, SEVEN] });
      s.play(STOCKPILE);
      const types = s.events.map((event) => event.type);
      const castAt = s.events.findIndex((event) => event.type === "cardPlayed" && event.defId === JELLY_BEAN);
      expect(castAt).toBeGreaterThan(-1);
      expect(castAt).toBeLessThan(types.indexOf("trapFired"));
      s.expectHealth("p1", 27); // 30 − 5 + 2
    });

    it("§2.4 a draw a limit stops does not happen and does not count: under Palantir's limit of 1 it never fires", () => {
      const s = taxBoard({ p2Backrow: [PALANTIR] });
      s.startTurn(); // the one draw p1 may make this turn
      s.play(STOCKPILE); // both draws stopped
      expect(fired(s)).toBe(false);
      expect(s.backrow("p2", 1)?.defId).toBe(TAX);
      expect(s.events.filter((event) => event.type === "drawLimited")).toHaveLength(2);
    });

    it("R177 the keep prompt's options are the opponent's own hand, and your view names none of them", () => {
      const s = taxBoard();
      s.play(STOCKPILE);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      const yours = JSON.stringify(s.view("p2"));
      for (const card of s.hand("p1")) {
        expect(yours).not.toContain(`"${card.id}"`);
        expect(yours).not.toContain(card.defId);
      }
    });

    it("R317 your hand cap burns the overflow into your graveyard, both players seeing which", () => {
      const nine = Array.from({ length: 9 }, () => VANILLA);
      const s = taxBoard({ p2Hand: nine });
      s.play(STOCKPILE);
      s.answer(s.card(MENACE).id);
      expect(s.hand("p2")).toHaveLength(10);
      const burned = s.events.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(1);
      const burnedCard = must(burned[0], "a burned card");
      if (burnedCard.type !== "burned") throw new Error("not a burn");
      expect(s.pile("p2", "graveyard").map((card) => card.id)).toContain(burnedCard.instanceId);
      expect(s.card(burnedCard.instanceId).owner).toBe("p2");
      for (const viewer of ["p1", "p2"] as const) {
        expect(s.view(viewer).events.filter((event) => event.type === "burned")).toEqual(burned);
      }
    });

    it("R521 a card burned on a full hand was drawn, and counts", () => {
      // p1 holds ten after the Stockpile leaves and draws one: the first draw fills the hand, the second burns.
      const eight = Array.from({ length: 8 }, () => MENACE);
      const s = taxBoard({ p1Hand: [STOCKPILE, TIMMY, ...eight], p1Library: [FELINORS, SEVEN] });
      s.play(STOCKPILE);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(1);
      expect(fired(s)).toBe(true);
      expect(open(s).playerId).toBe("p1");
    });

    it("R521 a card cast on draw was drawn, and counts", () => {
      const s = taxBoard({ p1Library: [JELLY_BEAN, FELINORS, SEVEN] });
      s.play(STOCKPILE);
      // The Jelly Bean was cast on the first draw; the Felinors is the second.
      expect(s.events.some((event) => event.type === "cardPlayed" && event.defId === JELLY_BEAN)).toBe(true);
      expect(fired(s)).toBe(true);
    });

    it("R521 a draw from an empty deck draws no card and fires nothing", () => {
      const s = taxBoard({ p1Library: [] });
      s.play(STOCKPILE);
      expect(s.events.filter((event) => event.type === "fatigue")).toHaveLength(2);
      expect(fired(s)).toBe(false);
      expect(s.backrow("p2", 1)?.defId).toBe(TAX);
    });

    it("an opponent holding one card keeps it and you get nothing: no prompt", () => {
      // The first draw is cast on draw (so it never reaches the hand), the second is the only card left.
      const s = taxBoard({ p1Hand: [STOCKPILE], p1Library: [JELLY_BEAN, FELINORS] });
      s.play(STOCKPILE);
      expect(fired(s)).toBe(true);
      expect(s.state.pending).toBeNull();
      expect(handDefs(s, "p1")).toEqual([FELINORS]);
      expect(handDefs(s, "p2")).toEqual([VANILLA]);
    });

    it("R97 once in your hand the moved cards are named in none of the opponent's view", () => {
      const s = taxBoard();
      s.play(STOCKPILE);
      const moved = [SEVEN, FELINORS].map((defId) => must(s.hand("p1").find((card) => card.defId === defId), defId));
      s.answer(s.card(MENACE).id);
      const theirs = JSON.stringify(s.view("p1"));
      for (const card of moved) {
        expect(theirs).not.toContain(`"${card.id}"`);
        expect(theirs).not.toContain(card.defId);
      }
    });

    it("§9.3 the keep prompt survives a JSON round trip and resumes through reduce", () => {
      const s = taxBoard();
      s.play(STOCKPILE);
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const pending = must(revived.pending, "revived prompt");
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: pending.id,
        selection: [{ pick: "instance", instanceId: s.card(SEVEN).id }],
        nonce: "income-tax-round-trip",
      });
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
    });

    it("the base face gives no discount", () => {
      const s = taxBoard();
      s.play(STOCKPILE);
      const felinors = must(s.hand("p1").find((card) => card.defId === FELINORS), "Felinors");
      s.answer(s.card(MENACE).id);
      expect(s.card(felinors).costMod).toBe(0);
    });

    it("R386 a Degrade of the trigger draw makes it the 3rd draw", () => {
      const s = taxBoard({ p1Library: [FELINORS, SEVEN, TIMMY, VANILLA] });
      stepParam(s.card(TAX), "draws", 1);
      s.startTurn(); // 1
      s.play(STOCKPILE); // 2, 3
      expect(fired(s)).toBe(true);
      const t = taxBoard({ p1Library: [FELINORS, SEVEN] });
      stepParam(t.card(TAX), "draws", 1);
      t.play(STOCKPILE); // 1, 2
      expect(fired(t)).toBe(false);
    });

    it("R386 never below 2: an Upgrade of the trigger draw leaves it at the 2nd", () => {
      const s = taxBoard();
      stepParam(s.card(TAX), "draws", -1);
      s.startTurn(); // 1 — not yet
      expect(fired(s)).toBe(false);
    });
  });

  describe("radiant", () => {
    it("the cards you get cost (1) less, and keep that in every zone (R78)", () => {
      const s = taxBoard({ radiantFace: true });
      s.play(STOCKPILE);
      const moved: CardInstance[] = [FELINORS, SEVEN].map((defId) => must(s.hand("p1").find((card) => card.defId === defId), defId));
      s.answer(s.card(MENACE).id);
      for (const card of moved) {
        const live = s.card(card);
        expect(live.costMod).toBe(-1);
        expect(live.owner).toBe("p2");
      }
      expect(effectiveCost(s.state, s.card(must(moved[0], "Felinors")))).toBe(1);
      expect(effectiveCost(s.state, s.card(must(moved[1], "7/7")))).toBe(3);
    });

    it("the kept card is not discounted", () => {
      const s = taxBoard({ radiantFace: true });
      s.play(STOCKPILE);
      s.answer(s.card(MENACE).id);
      expect(s.card(MENACE).costMod).toBe(0);
    });

    it("R386 an Upgrade of the discount makes them cost (2) less", () => {
      const s = taxBoard({ radiantFace: true });
      stepParam(s.card(TAX), "discount", 1);
      s.play(STOCKPILE);
      const seven = must(s.hand("p1").find((card) => card.defId === SEVEN), "7/7");
      s.answer(s.card(MENACE).id);
      expect(s.card(seven).costMod).toBe(-2);
    });

    it("the same condition: a single draw leaves it set", () => {
      const s = taxBoard({ radiantFace: true, p1Library: [FELINORS] });
      s.startTurn();
      expect(fired(s)).toBe(false);
    });
  });
});
