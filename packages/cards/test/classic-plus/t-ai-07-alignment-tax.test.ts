// T-AI-7 Alignment Tax — SPEC §8.7 row T-AI-7, BUILD M9 Classic+ row T-AI-7: "A player modifier on the
// opponent: their cards cost (1) more during their next turn only, inert for the rest of this turn,
// active through their next turn and gone at its cleanup (R48's timing turned outward, R363); X-cost
// cards are untouched (R65); two Taxes add; a price the tax lifts past their mana drops the play from
// `legalActions`; radiant (2) more".

import { legalActions } from "@jackioh/engine";
import type { CardView, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-07-alignment-tax";

const TAX = "classicplus-t-ai-07";
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const FELINOR = "core-043"; // (4) Spell
const ADAPTIVE_UI = "core-074"; // (X) Spell
const STOCKPILE = "core-005"; // (1) Spell

/** p2's hand holds one card of each price; both decks are deep enough to cross several turns. */
function setup(taxes: readonly (string | { def: string; radiant: boolean })[] = [TAX], p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [...taxes, STOCKPILE], library: [VANILLA, VANILLA, VANILLA, VANILLA], mana: 8 },
    p2: { hand: [VANILLA, MENACE, FELINOR, ADAPTIVE_UI], library: [VANILLA, VANILLA, VANILLA, VANILLA], ...p2 },
  });
}

/** What `player` reads as the price of their own hand card of `defId` (the client prints this). */
function priceOf(s: Scenario, player: PlayerId, defId: string): number {
  const hand = s.view(player).you.hand as CardView[];
  const card = s.hand(player).find((c) => c.defId === defId);
  const shown = hand.find((c) => c.instanceId === card?.id);
  if (shown === undefined) throw new Error(`${player} holds no ${defId}`);
  return shown.cost;
}

function playable(s: Scenario, player: PlayerId, defId: string): boolean {
  const ids = s.hand(player).filter((c) => c.defId === defId).map((c) => c.id);
  return legalActions(s.state, player).some((action) => action.type === "play" && ids.includes(action.instanceId));
}

describe("T-AI-7 Alignment Tax", () => {
  it("is a (1) AI Spell token with no params", () => {
    expect(def.cost).toBe(1);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["AI", "Token"]);
    expect(def.params).toBeUndefined();
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R48 inert for the rest of this turn: the opponent's prices don't move yet", () => {
      const s = setup().play(TAX);
      expect(priceOf(s, "p2", VANILLA)).toBe(1);
      expect(priceOf(s, "p2", MENACE)).toBe(3);
      expect(s.state.players.p2.mods.some((mod) => mod.kind === "costRule")).toBe(true);
    });

    it("R48 R363 their cards cost (1) more during their next turn", () => {
      const s = setup().play(TAX).endTurn();
      expect(s.state.active).toBe("p2");
      expect(priceOf(s, "p2", VANILLA)).toBe(2);
      expect(priceOf(s, "p2", MENACE)).toBe(4);
      expect(priceOf(s, "p2", FELINOR)).toBe(5);
      s.play(VANILLA, { zone: 1 });
      // 4 mana, the taxed Vanilla paid 2.
      s.expectMana("p2", 2);
    });

    it("R48 gone at the cleanup of their next turn: back to printed prices from then on", () => {
      const s = setup().play(TAX).endTurn().endTurn();
      expect(s.state.active).toBe("p1");
      expect(priceOf(s, "p2", VANILLA)).toBe(1);
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(priceOf(s, "p2", VANILLA)).toBe(1);
      expect(s.state.players.p2.mods.some((mod) => mod.kind === "costRule")).toBe(false);
    });

    it("taxes only the opponent: your own cards keep their prices on both turns", () => {
      const s = setup().play(TAX);
      expect(priceOf(s, "p1", STOCKPILE)).toBe(1);
      s.endTurn().endTurn();
      expect(priceOf(s, "p1", STOCKPILE)).toBe(1);
    });

    it("R65 an X-cost card is untouched", () => {
      const s = setup().play(TAX).endTurn();
      expect(priceOf(s, "p2", ADAPTIVE_UI)).toBe(0);
      s.play(ADAPTIVE_UI, { x: 2, targets: [{ pick: "hero", player: "p1" }] });
      s.expectMana("p2", 2);
    });

    it("two Taxes add: (2) more", () => {
      const s = setup([TAX, TAX]).play(TAX).play(TAX).endTurn();
      expect(priceOf(s, "p2", VANILLA)).toBe(3);
      expect(priceOf(s, "p2", MENACE)).toBe(5);
    });

    it("a price the tax lifts past their mana drops the play from legalActions", () => {
      const s = setup();
      s.endTurn();
      expect(playable(s, "p2", FELINOR)).toBe(true);
      s.endTurn();
      s.play(TAX).endTurn();
      expect(s.state.players.p2.mana.current).toBe(4);
      expect(playable(s, "p2", FELINOR)).toBe(false);
      expect(() => s.play(FELINOR)).toThrow();
      expect(playable(s, "p2", MENACE)).toBe(true);
    });
  });

  describe("radiant", () => {
    it("R48 their cards cost (2) more during their next turn, and nothing before or after", () => {
      const s = setup([{ def: TAX, radiant: true }]).play(TAX);
      expect(priceOf(s, "p2", VANILLA)).toBe(1);
      s.endTurn();
      expect(priceOf(s, "p2", VANILLA)).toBe(3);
      expect(priceOf(s, "p2", MENACE)).toBe(5);
      expect(priceOf(s, "p2", ADAPTIVE_UI)).toBe(0);
      expect(playable(s, "p2", MENACE)).toBe(false);
      s.endTurn();
      expect(priceOf(s, "p2", VANILLA)).toBe(1);
    });
  });
});
