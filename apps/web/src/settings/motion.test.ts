// Reduce motion (`./motion.ts`, R200, R201): the media query, the panel's switch, or the effects
// store's `motion: "reduce"` each behave exactly like the query. One `it` per source, with the test
// page's `setReducedMotion` switch for the query.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { setReducedMotion } from "../test/setup.ts";
import { __resetSettingsForTests, writeSettings } from "./store.ts";
import { REDUCED_MOTION_QUERY, prefersReducedMotion, reducedMotionNow } from "./motion.ts";

beforeEach(() => {
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

afterEach(() => {
  setReducedMotion(false);
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

describe("prefersReducedMotion", () => {
  it("asks the media query for the reduced-motion query", () => {
    setReducedMotion(true);
    expect(prefersReducedMotion()).toBe(true);
    setReducedMotion(false);
    expect(prefersReducedMotion()).toBe(false);
  });

  it("names the query it asks", () => {
    expect(REDUCED_MOTION_QUERY).toBe("(prefers-reduced-motion: reduce)");
  });
});

describe("reducedMotionNow", () => {
  it("is false when nothing asks for it", () => {
    expect(reducedMotionNow()).toBe(false);
  });

  it("follows the media query", () => {
    setReducedMotion(true);
    expect(reducedMotionNow()).toBe(true);
  });

  it("follows the panel's switch", () => {
    writeSettings({ reduceMotion: true });
    expect(reducedMotionNow()).toBe(true);
  });

  it("follows the effects store's reduce motion", () => {
    setFxSettings({ motion: "reduce" });
    expect(reducedMotionNow()).toBe(true);
  });

  it("takes a handed motion without reading the store", () => {
    setFxSettings({ motion: "reduce" });
    expect(reducedMotionNow({ motion: "system" })).toBe(false);
    expect(reducedMotionNow({ motion: "reduce" })).toBe(true);
  });
});
