// C+ #61 Bauble Bubble — SPEC §8.7 row 61, BUILD M9 Classic+ row C+ 61: "Field Spell with no effect
// while it sits; its Death fires as it goes from its backrow zone to your graveyard (§4.5: destroyed by
// Guy Att, Crushing Walls, Twisting Nether) and adds 2 Stockpiles (Core #5) that cost (0) to your hand;
// exile, a bounce or a Transform adds nothing; Carnivorous Cube can't eat it (R428: Units only); a full
// hand burns; hidden from the opponent (R97); the count reads through `param()`; radiant the
// Stockpiles are Radiant".

import { legalActions, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/061-bauble-bubble";

const BAUBLE = "classicplus-061";
const STOCKPILE = "core-005";
const NETHER = "core-088"; // (4) Spell: destroy all permanents
const JAMMED = "core-036"; // (1) Spell: destroy target backrow card, Lock its zone
const COLLATERAL = "core-034"; // (4) Spell: exile target permanent and a random card of the enemy deck
const TRANSMOGULATE = "core-083"; // (2) Spell: replace every card of yours with a random Legendary
const SILAS = "core-052"; // Radiant: cards crossing to the opponent bounce to their owner's hand
const CUBE = "core-022"; // Carnivorous Cube: its Cry tributes one of your other Units
const MIND_CONTROL = "core-049"; // (4) Spell: steal target enemy permanent
const VANILLA = "core-008";
const FILLER = "core-011"; // Tempo Timmy, a (1) Unit (never a Stockpile, so the adds read plainly)

function stockpiles(s: Scenario): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.filter(
    (event): event is Extract<GameEvent, { type: "addedToHand" }> =>
      event.type === "addedToHand" && event.player === "p1" && event.defId === STOCKPILE,
  );
}

function bubble(opts: { radiant?: boolean; hand: readonly (string | { def: string; radiant: boolean })[]; lane?: number; field?: readonly string[] }): Scenario {
  return scenario({
    p1: {
      backrow: [{ def: BAUBLE, ...(opts.radiant === true ? { radiant: true } : {}), ...(opts.lane === undefined ? {} : { lane: opts.lane }) }],
      hand: opts.hand,
      field: opts.field ?? [],
      library: [FILLER, FILLER, FILLER],
    },
    p2: { hand: [FILLER], field: [VANILLA], library: [FILLER, FILLER] },
  });
}

describe("C+ #61 Bauble Bubble", () => {
  it("is a (1) Field Spell, Fruit naming Stockpile", () => {
    expect(def.type).toBe("Field Spell");
    expect(def.tags).toEqual(["Fruit"]);
    expect(def.refs).toEqual([STOCKPILE]);
  });

  describe("base", () => {
    it("a bait card: nothing happens while it sits, across both players' turns", () => {
      const s = bubble({ hand: [FILLER] });
      s.endTurn().endTurn();
      expect(s.events.some((event) => event.type === "addedToHand" && event.defId === STOCKPILE)).toBe(false);
      s.expectInZone(BAUBLE, "field");
    });

    it("§4.5 destroyed by Twisting Nether, its Death adds 2 Stockpiles that cost (0)", () => {
      const s = bubble({ hand: [NETHER, FILLER] });
      s.play(NETHER);
      s.expectInZone(BAUBLE, "graveyard");
      const added = stockpiles(s).map((event) => s.card(event.instanceId));
      expect(added).toHaveLength(2);
      for (const card of added) {
        expect(card.zone.z).toBe("hand");
        expect(card.costOverride).toBe(0);
        expect(card.radiant).toBe(false);
      }
      s.expectEvents("destroyed", "addedToHand");
    });

    it("§4.5 destroyed on the opponent's turn by their Nether, the Stockpiles still go to your hand", () => {
      const s = scenario({
        active: "p2",
        p1: { backrow: [BAUBLE], hand: [FILLER], library: [FILLER] },
        p2: { hand: [NETHER, FILLER], library: [FILLER] },
      });
      s.play(NETHER);
      s.expectInZone(BAUBLE, "graveyard");
      expect(stockpiles(s)).toHaveLength(2);
      expect(s.hand("p2").some((card) => card.defId === STOCKPILE)).toBe(false);
    });

    it("§4.5 a targeted destroy of the backrow card (Magic Jammed) fires it too", () => {
      const s = bubble({ hand: [JAMMED, FILLER] });
      s.play(JAMMED, { targets: [{ pick: "instance", instanceId: s.card(BAUBLE).id }] });
      s.expectInZone(BAUBLE, "graveyard");
      expect(stockpiles(s)).toHaveLength(2);
    });

    it("§6.2 an exile adds nothing", () => {
      const s = bubble({ hand: [COLLATERAL, FILLER] });
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(BAUBLE).id }] });
      s.expectInZone(BAUBLE, "exile");
      expect(stockpiles(s)).toEqual([]);
    });

    it("§6.2 a bounce adds nothing (a Radiant Silly Silas rotates it across, so it bounces home)", () => {
      const s = bubble({ hand: [{ def: SILAS, radiant: true }, FILLER], lane: 5 });
      s.play(SILAS, { modes: ["right"] });
      s.expectInZone(BAUBLE, "hand");
      expect(s.events.some((event) => event.type === "bounced" && event.instanceId === s.card(BAUBLE).id)).toBe(true);
      expect(stockpiles(s)).toEqual([]);
    });

    it("§6.2 a Transform adds nothing (Transmogulate replaces it)", () => {
      const s = bubble({ hand: [TRANSMOGULATE, FILLER] });
      const id = s.card(BAUBLE).id;
      s.play(TRANSMOGULATE);
      expect(s.events.some((event) => event.type === "transformed")).toBe(true);
      expect(s.events.some((event) => event.type === "addedToHand" && event.defId === STOCKPILE)).toBe(false);
      s.expectInZone(id, "gone");
    });

    it("§6.2 a steal adds nothing: it changes sides and stays on the field", () => {
      const s = scenario({
        active: "p2",
        p1: { backrow: [BAUBLE], hand: [FILLER], library: [FILLER] },
        p2: { hand: [MIND_CONTROL, FILLER], library: [FILLER] },
      });
      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: s.card(BAUBLE).id }] });
      s.expectInZone(BAUBLE, "field");
      expect(s.card(BAUBLE).controller).toBe("p2");
      expect(s.events.some((event) => event.type === "addedToHand" && event.defId === STOCKPILE)).toBe(false);
    });

    it("R428 Carnivorous Cube can't eat it: Units only", () => {
      const s = bubble({ hand: [CUBE, FILLER], field: [VANILLA] });
      const cube = s.card(CUBE);
      const offered = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === cube.id ? (action.targets ?? []) : [],
      );
      const ids = offered.flatMap((pick) => (pick.pick === "instance" ? [pick.instanceId] : []));
      expect(ids).toContain(s.unit("p1", 1)?.id);
      expect(ids).not.toContain(s.card(BAUBLE).id);
      expect(() => s.play(CUBE, { targets: [{ pick: "instance", instanceId: s.card(BAUBLE).id }] })).toThrow();
    });

    it("§2.4 R4 a full hand burns what does not fit", () => {
      const s = bubble({ hand: [NETHER, ...Array.from({ length: 9 }, () => FILLER)] });
      s.play(NETHER);
      expect(stockpiles(s)).toHaveLength(1);
      const burned = s.lastEvents.filter((event) => event.type === "burned" && event.defId === STOCKPILE);
      expect(burned).toHaveLength(1);
    });

    it("R97 the opponent sees the adds under the sentinel", () => {
      const s = bubble({ hand: [NETHER, FILLER] });
      s.play(NETHER);
      const ids = stockpiles(s).map((event) => event.instanceId);
      const theirs = s.view("p2");
      for (const id of ids) expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
      const events = theirs.events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(events.length).toBeGreaterThanOrEqual(2);
      for (const event of events) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
    });

    it("R386 an Upgrade adds 3; a Degrade 1", () => {
      const up = bubble({ hand: [NETHER, FILLER] });
      stepParam(up.card(BAUBLE), "cards", 1);
      up.play(NETHER);
      expect(stockpiles(up)).toHaveLength(3);

      const down = bubble({ hand: [NETHER, FILLER] });
      stepParam(down.card(BAUBLE), "cards", -4);
      down.play(NETHER);
      expect(stockpiles(down)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("R74 its Death adds 2 Radiant Stockpiles that cost (0)", () => {
      const s = bubble({ radiant: true, hand: [NETHER, FILLER] });
      s.play(NETHER);
      const added = stockpiles(s).map((event) => s.card(event.instanceId));
      expect(added).toHaveLength(2);
      expect(added.every((card) => card.radiant && card.costOverride === 0)).toBe(true);
    });

    it("§6.2 the Radiant face is no Death on an exile either", () => {
      const s = bubble({ radiant: true, hand: [COLLATERAL, FILLER] });
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(BAUBLE).id }] });
      expect(stockpiles(s)).toEqual([]);
    });
  });
});
