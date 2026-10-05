// C #50 Voidwalker — SPEC §8.6 row 50, BUILD M9 Classic row C 50: "Cry: exile every card in both
// graveyards; Aura, while it is on the field: every card that would go to a graveyard (a death, a
// discard, a resolved Spell, a fired trap, a burn) is exiled instead; unit tokens still cease to exist
// (R11); its own card goes to its owner's graveyard when it dies, its aura having left with it (R398);
// radiant: exile only the opponent's graveyard, and only cards the opponent owns are exiled instead,
// judged by owner, so a stolen Unit of theirs dying on your side is exiled and your own cards reach
// your graveyard; no tuned numbers".
//
// The moves come from Core cards with their own tests: Hit Job destroys a Unit (a death, and a Spell
// that resolves), Zao Gao discards 2 at random and summons 2 Rush Tokens, Stockpile draws into a full
// hand (a burn), Sheepish is a Trap that fires on a played Unit and is spent, Snom Bunny Mind Control
// steals a permanent, and Felinor Fiender's Stack buries a card beneath it.
//
// A scenario places its graveyards after its field, through the engine's own moves, so a Voidwalker
// already on the field would exile them as they were laid; the Cry's cases play it from hand instead.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/050-voidwalker";

const VOID = "classic-050";
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit.
const ZAO_GAO = "core-080"; // (2) Spell: discard 2 random cards; summon 2 Rush Tokens with 2 random keywords.
const STOCKPILE = "core-005"; // (1) Spell: draw 2, heal your hero 2.
const SHEEPISH = "core-041"; // (1) Trap: when your opponent plays a Unit, after its Cry, transform it into a Sheep.
const MIND_CONTROL = "core-049"; // (4) Spell: steal target enemy permanent.
const FIENDER = "core-092"; // (2) Unit, Stack.
const VANILLA = "core-008"; // (1) Unit 4/4
const GARY = "core-004"; // (1) Unit 1/1
const LUNAR = "core-035"; // (1) Spell: deal 3 damage to a target.
const DEFENDER = "core-003"; // (1) Unit 1/1, Reborn; Radiant: Death: summon a base Right-house defender.
const RUSH_TOKEN = "core-t-rush";
const FILLER = "core-005";

function defIds(s: Scenario, player: PlayerId, zone: "graveyard" | "exile"): string[] {
  return s.pile(player, zone).map((card) => card.defId);
}

function target(s: Scenario, player: PlayerId, lane: number): { pick: "instance"; instanceId: string }[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

describe("C #50 Voidwalker", () => {
  it("declares a Cry and one replacement into exile on each face, and no numbers", () => {
    expect(def.id).toBe(VOID);
    expect(def.params).toBeUndefined();
    expect(base.replacements).toEqual([{ id: "voidwalker", on: "toGraveyard", instead: { to: "exile" } }]);
    expect(radiant.replacements?.map((each) => [each.on, each.instead])).toEqual([["toGraveyard", { to: "exile" }]]);
  });

  describe("base", () => {
    it("Cry: exiles every card in both graveyards, each to its owner's exile", () => {
      const s = scenario({
        p1: { hand: [VOID, FILLER], graveyard: [STOCKPILE, VANILLA] },
        p2: { hand: [FILLER], graveyard: [HIT_JOB, GARY] },
      });
      s.play(VOID);
      expect(defIds(s, "p1", "graveyard")).toEqual([]);
      expect(defIds(s, "p2", "graveyard")).toEqual([]);
      expect(defIds(s, "p1", "exile")).toEqual([STOCKPILE, VANILLA]);
      expect(defIds(s, "p2", "exile")).toEqual([HIT_JOB, GARY]);
      expect(s.events.filter((event) => event.type === "exiled")).toHaveLength(4);
      s.expectInZone(VOID, "field");
    });

    it("Cry: with both graveyards empty it exiles nothing, and Voidwalker still enters", () => {
      const s = scenario({ p1: { hand: [VOID, FILLER] }, p2: { hand: [FILLER] } });
      s.play(VOID);
      expect(s.events.filter((event) => event.type === "exiled")).toEqual([]);
      s.expectInZone(VOID, "field").expectStats(VOID, { attack: 6, health: 3 });
    });

    it("Aura: a Unit that dies is exiled, and so is the Spell that killed it once it resolves", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [VOID] }, p2: { hand: [FILLER], field: [VANILLA] } });
      const vanilla = s.card(VANILLA);
      const hitJob = s.card(HIT_JOB);
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      s.expectInZone(vanilla, "exile").expectInZone(hitJob, "exile");
      expect(defIds(s, "p2", "exile")).toEqual([VANILLA]);
      expect(defIds(s, "p1", "exile")).toEqual([HIT_JOB]);
      expect(defIds(s, "p1", "graveyard")).toEqual([]);
      expect(defIds(s, "p2", "graveyard")).toEqual([]);
    });

    it("Aura: your opponent's cards too — their Spell is exiled as it resolves", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [VOID] }, p2: { hand: [STOCKPILE, FILLER], library: [FILLER, FILLER] }, active: "p2" });
      const stockpile = s.card(STOCKPILE);
      s.play(stockpile);
      s.expectInZone(stockpile, "exile");
    });

    it("Aura: discarded cards are exiled", () => {
      const s = scenario({ p1: { hand: [ZAO_GAO, LUNAR, GARY], field: [VOID] }, p2: { hand: [FILLER] } });
      const [lunar, gary] = [s.card(LUNAR), s.card(GARY)];
      s.play(ZAO_GAO);
      s.expectInZone(lunar, "exile").expectInZone(gary, "exile").expectInZone(ZAO_GAO, "exile");
      expect(defIds(s, "p1", "graveyard")).toEqual([]);
    });

    it("Aura: a card burned at the hand cap is exiled (R317)", () => {
      const nine = Array.from({ length: 9 }, () => FILLER);
      const s = scenario({ p1: { hand: [STOCKPILE, ...nine], field: [VOID], library: [LUNAR, GARY] }, p2: { hand: [FILLER] } });
      s.play(STOCKPILE);
      expect(s.events.some((event) => event.type === "burned")).toBe(true);
      s.expectInZone(GARY, "exile");
      expect(s.hand("p1")).toHaveLength(10);
      expect(defIds(s, "p1", "graveyard")).toEqual([]);
    });

    it("Aura: a Trap spent after it fires is exiled", () => {
      const s = scenario({
        p1: { hand: [VANILLA, FILLER], field: [VOID] },
        p2: { hand: [FILLER], backrow: [{ def: SHEEPISH, faceUp: false }] },
      });
      const sheepish = s.card(SHEEPISH);
      s.play(VANILLA, { zone: 2 });
      s.expectEvents("trapFired");
      s.expectInZone(sheepish, "exile");
      expect(defIds(s, "p2", "graveyard")).toEqual([]);
    });

    it("R11 a unit token still ceases to exist: it reaches neither a graveyard nor an exile", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [VOID] }, p2: { hand: [FILLER], field: [RUSH_TOKEN] } });
      const token = s.card(RUSH_TOKEN);
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      s.expectInZone(token, "gone");
      expect(defIds(s, "p2", "exile")).toEqual([]);
    });

    it("a Reborn unit the aura exiles has not died: no Reborn body comes back and its Death hook does not fire", () => {
      const s = scenario({
        p1: { hand: [HIT_JOB, FILLER], field: [VOID] },
        p2: { hand: [FILLER], field: [{ def: DEFENDER, radiant: true }] },
      });
      const defender = s.card(DEFENDER);
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      s.expectInZone(defender, "exile");
      expect([1, 2, 3, 4, 5].map((lane) => s.unit("p2", lane))).toEqual([null, null, null, null, null]);
      expect(s.events.some((event) => event.type === "destroyed" || event.type === "summoned")).toBe(false);
    });

    it("R398 its own card reaches its owner's graveyard when it dies: its aura left the field with it", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [VOID] }, p2: { hand: [HIT_JOB, FILLER] }, active: "p2" });
      const voidwalker = s.card(VOID);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: voidwalker.id }] });
      s.expectInZone(voidwalker, "graveyard");
      expect(defIds(s, "p1", "graveyard")).toEqual([VOID]);
      expect(defIds(s, "p1", "exile")).toEqual([]);
    });

    it("R398 ... and once it has gone, cards reach the graveyard again", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [VOID] },
        p2: { hand: [HIT_JOB, STOCKPILE, FILLER], library: [FILLER, FILLER] },
        active: "p2",
      });
      s.play(HIT_JOB, { targets: target(s, "p1", 1) });
      s.play(STOCKPILE);
      expect(defIds(s, "p2", "graveyard")).toEqual([HIT_JOB, STOCKPILE]);
    });

    it("R398 a second Voidwalker still on the field exiles the first one's card as it dies", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [VOID, VOID] }, p2: { hand: [HIT_JOB, FILLER] }, active: "p2" });
      const first = s.unit("p1", 1);
      if (first === null) throw new Error("the first Voidwalker should be on the board");
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: first.id }] });
      s.expectInZone(first, "exile");
      s.expectInZone(HIT_JOB, "exile");
    });

    it("§3.2 a Voidwalker dormant under a Stack pile is not on the field: cards reach the graveyard", () => {
      const s = scenario({
        p1: { hand: [HIT_JOB, FILLER], field: [VOID, { def: FIENDER, stack: true }] },
        p2: { hand: [FILLER], field: [VANILLA] },
      });
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      expect(defIds(s, "p2", "graveyard")).toEqual([VANILLA]);
      expect(defIds(s, "p1", "graveyard")).toEqual([HIT_JOB]);
    });

    it("in a hand it replaces nothing: the Aura is the card on the field's", () => {
      const s = scenario({ p1: { hand: [VOID, HIT_JOB, FILLER] }, p2: { hand: [FILLER], field: [VANILLA] } });
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      expect(defIds(s, "p2", "graveyard")).toEqual([VANILLA]);
      expect(defIds(s, "p1", "graveyard")).toEqual([HIT_JOB]);
    });

    it("§10.8 an exiled card is public: both players read it", () => {
      const s = scenario({ p1: { hand: [ZAO_GAO, LUNAR, GARY], field: [VOID] }, p2: { hand: [FILLER] } });
      s.play(ZAO_GAO);
      const seen = s.view("p2").opponent.exile.map((card) => card.defId);
      expect(seen).toEqual(expect.arrayContaining([LUNAR, GARY, ZAO_GAO]));
    });
  });

  describe("radiant", () => {
    it("Cry: exiles only your opponent's graveyard; yours stays", () => {
      const s = scenario({
        p1: { hand: [{ def: VOID, radiant: true }, FILLER], graveyard: [STOCKPILE, VANILLA] },
        p2: { hand: [FILLER], graveyard: [HIT_JOB, GARY] },
      });
      s.play(VOID);
      expect(defIds(s, "p1", "graveyard")).toEqual([STOCKPILE, VANILLA]);
      expect(defIds(s, "p2", "graveyard")).toEqual([]);
      expect(defIds(s, "p2", "exile")).toEqual([HIT_JOB, GARY]);
      s.expectStats(VOID, { attack: 12, health: 6 });
    });

    it("Aura: only cards your opponent owns are exiled — their dead Unit is, your resolved Spell is not", () => {
      const s = scenario({
        p1: { hand: [HIT_JOB, FILLER], field: [{ def: VOID, radiant: true }] },
        p2: { hand: [FILLER], field: [VANILLA] },
      });
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      expect(defIds(s, "p2", "exile")).toEqual([VANILLA]);
      expect(defIds(s, "p1", "graveyard")).toEqual([HIT_JOB]);
    });

    it("Aura: judged by current owner — a Unit you stole is yours (R669), so its death reaches your graveyard", () => {
      const s = scenario({
        p1: { hand: [MIND_CONTROL, HIT_JOB, FILLER], field: [{ def: VOID, radiant: true }], mana: 7 },
        p2: { hand: [FILLER], field: [VANILLA] },
      });
      const vanilla = s.card(VANILLA);
      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      expect(s.card(vanilla).controller).toBe("p1");
      expect(s.card(vanilla).owner).toBe("p1");
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      s.expectInZone(vanilla, "graveyard");
      expect(defIds(s, "p2", "exile")).toEqual([]);
      expect(defIds(s, "p1", "graveyard")).toEqual([MIND_CONTROL, VANILLA, HIT_JOB]);
    });

    it("Aura: your own cards reach your graveyard — your Unit that dies, your discards", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VOID, radiant: true }, GARY] },
        p2: { hand: [HIT_JOB, FILLER] },
        active: "p2",
      });
      s.play(HIT_JOB, { targets: target(s, "p1", 2) });
      expect(defIds(s, "p1", "graveyard")).toEqual([GARY]);
      expect(defIds(s, "p2", "exile")).toEqual([HIT_JOB]);
    });

    it("Aura: your opponent's discards are exiled", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VOID, radiant: true }] },
        p2: { hand: [ZAO_GAO, LUNAR, GARY] },
        active: "p2",
      });
      s.play(ZAO_GAO);
      expect(defIds(s, "p2", "exile")).toEqual(expect.arrayContaining([LUNAR, GARY, ZAO_GAO]));
      expect(defIds(s, "p2", "graveyard")).toEqual([]);
    });

    it("R398 its own card reaches your graveyard when it dies", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VOID, radiant: true }] },
        p2: { hand: [HIT_JOB, FILLER] },
        active: "p2",
      });
      s.play(HIT_JOB, { targets: target(s, "p1", 1) });
      expect(defIds(s, "p1", "graveyard")).toEqual([VOID]);
    });

    it("R11 an enemy unit token still ceases to exist", () => {
      const s = scenario({
        p1: { hand: [HIT_JOB, FILLER], field: [{ def: VOID, radiant: true }] },
        p2: { hand: [FILLER], field: [RUSH_TOKEN] },
      });
      const token = s.card(RUSH_TOKEN);
      s.play(HIT_JOB, { targets: target(s, "p2", 1) });
      s.expectInZone(token, "gone");
      expect(defIds(s, "p2", "exile")).toEqual([]);
    });
  });

  it("events: every exile it causes is an `exiled` event naming the card", () => {
    const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [VOID] }, p2: { hand: [FILLER], field: [VANILLA] } });
    const vanilla = s.card(VANILLA);
    s.play(HIT_JOB, { targets: target(s, "p2", 1) });
    const exiled = s.events.filter((event): event is Extract<GameEvent, { type: "exiled" }> => event.type === "exiled");
    expect(exiled.map((event) => event.instanceId)).toContain(vanilla.id);
  });
});
