// C #33 Joro — SPEC §8.6 row 33, BUILD M9 Classic row C 33: "From your hand only (R394): when your
// opponent targets one of your Units, by declaring an attack on it (§4.2 step 2, before the trap
// window) or picking it with a play, cast, activation or prompt answer, Joro is summoned into your
// leftmost open zone (no Cry, summoning sick) and the attack or pick moves to it (`redirected`); a
// random pick or an "all" effect targets nothing; a play naming several of your Units moves the first
// only; one Joro answers one targeting; your own picks never set it off; no open unit zone → nothing
// happens and Joro stays in hand; it never answers from the deck or the field; the opponent's view and
// `legalActions` carry no sign of Joro in your hand until it is summoned (R97); radiant:
// Indestructible, still 1/1 (a named exception in the R275 radiant-standard test), so it survives the
// redirected hit, with no Taunt (R347); no tuned numbers".
//
// This worktree's engine wires the attack half of "a friendly unit is targeted" (`combat.ts`); the
// play, cast and prompt halves are the play pipeline's (§10.5 step 1 and every target prompt), wired
// in the play-pipeline workstream's engine: those tests wait for integration, as does the random-pick
// case, which needs C #42's Activate.

import { describe, expect, it } from "vitest";
import { createRng, legalActions, subsystems } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/033-joro";

const JORO = "classic-033";
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const SEVEN = "core-025"; // (4) Unit 7/7
const VANILLA = "core-008"; // (1) Unit 4/4
const TIMMY = "core-011"; // (1) Unit 3/3
const ECLIPSE = "core-035"; // (1) Spell: "Deal 3 damage to a target."
const STAB = "core-070"; // (3) Spell: "Deal 2 damage to a target, +1 per ..."
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const BIG_FELINOR = "core-043"; // (4) Unit: "Cry: Destroy all non-Felinor Units."
const MOTHS = "core-009"; // (2) Unit 1/14: "Start of turn: Every enemy Unit attacks this."
const TWINSPELL = "core-079"; // (2) Field Spell: "Your next Spell gains Echo +1."
const TOXINS = "classic-042"; // (2) Field Spell: "Activate: Place a Plague Token on each of 2 random Units."
const STOCKPILE = "core-005"; // (1) Spell, a spare card (§2.5)

function redirects(s: Scenario): Extract<GameEvent, { type: "redirected" }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: "redirected" }> => event.type === "redirected");
}

/** p2 (active) attacks with a 7/7; p1 has a Vanilla in lane 1 and Joro in hand. */
function underAttack(
  opts: { radiantFace?: boolean; myField?: readonly string[]; myHand?: readonly (string | { def: string; radiant?: boolean })[]; myLibrary?: readonly string[] } = {},
): Scenario {
  return scenario({
    active: "p2",
    p1: {
      hand: [...(opts.myHand ?? [{ def: JORO, radiant: opts.radiantFace === true }, STOCKPILE])],
      field: [...(opts.myField ?? [VANILLA])],
      library: [...(opts.myLibrary ?? [])],
    },
    p2: { hand: [STOCKPILE, HIT_JOB, ECLIPSE], field: [SEVEN], library: [STOCKPILE] },
  });
}

describe("C #33 Joro", () => {
  it("is one replacement at \"targeted\" from the hand, the same on both faces, with no numbers", () => {
    expect(def.params).toBeUndefined();
    expect(base.replacements).toEqual([{ id: "joro", on: "targeted", where: "hand", instead: { interpose: true } }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R394 an attack declared at one of your Units summons Joro into your leftmost open zone and moves to it", () => {
      const s = underAttack();
      const joro = s.card(JORO);
      const vanilla = s.card(VANILLA);
      s.attack(SEVEN, vanilla);
      expect(redirects(s)).toEqual([{ type: "redirected", what: "attack", fromId: vanilla.id, toId: joro.id, byInstanceId: joro.id }]);
      // The 7/7 killed the 1/1 Joro; the Vanilla was never hit.
      s.expectInZone(joro, "graveyard");
      s.expectStats(vanilla, { health: 4 });
      s.expectEvents("summoned", "redirected", "destroyed");
    });

    it("§4.2 step 2 the move happens before the trap window, and Joro arrives summoning sick with no Cry", () => {
      const s = underAttack({ myField: [MENACE] });
      const joro = s.card(JORO);
      s.attack(SEVEN, s.card(MENACE));
      const summonedAt = s.events.findIndex((event) => event.type === "summoned" && event.instanceId === joro.id);
      const declaredAt = s.events.findIndex((event) => event.type === "attackDeclared");
      expect(summonedAt).toBeGreaterThan(-1);
      expect(summonedAt).toBeLessThan(s.events.findIndex((event) => event.type === "damage"));
      expect(declaredAt).toBeGreaterThan(-1);
    });

    it("R64 the leftmost open zone", () => {
      const s = underAttack({ myField: [VANILLA, TIMMY] });
      const joro = s.card(JORO);
      s.attack(SEVEN, s.card(TIMMY));
      expect(redirects(s)).toHaveLength(1);
      // Joro stood in lane 3 when the 7/7 hit it.
      const summoned = s.events.find((event) => event.type === "summoned" && event.instanceId === joro.id);
      expect(summoned).toMatchObject({ row: "units", lane: 3 });
    });

    it("no open unit zone: nothing happens and Joro stays in hand", () => {
      const s = underAttack({ myField: [VANILLA, TIMMY, VANILLA, TIMMY, MENACE] });
      const joro = s.card(JORO);
      s.attack(SEVEN, s.card(MENACE));
      expect(redirects(s)).toEqual([]);
      s.expectInZone(joro, "hand");
    });

    it("an attack on your hero targets no Unit", () => {
      const s = underAttack({ myField: [] });
      s.attack(SEVEN, "hero");
      expect(redirects(s)).toEqual([]);
      s.expectInZone(JORO, "hand");
    });

    it("one Joro answers one targeting: with two in hand, one attack summons one", () => {
      const s = underAttack({ myHand: [JORO, JORO, STOCKPILE] });
      s.attack(SEVEN, s.card(VANILLA));
      expect(redirects(s)).toHaveLength(1);
      expect(s.hand("p1").filter((card) => card.defId === JORO)).toHaveLength(1);
    });

    it("R394 it never answers from the deck", () => {
      const s = underAttack({ myHand: [STOCKPILE], myLibrary: [JORO] });
      s.attack(SEVEN, s.card(VANILLA));
      expect(redirects(s)).toEqual([]);
      s.expectInZone(JORO, "library");
    });

    it("R394 it never answers from the field", () => {
      const s = underAttack({ myHand: [STOCKPILE], myField: [VANILLA, JORO] });
      const vanilla = s.card(VANILLA);
      s.attack(SEVEN, vanilla);
      expect(redirects(s)).toEqual([]);
      s.expectInZone(vanilla, "graveyard");
    });

    it("R121 a forced attack is the effect's, not a targeting: Moths to the Flame draws no Joro", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [JORO, STOCKPILE], field: [MOTHS], library: [STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [VANILLA], library: [STOCKPILE] },
      });
      s.endTurn(); // p1's start of turn: p2's Vanilla is forced to attack the Moths
      expect(s.events.some((event) => event.type === "attackDeclared")).toBe(true);
      expect(redirects(s)).toEqual([]);
      s.expectInZone(JORO, "hand");
    });

    it("an \"all\" effect targets nothing: Big Felinor's sweep draws no Joro", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [JORO, STOCKPILE], field: [VANILLA] },
        p2: { hand: [BIG_FELINOR, STOCKPILE] },
      });
      s.play(BIG_FELINOR);
      expect(redirects(s)).toEqual([]);
      s.expectInZone(JORO, "hand");
      s.expectInZone(VANILLA, "graveyard");
    });

    it("your own picks never set it off", () => {
      const s = scenario({ p1: { hand: [JORO, ECLIPSE, STOCKPILE], field: [MENACE] }, p2: { hand: [STOCKPILE] } });
      s.play(ECLIPSE, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] });
      expect(redirects(s)).toEqual([]);
      s.expectInZone(JORO, "hand");
      s.expectStats(MENACE, { health: 6 });
    });

    it("R97 R177 the opponent's view and legalActions carry no sign of Joro in your hand", () => {
      const withJoro = underAttack({ myHand: [JORO, STOCKPILE] });
      const withVanilla = underAttack({ myHand: [VANILLA, STOCKPILE] });
      expect(withJoro.view("p2")).toEqual(withVanilla.view("p2"));
      expect(legalActions(withJoro.state, "p2")).toEqual(legalActions(withVanilla.state, "p2"));
    });

    it("R394 a play's declared pick of your Unit moves to Joro", () => {
      const s = underAttack();
      const joro = s.card(JORO);
      const vanilla = s.card(VANILLA);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      expect(redirects(s).map((event) => [event.what, event.fromId, event.toId])).toEqual([["target", vanilla.id, joro.id]]);
      s.expectInZone(joro, "graveyard");
      s.expectInZone(vanilla, "field");
    });

    it("R394 a prompt answer's pick of your Unit moves to Joro: an Echo repeat's fresh target", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [JORO, STOCKPILE], field: [MENACE] },
        p2: { hand: [TWINSPELL, ECLIPSE, STOCKPILE] },
      });
      const joro = s.card(JORO);
      const menace = s.card(MENACE);
      s.play(TWINSPELL);
      s.play(ECLIPSE, { targets: [{ pick: "hero", player: "p1" }] });
      expect(s.state.pending?.kind).toBe("target");
      s.answer([{ pick: "instance", instanceId: menace.id }]);
      expect(redirects(s).map((event) => [event.what, event.toId])).toEqual([["target", joro.id]]);
      s.expectStats(menace, { health: 9 });
    });

    it("a play naming several of your Units moves the first only", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [JORO, STOCKPILE], field: [MENACE, SEVEN] },
        p2: { hand: [ECLIPSE, STAB, STOCKPILE], mana: 10 },
      });
      const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
      const fused = subsystems.fuse(sink, { ingredients: [s.card(ECLIPSE), s.card(STAB)], toHand: "p2" });
      if (fused === null) throw new Error("the crafted two-target Spell");
      const menace = s.card(MENACE);
      const seven = s.card(SEVEN);
      const joro = s.card(JORO);
      s.play(fused, {
        targets: [
          { pick: "instance", instanceId: menace.id },
          { pick: "instance", instanceId: seven.id },
        ],
      });
      expect(redirects(s).map((event) => [event.fromId, event.toId])).toEqual([[menace.id, joro.id]]);
    });

    it("a random pick targets nothing: C #42's random Plague Tokens draw no Joro", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [JORO, STOCKPILE], field: [MENACE] },
        p2: { hand: [STOCKPILE], backrow: [TOXINS] },
      });
      s.activate(TOXINS);
      expect(redirects(s)).toEqual([]);
      s.expectInZone(JORO, "hand");
      expect(s.unit("p1", 1)?.counters.plague).toBe(1);
    });
  });

  describe("radiant", () => {
    it("R275 Indestructible, still 1/1: it survives the redirected hit", () => {
      const s = underAttack({ radiantFace: true });
      const joro = s.card(JORO);
      s.attack(SEVEN, s.card(VANILLA));
      expect(redirects(s)).toHaveLength(1);
      s.expectInZone(joro, "field");
      s.expectStats(joro, { attack: 1, health: 1, maxHealth: 1 });
      expect(s.stats(joro).keywords.map((keyword) => keyword.kind)).toContain("Indestructible");
    });

    it("R347 no Taunt, even in Defense Position", () => {
      const s = underAttack({ radiantFace: true });
      const joro = s.card(JORO);
      s.attack(SEVEN, s.card(VANILLA));
      expect(s.stats(joro).keywords.map((keyword) => keyword.kind)).not.toContain("Taunt");
    });

    it("an endless decoy: it answers from the hand only, and once on the field it answers nothing", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [STOCKPILE], field: [VANILLA, { def: JORO, radiant: true }] },
        p2: { hand: [STOCKPILE], field: [SEVEN], library: [STOCKPILE] },
      });
      const vanilla = s.card(VANILLA);
      s.attack(SEVEN, vanilla);
      expect(redirects(s)).toEqual([]);
      s.expectInZone(vanilla, "graveyard");
    });
  });
});
