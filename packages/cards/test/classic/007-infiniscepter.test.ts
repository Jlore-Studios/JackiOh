// C #7 InfiniScepter — SPEC §8.6 row 7, BUILD M9 Classic row C 7: "Cry: exile a (1) Cost or less Spell
// (the Spell type) of your choice from your hand, a hand pick carried in the play action (R81),
// remembering its definition and Radiant flag; no such Spell → the Cry does nothing and the card can
// never activate; the exiled card stays in exile; Activate, once per turn (R384): cast a fresh copy
// (free, counted as played, R70), its targets and modes chosen by you in prompts as the cast begins
// (R70, R81), the `activate` action carrying none, the copy going to your graveyard afterwards (R87);
// usable the turn it is played, only in your main phase with no prompt open; a second activation that
// turn is refused and absent from `legalActions`, and it is back next turn; activating is not a play
// (Combo, Quickstriker and Ceaseless Void don't count it) while the cast is one; leaving the field
// clears the memory (R78), so a replayed one needs a new Cry; radiant: a (2) Cost or less Spell, still
// from your hand; its tuned number (cost limit) reads through `param()` (R386)".
//
// R520 (this workstream's ruling on OPEN-QUESTIONS' InfiniScepter item): an X-cost Spell it holds is
// cast with the X its caster picks as the cast begins, 1 to their current mana (R348), and not paid.

import { describe, expect, it } from "vitest";
import { legalActions, stepParam } from "@jackioh/engine";
import type { GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/007-infiniscepter";

const SCEPTER = "classic-007";
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const CALL = "core-069"; // (2) Spell: Recruit 3 (1) Cost or less Units.
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const FLAME = "classic-016"; // (1) Spell, Book: Deal 4 damage (8 Radiant).
const DIVIDEND = "core-024"; // (X) Spell: modes damage / heal / mana.
const JAMMED = "core-036"; // (1) Spell: Destroy target backrow card; Lock its zone.
const TIMMY = "core-011"; // (1) Unit.
const VANILLA = "core-008";
const FILLER = "core-010"; // (0) Spell Rapid Replenish.
const TUTOR = "core-051"; // (1) Spell: opens a type prompt as it resolves.

function setup(p1: SideSetup, radiantFace = false, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: SCEPTER, radiant: radiantFace }, FILLER], library: [VANILLA, VANILLA, VANILLA, VANILLA], health: 20, ...p1 },
    p2: { hand: [FILLER], library: [VANILLA, VANILLA], ...p2 },
  });
}

function pick(card: { id: string }): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

function activations(s: Scenario, player: PlayerId = "p1"): unknown[] {
  const card = s.card(SCEPTER);
  return legalActions(s.state, player).filter((action) => action.type === "activate" && action.instanceId === card.id);
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("C #7 InfiniScepter", () => {
  it("declares its hand pick, one Activate once per turn, and the same script on both faces", () => {
    expect(def.id).toBe(SCEPTER);
    expect(radiant).toBe(base);
    expect(base.targets?.[0]).toMatchObject({ kind: "hand", filter: { type: "Spell", check: "spellWithinLimit" } });
    expect(base.activations?.[0]?.uses).toBe(1);
  });

  describe("base: the Cry", () => {
    it("R81 exiles the (1) Cost or less Spell picked in the play action, and remembers it", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] });
      const spell = s.card(STOCKPILE);

      s.play(SCEPTER, { zone: 1, targets: pick(spell) });

      s.expectInZone(spell, "exile");
      expect(s.card(SCEPTER).memory.scepter).toEqual({ defId: STOCKPILE, radiant: false });
    });

    it("R57 remembers the Spell's Radiant flag", () => {
      const s = setup({ hand: [SCEPTER, { def: STOCKPILE, radiant: true }, FILLER] });

      s.play(SCEPTER, { zone: 1, targets: pick(s.card(STOCKPILE)) });

      expect(s.card(SCEPTER).memory.scepter).toEqual({ defId: STOCKPILE, radiant: true });
    });

    it("§10.6 a (2) Cost Spell, a Unit or a Spell elsewhere is no pick for it", () => {
      const s = setup({ hand: [SCEPTER, CALL, TIMMY, FILLER] });
      const card = s.card(SCEPTER);
      const offered = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === card.id
          ? (action.targets ?? []).flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []))
          : [],
      );

      expect(offered).not.toContain(s.card(CALL).id);
      expect(offered).not.toContain(s.card(TIMMY).id);
      expect(offered).toContain(s.card(FILLER).id);
      expect(() => s.play(SCEPTER, { zone: 1, targets: pick(s.card(CALL)) })).toThrow();
    });

    it("R90 with no such Spell the play is legal, the Cry does nothing, and it can never activate", () => {
      const s = setup({ hand: [SCEPTER, CALL, TIMMY] });

      s.play(SCEPTER, { zone: 1 });

      s.expectInZone(SCEPTER, "field");
      expect(s.card(SCEPTER).memory.scepter).toBeUndefined();
      expect(activations(s)).toHaveLength(0);
      expect(() => s.activate(SCEPTER)).toThrow();
    });

    it("the exiled card stays in exile after the copies are cast", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] });
      const spell = s.card(STOCKPILE);

      s.play(SCEPTER, { zone: 1, targets: pick(spell) }).activate(SCEPTER);

      s.expectInZone(spell, "exile");
    });

    it("R386 an Upgrade of its cost limit admits a (2) Cost Spell", () => {
      const s = setup({ hand: [SCEPTER, CALL, FILLER] });
      stepParam(s.card(SCEPTER), "costLimit", 1);

      s.play(SCEPTER, { zone: 1, targets: pick(s.card(CALL)) });

      s.expectInZone(CALL, "exile");
    });
  });

  describe("base: the Activate", () => {
    it("R70 casts a fresh copy, free and counted as a play; the copy lands in your graveyard (R87)", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] });
      const spell = s.card(STOCKPILE);
      s.play(SCEPTER, { zone: 1, targets: pick(spell) });
      const played = s.state.players.p1.turnLog.cardsPlayed;

      s.activate(SCEPTER);

      expect(count(s.lastEvents, "drawn")).toBe(2);
      s.expectHealth("p1", 22);
      s.expectMana("p1", 3);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played + 1);
      const copies = s.pile("p1", "graveyard").filter((card) => card.defId === STOCKPILE);
      expect(copies).toHaveLength(1);
      expect(copies[0]?.id).not.toBe(spell.id);
    });

    it("R81 the copy's targets are chosen in a prompt as the cast begins; the action carries none", () => {
      const s = setup({ hand: [SCEPTER, FLAME, FILLER] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(FLAME)) });

      s.activate(SCEPTER);
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer([{ pick: "hero", player: "p2" }]);

      s.expectHealth("p2", 26);
    });

    it("R57 a Radiant Spell's copy is cast on its Radiant face", () => {
      const s = setup({ hand: [SCEPTER, { def: FLAME, radiant: true }, FILLER] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(FLAME)) });

      s.activate(SCEPTER).answer([{ pick: "hero", player: "p2" }]);

      s.expectHealth("p2", 22);
    });

    it("R384 usable the turn it is played; once per turn: a second is refused and not listed; back next turn", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(STOCKPILE)) });

      s.activate(SCEPTER);
      expect(activations(s)).toHaveLength(0);
      expect(() => s.activate(SCEPTER)).toThrow();

      s.endTurn().endTurn();
      expect(s.state.active).toBe("p1");
      expect(activations(s).length).toBeGreaterThan(0);
    });

    it("R384 only in its controller's own turn", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] }, false, { hand: [FILLER, VANILLA] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(STOCKPILE)) });
      expect(activations(s).length).toBeGreaterThan(0);

      s.endTurn();

      expect(s.state.active).toBe("p2");
      expect(activations(s)).toHaveLength(0);
      expect(() => s.activate(SCEPTER)).toThrow();
    });

    it("R384 not while a prompt is open", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, TUTOR, FILLER] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(STOCKPILE)) });
      expect(activations(s).length).toBeGreaterThan(0);

      s.play(TUTOR);

      expect(s.state.pending?.playerId).toBe("p1");
      expect(activations(s)).toHaveLength(0);
      expect(() => s.activate(SCEPTER)).toThrow();
    });

    it("R384 the activation is no play: it adds nothing to the turn's plays beyond the cast", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(STOCKPILE)) });
      const before = s.state.players.p1.turnLog.cardsPlayed;

      s.activate(SCEPTER);

      const plays = s.lastEvents.filter((event) => event.type === "cardPlayed");
      expect(plays).toHaveLength(1);
      expect(plays[0]).toMatchObject({ defId: STOCKPILE });
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(before + 1);
    });

    it("R78 leaving the field clears what it remembered", () => {
      const s = setup({ hand: [SCEPTER, STOCKPILE, FILLER] }, false, { hand: [JAMMED, FILLER] });
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(STOCKPILE)) });
      const scepter = s.card(SCEPTER);

      s.endTurn();
      s.play(JAMMED, { targets: pick(scepter) });

      s.expectInZone(scepter, "graveyard");
      expect(s.card(scepter).memory.scepter).toBeUndefined();
    });

    it("R520 an X-cost Spell it holds is cast with the X its caster picks, from 1 to their current mana", () => {
      const s = setup({ hand: [SCEPTER, DIVIDEND, FILLER] });
      // In hand an X Spell costs 0 (R65), so it is within the limit.
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(DIVIDEND)) });
      s.expectMana("p1", 3);

      s.activate(SCEPTER);
      const asked = s.state.pending;
      expect(asked?.playerId).toBe("p1");
      expect(asked?.options.map((option) => option.selection)).toEqual(
        [1, 2, 3].map((n) => ({ pick: "mode", option: String(n) })),
      );
      s.answer("3");
      s.answer("damage");
      s.answer([{ pick: "hero", player: "p2" }]);

      // Efficiency Dividend's damage mode deals X = 3, and the cast paid nothing for its X.
      s.expectHealth("p2", 27);
      s.expectMana("p1", 3);
    });
  });

  describe("radiant", () => {
    it("R386 its limit is (2): a (2) Cost Spell may be exiled and cast", () => {
      const s = setup({ hand: [{ def: SCEPTER, radiant: true }, CALL, FILLER], library: [TIMMY, TIMMY, VANILLA] }, true);

      s.play(SCEPTER, { zone: 1, targets: pick(s.card(CALL)) });
      s.expectInZone(CALL, "exile");
      s.activate(SCEPTER);

      // Call to Arms recruits the (1) Cost Units from the top of the deck.
      expect(count(s.lastEvents, "summoned")).toBeGreaterThan(0);
    });

    it("a (3) Cost Spell is still no pick for it", () => {
      const s = setup({ hand: [{ def: SCEPTER, radiant: true }, HIT_JOB, FILLER] }, true);

      expect(() => s.play(SCEPTER, { zone: 1, targets: pick(s.card(HIT_JOB)) })).toThrow();
    });

    it("R386 a Degrade of the Radiant limit to (1) refuses a (2) Cost Spell", () => {
      const s = setup({ hand: [{ def: SCEPTER, radiant: true }, CALL, FILLER] }, true);
      stepParam(s.card(SCEPTER), "costLimit", -1);

      expect(() => s.play(SCEPTER, { zone: 1, targets: pick(s.card(CALL)) })).toThrow();
      s.play(SCEPTER, { zone: 1, targets: pick(s.card(FILLER)) });
      s.expectInZone(FILLER, "exile");
    });

    it("the Radiant face still picks from your hand only", () => {
      const s = setup({ hand: [{ def: SCEPTER, radiant: true }, FILLER], graveyard: [STOCKPILE] }, true);
      const grave = s.pile("p1", "graveyard")[0];
      if (grave === undefined) throw new Error("fixture");

      expect(() => s.play(SCEPTER, { zone: 1, targets: pick(grave) })).toThrow();
    });
  });
});
