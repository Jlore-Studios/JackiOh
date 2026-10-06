// C+ #19 League of Losers — SPEC §8.7 row 19, BUILD M9 Classic+ row C+ 19: "Summons Top, Jungle, Mid,
// Support and Bot Loser (C+ #19.1–#19.5) into your unit zones 1 to 5 in that order, each aimed at its
// own zone: an occupied, Locked or reserved zone is skipped and that Loser is not summoned elsewhere;
// Mid Loser's Cry fires when this summons it (R411); a full board summons nothing; a Loser summoned by
// any other effect (a copy, a Rollback's recreation, Frostspatula's copies) fires no Cry (R1); radiant
// all five Radiant".

import { createRng, lockZone, reserveZone } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/019-league-of-losers";

const LEAGUE = "classicplus-019";
const FIVE = ["classicplus-019-1", "classicplus-019-2", "classicplus-019-3", "classicplus-019-4", "classicplus-019-5"];
const MID = "classicplus-019-3";
const BODY = "core-008"; // 4/4
const CUBE = "core-022";
const HIT_JOB = "core-016";
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER];

function league(p1: SideSetup = {}, radiantFace = false, seed?: string): Scenario {
  return scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: { library: DECK, ...p1, hand: [{ def: LEAGUE, radiant: radiantFace }, FILLER, ...(p1.hand ?? [])] },
    p2: { hand: [FILLER], library: DECK },
  });
}

function row(s: Scenario): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null);
}

/** The Mid Loser's Cry is the first draw the play makes: heads wanted or tails. */
function seededFor(heads: boolean, p1: SideSetup = {}, radiantFace = false): Scenario {
  for (let n = 1; n <= 200; n += 1) {
    const s = league(p1, radiantFace, `league-${n}`);
    if (createRng(s.state.seed, s.state.rngCursor).coin() === heads) return s;
  }
  throw new Error("no seed");
}

describe("C+ #19 League of Losers", () => {
  it("is a (4) Legendary Spell with no play-time choice", () => {
    expect(def.cost).toBe(4);
    expect(def.rarity).toBe("Legendary");
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
    expect(def.refs).toEqual(FIVE);
  });

  describe("base", () => {
    it("summons the five-stack into unit zones 1 to 5 in order, each on its base face, summoning sick", () => {
      const s = league();
      s.play(LEAGUE);
      expect(row(s)).toEqual(FIVE);
      for (let lane = 1; lane <= 5; lane += 1) {
        const unit = s.unit("p1", lane);
        expect(unit?.radiant).toBe(false);
        expect(unit?.summonedTurn).toBe(s.state.turn);
      }
      const order = s.events.flatMap((event) => (event.type === "summoned" ? [event.defId] : []));
      expect(order).toEqual(FIVE);
    });

    it("R411 Mid Loser's Cry fires when this summons it: heads", () => {
      const s = seededFor(true);
      s.play(LEAGUE);
      s.expectStats(s.unit("p1", 3) ?? "", { attack: 10, health: 10 });
      // Only the Mid Loser was buffed: the other four print no Cry.
      for (const lane of [1, 2, 4, 5]) expect(s.card(s.unit("p1", lane) ?? "").buffs).toEqual({ attack: 0, health: 0 });
    });

    it("R411 Mid Loser's Cry fires when this summons it: tails, and the opponent's next refresh is 1 higher", () => {
      const s = seededFor(false);
      s.play(LEAGUE);
      s.expectStats(s.unit("p1", 3) ?? "", { attack: 2, health: 2 });
      expect(s.state.players.p2.mana.nextTurnMod).toBe(1);
    });

    it("R411 the Cry runs right after the Mid Loser's own summon, before Support Loser's", () => {
      const s = league();
      s.play(LEAGUE);
      const types = s.lastEvents.flatMap((event) =>
        event.type === "summoned" ? [`summoned:${event.defId}`] : event.type === "buffed" ? ["buffed"] : [],
      );
      expect(types.indexOf("buffed")).toBe(types.indexOf(`summoned:${MID}`) + 1);
    });

    it("§3.2 an occupied zone is skipped and its Loser is not summoned elsewhere", () => {
      const s = league({ field: [{ def: BODY, lane: 2 }] });
      s.play(LEAGUE);
      expect(row(s)).toEqual([FIVE[0], BODY, FIVE[2], FIVE[3], FIVE[4]]);
    });

    it("R688 a Locked zone is skipped — the card's own override of the Locked-takes-summons rule", () => {
      const s = league();
      lockZone(s.state, { player: "p1", row: "units", lane: 4 });
      s.play(LEAGUE);
      expect(row(s)).toEqual([FIVE[0], FIVE[1], FIVE[2], null, FIVE[4]]);
    });

    it("R64 a reserved zone is skipped", () => {
      const s = league();
      reserveZone(s.state, { player: "p1", row: "units", lane: 5 });
      s.play(LEAGUE);
      expect(row(s)).toEqual([FIVE[0], FIVE[1], FIVE[2], FIVE[3], null]);
    });

    it("R411 with Mid Loser's zone taken there is no Mid Loser and no Cry", () => {
      const s = league({ field: [{ def: MID, lane: 3 }] });
      const standing = s.unit("p1", 3);
      const cursor = s.state.rngCursor;
      s.play(LEAGUE);
      expect(s.unit("p1", 3)?.id).toBe(standing?.id);
      s.expectStats(standing ?? "", { attack: 5, health: 5 });
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("a full board summons nothing", () => {
      const s = league({ field: [BODY, BODY, BODY, BODY, BODY] });
      s.play(LEAGUE);
      expect(row(s)).toEqual([BODY, BODY, BODY, BODY, BODY]);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
    });

    it("R1 a Loser summoned by any other effect fires no Cry: Carnivorous Cube's copies of a Mid Loser", () => {
      const s = league({ hand: [CUBE, HIT_JOB, HIT_JOB], mana: 20 });
      s.play(LEAGUE);
      const mid = s.unit("p1", 3);
      const top = s.unit("p1", 1);
      if (mid === null || top === null) throw new Error("setup");
      // Free lane 1 for the Cube, which eats the Mid Loser; then destroy the Cube, whose Death summons
      // two copies of it.
      s.play(s.hand("p1").find((card) => card.defId === HIT_JOB) ?? HIT_JOB, { targets: [{ pick: "instance", instanceId: top.id }] });
      s.play(CUBE, { zone: 1, targets: [{ pick: "instance", instanceId: mid.id }] });
      const cube = s.unit("p1", 1);
      if (cube === null) throw new Error("no Cube");
      const cursor = s.state.rngCursor;
      const refresh = s.state.players.p2.mana.nextTurnMod;
      s.play(s.hand("p1").find((card) => card.defId === HIT_JOB) ?? HIT_JOB, { targets: [{ pick: "instance", instanceId: cube.id }] });
      const copies = [1, 2, 3, 4, 5].flatMap((lane) => {
        const unit = s.unit("p1", lane);
        return unit !== null && unit.defId === MID ? [unit] : [];
      });
      expect(copies).toHaveLength(2);
      // No coin was flipped, nothing was buffed, and the opponent's refresh is as it was.
      expect(s.state.rngCursor).toBe(cursor);
      for (const copy of copies) expect(s.card(copy).buffs).toEqual({ attack: 0, health: 0 });
      expect(s.state.players.p2.mana.nextTurnMod).toBe(refresh);
    });

    it("R11 the Losers are tokens: one that leaves the field ceases to exist", () => {
      const s = league({ hand: [HIT_JOB], mana: 20 });
      s.play(LEAGUE);
      const top = s.unit("p1", 1);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: top?.id ?? "" }] });
      s.expectInZone(top ?? "", "gone");
    });
  });

  describe("radiant", () => {
    it("summons all five Radiant", () => {
      const s = league({}, true);
      s.play(LEAGUE);
      expect(row(s)).toEqual(FIVE);
      for (let lane = 1; lane <= 5; lane += 1) expect(s.unit("p1", lane)?.radiant).toBe(true);
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 10, health: 10 });
    });

    it("R411 the Radiant Mid Loser's Cry fires with its Lucky 1: two flips", () => {
      const s = league({}, true);
      const cursor = s.state.rngCursor;
      s.play(LEAGUE);
      expect(s.state.rngCursor - cursor).toBe(2);
    });
  });
});
