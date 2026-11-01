// R2575000 (issue #2575): the practice AI's budget doubled, read from the WASM module's own
// constants, and the browser's wall-clock cap doubled with it, so the cap does not cut the doubled
// search short.

import { describe, expect, it } from "vitest";

import { AI_BUDGET, AI_GATE_BUDGET } from "@jackioh/ai";

import { PRACTICE_AI_CLOCK_MS } from "./config.ts";

describe("R2575000 the practice AI's doubled budget", () => {
  it("R2575000 the browser's AI_BUDGET is 1200 nodes and 300 lethal nodes in the shape of before", () => {
    expect(AI_BUDGET).toEqual({
      nodes: 1200,
      lethalNodes: 300,
      determinizations: 3,
      beamWidth: 4,
      rootBranching: 20,
      branching: 6,
      maxDepth: 8,
      finalists: 3,
    });
    expect(AI_GATE_BUDGET).toEqual(AI_BUDGET);
  });

  it("R2575000 caps one decision at 3 s", () => {
    expect(PRACTICE_AI_CLOCK_MS).toBe(3000);
  });
});
