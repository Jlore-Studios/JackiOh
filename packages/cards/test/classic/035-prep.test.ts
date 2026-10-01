// C #35 Prep — SPEC §8.6 row 35, BUILD M9 Classic row C 35: "Your next Spell this turn costs (2) less
// (#35 Lunar Eclipse's modifier: the Spell type only, floored at (0), R65); a Unit, Field Spell or
// Trap play doesn't consume it and the first Spell does; it expires at cleanup; a cast never uses it
// (R70); radiant: (4) less; its tuned number (discount) reads through `param()` (R386)".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/035-prep";

const PREP = "classic-035";
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const VANILLA = "core-008"; // (1) Unit 4/4.
const ARMOR = "core-073"; // (2) Field Spell; Cry: Draw 1.
const SHEEPISH = "core-041"; // (1) Trap.
const VIRUS = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
const ANCHOR = "core-010"; // (0) Spell, Combo 3 — a free play that keeps a turn open.
const MENACE = "core-019";

function spellsPaid(s: Scenario): { defId: string; costPaid: number }[] {
  return s.events.flatMap((event) => (event.type === "cardPlayed" ? [{ defId: event.defId, costPaid: event.costPaid }] : []));
}

function paidFor(s: Scenario, defId: string): number[] {
  return spellsPaid(s)
    .filter((play) => play.defId === defId)
    .map((play) => play.costPaid);
}

describe("C #35 Prep", () => {
  it("runs one script on both faces", () => {
    expect(def.id).toBe(PREP);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R65 the next Spell costs (2) less, floored at (0): a (1) Spell is free and gives no mana back", () => {
      const s = scenario({ p1: { hand: [PREP, STOCKPILE, ANCHOR], library: [MENACE, MENACE] }, p2: { hand: [ANCHOR] } });

      s.play(PREP).play(STOCKPILE);

      expect(paidFor(s, STOCKPILE)).toEqual([0]);
      s.expectMana("p1", 4);
    });

    it("the next Spell is the next one whatever it costs: a (0) Spell played next consumes it, and the Spell after pays in full", () => {
      const s = scenario({ p1: { hand: [PREP, ANCHOR, STOCKPILE, FLOOD], library: [MENACE, MENACE] }, p2: { hand: [ANCHOR] } });

      s.play(PREP).play(ANCHOR).play(STOCKPILE);

      expect(paidFor(s, ANCHOR)).toEqual([0]);
      expect(paidFor(s, STOCKPILE)).toEqual([1]);
      s.expectMana("p1", 3);
    });

    it("only the next Spell: the first Spell consumes it and the second pays in full", () => {
      const s = scenario({ p1: { hand: [PREP, STOCKPILE, STOCKPILE, ANCHOR], library: [MENACE, MENACE, MENACE, MENACE] }, p2: { hand: [ANCHOR] } });
      const [, first, second] = s.hand("p1");
      if (first === undefined || second === undefined) throw new Error("two Stockpiles in hand");

      s.play(PREP).play(first).play(second);

      expect(paidFor(s, STOCKPILE)).toEqual([0, 1]);
      s.expectMana("p1", 3);
    });

    it("a Unit play pays in full and does not consume it", () => {
      const s = scenario({ p1: { hand: [PREP, VANILLA, STOCKPILE, ANCHOR], library: [MENACE, MENACE] }, p2: { hand: [ANCHOR] } });

      s.play(PREP).play(VANILLA).play(STOCKPILE);

      expect(paidFor(s, VANILLA)).toEqual([1]);
      expect(paidFor(s, STOCKPILE)).toEqual([0]);
      s.expectMana("p1", 3);
    });

    it("a Trap play pays in full and does not consume it", () => {
      const s = scenario({ p1: { hand: [PREP, SHEEPISH, STOCKPILE, ANCHOR], library: [MENACE, MENACE] }, p2: { hand: [ANCHOR] } });

      s.play(PREP).play(SHEEPISH).play(STOCKPILE);

      expect(paidFor(s, SHEEPISH)).toEqual([1]);
      expect(paidFor(s, STOCKPILE)).toEqual([0]);
    });

    it("R70 a Field Spell play does not consume it, nor does the Spell its draw casts: the next Spell played is still cheaper", () => {
      const s = scenario({
        p1: { hand: [PREP, ARMOR, STOCKPILE, ANCHOR], library: [VIRUS, MENACE, MENACE, MENACE] },
        p2: { hand: [ANCHOR] },
      });

      s.play(PREP).play(ARMOR);

      expect(paidFor(s, ARMOR)).toEqual([2]);
      // The Field Spell's Cry drew the CN-Virus, which cast itself for free (R70) and drew again.
      expect(paidFor(s, VIRUS)).toEqual([0]);
      s.expectHealth("p1", 29);

      s.play(STOCKPILE);

      expect(paidFor(s, STOCKPILE)).toEqual([0]);
      s.expectMana("p1", 2);
    });

    it("§2.2 it expires at cleanup: a Spell on your next turn pays in full", () => {
      const s = scenario({
        p1: { hand: [PREP, STOCKPILE, ANCHOR], library: [MENACE, MENACE, MENACE] },
        p2: { hand: [ANCHOR], library: [MENACE, MENACE] },
      });

      s.play(PREP).endTurn().endTurn();
      expect(s.state.active).toBe("p1");
      s.play(STOCKPILE);

      expect(paidFor(s, STOCKPILE)).toEqual([1]);
    });

    it("R386 an Upgrade makes it (3) less, a Degrade (1) less: a (4) Flood costs 1, then 3", () => {
      const up = scenario({ p1: { hand: [PREP, FLOOD, ANCHOR] }, p2: { hand: [ANCHOR] } });
      stepParam(up.card(PREP), "discount", 1);
      up.play(PREP).play(FLOOD);
      expect(paidFor(up, FLOOD)).toEqual([1]);

      const down = scenario({ p1: { hand: [PREP, FLOOD, ANCHOR] }, p2: { hand: [ANCHOR] } });
      stepParam(down.card(PREP), "discount", -1);
      down.play(PREP).play(FLOOD);
      expect(paidFor(down, FLOOD)).toEqual([3]);
    });
  });

  describe("radiant", () => {
    it("the next Spell costs (4) less: a (4) Flood is free", () => {
      const s = scenario({ p1: { hand: [{ def: PREP, radiant: true }, FLOOD, ANCHOR] }, p2: { hand: [ANCHOR] } });

      s.play(PREP).play(FLOOD);

      expect(paidFor(s, FLOOD)).toEqual([0]);
      s.expectMana("p1", 4);
    });

    it("still only the next Spell, and still not a Unit", () => {
      const s = scenario({
        p1: { hand: [{ def: PREP, radiant: true }, VANILLA, FLOOD, STOCKPILE, ANCHOR], library: [MENACE, MENACE], mana: 10 },
        p2: { hand: [ANCHOR] },
      });

      s.play(PREP).play(VANILLA).play(FLOOD).play(STOCKPILE);

      expect(paidFor(s, VANILLA)).toEqual([1]);
      expect(paidFor(s, FLOOD)).toEqual([0]);
      expect(paidFor(s, STOCKPILE)).toEqual([1]);
    });

    it("R386 a Degrade on the Radiant face steps 4 to 3: a (4) Flood costs 1", () => {
      const s = scenario({ p1: { hand: [{ def: PREP, radiant: true }, FLOOD, ANCHOR] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(PREP), "discount", -1);

      s.play(PREP).play(FLOOD);

      expect(paidFor(s, FLOOD)).toEqual([1]);
    });
  });
});
