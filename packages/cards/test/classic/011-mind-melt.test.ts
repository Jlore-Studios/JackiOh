// C #11 Mind Melt — SPEC §8.6 row 11, BUILD M9 Classic row C 11: "Opens a prompt whose options are
// the opponent's hand cards, readable by you alone (§10.8), and exiles the one chosen; an empty hand
// opens no prompt; the opponent's view shows a prompt open for you and names none of its options
// (R177), then the exiled card (exile is public); once it closes your view names none of their
// remaining hand; the open prompt survives a JSON round trip; radiant: the options are their hand
// grouped by cost (each card's cost to play now, R65) and every card of the chosen cost is exiled;
// its tuned number (cards exiled) reads through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { reduce, stepParam, type GameState, type PendingChoice } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/011-mind-melt";

const MELT = "classic-011";
const FILLER = "core-005"; // (1) Spell, p1's spare card (§2.5).
// p2's hand: five distinct definitions no other pile holds, so a def id in a view names exactly one card.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const SEVEN = "core-025"; // (4) Unit 7/7
const FELINORS = "core-012"; // (2) Unit 3/4
const VANILLA = "core-008"; // (1) Unit 4/4
const REPLENISH = "core-010"; // (0) Spell
const BIGOT = "core-002"; // (2) Unit 6/1
const THEIR_HAND = [MENACE, SEVEN, FELINORS, VANILLA, REPLENISH] as const;

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the scenario has no ${what}`);
  return value;
}

function open(s: Scenario): PendingChoice {
  return must(s.state.pending, "open prompt");
}

function melt(radiantFace: boolean, theirHand: readonly (string | { def: string; costMod?: number })[] = THEIR_HAND): Scenario {
  return scenario({
    p1: { hand: [{ def: MELT, radiant: radiantFace }, FILLER] },
    p2: { hand: [...theirHand], library: [FILLER] },
  });
}

function handDefs(s: Scenario, player: "p1" | "p2"): string[] {
  return s.hand(player).map((card) => card.defId);
}

describe("C #11 Mind Melt", () => {
  it("declares its one number, cards exiled (R386)", () => {
    expect(def.params).toEqual([{ key: "cards", base: 1, radiant: 1, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("E17 opens a prompt for you whose options are the opponent's hand cards, one to pick", () => {
      const s = melt(false);
      s.play(MELT);
      const pending = open(s);
      expect(pending.playerId).toBe("p1");
      expect(pending.kind).toBe("hand");
      expect(pending.min).toBe(1);
      expect(pending.max).toBe(1);
      const offered = pending.options.map((option) =>
        option.selection.pick === "instance" ? s.card(option.selection.instanceId).defId : "?",
      );
      expect(offered).toEqual([...THEIR_HAND]);
      // You read them: your view of the prompt names each card.
      const mine = must(s.view("p1").pending, "p1's view of the prompt");
      expect(mine.forYou).toBe(true);
      expect(JSON.stringify(mine)).toContain("Midrange Menace");
    });

    it("exiles the card you choose from their hand, and only that one", () => {
      const s = melt(false);
      s.play(MELT);
      const menace = s.card(MENACE);
      s.answer(menace.id);
      s.expectInZone(menace, "exile");
      expect(handDefs(s, "p2")).toEqual([SEVEN, FELINORS, VANILLA, REPLENISH]);
      expect(s.state.pending).toBeNull();
      s.expectInZone(MELT, "graveyard");
      s.expectEvents("promptOpened", "promptAnswered", "exiled");
    });

    it("an empty hand opens no prompt, and the Spell still resolves", () => {
      const s = melt(false, []);
      s.play(MELT);
      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "promptOpened")).toBe(false);
      s.expectInZone(MELT, "graveyard");
    });

    it("R177 the opponent's view shows a prompt open for you and names none of its options", () => {
      const s = melt(false);
      s.play(MELT);
      const theirs = must(s.view("p2").pending, "p2's view of the prompt");
      expect(theirs).toEqual({ forYou: false, pendingFor: "p1" });
      // p2 knows their own hand, so the check is that the prompt itself carries no option.
      expect(JSON.stringify(theirs)).not.toContain("Menace");
    });

    it("exile is public: both players read the exiled card, and your view names none of their remaining hand", () => {
      const s = melt(false);
      s.play(MELT);
      const menace = s.card(MENACE);
      s.answer(menace.id);
      for (const viewer of ["p1", "p2"] as const) {
        const exiled = s.view(viewer).events.filter((event) => event.type === "exiled");
        expect(exiled).toEqual([{ type: "exiled", instanceId: menace.id, defId: MENACE, owner: "p2" }]);
      }
      const mine = JSON.stringify(s.view("p1"));
      for (const card of s.hand("p2")) {
        expect(mine).not.toContain(`"${card.id}"`);
        expect(mine).not.toContain(card.defId);
      }
    });

    it("§9.3 the open prompt survives a JSON round trip and resumes through reduce", () => {
      const s = melt(false);
      s.play(MELT);
      const paused = s.state;
      const revived = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(revived).toEqual(paused);
      const pending = must(revived.pending, "revived prompt");
      const seven = s.card(SEVEN);
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: pending.id,
        selection: [{ pick: "instance", instanceId: seven.id }],
        nonce: "mind-melt-round-trip",
      });
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(result.events.some((event) => event.type === "exiled" && event.instanceId === seven.id)).toBe(true);
    });

    it("R386 an Upgrade of cards exiled makes it two picks, both exiled", () => {
      const s = melt(false);
      stepParam(s.card(MELT), "cards", 1);
      s.play(MELT);
      const pending = open(s);
      expect(pending.min).toBe(2);
      expect(pending.max).toBe(2);
      const menace = s.card(MENACE);
      const vanilla = s.card(VANILLA);
      s.answer([menace.id, vanilla.id]);
      s.expectInZone(menace, "exile");
      s.expectInZone(vanilla, "exile");
      expect(handDefs(s, "p2")).toEqual([SEVEN, FELINORS, REPLENISH]);
    });

    it("R386 an Upgraded count larger than their hand picks the whole hand", () => {
      const s = melt(false, [MENACE]);
      stepParam(s.card(MELT), "cards", 2);
      s.play(MELT);
      expect(open(s).max).toBe(1);
      s.answer(s.card(MENACE).id);
      expect(s.hand("p2")).toEqual([]);
    });
  });

  describe("radiant", () => {
    it("R65 the options are their hand grouped by the cost each would be played for now", () => {
      // Duplicating Felinors (2) with costMod −1 costs (1) now, so it joins Mr. Vanilla's group.
      const s = melt(true, [MENACE, { def: FELINORS, costMod: -1 }, VANILLA, REPLENISH, BIGOT]);
      s.play(MELT);
      const pending = open(s);
      expect(pending.playerId).toBe("p1");
      expect(pending.options.map((option) => (option.selection.pick === "mode" ? option.selection.option : "?"))).toEqual([
        "0",
        "1",
        "2",
        "3",
      ]);
    });

    it("exiles every card of the chosen cost from their hand and leaves the rest", () => {
      const s = melt(true, [MENACE, { def: FELINORS, costMod: -1 }, VANILLA, REPLENISH, BIGOT]);
      s.play(MELT);
      const felinors = s.card(FELINORS);
      const vanilla = s.card(VANILLA);
      s.answer("1");
      s.expectInZone(felinors, "exile");
      s.expectInZone(vanilla, "exile");
      expect(handDefs(s, "p2")).toEqual([MENACE, REPLENISH, BIGOT]);
      s.expectInZone(MELT, "graveyard");
    });

    it("a cost only one card has exiles that card alone", () => {
      const s = melt(true);
      s.play(MELT);
      s.answer("4");
      s.expectInZone(SEVEN, "exile");
      expect(handDefs(s, "p2")).toEqual([MENACE, FELINORS, VANILLA, REPLENISH]);
    });

    it("an empty hand opens no prompt", () => {
      const s = melt(true, []);
      s.play(MELT);
      expect(s.state.pending).toBeNull();
      s.expectInZone(MELT, "graveyard");
    });

    it("R177 the opponent's view shows a prompt open for you and names none of its options", () => {
      const s = melt(true);
      s.play(MELT);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      // Your view reads the cards of each cost.
      expect(JSON.stringify(s.view("p1").pending)).toContain("Midrange Menace");
    });

    it("once it closes, your view names none of their remaining hand", () => {
      const s = melt(true);
      s.play(MELT);
      s.answer("3");
      const mine = JSON.stringify(s.view("p1"));
      for (const card of s.hand("p2")) {
        expect(mine).not.toContain(`"${card.id}"`);
        expect(mine).not.toContain(card.defId);
      }
      expect(mine).toContain(MENACE);
    });

    it("§9.3 the open prompt survives a JSON round trip and resumes through reduce", () => {
      const s = melt(true);
      s.play(MELT);
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const pending = must(revived.pending, "revived prompt");
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: pending.id,
        selection: [{ pick: "mode", option: "2" }],
        nonce: "mind-melt-radiant-round-trip",
      });
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      const felinors = s.card(FELINORS);
      expect(result.events.some((event) => event.type === "exiled" && event.instanceId === felinors.id)).toBe(true);
    });
  });
});
