// C+ #14 Forever& — SPEC §8.7 row 14, R410, BUILD M9 Classic+ row C+ 14: "Leaves a waiting player
// modifier, not turn-scoped (it survives cleanup), that stamps the next Spell you play, never Forever&
// itself, with "After this resolves, Bounce it. This can't cost less than (2)": the Spell
// resolves and comes back to your hand, and does so after every later play too, the enchantment riding
// the card in every zone; its floor applies after every discount; a discarded or countered stamped
// Spell does not come back (R410); a Unit, Field Spell or Trap play leaves the modifier waiting; a full
// hand burns the returning card (R4); the floor reads through `param()` and never drops below 1;
// radiant the floor is (1) with no draw (balance patch 1)".

import { HAND_CAP, HERO_HEALTH, effectiveCost, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const FOREVER = "classicplus-014";
const LUNAR = "core-035"; // (1) Spell: 3 damage; your next Spell this turn costs (1) less
const STOCKPILE = "core-005"; // (1) Spell: draw 2, heal 2
const MENACE = "core-019"; // a Unit
const MANA_WELL = "core-006"; // a Field Spell
const SHEEPISH = "core-041"; // a Trap
const RAPID_DRAW = "classic-026"; // (0) Spell: draw 4, then discard 4 at random
const COUNTERSPELL = "classic-017"; // Trap: counters the opponent's Spell
const JELLY_BEAN = "core-027"; // (1) Spell, cast on draw: make a random hand card Radiant, lose 5 health
const FILLER = "core-008"; // Mr. Vanilla: a Unit, inert

const AT_HERO = [{ pick: "hero", player: "p2" }] as const;

function forever(radiant: boolean, hand: readonly string[], p1: SideSetup = {}, p2: SideSetup = {}): Scenario {
  const s = scenario({
    p1: { hand: [{ def: FOREVER, radiant }, ...hand], library: [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE], mana: 10, ...p1 },
    p2: { hand: [STOCKPILE], library: [STOCKPILE, STOCKPILE, STOCKPILE], ...p2 },
  });
  s.play(FOREVER);
  return s;
}

const lunarIn = (s: Scenario): string => {
  const card = s.hand("p1").find((held) => held.defId === LUNAR);
  if (card === undefined) throw new Error("no Lunar Eclipse in hand");
  return card.id;
};

describe("C+ #14 Forever&", () => {
  describe("base", () => {
    it("R410 never stamps itself: Forever& goes to the graveyard and a modifier waits", () => {
      const s = forever(false, [LUNAR]);
      s.expectInZone(FOREVER, "graveyard");
      expect(s.view("p1").you.modifiers.length).toBeGreaterThan(0);
    });

    it("R410 the next Spell resolves and comes back to your hand, and again after every later play", () => {
      const s = forever(false, [LUNAR]);
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 3);
      expect(s.card(id).zone.z).toBe("hand");
      s.play(id, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 6);
      expect(s.card(id).zone.z).toBe("hand");
      expect(s.card(id).enchantments).toContainEqual({ kind: "returnAfterResolve", floor: 2 });
    });

    it("only the next Spell: the one after is not stamped", () => {
      const s = forever(false, [LUNAR, STOCKPILE]);
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(lunarIn(s), { targets: AT_HERO });
      s.play(stockpile);
      expect(s.card(stockpile).zone.z).toBe("graveyard");
    });

    it("the modifier waits across turns: it survives cleanup", () => {
      const s = forever(false, [LUNAR, MENACE]);
      s.endTurn().endTurn();
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      expect(s.card(id).zone.z).toBe("hand");
    });

    it("R70 a cast is a play: a Spell cast as the next one is stamped and comes back to your hand", () => {
      // No Spell is played: the turn's draw takes the Jelly Bean, which casts itself (R70).
      const s = forever(false, [FILLER], { library: [JELLY_BEAN, STOCKPILE, STOCKPILE] });
      s.endTurn().endTurn();
      const jelly = s.card(JELLY_BEAN);
      s.expectHealth("p1", HERO_HEALTH - 5);
      expect(jelly.zone.z).toBe("hand");
      expect(jelly.enchantments).toContainEqual({ kind: "returnAfterResolve", floor: 2 });
    });

    it("a Unit, Field Spell or Trap play leaves the modifier waiting", () => {
      const s = forever(false, [MENACE, MANA_WELL, SHEEPISH, LUNAR]);
      s.play(MENACE);
      s.play(MANA_WELL);
      s.play(SHEEPISH);
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      expect(s.card(id).zone.z).toBe("hand");
    });

    it("R65 its floor applies after every discount: a (1) Spell stamped costs (2), and a discount can't lower it", () => {
      const s = forever(false, [LUNAR, LUNAR]);
      const [first, second] = s.hand("p1").filter((card) => card.defId === LUNAR);
      if (first === undefined || second === undefined) throw new Error("two Lunar Eclipses");
      s.play(first, { targets: AT_HERO });
      expect(effectiveCost(s.state, s.card(first.id))).toBe(2);
      // The second Lunar's discount (the next Spell costs (1) less) cannot take it below the floor.
      s.play(second, { targets: AT_HERO });
      expect(effectiveCost(s.state, s.card(first.id))).toBe(2);
    });

    it("R410 a discarded stamped Spell does not come back", () => {
      // Only two cards come back from the library: all three in hand go, the stamped Lunar among them.
      const s = forever(false, [LUNAR, RAPID_DRAW], { library: [FILLER, FILLER] });
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO }); // stamped: back in hand
      expect(s.card(id).zone.z).toBe("hand");
      s.play(RAPID_DRAW); // draws 2, then discards all 3 at random (R641)
      expect(s.state.pending).toBeNull();
      expect(s.card(id).zone.z).toBe("graveyard");
      s.endTurn().endTurn();
      expect(s.card(id).zone.z).toBe("graveyard");
    });

    it("R410 a countered stamped Spell does not come back", () => {
      const s = forever(false, [LUNAR, MENACE], {}, { hand: [COUNTERSPELL, STOCKPILE] });
      s.endTurn();
      s.play(COUNTERSPELL); // p2 sets it on their turn
      s.endTurn();
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      expect(s.events.some((event) => event.type === "countered" && event.instanceId === id)).toBe(true);
      expect(s.card(id).zone.z).toBe("graveyard");
      s.expectHealth("p2", HERO_HEALTH);
    });

    it("R4 a full hand burns the returning card", () => {
      // Stockpile is the stamped Spell: with ten cards in hand, its own draws fill the hand again.
      const s = forever(false, [STOCKPILE, ...Array.from({ length: HAND_CAP - 1 }, () => FILLER)]);
      const id = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(id);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.card(id).zone.z).toBe("graveyard");
      expect(s.events.some((event) => event.type === "burned" && event.instanceId === id)).toBe(true);
    });

    it("R386 the floor reads through param() and never drops below 1", () => {
      const s = scenario({ p1: { hand: [FOREVER, LUNAR, FILLER], mana: 10 }, p2: { hand: [STOCKPILE] } });
      stepParam(s.card(FOREVER), "floor", -5);
      s.play(FOREVER);
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      expect(s.card(id).enchantments).toContainEqual({ kind: "returnAfterResolve", floor: 1 });
      expect(effectiveCost(s.state, s.card(id))).toBe(1);
    });
  });

  describe("radiant", () => {
    it("the floor is (1), with no draw", () => {
      const s = forever(true, [LUNAR]);
      // Balance patch 1: the Radiant face draws nothing.
      expect(s.hand("p1").map((card) => card.defId)).toEqual([LUNAR]);
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      expect(s.card(id).zone.z).toBe("hand");
      expect(s.card(id).enchantments).toContainEqual({ kind: "returnAfterResolve", floor: 1 });
      expect(effectiveCost(s.state, s.card(id))).toBe(1);
    });

    it("R386 the floor reads through param(): a Degrade keeps it at (2)", () => {
      const s = scenario({ p1: { hand: [{ def: FOREVER, radiant: true }, LUNAR, FILLER], mana: 10 }, p2: { hand: [STOCKPILE] } });
      stepParam(s.card(FOREVER), "floor", 1);
      s.play(FOREVER);
      const id = lunarIn(s);
      s.play(id, { targets: AT_HERO });
      expect(s.card(id).enchantments).toContainEqual({ kind: "returnAfterResolve", floor: 2 });
    });
  });
});
