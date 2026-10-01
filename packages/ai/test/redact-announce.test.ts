// R448 and R185: a card the opponent is setting face-down waits in their resolving zone while its
// announce window is open (docs/classic-sets.md B5 E1), and the AI's seat may not read it there any
// more than in the backrow (R33) — `redact` hides it, and `determinize` fills its placeholder from
// the trap pool, as it fills a face-down backrow card.

import { describe, expect, it } from "vitest";
import { beginAnnounce, createRng, newInstance, type GameState } from "@jackioh/engine";
import { HIDDEN_DEF_ID, determinize, hiddenInstanceIds, redact } from "../src/index";
import { AI, HUMAN, cardById, scenario, trapPool } from "./_support";

function setting(faceDown: boolean): { state: GameState; id: string } {
  const { state } = scenario({
    seed: "redact-announce",
    p1: { hand: ["core-008"], field: ["core-011"], library: ["core-020", "core-053"] },
    p2: { hand: ["core-002"], field: ["core-019"], library: ["core-005", "core-016"] },
  });
  const card = newInstance(state, faceDown ? "core-041" : "core-002", HUMAN, { z: "resolving", player: HUMAN });
  state.players[HUMAN].resolving.push(card);
  beginAnnounce(state, { instanceId: card.id, player: HUMAN, ...(faceDown ? { faceDown: true as const } : {}) });
  return { state, id: card.id };
}

describe("R448 the AI never reads a card being set face-down", () => {
  it("R448 redact hides the opponent's card waiting to be set face-down, and determinize makes it a trap", () => {
    const { state, id } = setting(true);
    expect(hiddenInstanceIds(state, AI).has(id)).toBe(true);
    const seen = redact(state, AI);
    expect(cardById(seen, id)?.defId).toBe(HIDDEN_DEF_ID);
    const world = determinize(seen, AI, createRng("redact-announce-world"));
    expect(trapPool()).toContain(cardById(world, id)?.defId);
  });

  it("R448 the player setting it reads it, and a face-up play waiting in the window is public", () => {
    const own = setting(true);
    expect(hiddenInstanceIds(own.state, HUMAN).has(own.id)).toBe(false);
    const open = setting(false);
    expect(hiddenInstanceIds(open.state, AI).has(open.id)).toBe(false);
    expect(cardById(redact(open.state, AI), open.id)?.defId).toBe("core-002");
  });
});
