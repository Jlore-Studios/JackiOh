// R650: a micro patch's version (`vA.B.Y`) is the newest shipped version plus the next letter.

import { describe, expect, it } from "vitest";
import { resolveVersion } from "../scripts/versions";

describe("resolveVersion", () => {
  it("R650 names a micro patch after the newest version, with the next letter", () => {
    expect(resolveVersion("v0.2.Y", ["v0.1.1", "v0.2.0", "v0.2.5"])).toBe("v0.2.5b");
    expect(resolveVersion("v0.2.Y", ["v0.2.5", "v0.2.7c"])).toBe("v0.2.7d");
    // The newest is the last entry in patches.json, whatever its name (R388), never the largest.
    expect(resolveVersion("v0.2.Y", ["v0.2.9", "v0.2.4"])).toBe("v0.2.4b");
  });

  it("leaves a version the designer named as it is", () => {
    expect(resolveVersion("v0.2.6", ["v0.2.5"])).toBe("v0.2.6");
    expect(resolveVersion("v0.2.5b", ["v0.2.5"])).toBe("v0.2.5b");
  });

  it("refuses what it cannot name", () => {
    expect(() => resolveVersion("v0.2.X", ["v0.2.5"])).toThrow(/designer picks the X/);
    expect(() => resolveVersion("v0.2.Y", ["v0.3.1"])).toThrow(/not a v0.2 one/);
    expect(() => resolveVersion("v0.2.Y", [])).toThrow(/none/);
    expect(() => resolveVersion("v0.2.Y", ["core-1"])).toThrow(/no vA.B.C form/);
    expect(() => resolveVersion("v0.2.Y", ["v0.2.5z"])).toThrow(/no letter left/);
  });
});
