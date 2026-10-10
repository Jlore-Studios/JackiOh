// The one seam between the hotseat client and the engine (CLAUDE.md rule 7, SPEC §10.8).
//
// The client never holds rules. It holds an `EnginePort`: `createGame` / `beginGame` / `reduce`
// to advance the game, `legalActions` to learn what it is allowed to offer, and `viewFor` to get
// the only thing it is allowed to draw. `EngineState` is opaque on purpose — it carries both
// hands and both libraries, so a client that could read a field could leak hidden information.
// Everything the UI needs comes back through `viewFor`.
//
// The real port is the Rust engine compiled to WebAssembly (`../wasm`, docs/v0.3.0/SURFACE.md
// §10.3): `enginePort()` maps its functions onto the port, synchronously once `loadWasm()` has
// finished, and `loadEnginePort()` is that load and that mapping. The presentation layer, the
// action builders and the animation table compile and test with no engine at all, and tests install
// a scripted port with `setEnginePort`.

import type { Action, ActionBody, CardDefs, EmoteId, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import type { Handicap } from "@jackioh/engine/config";

import * as wasm from "../wasm/index.ts";

declare const engineStateBrand: unique symbol;

/**
 * The engine's `GameState`, opaque to the client. It holds hidden information, so the client
 * passes it back to the port and never inspects it. The dev hotseat route puts it on
 * `window.__jackioh.state` for Cypress (BUILD M5-T3) and hashes it, nothing more.
 */
export type EngineState = { readonly [engineStateBrand]: never };

export type ReduceResult = { state: EngineState; events: GameEvent[]; error?: string };

export type CreateGameArgs = {
  seed: string;
  /** Two decks of card ids, in library order; the engine shuffles them with the match rng. */
  decks: [string[], string[]];
  catalog?: CardDefs;
  /**
   * A seat's resources when they are not SPEC's own (R180): practice's tiers, and the E2E build of
   * `/dev/hotseat`, whose fixture decks may carry one (R316's full library, R315's fatigue). The
   * engine validates it (R184) and folds it into the replay like the decks.
   */
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
  /**
   * R1341: the emote hand a seat is dealt in the game seeded `seed`, as the server deals it. A
   * scripted port may leave it out, and both seats then show the default hand (R1343).
   */
  dealEmoteHand?: (seed: string, seat: PlayerId) => EmoteId[];
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

export function injectedEnginePort(): EnginePort | null {
  return injected;
}

/** The engine's own `GameState` behind the client's opaque brand, and back. The casts strip only the brand. */
type GameState = Parameters<typeof wasm.hashState>[0];
const raw = (state: EngineState): GameState => state as unknown as GameState;
const opaque = (state: GameState): EngineState => state as unknown as EngineState;

/**
 * The real port over the loaded WebAssembly module. Synchronous: call it after `loadWasm()` has
 * resolved (`loadEnginePort` does both). Nothing here decides a rule; it renames the wrapper's
 * functions onto the port.
 */
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
  };
}

/**
 * The port: the injected one when a test installed it, else the real one once the WebAssembly
 * module has loaded. Rejects with `EngineUnavailableError` when the module cannot be loaded.
 */
export function loadEnginePort(): Promise<EnginePort> {
  if (injected !== null) return Promise.resolve(injected);
  return wasm.loadWasm().then(
    () => enginePort(),
    (cause: unknown) => {
      throw new EngineUnavailableError([], cause);
    },
  );
}
