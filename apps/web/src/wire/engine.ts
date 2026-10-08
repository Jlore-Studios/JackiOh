// `@jackioh/engine` for the web client (docs/v0.3.0/SURFACE.md §10.4): the rules as the client
// reaches them. The engine is Rust (`crates/engine`), compiled into the WebAssembly module; every
// function here is `../wasm`'s, under the name the TypeScript engine gave it, so the practice core,
// the tutorial harness and the tests that drive real games read exactly as they did.
//
// Also here, as `@jackioh/engine` exported them: the engine's constants (`./engineConfig.ts`,
// generated from `config.rs`), the `GameState` type (generated), the seeded generator
// (`./rng.ts`, bit for bit the engine's), and `subsystems`, which the web's tests reach for four of
// the engine's tables and §10.7's random policy.
//
// The client never reads a `GameState` field to decide anything (CLAUDE.md rule 7): the state is
// handed back to the engine, hashed, or put on the dev handle for Cypress, nothing more.

import { chooseAction as wasmChooseAction, engineTables, type EngineTables } from "../wasm/index.ts";
import { advanceTo, type Rng } from "./rng.ts";

import type { GameState } from "./generated/GameState.ts";
import type { ActionBody, PlayerId } from "./index.ts";

export * from "./engineConfig.ts";
export type { GameState } from "./generated/GameState.ts";
export type { CardInstance } from "./generated/CardInstance.ts";
export type { ReplayCheckpoints } from "./generated/ReplayCheckpoints.ts";
export type { ReplayOpen } from "./generated/ReplayOpen.ts";
export type { ReplayPage } from "./generated/ReplayPage.ts";
export type { ReplayRecord } from "./generated/ReplayRecord.ts";
export type { ReplayRefusal } from "./generated/ReplayRefusal.ts";
export type { ReplayStep } from "./generated/ReplayStep.ts";
export { createRng, type Rng } from "./rng.ts";

export {
  beginGame,
  createGame,
  dealEmoteHand,
  findInstance,
  fold,
  hashState,
  lastBoardFor,
  legalActions,
  reduce,
  registeredCatalog,
  replayOpen,
  replayPage,
  seatPlayedBy,
  seatToAct,
  viewFor,
  type CreateGameArgs,
  type FoldInput,
  type FoldResult,
  type LastBoardCard,
  type ReduceResult,
} from "../wasm/index.ts";

/** TypeScript's names for `fold`'s input and answer. */
export type { FoldInput as ReplayInput, FoldResult as ReplayResult } from "../wasm/index.ts";

/**
 * The engine's `subsystems`, as far as the client reaches into them: the Call to Chaos tables and
 * the Heroic Power table its tests check the card text against, and §10.7's random policy. The
 * tables are read from the module when first asked for, so importing this file never needs it loaded.
 */
export const subsystems = {
  /** R28: Call to Chaos's table, each entry by the clause the card prints. */
  get CHAOS_EFFECTS(): EngineTables["chaosEffects"] {
    return engineTables().chaosEffects;
  },
  /** R423: Call to Chaos (Classic+ Edition)'s table. */
  get CHAOS_PLUS_EFFECTS(): EngineTables["chaosPlusEffects"] {
    return engineTables().chaosPlusEffects;
  },
  /** R752: every power Heroic Power can roll, by name. */
  get HERO_POWER_NAMES(): EngineTables["heroPowerNames"] {
    return engineTables().heroPowerNames;
  },
  /** R752–R761: each power's X, names and words, base and Radiant. */
  get HERO_POWERS(): EngineTables["heroPowers"] {
    return engineTables().heroPowers;
  },
  /**
   * One step of §10.7's random policy: end the turn when nothing else is on offer, otherwise end it
   * with AI_END_TURN_PROBABILITY, otherwise pick uniformly (R44). Draws from `rng`, which stands
   * where the policy left it afterwards. Null only when the player has no action at all.
   */
  chooseAction(state: GameState, player: PlayerId, rng: Rng): ActionBody | null {
    const { action, rngCursor } = wasmChooseAction(state, player, { rngSeed: rng.seed, rngCursor: rng.cursor });
    advanceTo(rng, rngCursor);
    return action;
  },
};
