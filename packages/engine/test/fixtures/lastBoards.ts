// Fixture cards for B5 E30's last boards (SPEC §8.7 C+ #29, R417, R564): C+ #29's two faces in the
// smallest script that has them, and a Trap and a Field Trap to put face-down. The engine never
// imports packages/cards (CLAUDE.md), so the real card's test covers the same cases again.

import type { Action, CardDef, PlayerId } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { DECK_SIZE } from "../../src/config";
import { LAST_BOARD_CARD_COST, addFromLastBoard, addRandomFromLastBoard, discoverFromLastBoard } from "../../src/effects";
import { beginGame, reduce } from "../../src/reduce";
import type { CardScripts } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { createGame, type GameState, type LastBoardInput } from "../../src/state";
import { spellDef, vanillaDeck } from "./catalog";
import { setupCatalog } from "./harness";

/** C+ #29's shape: base Discovers from the caster's last board, Radiant adds 3 random; each costs (0). */
export const PORTAL = "lb-portal";
/** How many the Radiant face adds (the real card reads `param(ctx, "cards")`). */
export const PORTAL_RADIANT_CARDS = 3;
export const TRAP = "lb-trap";
export const FIELD_TRAP = "lb-field-trap";

const DEFS: CardDef[] = [
  spellDef(9701, { id: PORTAL, name: "LB Portal" }),
  spellDef(9702, { id: TRAP, name: "LB Trap", type: "Trap" }),
  spellDef(9703, { id: FIELD_TRAP, name: "LB Field Trap", type: "Field Trap" }),
];

const SCRIPTS: Record<string, CardScripts> = {
  [PORTAL]: {
    base: {
      cry: () => [discoverFromLastBoard({ step: "picked" })],
      resume: { picked: () => [addFromLastBoard({ costOverride: LAST_BOARD_CARD_COST })] },
    },
    radiant: { cry: () => [addRandomFromLastBoard({ count: PORTAL_RADIANT_CARDS, costOverride: LAST_BOARD_CARD_COST })] },
  },
};

/** The vanilla fixture catalog plus this file's cards. */
export function registerLastBoards(): void {
  setupCatalog();
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((def) => [def.id, def])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}

/** p1's deck holds the Portal; p2's is vanilla. */
export const LB_DECKS: [string[], string[]] = [[PORTAL, ...vanillaDeck(DECK_SIZE - 1, 1)], vanillaDeck(DECK_SIZE, 21)];

let nonce = 0;

/** One action through `reduce`, appended to `log`; throws on a refusal. */
export function act(state: GameState, log: Action[], body: Record<string, unknown> & { type: string; playerId: PlayerId }): GameState {
  nonce += 1;
  const action = { ...body, nonce: `lb${nonce}` } as Action;
  const result = reduce(state, action);
  if (result.error !== undefined) throw new Error(`${body.type} refused: ${result.error}`);
  log.push(action);
  return result.state;
}

/**
 * A real game from `(seed, decks, lastBoards)`: the first seed from `seedBase` whose opening deal puts
 * the Portal in p1's hand, past both mulligans (everything kept), in p1's first main phase.
 */
export function portalGame(seedBase: string, lastBoards?: LastBoardInput): { seed: string; state: GameState; log: Action[] } {
  registerLastBoards();
  for (let at = 0; at < 200; at += 1) {
    const seed = `${seedBase}-${at}`;
    let state = beginGame(createGame({ seed, decks: LB_DECKS, ...(lastBoards === undefined ? {} : { lastBoards }) })).state;
    if (!state.players.p1.hand.some((card) => card.defId === PORTAL)) continue;
    const log: Action[] = [];
    for (const player of ["p1", "p2"] as const) {
      state = act(state, log, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    }
    return { seed, state, log };
  }
  throw new Error(`no seed from ${seedBase} deals p1 the Portal`);
}
