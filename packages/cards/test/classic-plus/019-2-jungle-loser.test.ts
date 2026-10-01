// C+ #19.2 Jungle Loser — SPEC §8.7 row 19.2, BUILD M9 Classic+ row C+ 19.2: "At its controller's end of
// turn a seeded 25% roll; on success a forced attack (R53: no exertion, sickness ignored, the target
// strikes back) on a random enemy Unit it may attack under §4.2 step 2 (never a Can't be attacked
// Unit, a Top Loser only from its lane); an empty enemy board draws no roll (R129); if it destroys
// (R42) the enemy Unit in your Bot Loser's lane, your Bot Loser goes Berserk, and with no Bot Loser
// nothing more happens; the chance reads through `param()` (step 10%); radiant 50%, and that kill is
// credited to your Bot Loser instead: its "Whenever this destroys a Unit" trigger fires and the
// `destroyed` event names it as the killer".

import { createRng, setParam, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/019-2-jungle-loser";

const JUNGLE = "classicplus-019-2";
const BOT = "classicplus-019-5";
const TOP = "classicplus-019-1";
const VANILLA = "core-008"; // 4/4
const MROW = "core-086"; // 1/1, Can't attack; Death: take control of the Unit that destroyed this
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

/** p1's Jungle Loser in lane 2 (face as given), the rest of p1's field and p2's side. */
function jungle(p1Field: SideSetup["field"] = [], radiantFace = false, p2: SideSetup = {}, seed?: string): Scenario {
  return scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: { hand: [FILLER], library: DECK, field: [{ def: JUNGLE, lane: 2, radiant: radiantFace }, ...(p1Field ?? [])] },
    p2: { hand: [FILLER], library: DECK, ...p2 },
  });
}

function loser(s: Scenario): ReturnType<Scenario["card"]> {
  const unit = s.unit("p1", 2);
  if (unit === null || unit.defId !== JUNGLE) throw new Error("no Jungle Loser in lane 2");
  return unit;
}

/** Make the roll a certainty: the declared chance, tuned to its 100% bound. */
function always(s: Scenario): Scenario {
  setParam(loser(s), "chance", 100);
  return s;
}

function attacksBy(s: Scenario, id: string): Extract<GameEvent, { type: "attackDeclared" }>[] {
  return s.events.flatMap((event) => (event.type === "attackDeclared" && event.attackerId === id ? [event] : []));
}

describe("C+ #19.2 Jungle Loser", () => {
  it("is a (2) 5/5 Unit token printed Legendary that acts at its end of turn", () => {
    expect(def.token).toBe(true);
    expect(def.params?.[0]).toMatchObject({ key: "chance", base: 25, radiant: 50, step: 10, max: 100 });
    expect(base.endOfTurn).toBeDefined();
    expect(radiant.endOfTurn).toBeDefined();
  });

  describe("base", () => {
    it("R53 on a success it makes a forced attack on an enemy Unit: no exertion, sickness ignored, the target strikes back", () => {
      const s = always(jungle([], false, { field: [{ def: VANILLA, lane: 4 }] }));
      loser(s).summonedTurn = s.state.turn;
      const victim = s.unit("p2", 4);
      if (victim === null) throw new Error("setup");
      s.endTurn();
      expect(attacksBy(s, loser(s).id)).toEqual([expect.objectContaining({ targetId: victim.id, forced: true })]);
      s.expectInZone(victim, "graveyard");
      s.expectStats(loser(s), { health: 1 });
      expect(s.card(loser(s)).exertion.attacked).toBe(false);
    });

    it("the 25% roll: across seeds it attacks on some ends of turn and not on others, one draw each", () => {
      let attacked = 0;
      const tries = 24;
      for (let seed = 1; seed <= tries; seed += 1) {
        const s = jungle([], false, { field: [{ def: VANILLA, lane: 4 }] }, `jungle-roll-${seed}`);
        const rng = createRng(s.state.seed, s.state.rngCursor);
        const expected = rng.chance(0.25);
        s.endTurn();
        const made = attacksBy(s, loser(s).id).length > 0;
        expect(made).toBe(expected);
        if (made) attacked += 1;
      }
      expect(attacked).toBeGreaterThan(0);
      expect(attacked).toBeLessThan(tries);
    });

    it("R129 an empty enemy board draws no roll", () => {
      const s = jungle();
      const cursor = s.state.rngCursor;
      // The end of p1's turn rolls nothing; the next draw the rng makes is p2's own.
      s.endTurn();
      expect(attacksBy(s, loser(s).id)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("§4.2 step 2 a Top Loser is never drawn from another lane", () => {
      for (let seed = 1; seed <= 10; seed += 1) {
        const s = always(jungle([], false, { field: [{ def: TOP, lane: 1 }, { def: VANILLA, lane: 5 }] }, `jungle-top-${seed}`));
        const top = s.unit("p2", 1);
        s.endTurn();
        for (const attack of attacksBy(s, loser(s).id)) expect(attack.targetId).not.toBe(top?.id);
      }
    });

    it("§4.2 step 2 with only a Unit it may not attack, nothing is rolled or attacked", () => {
      const s = jungle([], false, { field: [{ def: TOP, lane: 1 }] });
      const cursor = s.state.rngCursor;
      s.endTurn();
      expect(attacksBy(s, loser(s).id)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R42 if it destroys the enemy Unit across from your Bot Loser, your Bot Loser goes Berserk", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], false, { field: [{ def: VANILLA, lane: 4 }] }));
      const bot = s.unit("p1", 4);
      s.endTurn();
      expect(s.card(bot ?? "").berserk).toBe(true);
      // The kill is the Jungle Loser's own on the base face.
      const death = s.events.find((event) => event.type === "destroyed");
      expect(death).toMatchObject({ killerId: loser(s).id });
      s.expectStats(bot ?? "", { attack: 5 });
    });

    it("a kill of a Unit in another lane sends nobody Berserk", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], false, { field: [{ def: VANILLA, lane: 1 }] }));
      s.endTurn();
      expect(s.unit("p2", 1)).toBeNull();
      expect(s.card(s.unit("p1", 4) ?? "").berserk).toBeUndefined();
    });

    it("with no Bot Loser nothing more happens", () => {
      const s = always(jungle([{ def: VANILLA, lane: 4 }], false, { field: [{ def: VANILLA, lane: 4 }] }));
      s.endTurn();
      expect(s.unit("p2", 4)).toBeNull();
      expect(s.events.some((event) => event.type === "marked")).toBe(false);
    });

    it("an attack that leaves the Unit across standing sends nobody Berserk", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], false, { field: [{ def: "core-019", lane: 4 }] }));
      s.endTurn();
      expect(s.card(s.unit("p1", 4) ?? "").berserk).toBeUndefined();
    });

    it("R386 the chance reads through param: a Degrade to 15% and an Upgrade to 35%", () => {
      for (const steps of [-1, 1]) {
        let hits = 0;
        for (let seed = 1; seed <= 20; seed += 1) {
          const s = jungle([], false, { field: [{ def: VANILLA, lane: 4 }] }, `jungle-tuned-${seed}`);
          stepParam(loser(s), "chance", steps);
          const expected = createRng(s.state.seed, s.state.rngCursor).chance((25 + 10 * steps) / 100);
          s.endTurn();
          const made = attacksBy(s, loser(s).id).length > 0;
          expect(made).toBe(expected);
          if (made) hits += 1;
        }
        expect(hits).toBeGreaterThanOrEqual(0);
      }
    });

    it("§9.3 the state after its attack is plain JSON", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], false, { field: [{ def: VANILLA, lane: 4 }] }));
      s.endTurn();
      expect(JSON.parse(JSON.stringify(s.state))).toEqual(s.state);
    });
  });

  describe("radiant", () => {
    it("the 50% roll", () => {
      for (let seed = 1; seed <= 16; seed += 1) {
        const s = jungle([], true, { field: [{ def: VANILLA, lane: 4 }] }, `jungle-radiant-${seed}`);
        const expected = createRng(s.state.seed, s.state.rngCursor).chance(0.5);
        s.endTurn();
        expect(attacksBy(s, loser(s).id).length > 0).toBe(expected);
      }
    });

    it("R412 the kill across from your Bot Loser is credited to it: the destroyed event names it and its trigger fires", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], true, { field: [{ def: VANILLA, lane: 4 }] }));
      const bot = s.unit("p1", 4);
      const victim = s.unit("p2", 4);
      if (bot === null || victim === null) throw new Error("setup");
      s.endTurn();
      const death = s.events.find((event) => event.type === "destroyed" && event.instanceId === victim.id);
      expect(death).toMatchObject({ killerId: bot.id });
      s.expectStats(bot, { attack: 10 });
      expect(s.card(bot).berserk).toBeUndefined();
    });

    it("R412 a kill in another lane stays the Jungle Loser's", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], true, { field: [{ def: VANILLA, lane: 1 }] }));
      s.endTurn();
      const death = s.events.find((event) => event.type === "destroyed");
      expect(death).toMatchObject({ killerId: loser(s).id });
      s.expectStats(s.unit("p1", 4) ?? "", { attack: 5 });
    });

    it("R42 R412 the credited kill is the Bot Loser's for every reader: \"Miss\" Mrow's Death takes control of it", () => {
      const s = always(jungle([{ def: BOT, lane: 4 }], true, { field: [{ def: MROW, lane: 4 }] }));
      const bot = s.unit("p1", 4);
      if (bot === null) throw new Error("setup");
      s.endTurn();
      expect(s.card(bot).controller).toBe("p2");
    });
  });
});
