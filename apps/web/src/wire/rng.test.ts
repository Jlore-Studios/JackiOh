// The engine-compatible seeded stream (`./rng.ts`, §10.7, §9.3): `(seed, cursor)` reproduces
// the engine's `Rng::new` exactly, so the client and the module draw the same sequence.

import { describe, expect, it } from "vitest";

import { advanceTo, createRng } from "./rng.ts";

describe("createRng", () => {
  it("replays the same stream from the same seed", () => {
    const a = createRng("seed-1");
    const b = createRng("seed-1");
    expect([a.next(), a.next(), a.next()]).toEqual([b.next(), b.next(), b.next()]);
  });

  it("draws a different stream from another seed", () => {
    const a = createRng("seed-1");
    const b = createRng("seed-2");
    const as = [a.next(), a.next(), a.next()];
    const bs = [b.next(), b.next(), b.next()];
    expect(as).not.toEqual(bs);
  });

  it("draws every next in [0, 1) and moves the cursor one per draw", () => {
    const rng = createRng("cursor-check");
    expect(rng.cursor).toBe(0);
    const value = rng.next();
    expect(value).toBeGreaterThanOrEqual(0);
    expect(value).toBeLessThan(1);
    expect(rng.cursor).toBe(1);
  });

  it("starts a cursor where asked", () => {
    const fromZero = createRng("seed-1");
    const first = fromZero.next();
    void first;
    const second = fromZero.next();
    const fromOne = createRng("seed-1", 1);
    expect(fromOne.next()).toBe(second);
  });
});

// The engine's own unit tests (`crates/engine/src/rng.rs`, `mod tests`) pin these same values, so the two
// streams cannot drift apart without one side's test failing.
describe("SURFACE §6.3: draws bit for bit the engine's Rng::new", () => {
  it("draws the engine's first ten values for two seeds", () => {
    const golden = createRng("golden");
    expect(Array.from({ length: 10 }, () => golden.next())).toEqual([
      0.6569062923081219, 0.9212453747168183, 0.48674044315703213, 0.8353869873099029, 0.1645236941985786,
      0.9309415211901069, 0.44464601506479084, 0.746172602288425, 0.2840092333499342, 0.3314517338294536,
    ]);
    expect(golden.cursor).toBe(10);
    const fuzz = createRng("jackioh-fuzz-1");
    expect(Array.from({ length: 10 }, () => fuzz.next())).toEqual([
      0.14485335885547101, 0.2014636874664575, 0.7321288040839136, 0.5006692679598927, 0.01700649783015251,
      0.9128239972051233, 0.10607194644398987, 0.1391449123620987, 0.9416085947304964, 0.5141714666970074,
    ]);
  });

  it("hashes the seed's UTF-16 code units, an emoji as two", () => {
    const rng = createRng("x\u00e9\u2665\u{1f600}");
    expect([rng.next(), rng.next()]).toEqual([0.4544040390755981, 0.2282192155253142]);
  });

  it("draws the engine's ints, int(0) with no draw", () => {
    const rng = createRng("golden");
    expect([rng.int(10), rng.int(7), rng.int(1), rng.int(0), rng.int(100)]).toEqual([6, 6, 0, 0, 83]);
    expect(rng.cursor).toBe(4);
  });

  it("shuffles as the engine does, one draw per swap", () => {
    const rng = createRng("jackioh-fuzz-1", 5);
    expect(rng.shuffle([1, 2, 3, 4, 5, 6, 7, 8])).toEqual([2, 6, 4, 3, 5, 7, 1, 8]);
    expect(rng.cursor).toBe(12);
  });

  it("picks the engine's member, and from nothing picks nothing without a draw", () => {
    const rng = createRng("golden");
    expect(rng.pick([])).toBeUndefined();
    expect(rng.cursor).toBe(0);
    expect(rng.pick(["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"])).toBe("g");
    expect(rng.cursor).toBe(1);
  });

  it("lands the engine's coins: tails, tails, heads for golden's 0.66, 0.92, 0.49", () => {
    const rng = createRng("golden");
    expect([rng.coin(), rng.coin(), rng.coin()]).toEqual([false, false, true]);
  });

  it("holds a chance to strictly below p: a draw of exactly p fails it", () => {
    expect(createRng("golden").chance(0.6569062923081219)).toBe(false);
    expect(createRng("golden").chance(0.656906292308122)).toBe(true);
  });

  it("rolls 1 + X times for Lucky X and keeps the better", () => {
    const rng = createRng("golden");
    expect(rng.lucky(2, () => rng.int(100), (a, b) => Math.max(a, b))).toBe(92);
    expect(rng.cursor).toBe(3);
  });
});

describe("int", () => {
  it("draws below n and answers 0 for nothing to draw from", () => {
    const rng = createRng("int-seed");
    for (let i = 0; i < 50; i += 1) {
      const value = rng.int(5);
      expect(Number.isInteger(value)).toBe(true);
      expect(value).toBeGreaterThanOrEqual(0);
      expect(value).toBeLessThan(5);
    }
    expect(createRng("int-seed").int(0)).toBe(0);
    expect(createRng("int-seed").int(-3)).toBe(0);
  });
});

describe("pick and shuffle", () => {
  it("picks nothing from nothing and a member otherwise", () => {
    expect(createRng("pick-seed").pick([])).toBeUndefined();
    const list = ["a", "b", "c", "d"] as const;
    const picked = createRng("pick-seed").pick(list);
    expect(list).toContain(picked);
  });

  it("shuffles to the same order from the same seed, keeping every member", () => {
    const list = [1, 2, 3, 4, 5, 6];
    expect(createRng("shuffle-seed").shuffle(list)).toEqual(createRng("shuffle-seed").shuffle(list));
    expect(createRng("shuffle-seed").shuffle(list).sort()).toEqual([1, 2, 3, 4, 5, 6]);
    expect(list).toEqual([1, 2, 3, 4, 5, 6]);
  });
});

describe("coin, chance and lucky", () => {
  it("draws a boolean coin and a chance gated by p", () => {
    expect(typeof createRng("coin-seed").coin()).toBe("boolean");
    expect(createRng("chance-seed").chance(1)).toBe(true);
    expect(createRng("chance-seed").chance(0)).toBe(false);
  });

  it("keeps the best of the rolls by the caller's better", () => {
    const rolls = [1, 5, 3];
    let at = 0;
    const best = createRng("lucky-seed").lucky(2, () => rolls[at++] ?? 0, (a, b) => Math.max(a, b));
    expect(best).toBe(5);
    expect(at).toBe(3);
  });

  it("rolls once with no extra rolls asked", () => {
    let calls = 0;
    const best = createRng("lucky-zero").lucky(0, () => {
      calls += 1;
      return 7;
    }, (a, b) => Math.max(a, b));
    expect(best).toBe(7);
    expect(calls).toBe(1);
  });
});

describe("advanceTo", () => {
  it("draws the stream forward to the cursor", () => {
    const rng = createRng("advance-seed");
    const first = rng.next();
    void first;
    advanceTo(rng, 5);
    expect(rng.cursor).toBe(5);
    expect(rng.next()).toBe(createRng("advance-seed", 5).next());
  });

  it("stays where it is asked to stay and refuses to go back or off the integers", () => {
    const rng = createRng("advance-seed", 5);
    advanceTo(rng, 5);
    expect(rng.cursor).toBe(5);
    expect(() => advanceTo(rng, 4)).toThrow(/behind/u);
    expect(() => advanceTo(rng, 5.5)).toThrow(/behind/u);
  });
});
