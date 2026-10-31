// Seam between hotseat client and engine (CLAUDE.md rule 7, SPEC §10.8; §10.3).
// Client holds opaque EngineState and advances via EnginePort without inspecting hidden info.

import type { Action, ActionBody, CardDefs, EmoteId, GameEvent, PlayerId, PlayerView, SetName } from "@jackioh/shared";
import type { Handicap } from "@jackioh/engine/config";

import * as wasm from "../wasm/index.ts";

declare const engineStateBrand: unique symbol;

/** Engine GameState, opaque to client to protect hidden information. */
export type EngineState = { readonly [engineStateBrand]: never };

export type ReduceResult = { state: EngineState; events: GameEvent[]; error?: string };

export type CreateGameArgs = {
  seed: string;
  /** Two decks of card ids, in library order; the engine shuffles them with the match rng. */
  decks: [string[], string[]];
  catalog?: CardDefs;
  /** Seat resources when not default (R180, R184, R315, R316). */
  handicaps?: Partial<Record<PlayerId, Handicap>>;
};

export type EnginePort = {
  createGame: (args: CreateGameArgs) => EngineState;
  beginGame: (state: EngineState) => ReduceResult;
  reduce: (state: EngineState, action: Action) => ReduceResult;
  /** Everything this player may legally do right now. The client greys out the rest (M5-T2). */
  legalActions: (state: EngineState, player: PlayerId) => ActionBody[];
  /** The only window the client has onto the game (SPEC §10.8). */
  viewFor: (state: EngineState, player: PlayerId) => PlayerView;
  hashState: (state: EngineState) => string;
  /** The card catalog, for the dev deck picker. */
  catalog?: () => CardDefs;
  /** R1341: emote hand dealt for match seed; fallback is default hand (R1343). */
  dealEmoteHand?: (seed: string, seat: PlayerId) => EmoteId[];
  /** R1420, #552: preview sets treated as shipped for deck validation in dev hotseat E2E. */
  previewSets?: (sets: readonly SetName[]) => void;
};

/** The functions `enginePort()` needs from the WebAssembly wrapper, for the missing-export report. */
export const REQUIRED_ENGINE_EXPORTS = [
  "createGame",
  "beginGame",
  "reduce",
  "legalActions",
  "viewFor",
  "hashState",
] as const;

export class EngineUnavailableError extends Error {
  readonly missing: readonly string[];

  constructor(missing: readonly string[], cause?: unknown) {
    super(
      missing.length > 0
        ? `the engine is missing: ${missing.join(", ")}. The client cannot run a game until the WebAssembly module exports them.`
        : `the engine could not be loaded: ${String(cause)}`,
    );
    this.name = "EngineUnavailableError";
    this.missing = missing;
  }
}

let injected: EnginePort | null = null;

/** Tests and Cypress fixtures install a scripted port instead of the real engine. */
export function setEnginePort(port: EnginePort | null): void {
  injected = port;
}

/** The engine's own `GameState` behind the client's opaque brand, and back. The casts strip only the brand. */
type GameState = Parameters<typeof wasm.hashState>[0];
const raw = (state: EngineState): GameState => state as unknown as GameState;
const opaque = (state: GameState): EngineState => state as unknown as EngineState;

/** Real port over loaded WebAssembly module; maps wrapper functions without rules logic. */
export function enginePort(): EnginePort {
  const mod = wasm as unknown as Record<string, unknown>;
  const missing = REQUIRED_ENGINE_EXPORTS.filter((name) => typeof mod[name] !== "function");
  if (missing.length > 0) throw new EngineUnavailableError(missing);
  if (!wasm.wasmLoaded()) throw new EngineUnavailableError([], "the WebAssembly module is not loaded yet");

  return {
    createGame: (args) => opaque(wasm.createGame(args)),
    beginGame: (state) => {
      const result = wasm.beginGame(raw(state));
      return { ...result, state: opaque(result.state) };
    },
    reduce: (state, action) => {
      const result = wasm.reduce(raw(state), action);
      return { ...result, state: opaque(result.state) };
    },
    legalActions: (state, player) => wasm.legalActions(raw(state), player),
    viewFor: (state, player) => wasm.viewFor(raw(state), player),
    hashState: (state) => wasm.hashState(raw(state)),
    catalog: () => wasm.registeredCatalog(),
    dealEmoteHand: (seed, seat) => wasm.dealEmoteHand(seed, seat),
    previewSets: (sets) => wasm.previewSets(sets),
  };
}

/** Injected port if set, else real port once WebAssembly loads; rejects on failure. */
export function loadEnginePort(): Promise<EnginePort> {
  if (injected !== null) return Promise.resolve(injected);
  return wasm.loadWasm().then(
    () => enginePort(),
    (cause: unknown) => {
      throw new EngineUnavailableError([], cause);
    },
  );
}
