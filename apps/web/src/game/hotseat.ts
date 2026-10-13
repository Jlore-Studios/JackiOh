// The hotseat session (BUILD M5-T3): one device, two seats, one engine.
//
// Framework-free and deterministic: M5-T3 requires the same seed and actions to reproduce the same
// state hash, so a random or clock-derived nonce cannot enter the recorded `Action`.
//
// `dispatch` hands actions to `reduce` (CLAUDE.md rule 7); opaque `EngineState` cannot leak hidden data.

import { opponentOf } from "@jackioh/shared";
import type { Action, ActionBody, CardDefs, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CreateGameArgs, EnginePort, EngineState } from "./engine.ts";

/** Seat order matches `createGame` (SPEC §10.1). */
export const SEATS: readonly [PlayerId, PlayerId] = ["p1", "p2"];

/** p1 moves first (§2.1). */
export const FIRST_SEAT: PlayerId = "p1";

export type HotseatOptions = {
  seed: string;
  decks: [string[], string[]];
  engine: EnginePort;
  catalog?: CardDefs;
  /** R180: E2E `/dev/hotseat` fixture handicaps cover R315 fatigue and R316 full libraries; the engine validates them (R184). */
  handicaps?: CreateGameArgs["handicaps"];
  nonce?: (n: number) => string;
};

export type DispatchResult = { events: GameEvent[]; error?: string };

export type HotseatSession = {
  seed: string;
  seat: PlayerId;
  setSeat(seat: PlayerId): void;
  /** `viewFor(state, seat)` — the only thing the UI may render. */
  view(): PlayerView;
  /** `legalActions(state, seat)` — the only source of legality the client has. */
  legal(): ActionBody[];
  dispatch(body: ActionBody): DispatchResult;
  /** For Cypress and the vitest replay: the ordered action log and the state hash. */
  log(): readonly Action[];
  hash(): string;
  state(): EngineState;
  subscribe(fn: () => void): () => void;
};

export function defaultNonce(n: number): string {
  return `n${n}`;
}

/**
 * `ActionBody` is a union, so the stamp is written once here: spreading the union distributes,
 * which keeps every member's own fields intact instead of collapsing to the shared ones.
 */
function stamp(body: ActionBody, playerId: PlayerId, nonce: string): Action {
  return { ...body, playerId, nonce };
}

export function createHotseat(options: HotseatOptions): HotseatSession {
  const { engine } = options;
  const nonceFor = options.nonce ?? defaultNonce;

  // `createGame` throws for illegal decks (§2.6 L2/L3); `beginGame` refusals also surface as throws.
  const created = engine.createGame({
    seed: options.seed,
    decks: options.decks,
    ...(options.catalog === undefined ? {} : { catalog: options.catalog }),
    ...(options.handicaps === undefined ? {} : { handicaps: options.handicaps }),
  });
  const begun = engine.beginGame(created);
  if (begun.error !== undefined) {
    throw new Error(`the engine refused to begin the game: ${begun.error}`);
  }

  let state: EngineState = begun.state;
  let seat: PlayerId = FIRST_SEAT;
  /** Advanced only after an accepted action. */
  let nonceCount = 0;
  const actions: Action[] = [];
  const subscribers = new Set<() => void>();

  function notify(): void {
    // Copy first: a subscriber may unsubscribe while being notified.
    for (const fn of [...subscribers]) fn();
  }

  /**
   * BUILD M5-T3: questions switch seats; ACTIVE changes use the manual, hidden-board switch.
   * Mulligans are simultaneous (R265); draw offers (§2.5, R36) switch to the responder and lapse
   * with the turn (R269). Ownership comes from VIEW (`pending`, `drawOffer`; SPEC §10.8).
   */
  function followQuestion(offered = false): void {
    const view = engine.viewFor(state, seat);
    const pending = view.pending;
    if (pending !== null) {
      if (!pending.forYou && pending.pendingFor !== seat) seat = pending.pendingFor;
      return;
    }
    if (offered && view.result === null && view.drawOffer?.by === seat) seat = opponentOf(seat);
  }

  // Opening mulligans (§2.1, R9, R265) start with p1 unless the prompt says otherwise.
  followQuestion();

  const session: HotseatSession = {
    seed: options.seed,

    get seat(): PlayerId {
      return seat;
    },

    setSeat(next: PlayerId): void {
      if (next === seat) return;
      seat = next;
      notify();
    },

    view(): PlayerView {
      return engine.viewFor(state, seat);
    },

    legal(): ActionBody[] {
      return engine.legalActions(state, seat);
    },

    dispatch(body: ActionBody): DispatchResult {
      const action = stamp(body, seat, nonceFor(nonceCount));
      const result = engine.reduce(state, action);

      if (result.error !== undefined) {
        // Rejections do not log or consume nonces, keeping `log()` foldable to the browser's hash.
        return { events: result.events, error: result.error };
      }

      state = result.state;
      nonceCount += 1;
      actions.push(action);
      // Draw-offer answers return to the active player.
      if (body.type === "answerDraw") seat = engine.viewFor(state, seat).active;
      followQuestion(body.type === "offerDraw");
      notify();
      return { events: result.events };
    },

    log(): readonly Action[] {
      return actions;
    },

    hash(): string {
      return engine.hashState(state);
    },

    state(): EngineState {
      return state;
    },

    subscribe(fn: () => void): () => void {
      subscribers.add(fn);
      return () => {
        subscribers.delete(fn);
      };
    },
  };

  return session;
}

export function otherSeat(seat: PlayerId): PlayerId {
  return opponentOf(seat);
}
