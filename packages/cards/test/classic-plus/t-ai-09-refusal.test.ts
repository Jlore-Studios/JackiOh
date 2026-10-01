// T-AI-9 Refusal — SPEC §8.7 row T-AI-9, BUILD M9 Classic+ row T-AI-9: "Face-down Trap in the announce
// window of §10.5 (the price paid, the card not yet moved): when the opponent plays or casts a Spell
// whose declared targets include one of your Units, it Counters it: the Spell goes to its owner's
// graveyard unresolved and is treated as never played (no `cardPlayed`, no count for Combo,
// Quickstriker, Ceaseless Void or the turn log; its Echo repeats never happen), the mana and Tributes
// staying spent; `countered` is public; a Spell with no declared target, a Field Spell or a Trap never
// sets it off; with two Refusals the first cancels and the second stays set; hidden until it fires
// (R33); radiant also when the Spell targets you or any card of yours, and you draw 1".
//
// Every case sets the trap face-down in p1's backrow and makes p2 the active player.

import type { GameEvent, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-09-refusal";

const REFUSAL = "classicplus-t-ai-09";
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const TRUE_STRIKE = "core-044"; // (1) Spell: Pierce. Deal 4 damage. Exile this.
const MAGIC_JAMMED = "core-036"; // (1) Spell: Destroy target backrow card. Lock its zone.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const TWINSPELL = "core-079"; // (2) Field Spell: your next Spell gains Echo +1.
const BEAR = "core-060"; // (1) Trap
const COUNTERSPELL_TEST_FILLER = "core-011"; // (1) Unit

function refusal(radiantFace = false, lane = 2): { def: string; radiant: boolean; faceUp: boolean; lane: number } {
  return { def: REFUSAL, radiant: radiantFace, faceUp: false, lane };
}

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [VANILLA], field: [MENACE], backrow: [refusal(radiantFace)], library: [VANILLA, STOCKPILE], ...p1 },
    p2: {
      hand: [HIT_JOB, TRUE_STRIKE, STOCKPILE, VANILLA],
      field: [VANILLA],
      library: [VANILLA, VANILLA, VANILLA],
      mana: 8,
      ...p2,
    },
  });
}

function at(s: Scenario, player: "p1" | "p2", lane = 1): Selection[] {
  return [{ pick: "instance", instanceId: s.unit(player, lane)?.id ?? "" }];
}

const P1_HERO: Selection[] = [{ pick: "hero", player: "p1" }];

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("T-AI-9 Refusal", () => {
  it("is a (1) AI Trap token whose condition lives in `when` (R99)", () => {
    expect(def.type).toBe("Trap");
    expect(def.cost).toBe(1);
    expect(def.tags).toEqual(["AI", "Token"]);
    expect(base.triggers?.[0]?.on).toEqual(["cardAnnounced"]);
    expect(radiant.triggers?.[0]?.when).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R448 counters a Spell that targets one of your Units: it never resolves and goes to its owner's graveyard", () => {
      const s = setup();
      const menace = s.unit("p1", 1);
      const hitJob = s.card(HIT_JOB);
      s.play(HIT_JOB, { targets: at(s, "p1") });

      s.expectInZone(hitJob, "graveyard");
      s.expectInZone(menace ?? "", "field");
      s.expectInZone(REFUSAL, "graveyard");
      s.expectEvents("cardAnnounced", "trapFired", "countered");
      expect(count(s.lastEvents, "cardPlayed")).toBe(0);
      expect(count(s.lastEvents, "cardResolved")).toBe(0);
    });

    it("R448 treated as never played: the turn's and the game's counts don't move, and its mana stays spent", () => {
      const s = setup();
      const turn = s.state.players.p2.turnLog.cardsPlayed;
      const game = s.state.counters.played;
      s.play(HIT_JOB, { targets: at(s, "p1") });
      expect(s.state.players.p2.turnLog.cardsPlayed).toBe(turn);
      expect(s.state.counters.played).toBe(game);
      s.expectMana("p2", 5);
    });

    it("R448 a countered Spell makes no Echo repeats", () => {
      const s = setup({}, { backrow: [{ def: TWINSPELL, faceUp: true }] });
      s.play(HIT_JOB, { targets: at(s, "p1") });
      expect(count(s.lastEvents, "cardResolved")).toBe(0);
      expect(count(s.lastEvents, "destroyed")).toBe(0);
    });

    it("R33 hidden until it fires; `countered` is public to both players", () => {
      const s = setup();
      expect(JSON.stringify(s.view("p2"))).not.toContain(REFUSAL);
      s.play(HIT_JOB, { targets: at(s, "p1") });
      for (const player of ["p1", "p2"] as const) {
        const countered = s.view(player).events.filter((event) => event.type === "countered");
        expect(countered).toHaveLength(1);
        expect(countered[0]).toMatchObject({ defId: HIT_JOB, to: "graveyard" });
      }
    });

    it("R81 a Spell that targets your hero, or one of their own Units, leaves it set", () => {
      const s = setup();
      s.play(TRUE_STRIKE, { targets: P1_HERO });
      s.expectHealth("p1", 26);
      s.play(HIT_JOB, { targets: at(s, "p2") });
      expect(s.unit("p2", 1)).toBeNull();
      expect(s.card(REFUSAL).faceUp).toBe(false);
      expect(count(s.events, "countered")).toBe(0);
    });

    it("R81 a Spell with no declared target never sets it off, even one that reaches your Units", () => {
      const s = setup({}, { hand: [FLOOD, STOCKPILE, VANILLA] });
      s.play(STOCKPILE);
      s.play(FLOOD);
      // Flood bounced every Unit, p1's included: it named none of them.
      expect(s.unit("p1", 1)).toBeNull();
      expect(s.card(REFUSAL).faceUp).toBe(false);
      expect(count(s.events, "countered")).toBe(0);
    });

    it("a Field Spell, a Trap and a Unit never set it off", () => {
      const s = setup({}, { hand: [TWINSPELL, BEAR, COUNTERSPELL_TEST_FILLER] });
      s.play(TWINSPELL, { zone: 1 }).play(BEAR, { zone: 2 }).play(COUNTERSPELL_TEST_FILLER, { zone: 2 });
      expect(count(s.events, "countered")).toBe(0);
      expect(s.card(REFUSAL).faceUp).toBe(false);
    });

    it("your own Spells never set it off", () => {
      const s = scenario({
        p1: { hand: [HIT_JOB, VANILLA], field: [MENACE], backrow: [refusal()], library: [VANILLA], mana: 8 },
        p2: { hand: [VANILLA], library: [VANILLA] },
      });
      s.play(HIT_JOB, { targets: at(s, "p1") });
      expect(s.unit("p1", 1)).toBeNull();
      expect(count(s.events, "countered")).toBe(0);
    });

    it("R448 with two Refusals the first cancels the Spell and the second stays set", () => {
      const s = setup({ backrow: [refusal(false, 2), refusal(false, 3)] });
      s.play(HIT_JOB, { targets: at(s, "p1") });
      expect(count(s.events, "countered")).toBe(1);
      expect(count(s.events, "trapFired")).toBe(1);
      expect(s.backrow("p1", 2)).toBeNull();
      expect(s.backrow("p1", 3)?.faceUp).toBe(false);
    });

    it("base: a Spell aimed at one of your backrow cards leaves it set", () => {
      const s = setup({ backrow: [refusal(false, 2), { def: TWINSPELL, faceUp: true, lane: 3 }] }, { hand: [MAGIC_JAMMED, VANILLA] });
      const jammed = s.backrow("p1", 3);
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: jammed?.id ?? "" }] });
      expect(count(s.events, "countered")).toBe(0);
      s.expectInZone(jammed ?? "", "graveyard");
    });
  });

  describe("radiant", () => {
    it("R448 counters a Spell that targets your hero, and you draw 1", () => {
      const s = setup({}, {}, true);
      const hand = s.hand("p1").length;
      s.play(TRUE_STRIKE, { targets: P1_HERO });
      s.expectHealth("p1", 30);
      s.expectInZone(TRUE_STRIKE, "graveyard");
      expect(s.hand("p1")).toHaveLength(hand + 1);
      expect(s.hand("p2").length).toBe(3);
      s.expectEvents("countered", "drawn");
    });

    it("R448 counters a Spell that targets one of your backrow cards", () => {
      const s = setup({ backrow: [refusal(true, 2), { def: TWINSPELL, faceUp: true, lane: 3 }] }, { hand: [MAGIC_JAMMED, VANILLA] }, true);
      const jammed = s.backrow("p1", 3);
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: jammed?.id ?? "" }] });
      expect(count(s.events, "countered")).toBe(1);
      expect(s.backrow("p1", 3)?.id).toBe(jammed?.id);
    });

    it("R448 still counters a Spell that targets one of your Units", () => {
      const s = setup({}, {}, true);
      s.play(HIT_JOB, { targets: at(s, "p1") });
      expect(s.unit("p1", 1)?.defId).toBe(MENACE);
      expect(count(s.events, "countered")).toBe(1);
    });

    it("a Spell aimed only at their own side leaves it set, and nobody draws", () => {
      const s = setup({}, {}, true);
      const hand = s.hand("p1").length;
      s.play(HIT_JOB, { targets: at(s, "p2") });
      expect(count(s.events, "countered")).toBe(0);
      expect(s.hand("p1")).toHaveLength(hand);
      expect(s.card(REFUSAL).faceUp).toBe(false);
    });
  });
});
