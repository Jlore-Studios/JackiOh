// R185 and C+ #35 Rollback's history (R419): `state.boardHistory` holds whole instances of the field as
// each recent turn began — the opponent's face-down traps among them, and cards since gone to its hand —
// so `redact` drops it and the seat never reads a hidden card through it. Two true states that differ
// only in a snapshot's hidden card redact to the same hash. Real catalog, real dealt game; the AI is p1.

import { describe, expect, it } from "vitest";
import { hashState, placeOnField, newInstance, subsystems } from "@jackioh/engine";
import { redact } from "../src/index";
import { AI, clone, dealtGame, trapPool } from "./_support";

describe("R185 the AI's seat and the board history (R419)", () => {
  it("R185 redact drops the board history, so a snapshot's face-down trap never reaches the seat", () => {
    const state = dealtGame("ai-board-history");
    const trap = newInstance(state, trapPool()[0] ?? "", "p2", { z: "hand", player: "p2" });
    if (!placeOnField(state, trap, { player: "p2", row: "backrow", lane: 2 })) throw new Error("no zone");
    subsystems.recordBoardSnapshot(state);
    expect(state.boardHistory?.at(-1)?.sides.p2.backrow[1]?.defId).toBe(trap.defId);

    const seen = redact(state, AI);
    expect(seen.boardHistory).toBeUndefined();
    expect(state.boardHistory).toBeDefined();

    const other = clone(state);
    const copy = other.boardHistory?.at(-1)?.sides.p2.backrow[1];
    if (copy === null || copy === undefined) throw new Error("no snapshot copy");
    copy.defId = trapPool()[1] ?? copy.defId;
    expect(hashState(other)).not.toBe(hashState(state));
    expect(hashState(redact(other, AI))).toBe(hashState(seen));
  });
});
