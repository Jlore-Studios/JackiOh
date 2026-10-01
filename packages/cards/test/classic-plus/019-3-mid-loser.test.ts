// C+ #19.3 Mid Loser — SPEC §8.7 row 19.3, BUILD M9 Classic+ row C+ 19.3: "Cry, on a play from hand, a
// cast or League of Losers' summon (R1, R411), flips a seeded coin: heads +5/+5 permanently; tails
// −3/−3 permanently (a damaged Mid Loser can die at the state check) and the opponent's next mana
// refresh is 1 higher (`nextTurnMod`); a copy or a Rollback's recreation flips nothing; heads and tails
// amounts read through `param()`; radiant Lucky 1 (two flips, heads kept if either lands) and heads
// +10/+10".

import { addStep, createRng, stepParam, tuningOf } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/019-3-mid-loser";

const MID = "classicplus-019-3";
const LEAGUE = "classicplus-019";
const CUBE = "core-022"; // Cry: Tribute one of your other Units and remember it. Death: summon 2 copies of it.
const HIT_JOB = "core-016";
const TESLA = "classic-005"; // Field Trap: when your opponent summons a Unit, deal 4 damage to it
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER];

type Flip = "heads" | "tails";

/** The coin the next draw of the match rng lands, and with Lucky X the best of X + 1. */
function nextFlip(s: Scenario, lucky = 0): Flip {
  const rng = createRng(s.state.seed, s.state.rngCursor);
  let heads = rng.coin();
  for (let extra = 0; extra < lucky; extra += 1) heads = rng.coin() || heads;
  return heads ? "heads" : "tails";
}

/** A seed whose next flips land as wanted, for a scenario built by `make`. */
function seeded(make: (seed: string) => Scenario, want: (s: Scenario) => boolean): Scenario {
  for (let n = 1; n <= 200; n += 1) {
    const s = make(`mid-${n}`);
    if (want(s)) return s;
  }
  throw new Error("no seed lands the flips wanted");
}

function fromHand(radiantFace = false): (seed: string) => Scenario {
  return (seed) =>
    scenario({ seed, p1: { hand: [{ def: MID, radiant: radiantFace }, FILLER], library: DECK }, p2: { hand: [FILLER], library: DECK } });
}

function mid(s: Scenario): ReturnType<Scenario["card"]> {
  for (let lane = 1; lane <= 5; lane += 1) {
    const unit = s.unit("p1", lane);
    if (unit?.defId === MID) return unit;
  }
  throw new Error("no Mid Loser on the field");
}

describe("C+ #19.3 Mid Loser", () => {
  it("is a (2) 5/5 Unit token printed Legendary whose Radiant face prints Lucky 1; one script for both", () => {
    expect(def.token).toBe(true);
    expect(def.printedRarity).toBe("Legendary");
    expect(def.radiant.keywords).toEqual([{ kind: "Lucky", n: 1 }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R1 played from a hand, heads: +5/+5 permanently, and the opponent gains nothing", () => {
      const s = seeded(fromHand(), (each) => nextFlip(each) === "heads");
      s.play(MID);
      s.expectStats(mid(s), { attack: 10, health: 10, maxHealth: 10 });
      expect(s.state.players.p2.mana.nextTurnMod).toBe(0);
    });

    it("R1 played from a hand, tails: −3/−3 permanently and the opponent's next refresh is 1 higher", () => {
      const s = seeded(fromHand(), (each) => nextFlip(each) === "tails");
      const before = s.state.players.p2.mana.max;
      s.play(MID);
      s.expectStats(mid(s), { attack: 2, health: 2, maxHealth: 2 });
      expect(s.state.players.p2.mana.nextTurnMod).toBe(1);
      s.endTurn();
      expect(s.state.players.p2.mana.current).toBe(before + 1);
    });

    it("§4.5 tails on a damaged Mid Loser can kill it at the state check; heads saves it", () => {
      // The opponent's Tesla answers the summon with 4 damage: a 5/5 left at 1 health.
      const tesla = (seed: string): Scenario =>
        scenario({
          seed,
          p1: { hand: [MID, FILLER], library: DECK },
          p2: { hand: [FILLER], library: DECK, backrow: [{ def: TESLA, lane: 1, faceUp: false }] },
        });
      const tails = seeded(tesla, (each) => nextFlip(each) === "tails");
      const doomed = tails.card(MID);
      tails.play(MID);
      expect(tails.events.some((event) => event.type === "trapFired")).toBe(true);
      tails.expectInZone(doomed, "gone");
      expect(tails.state.players.p2.mana.nextTurnMod).toBe(1);

      const heads = seeded(tesla, (each) => nextFlip(each) === "heads");
      heads.play(MID);
      heads.expectStats(mid(heads), { attack: 10, health: 6, maxHealth: 10 });
    });

    it("R1 a copy flips nothing: Carnivorous Cube's copies of an eaten Mid Loser are plain 5/5s", () => {
      const s = scenario({
        p1: { hand: [CUBE, HIT_JOB, FILLER], field: [{ def: MID, lane: 1 }], mana: 6, library: DECK },
        p2: { hand: [FILLER], library: DECK },
      });
      const eaten = s.unit("p1", 1);
      if (eaten === null) throw new Error("setup");
      s.play(CUBE, { zone: 2, targets: [{ pick: "instance", instanceId: eaten.id }] });
      const cube = s.unit("p1", 2);
      if (cube === null) throw new Error("no cube");
      const cursor = s.state.rngCursor;
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: cube.id }] });
      const copies = [1, 2, 3, 4, 5].flatMap((lane) => (s.unit("p1", lane)?.defId === MID ? [s.unit("p1", lane)] : []));
      expect(copies).toHaveLength(2);
      for (const copy of copies) s.expectStats(copy ?? "", { attack: 5, health: 5 });
      expect(s.state.rngCursor).toBe(cursor);
      expect(s.state.players.p2.mana.nextTurnMod).toBe(0);
    });

    it("R411 League of Losers' summon fires it", () => {
      const s = seeded(
        (seed) => scenario({ seed, p1: { hand: [LEAGUE, FILLER], library: DECK }, p2: { hand: [FILLER], library: DECK } }),
        (each) => nextFlip(each) === "heads",
      );
      s.play(LEAGUE);
      s.expectStats(s.unit("p1", 3) ?? "", { attack: 10, health: 10 });
    });

    it("R386 heads and tails read through param: an Upgrade makes heads +6/+6 and tails −2/−2", () => {
      const heads = seeded(fromHand(), (each) => nextFlip(each) === "heads");
      stepParam(heads.card(MID), "heads", 1);
      heads.play(MID);
      heads.expectStats(mid(heads), { attack: 11, health: 11 });

      const tails = seeded(fromHand(), (each) => nextFlip(each) === "tails");
      stepParam(tails.card(MID), "tails", -1);
      tails.play(MID);
      tails.expectStats(mid(tails), { attack: 3, health: 3 });
    });

    it("draws one coin from the match rng", () => {
      const s = fromHand()("mid-one-coin");
      const cursor = s.state.rngCursor;
      s.play(MID);
      expect(s.state.rngCursor - cursor).toBe(1);
    });
  });

  describe("radiant", () => {
    it("§6.1 Lucky 1: tails then heads is heads, +10/+10", () => {
      const s = seeded(fromHand(true), (each) => {
        const rng = createRng(each.state.seed, each.state.rngCursor);
        return !rng.coin() && rng.coin();
      });
      s.play(MID);
      s.expectStats(mid(s), { attack: 20, health: 20 });
      expect(s.state.players.p2.mana.nextTurnMod).toBe(0);
    });

    it("§6.1 Lucky 1: two tails is tails, −3/−3 and a mana for the opponent", () => {
      const s = seeded(fromHand(true), (each) => nextFlip(each, 1) === "tails");
      s.play(MID);
      s.expectStats(mid(s), { attack: 7, health: 7 });
      expect(s.state.players.p2.mana.nextTurnMod).toBe(1);
    });

    it("§6.1 Lucky 1 flips twice", () => {
      const s = fromHand(true)("mid-two-coins");
      const cursor = s.state.rngCursor;
      s.play(MID);
      expect(s.state.rngCursor - cursor).toBe(2);
    });

    it("R386 Lucky is tuned like any numbered keyword: Lucky 2 flips three times", () => {
      const s = fromHand(true)("mid-three-coins");
      const tuning = tuningOf(s.card(MID));
      tuning.x = addStep(tuning.x, "Lucky", 1);
      const cursor = s.state.rngCursor;
      s.play(MID);
      expect(s.state.rngCursor - cursor).toBe(3);
    });
  });
});
