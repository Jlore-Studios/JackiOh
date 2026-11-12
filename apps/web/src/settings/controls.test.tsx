// The panel's pure control helpers (`./controls.tsx`, R435): the speed slider's readout.
// The switches themselves are proved by `wiring.test.tsx` on a board; here the wording a
// mutant would change is pinned.

import { describe, expect, it } from "vitest";

import { speedLabel } from "./controls.tsx";

describe("speedLabel", () => {
  it("reads the speed to hundredths with a times sign", () => {
    expect(speedLabel(1)).toBe("1×");
    expect(speedLabel(0.25)).toBe("0.25×");
    expect(speedLabel(3)).toBe("3×");
  });

  it("rounds the readout, never the slider's value", () => {
    expect(speedLabel(1.005)).toBe("1×");
    expect(speedLabel(2.5)).toBe("2.5×");
  });
});
