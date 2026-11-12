// The feature-owned settings slots (`./slots.ts`, R669): one entry per section control,
// each with a unique id, a render and a reset. The panel integration that proves they mount
// is `wiring.test.tsx`; here the table's own shape is pinned.

import { describe, expect, it } from "vitest";

import { SETTINGS_SLOTS } from "./slots.ts";

describe("SETTINGS_SLOTS", () => {
  it("lists one entry per control with a unique id", () => {
    const ids = SETTINGS_SLOTS.map((slot) => slot.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain("haptics");
    expect(ids).toContain("fx");
    expect(ids).toContain("audio");
    expect(ids).toContain("language");
  });

  it("files every slot under a known section", () => {
    for (const slot of SETTINGS_SLOTS) {
      expect(["gameplay", "visuals", "audio"]).toContain(slot.section);
      expect(typeof slot.render).toBe("function");
    }
  });

  it("gives every slot a reset", () => {
    for (const slot of SETTINGS_SLOTS) expect(typeof slot.reset).toBe("function");
  });
});
