// Classic #62 Living Bomb's card-specific hook (R400, R68): `Script.startOfOpponentTurn`, "at the start
// of your opponent's turn", queued at R62's start-of-turn trigger point right after the turn player's own
// `startOfTurn` hooks — R68's order, the active player's cards and then the opponent's — and never on its
// controller's own turn. Proved through fixture scripts that note what fired, in order, on the watcher.

import type { Action, ActionInput, CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { beginGame, reduce } from "../src/reduce";
import type { CardScripts, Effect } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { GameState } from "../src/state";
import { newGame, put, slot } from "./fixtures/harness";

function def(id: string, type: CardDef["type"], index: number): CardDef {
  const face = type === "Unit" ? { attack: 1, health: 5, keywords: [], text: id } : { keywords: [], text: id };
  return { id, index: String(index), name: id, set: "Core", type, tags: [], rarity: "Common", token: false, cost: 0, base: face, radiant: face };
}

/** Living Bomb's shape: a Field Spell answering both its own turn's start and its opponent's. */
const watcher = def("sot-watcher", "Field Spell", 9611);
/** A unit with an ordinary start-of-turn hook, for the other side. */
const holder = def("sot-holder", "Unit", 9612);

/** Append `name` to the watcher's log (p1's backrow lane 1), whoever's hook it is. */
function note(name: string): Effect {
  return {
    kind: "sot:note",
    apply(ctx): void {
      const log = ctx.state.players.p1.backrow[0];
      if (log === null || log === undefined) return;
      log.memory.steps = [...((log.memory.steps as string[] | undefined) ?? []), name];
    },
  };
}

const SCRIPTS: Record<string, CardScripts> = {
  [watcher.id]: {
    base: { startOfTurn: () => [note("watcher:own")], startOfOpponentTurn: () => [note("watcher:opponent's")] },
    radiant: {},
  },
  [holder.id]: { base: { startOfTurn: () => [note("holder:own")] }, radiant: {} },
};

let nonce = 0;
function act(state: GameState, body: ActionInput): GameState {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `sot${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

function steps(state: GameState): string[] {
  return (state.players.p1.backrow[0]?.memory.steps as string[] | undefined) ?? [];
}

describe("'At the start of your opponent's turn' (C #62, R400, R68)", () => {
  it("R68 it fires on the opponent's turn after their own start-of-turn hooks, and never on its controller's turn", () => {
    const fresh = newGame("start-of-opponent-turn");
    registerCatalog({ ...registeredCatalog(), [watcher.id]: watcher, [holder.id]: holder });
    registerScripts({ ...registeredScripts(), ...SCRIPTS });
    let state = beginGame(fresh).state;
    for (const playerId of ["p1", "p2"] as const) {
      state = act(state, { type: "mulligan", playerId, keep: state.players[playerId].hand.map((card) => card.id) });
    }
    put(state, watcher.id, slot("p1", "backrow", 1));
    put(state, holder.id, slot("p2", "units", 1));

    state = act(state, { type: "endTurn", playerId: "p1" });
    expect(state.active).toBe("p2");
    expect(steps(state)).toEqual(["holder:own", "watcher:opponent's"]);

    state = act(state, { type: "endTurn", playerId: "p2" });
    expect(state.active).toBe("p1");
    expect(steps(state)).toEqual(["holder:own", "watcher:opponent's", "watcher:own"]);
  });
});
