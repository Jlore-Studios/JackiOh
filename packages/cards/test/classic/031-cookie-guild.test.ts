// C #31 Cookie Guild — SPEC §8.6 row 31, BUILD M9 Classic row C 31: "Cry: Recruit the first (2) Cost
// or less Unit from the top of your deck (its cost in the deck per R65: an X Unit 0, skipped unless the
// only valid target per R690), summoned without
// a Cry into your leftmost open zone; none, or a full unit row, → nothing; the deck otherwise keeps its
// order and no event carries a position; radiant 4/8: three scans, stopping when the row fills (as
// #69); its tuned numbers (cost limit, units) read through `param()` (R386)".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/031-cookie-guild";

const GUILD = "classic-031";
const FELINORS = "core-012"; // (2) Unit 3/4, Cry: summon a copy of this.
const BIG_D = "core-001"; // (2) Unit.
const TEMPO = "core-011"; // (1) Unit 3/3.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9.
const STOCKPILE = "core-005"; // (1) Spell.
const ARMOR = "core-073"; // (2) Field Spell: a permanent, not a Unit.
const BUFF_BILLY = "classicplus-069"; // (X) Unit.
const ANCHOR = "core-010";

function libraryDefs(s: Scenario, player: "p1" | "p2" = "p1"): string[] {
  return s.pile(player, "library").map((card) => card.defId);
}

function unitDefs(s: Scenario, player: "p1" | "p2" = "p1"): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit(player, lane)?.defId ?? null);
}

function summonedDefs(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "summoned" ? [event.defId] : []));
}

describe("C #31 Cookie Guild", () => {
  it("runs one script on both faces", () => {
    expect(def.id).toBe(GUILD);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("is a 2/4", () => {
      const s = scenario({ p1: { field: [GUILD], hand: [ANCHOR] } });
      s.expectStats(GUILD, { attack: 2, health: 4 });
    });

    it("recruits the first (2) Cost or less Unit from the top, passing over bigger Units, Spells and other permanents", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [MENACE, STOCKPILE, ARMOR, FELINORS, TEMPO] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, FELINORS, null, null, null]);
      expect(libraryDefs(s)).toEqual([MENACE, STOCKPILE, ARMOR, TEMPO]);
    });

    it("R1 the recruited Unit is summoned without a Cry: a Duplicating Felinors makes no copy", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [FELINORS] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(summonedDefs(s).filter((id) => id === FELINORS)).toHaveLength(1);
      expect(unitDefs(s).filter((id) => id === FELINORS)).toHaveLength(1);
    });

    it("R64 it lands in your leftmost open zone", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [TEMPO], field: [{ def: MENACE, lane: 2 }] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD, { zone: 4 });

      expect(unitDefs(s)).toEqual([TEMPO, MENACE, null, GUILD, null]);
    });

    it("R65 the cost is the card's own in the deck: a (3) Unit made to cost 1 less is recruited, a (1) Unit made to cost 2 more is not", () => {
      const s = scenario({
        p1: { hand: [GUILD, ANCHOR], library: [{ def: TEMPO, costMod: 2 }, { def: MENACE, costMod: -1 }] },
        p2: { hand: [ANCHOR] },
      });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, MENACE, null, null, null]);
      expect(libraryDefs(s)).toEqual([TEMPO]);
    });

    it("R690 an X Unit is skipped when a non-X match sits further down: it recruits that one instead", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [MENACE, BUFF_BILLY, TEMPO] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(summonedDefs(s)).toContain(TEMPO);
      expect(summonedDefs(s)).not.toContain(BUFF_BILLY);
      expect(libraryDefs(s)).toEqual([MENACE, BUFF_BILLY]);
    });

    it("R396 R690 an X Unit costs (0) in the deck, so it is recruited when it is the only valid target", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [MENACE, BUFF_BILLY] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(summonedDefs(s)).toContain(BUFF_BILLY);
      expect(libraryDefs(s)).toEqual([MENACE]);
    });

    it("with no match in the deck nothing happens, and the deck keeps its order", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [MENACE, STOCKPILE, ARMOR] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, null, null, null, null]);
      expect(libraryDefs(s)).toEqual([MENACE, STOCKPILE, ARMOR]);
    });

    it("with a full unit row nothing is summoned and the Unit stays in the deck", () => {
      const s = scenario({
        p1: { hand: [GUILD, ANCHOR], library: [TEMPO], field: [MENACE, MENACE, MENACE, MENACE] },
        p2: { hand: [ANCHOR] },
      });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([MENACE, MENACE, MENACE, MENACE, GUILD]);
      expect(libraryDefs(s)).toEqual([TEMPO]);
    });

    it("no event carries a library position, and the opponent reads only the summon", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [MENACE, TEMPO, BIG_D] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(s.lastEvents.some((event) => "position" in event)).toBe(false);
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).toContain(TEMPO);
      expect(theirs).not.toContain(MENACE);
      expect(theirs).not.toContain(BIG_D);
    });

    it("R386 an Upgrade of units makes two scans; of the cost limit, a (3) Unit qualifies", () => {
      const units = scenario({ p1: { hand: [GUILD, ANCHOR], library: [TEMPO, VANILLA, BIG_D] }, p2: { hand: [ANCHOR] } });
      stepParam(units.card(GUILD), "units", 1);
      units.play(GUILD);
      expect(unitDefs(units)).toEqual([GUILD, TEMPO, VANILLA, null, null]);

      const limit = scenario({ p1: { hand: [GUILD, ANCHOR], library: [MENACE, TEMPO] }, p2: { hand: [ANCHOR] } });
      stepParam(limit.card(GUILD), "costLimit", 1);
      limit.play(GUILD);
      expect(unitDefs(limit)).toEqual([GUILD, MENACE, null, null, null]);
    });

    it("R386 a Degrade of the cost limit to (1) passes a (2) Unit over", () => {
      const s = scenario({ p1: { hand: [GUILD, ANCHOR], library: [FELINORS, TEMPO] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(GUILD), "costLimit", -1);

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, TEMPO, null, null, null]);
      expect(libraryDefs(s)).toEqual([FELINORS]);
    });
  });

  describe("radiant", () => {
    it("is a 4/8", () => {
      const s = scenario({ p1: { field: [{ def: GUILD, radiant: true }], hand: [ANCHOR] } });
      s.expectStats(GUILD, { attack: 4, health: 8 });
    });

    it("makes three top-down scans, each taking the next match", () => {
      const s = scenario({
        p1: { hand: [{ def: GUILD, radiant: true }, ANCHOR], library: [FELINORS, MENACE, TEMPO, STOCKPILE, BIG_D, VANILLA] },
        p2: { hand: [ANCHOR] },
      });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, FELINORS, TEMPO, BIG_D, null]);
      expect(libraryDefs(s)).toEqual([MENACE, STOCKPILE, VANILLA]);
    });

    it("fewer matches than three: it recruits what there is", () => {
      const s = scenario({ p1: { hand: [{ def: GUILD, radiant: true }, ANCHOR], library: [MENACE, TEMPO] }, p2: { hand: [ANCHOR] } });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, TEMPO, null, null, null]);
    });

    it("stops when the row fills: the third match stays in the deck", () => {
      const s = scenario({
        p1: { hand: [{ def: GUILD, radiant: true }, ANCHOR], library: [TEMPO, VANILLA, BIG_D], field: [MENACE, MENACE] },
        p2: { hand: [ANCHOR] },
      });

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([MENACE, MENACE, GUILD, TEMPO, VANILLA]);
      expect(libraryDefs(s)).toEqual([BIG_D]);
    });

    it("R386 a Degrade of units makes two scans", () => {
      const s = scenario({ p1: { hand: [{ def: GUILD, radiant: true }, ANCHOR], library: [TEMPO, VANILLA, BIG_D] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(GUILD), "units", -1);

      s.play(GUILD);

      expect(unitDefs(s)).toEqual([GUILD, TEMPO, VANILLA, null, null]);
      expect(libraryDefs(s)).toEqual([BIG_D]);
    });
  });
});
