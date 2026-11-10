// The engine adapter (`./engine.ts`, SURFACE §10.4): the client's door to the Rust engine.
// Every rule lives in the module; this file only names it and the `subsystems` tables the
// tests read. The full games that prove the door are `practice/driver.test.ts` and the
// tutorial suites; here the adapter's own shape is pinned, so a dropped re-export or an
// emptied table fails colocated with the source.

import { describe, expect, it } from "vitest";

import {
  beginGame,
  createGame,
  createRng,
  reduce,
  subsystems,
  viewFor,
} from "./engine.ts";

describe("re-exports", () => {
  it("names the engine's entry points and the seeded stream", () => {
    expect(typeof createGame).toBe("function");
    expect(typeof beginGame).toBe("function");
    expect(typeof reduce).toBe("function");
    expect(typeof viewFor).toBe("function");
    expect(typeof createRng).toBe("function");
  });
});

describe("subsystems", () => {
  it("reads the Call to Chaos tables the card text is checked against", () => {
    expect(subsystems.CHAOS_EFFECTS.length).toBeGreaterThan(0);
    expect(subsystems.CHAOS_PLUS_EFFECTS.length).toBeGreaterThan(0);
    expect(subsystems.CHAOS_MED_EFFECTS.length).toBeGreaterThan(0);
  });

  it("reads the Heroic Power table by name", () => {
    expect(subsystems.HERO_POWER_NAMES.length).toBeGreaterThan(0);
    expect(subsystems.HERO_POWERS.length).toBe(subsystems.HERO_POWER_NAMES.length);
  });

  it("returns the same tables on every read", () => {
    expect(subsystems.CHAOS_EFFECTS).toBe(subsystems.CHAOS_EFFECTS);
    expect(subsystems.HERO_POWERS).toBe(subsystems.HERO_POWERS);
  });
});
