// `redact` and last boards (R185, R417): a seat's last board is that player's own, as C+ #29's options
// reach only their chooser (§10.8), so the AI keeps its own seat's and never reads the other's.

import { describe, expect, it } from "vitest";
import { hashState } from "@jackioh/engine";
import { redact } from "../src/index";
import { AI, HUMAN, scenario } from "./_support";

const MINE = [{ defId: "core-012", radiant: true }];
const THEIRS = [{ defId: "core-025", radiant: false }];

describe("redact keeps only the seat's own last board (R185, R417)", () => {
  it("R417 the other seat's board is gone, the seat's own stays", () => {
    const state = scenario({ active: AI, lastBoards: [MINE, THEIRS] }).state;
    expect(redact(state, AI).lastBoards).toEqual({ [AI]: MINE });
    expect(redact(state, HUMAN).lastBoards).toEqual({ [HUMAN]: THEIRS });
  });

  it("R185 two states that differ only in the other seat's board redact to the same hash", () => {
    const one = scenario({ active: AI, lastBoards: [MINE, THEIRS] }).state;
    const other = scenario({ active: AI, lastBoards: [MINE, []] }).state;
    expect(hashState(redact(one, AI))).toBe(hashState(redact(other, AI)));
    expect(redact(scenario({ active: AI, lastBoards: [[], THEIRS] }).state, AI).lastBoards).toBeUndefined();
  });
});
