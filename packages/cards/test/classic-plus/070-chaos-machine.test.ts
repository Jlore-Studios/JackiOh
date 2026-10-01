// C+ #70 Chaos Machine — SPEC §8.7 row 70, BUILD M9 row C+ 70: at the start and at the end of its
// controller's turn it Upgrades one random card among their hand and side of the field and Degrades one
// random card among the opponent's (R386: one draw each; a pick on an Immutable card or one nothing fits
// changes nothing); a pick over a hand and a field is split by the piles' sizes (R242), and a hidden
// card's change reaches the other seat under the sentinel (R177); empty zones, nothing and no draw
// (R129); the count reads through `param()`; radiant two different cards each way. R585: its own side
// of the field includes Chaos Machine itself.

import { describe, expect, it } from "vitest";
import {
  HIDDEN_ID,
  applyEffects,
  createRng,
  makeContext,
  stepParam,
  type EngineSink,
} from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/070-chaos-machine";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const MACHINE = "classicplus-070";
const UNIT = "core-008"; // Mr. Vanilla 4/4.
const MENACE = "core-019"; // Midrange Menace; Radiant it is Immutable.
const FILLER = "core-005";
const NETHER = "core-088"; // Twisting Nether: a (4) Spell with no keywords and no declared numbers.

type Tune = Extract<GameEvent, { type: "upgraded" | "degraded" }>;

function tunes(events: readonly GameEvent[], type: "upgraded" | "degraded"): Tune[] {
  return events.filter((event): event is Tune => event.type === type);
}

function ownerOf(s: Scenario, id: string): PlayerId | null {
  for (const player of ["p1", "p2"] as const) {
    const side = s.state.players[player];
    if (side.hand.some((card) => card.id === id)) return player;
    if (side.units.some((pile) => pile?.[0]?.id === id)) return player;
    if (side.backrow.some((card) => card?.id === id)) return player;
  }
  return null;
}

function machine(p1: SideSetup, p2: SideSetup, opts: { radiant?: boolean; seed?: string } = {}): Scenario {
  return scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { ...p1, backrow: [{ def: MACHINE, radiant: opts.radiant === true }, ...(p1.backrow ?? [])] },
    p2,
  });
}

describe("C+ #70 Chaos Machine", () => {
  it("is a (2) Field Spell whose two hooks run one tick, on both faces", () => {
    expect(def).toMatchObject({ cost: 2, type: "Field Spell" });
    expect(base.startOfTurn).toBe(base.endOfTurn);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R62 R386 at the end of your turn: one Upgrade among your cards, one Degrade among theirs, each its own draw", () => {
      const s = machine({ hand: [FILLER], field: [UNIT] }, { hand: [FILLER], field: [UNIT] });
      s.endTurn();
      const up = tunes(s.lastEvents, "upgraded");
      const down = tunes(s.lastEvents, "degraded");
      expect(up).toHaveLength(1);
      expect(down).toHaveLength(1);
      expect(ownerOf(s, up[0]!.instanceId)).toBe("p1");
      expect(ownerOf(s, down[0]!.instanceId)).toBe("p2");
    });

    it("R62 at the start of your turn too, and never on the opponent's turn", () => {
      const s = machine({ hand: [FILLER, FILLER], field: [UNIT] }, { hand: [FILLER, FILLER], field: [UNIT] });
      s.endTurn();
      expect(s.state.active).toBe("p2");
      // p1's end of turn ticked once; p2's start of turn did not.
      expect(tunes(s.lastEvents, "upgraded")).toHaveLength(1);
      s.endTurn();
      expect(s.state.active).toBe("p1");
      // p2's end of turn: nothing. p1's start of turn: one pair.
      expect(tunes(s.lastEvents, "upgraded")).toHaveLength(1);
      expect(tunes(s.lastEvents, "degraded")).toHaveLength(1);
      expect(ownerOf(s, tunes(s.lastEvents, "degraded")[0]!.instanceId)).toBe("p2");
    });

    it("R585 your side of the field includes Chaos Machine itself: alone, it Upgrades itself", () => {
      const s = machine({}, { hand: [FILLER] });
      const self = s.backrow("p1", 1)!;
      s.startTurn();
      const up = tunes(s.lastEvents, "upgraded");
      expect(up.map((event) => event.instanceId)).toEqual([self.id]);
      expect(["cost", "number"]).toContain(up[0]!.change.kind);
    });

    it("R386 a pick on an Immutable card changes nothing and reports nothing", () => {
      const s = machine({ hand: [FILLER] }, { hand: [], field: [{ def: MENACE, radiant: true }] });
      const menace = s.unit("p2", 1)!;
      s.startTurn();
      expect(tunes(s.lastEvents, "degraded")).toEqual([]);
      expect(s.card(menace).tuning).toBeUndefined();
      expect(s.card(menace).costMod).toBe(0);
    });

    it("R386 R440 a pick on a card nothing fits changes nothing: their (4) Spell with no numbers is cued none, unchanged", () => {
      const s = machine({ hand: [FILLER], library: [FILLER] }, { hand: [NETHER] });
      const nether = s.hand("p2")[0]!;
      s.startTurn();
      const down = tunes(s.lastEvents, "degraded");
      expect(down).toHaveLength(1);
      expect(down[0]).toMatchObject({ instanceId: nether.id, change: { kind: "none" } });
      expect(s.card(nether).costMod).toBe(0);
      expect(s.card(nether).tuning).toBeUndefined();
    });

    it("§3.2 R13 a Stack pile offers only its top: the card dormant beneath is never picked", () => {
      for (let n = 0; n < 12; n += 1) {
        const s = machine(
          { hand: [FILLER], library: [FILLER] },
          { hand: [], field: [{ def: "core-043", lane: 1 }, { def: "core-092", stack: true }, { def: UNIT, lane: 2 }] },
          { seed: `chaos-stack-${n}` },
        );
        const dormant = s.state.players.p2.units[0]?.[1];
        if (dormant === undefined) throw new Error("a dormant card under the pile");
        s.startTurn();
        const [down] = tunes(s.lastEvents, "degraded");
        expect([s.unit("p2", 1)!.id, s.unit("p2", 2)!.id]).toContain(down?.instanceId);
        expect(s.card(dormant).tuning).toBeUndefined();
        expect(s.card(dormant).costMod).toBe(0);
      }
    });

    it("R242 a pick over a hand and a field takes either, by the piles' sizes alone", () => {
      const where = { hand: 0, field: 0 };
      for (let n = 0; n < 30; n += 1) {
        const s = machine({ hand: [FILLER] }, { hand: [FILLER], field: [UNIT] }, { seed: `chaos-split-${n}` });
        const unit = s.unit("p2", 1)!;
        s.startTurn();
        const [down] = tunes(s.lastEvents, "degraded");
        if (down?.instanceId === unit.id) where.field += 1;
        else where.hand += 1;
      }
      expect(where.field).toBeGreaterThan(5);
      expect(where.hand).toBeGreaterThan(5);
    });

    it("R177 the Degrade of an opponent's hand card reaches you under the sentinel; your hand's Upgrade reaches them so", () => {
      const s = machine({ hand: [FILLER, FILLER] }, { hand: [FILLER, FILLER] });
      s.endTurn();
      const down = tunes(s.view("p1").events, "degraded");
      expect(down).toHaveLength(1);
      expect(down[0]).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID });
      expect(tunes(s.view("p2").events, "degraded")[0]?.instanceId).not.toBe(HIDDEN_ID);
      let hidden = false;
      for (let n = 0; n < 20 && !hidden; n += 1) {
        const t = machine({ hand: [FILLER, FILLER] }, { hand: [FILLER, FILLER] }, { seed: `chaos-view-${n}` });
        const machineId = t.backrow("p1", 1)!.id;
        t.endTurn();
        const [up] = tunes(t.lastEvents, "upgraded");
        if (up === undefined || up.instanceId === machineId) continue;
        expect(tunes(t.view("p2").events, "upgraded")[0]).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID });
        expect(tunes(t.view("p1").events, "upgraded")[0]?.instanceId).toBe(up.instanceId);
        hidden = true;
      }
      expect(hidden).toBe(true);
    });

    it("R129 an opponent with no hand and no field takes no Degrade and no draw", () => {
      const s = machine({ hand: [FILLER] }, { hand: [] });
      const state = s.state;
      const self = s.backrow("p1", 1)!;
      const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
      const ctx = makeContext(sink, self, { controller: "p1" });
      const [, degradeTheirs] = base.endOfTurn!(ctx);
      const cursor = sink.rng.cursor;
      applyEffects([degradeTheirs!], ctx);
      expect(sink.rng.cursor).toBe(cursor);
      expect(sink.events).toEqual([]);
    });

    it("R386 the count reads through param(): an Upgrade of `cards` makes it two each way", () => {
      const s = machine({ hand: [FILLER, FILLER], field: [UNIT] }, { hand: [FILLER, FILLER], field: [UNIT] });
      stepParam(s.backrow("p1", 1)!, "cards", 1);
      s.endTurn();
      expect(tunes(s.lastEvents, "upgraded")).toHaveLength(2);
      expect(tunes(s.lastEvents, "degraded")).toHaveLength(2);
    });
  });

  describe("radiant", () => {
    it("R60 two different cards each way, at the end and at the start of your turn", () => {
      const s = machine(
        { hand: [FILLER, FILLER], field: [UNIT] },
        { hand: [FILLER, FILLER], field: [UNIT, UNIT] },
        { radiant: true },
      );
      s.endTurn();
      for (const type of ["upgraded", "degraded"] as const) {
        const ids = tunes(s.lastEvents, type).map((event) => event.instanceId);
        expect(ids).toHaveLength(2);
        expect(new Set(ids).size).toBe(2);
      }
      s.endTurn();
      expect(tunes(s.lastEvents, "upgraded")).toHaveLength(2);
      expect(tunes(s.lastEvents, "degraded")).toHaveLength(2);
    });

    it("R129 R60 with one card on a side, that one card only", () => {
      const s = machine({ hand: [FILLER] }, { hand: [FILLER] }, { radiant: true });
      s.startTurn();
      expect(tunes(s.lastEvents, "degraded")).toHaveLength(1);
      expect(tunes(s.lastEvents, "upgraded")).toHaveLength(2);
    });
  });
});
