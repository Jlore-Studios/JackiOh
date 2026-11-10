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
