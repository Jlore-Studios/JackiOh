// R185 and B5 E21 (R447): a card dormant under a backrow pile is on the board, and a face-down one there
// is as hidden from the other seat as a face-down trap on top — `redact` makes it a placeholder,
// `determinize` fills it from the trap pool, and a carrier's Unit (R446) stays a public card. Real
// catalog, real dealt game; the AI is p1 as in every AI test.

import { describe, expect, it } from "vitest";
import { createRng, hashState, newInstance, placeOnField, query, type GameState } from "@jackioh/engine";
import { HIDDEN_DEF_ID, determinize, hiddenInstanceIds, redact } from "../src/index";
import { dealtGame, trapPool } from "./_support";

const FIELD_SPELL = (query({ set: "Core", type: ["Field Spell"] })[0]?.id ?? "") as string;

/** p2 sets `trapDef` in backrow lane 1 and tops it with a public Field Spell (a Stack card would). */
function pileGame(seed: string, trapDef: string): { state: GameState; trap: string; top: string } {
  const state = dealtGame(seed);
  const trap = newInstance(state, trapDef, "p2", { z: "hand", player: "p2" });
  if (!placeOnField(state, trap, { player: "p2", row: "backrow", lane: 1 })) throw new Error("no zone");
  const top = newInstance(state, FIELD_SPELL, "p2", { z: "hand", player: "p2" });
  if (!placeOnField(state, top, { player: "p2", row: "backrow", lane: 1 }, { stack: true })) throw new Error("no stack");
  return { state, trap: trap.id, top: top.id };
}

describe("R447 the AI's seat and a backrow pile", () => {
  it("R447 a face-down trap under the opponent's pile is hidden, placeholdered and resampled from the trap pool", () => {
    const traps = trapPool();
    const first = traps[0] ?? "";
    const { state, trap, top } = pileGame("ai-backrow-pile", first);
    const hidden = hiddenInstanceIds(state, "p1");
    expect(hidden.has(trap)).toBe(true);
    expect(hidden.has(top)).toBe(false);

    const seen = redact(state, "p1");
    expect(seen.players.p2.backrowPiles?.[0]?.[0]?.defId).toBe(HIDDEN_DEF_ID);
    expect(seen.players.p2.backrow[0]?.defId).toBe(FIELD_SPELL);

    // Two true states that differ only in the buried trap look the same to the seat.
    const other = pileGame("ai-backrow-pile", traps[1] ?? first);
    expect(hashState(redact(other.state, "p1"))).toBe(hashState(seen));

    const world = determinize(seen, "p1", createRng("ai-backrow-pile-world"));
    expect(traps).toContain(world.players.p2.backrowPiles?.[0]?.[0]?.defId);
  });

  it("R447 the seat's own buried trap stays readable to it", () => {
    const { state, trap } = pileGame("ai-backrow-pile-own", trapPool()[0] ?? "");
    expect(hiddenInstanceIds(state, "p2").has(trap)).toBe(false);
    expect(redact(state, "p2").players.p2.backrowPiles?.[0]?.[0]?.defId).toBe(trapPool()[0]);
  });
});
