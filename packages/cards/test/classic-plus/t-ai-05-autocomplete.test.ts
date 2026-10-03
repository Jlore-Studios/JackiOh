// T-AI-5 Autocomplete — SPEC §8.7 row T-AI-5, BUILD M9 Classic+ row T-AI-5: "Adds a copy, by definition
// and face, of the last Unit, Spell or Field Spell your opponent played face-up (a cast counts, R70),
// skipping AI generated cards so two Autocompletes never feed each other; Traps and Field Traps never
// count, being set face-down, so nothing hidden is copied; nothing played yet, nothing; the record
// outlives the card leaving play; the copy reaches your hand under the sentinel for the opponent (R97);
// radiant the copy costs (0)".
//
// p2 plays first on its own turn, then ends it, and p1 plays Autocomplete on the next.

import type { CardView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-05-autocomplete";

const AUTOCOMPLETE = "classicplus-t-ai-05";
const TAX = "classicplus-t-ai-07"; // (1) AI Spell
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const TWINSPELL = "core-079"; // (2) Field Spell
const BEAR = "core-060"; // (1) Trap
const HINDER = "core-021"; // (0) Spell, cast on draw
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const TIMMY = "core-011"; // (1) Unit: p1's library, so its draw is never a card under test
const REFUSAL = "classicplus-t-ai-09"; // (1) Trap: counters a Spell that targets one of your Units

/** p2 makes `plays` on its turn (each a hand card, played in order), then p1's turn begins. */
function afterTheirTurn(
  plays: readonly (string | { card: string; zone?: number })[],
  opts: { p1?: SideSetup; p2?: SideSetup; radiantFace?: boolean } = {},
): Scenario {
  const s = scenario({
    active: "p2",
    p1: {
      hand: [{ def: AUTOCOMPLETE, radiant: opts.radiantFace === true }, VANILLA],
      library: [TIMMY, TIMMY, TIMMY],
      ...opts.p1,
    },
    p2: { hand: [VANILLA, STOCKPILE, TWINSPELL, BEAR, TAX, MENACE], library: [VANILLA, VANILLA, VANILLA], mana: 9, ...opts.p2 },
  });
  for (const play of plays) {
    const card = typeof play === "string" ? play : play.card;
    const zone = typeof play === "string" ? undefined : play.zone;
    s.play(card, zone === undefined ? {} : { zone });
  }
  return s.endTurn();
}

function copiesIn(s: Scenario, defId: string): number {
  return s.hand("p1").filter((card) => card.defId === defId).length;
}

describe("T-AI-5 Autocomplete", () => {
  it("is a (0) AI Spell token", () => {
    expect(def.cost).toBe(0);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["AI", "Token"]);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R451 adds a copy of the last Unit they played, at its printed cost", () => {
      const s = afterTheirTurn([{ card: MENACE, zone: 1 }]);
      s.play(AUTOCOMPLETE);
      const copy = s.hand("p1").find((card) => card.defId === MENACE);
      expect(copy).toBeDefined();
      expect(copy?.owner).toBe("p1");
      expect(copy?.costOverride).toBeUndefined();
      expect(copiesIn(s, MENACE)).toBe(1);
      expect(s.unit("p2", 1)?.defId).toBe(MENACE);
    });

    it("R451 the last one counts: a Spell after a Unit is the Spell", () => {
      const s = afterTheirTurn([{ card: VANILLA, zone: 1 }, STOCKPILE]);
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, STOCKPILE)).toBe(1);
      expect(copiesIn(s, VANILLA)).toBe(1); // only the one p1 already held
    });

    it("R451 a Field Spell counts", () => {
      const s = afterTheirTurn([STOCKPILE, { card: TWINSPELL, zone: 1 }]);
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, TWINSPELL)).toBe(1);
    });

    it("R33 R451 Traps never count, being set face-down: the face-up play before it is copied", () => {
      const s = afterTheirTurn([STOCKPILE, { card: BEAR, zone: 1 }]);
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, BEAR)).toBe(0);
      expect(copiesIn(s, STOCKPILE)).toBe(1);
    });

    it("R451 AI generated cards are skipped, so two Autocompletes never feed each other", () => {
      const s = afterTheirTurn([STOCKPILE, TAX]);
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, TAX)).toBe(0);
      expect(copiesIn(s, STOCKPILE)).toBe(1);

      const twice = scenario({
        active: "p2",
        p1: { hand: [AUTOCOMPLETE, VANILLA], library: [VANILLA, VANILLA] },
        p2: { hand: [AUTOCOMPLETE, VANILLA], library: [VANILLA, VANILLA] },
      });
      const theirs = twice.hand("p2").find((card) => card.defId === AUTOCOMPLETE);
      twice.play(theirs ?? AUTOCOMPLETE).endTurn();
      // p2's Autocomplete found nothing of p1's, and it is no record of p2's for p1's to find.
      expect(twice.state.active).toBe("p1");
      const handBefore = twice.hand("p1").length;
      twice.play(AUTOCOMPLETE);
      expect(twice.hand("p1")).toHaveLength(handBefore - 1);
      expect(copiesIn(twice, AUTOCOMPLETE)).toBe(0);
    });

    it("nothing played yet: nothing, and no rng draw (R129)", () => {
      const s = afterTheirTurn([]);
      const handBefore = s.hand("p1").length;
      const cursor = s.state.rngCursor;
      s.play(AUTOCOMPLETE);
      expect(s.hand("p1")).toHaveLength(handBefore - 1);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("your own plays never count", () => {
      const s = afterTheirTurn([STOCKPILE]);
      s.play(VANILLA, { zone: 1 });
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, STOCKPILE)).toBe(1);
      expect(copiesIn(s, VANILLA)).toBe(0);
    });

    it("R451 the record outlives the card leaving play", () => {
      const s = afterTheirTurn([{ card: MENACE, zone: 1 }], { p1: { hand: [AUTOCOMPLETE, HIT_JOB], mana: 8 } });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.unit("p2", 1)?.id ?? "" }] });
      s.expectInZone(MENACE, "graveyard");
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, MENACE)).toBe(1);
    });

    it("R70 a cast is a play: their cast-on-draw card is the last one", () => {
      const s = afterTheirTurn([STOCKPILE], { p2: { library: [{ def: HINDER, radiant: true }, VANILLA, VANILLA, VANILLA] } });
      s.play(AUTOCOMPLETE);
      expect(copiesIn(s, HINDER)).toBe(1);
      expect(s.hand("p1").find((card) => card.defId === HINDER)?.radiant).toBe(true);
    });

    it("R448 a countered card was never played: the play before it is the last one", () => {
      const s = scenario({
        active: "p2",
        p1: {
          hand: [AUTOCOMPLETE, VANILLA],
          field: [MENACE],
          backrow: [{ def: REFUSAL, faceUp: false }],
          library: [TIMMY, TIMMY, TIMMY],
        },
        p2: { hand: [STOCKPILE, HIT_JOB, VANILLA], library: [VANILLA, VANILLA, VANILLA], mana: 9 },
      });
      s.play(STOCKPILE).play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.unit("p1", 1)?.id ?? "" }] });
      expect(s.events.some((event) => event.type === "countered")).toBe(true);
      s.endTurn().play(AUTOCOMPLETE);
      expect(copiesIn(s, HIT_JOB)).toBe(0);
      expect(copiesIn(s, STOCKPILE)).toBe(1);
    });

    it("R57 a copy by definition and face: a Radiant play is copied Radiant", () => {
      const s = afterTheirTurn([{ card: MENACE, zone: 1 }], { p2: { hand: [{ def: MENACE, radiant: true }, VANILLA], mana: 4 } });
      s.play(AUTOCOMPLETE);
      const copy = s.hand("p1").find((card) => card.defId === MENACE);
      expect(copy?.radiant).toBe(true);
    });

    it("R97 the copy reaches your hand under the sentinel for the opponent", () => {
      const s = afterTheirTurn([{ card: MENACE, zone: 1 }]);
      s.play(AUTOCOMPLETE);
      const copy = s.hand("p1").find((card) => card.defId === MENACE);
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(copy?.id ?? "?");
      expect(s.view("p2").opponent.hand).toEqual({ count: s.hand("p1").length });
    });
  });

  describe("radiant", () => {
    it("the copy costs (0)", () => {
      const s = afterTheirTurn([{ card: MENACE, zone: 1 }], { radiantFace: true });
      s.play(AUTOCOMPLETE);
      const copy = s.hand("p1").find((card) => card.defId === MENACE);
      expect(copy?.costOverride).toBe(0);
      const shown = (s.view("p1").you.hand as CardView[]).find((card) => card.instanceId === copy?.id);
      expect(shown?.cost).toBe(0);
    });

    it("nothing played yet: nothing", () => {
      const s = afterTheirTurn([], { radiantFace: true });
      const handBefore = s.hand("p1").length;
      s.play(AUTOCOMPLETE);
      expect(s.hand("p1")).toHaveLength(handBefore - 1);
    });
  });
});
