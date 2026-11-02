// `/dev/hotseat?seed=42&a=<deckId>&b=<deckId>` (BUILD M5-T3): `reduce` in the browser, `viewFor`
// for whichever seat holds the device, and `window.__jackioh` for Cypress outside production builds.
//
// It computes no view of its own (CLAUDE.md rule 7, SPEC §10.8): when the WebAssembly module
// (`crates/wasm`, behind `src/wasm/`) cannot be loaded, this route renders why and stops, and
// nothing here stubs a fake engine to hide it.

import { useCallback, useEffect, useMemo, useState, useSyncExternalStore } from "react";

import type { Action, ActionBody, CardDefs, PlayerId, PlayerView, SetName } from "@jackioh/shared";
import type { Handicap } from "@jackioh/engine/config";

import Game from "../../game/Game.tsx";
import { getAudioEngine } from "../../audio/index.ts";
import type { EmoteHands } from "../../emotes/hand.ts";
import { useEmotes } from "../../emotes/useEmotes.ts";
import { useSetting } from "../../settings/index.ts";
import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import { testid } from "../../game/contract.ts";
import { DECK_SIZE, DEFAULT_DECK_ID, DECK_IDS, resolveDecks } from "../../game/decks.ts";
import { EngineUnavailableError, loadEnginePort } from "../../game/engine.ts";
import type { EnginePort, EngineState } from "../../game/engine.ts";
import { SEATS, createHotseat, otherSeat } from "../../game/hotseat.ts";
import type { DispatchResult, HotseatSession } from "../../game/hotseat.ts";
import { navigate, paths } from "../../net/navigate.ts";
import { navTestid } from "../nav.tsx";
import { messageOf } from "../../net/errors.ts";
import { getStored } from "../../net/storage.ts";

/** BUILD M5-T3: the dev handle exists only outside a production build. */
const DEV_ONLY = import.meta.env.MODE !== "production";

/** BUILD's own example URL uses seed 42. */
export const DEFAULT_SEED = "42";

/** ASSUMPTION A1 in `e2e/support/commands.ts`: where `cy.seedGame` parks its fixture decks. */
export const E2E_DECKS_KEY = "jackioh.e2e.decks";

/** R180: each seat's handicap, as `createGame` takes them. Empty when the game is SPEC's own. */
export type SeatHandicaps = Partial<Record<PlayerId, Handicap>>;

// the dev handle (BUILD M5-T3, consumed by e2e/support/commands.ts)

/** An action from Cypress, which names the seat it is acting as (`ActionInput`). */
type DevAction = ActionBody & { playerId?: PlayerId };

export type HotseatDevHandle = {
  /** The raw `GameState`. Hidden information: for the test harness and the replay hash only. */
  readonly state: EngineState;
  readonly seed: string;
  readonly seat: PlayerId;
  readonly log: readonly Action[];
  readonly decks: [string[], string[]];
  /** The game's handicaps (R180), `{}` for none: `replay.fold` needs them (`cy.replayCheck`). */
  readonly handicaps: SeatHandicaps;
  /**
   * Dispatch as a seat: a `playerId` other than the seat's switches the seat first rather than
   * forging an action for someone else; `reduce` still refuses anything illegal (SPEC §9.3).
   */
  dispatch: (action: DevAction) => DispatchResult;
  view: () => PlayerView;
  legal: () => ActionBody[];
  hash: () => string;
  setSeat: (seat: PlayerId) => void;
};

declare global {
  interface Window {
    /** BUILD M5-T3. */
    __jackioh?: HotseatDevHandle;
    /** ASSUMPTION A1: fixture decks for the E2E build, and each seat's handicap (R180). */
    __jackiohE2E?: {
      seed?: string;
      decks?: Record<string, string[]>;
      handicaps?: Partial<Record<PlayerId, unknown>>;
      /** R1420: sets the engine previews before the game is created (`readInjectedPreview`). */
      preview?: string[];
    };
  }
}

// URL and E2E deck injection

export type HotseatParams = { seed: string; a: string; b: string };

function param(params: URLSearchParams, key: string, fallback: string): string {
  const value = params.get(key);
  return value === null || value === "" ? fallback : value;
}

export function readParams(search: string): HotseatParams {
  const params = new URLSearchParams(search);
  return {
    seed: param(params, "seed", DEFAULT_SEED),
    a: param(params, "a", DEFAULT_DECK_ID),
    b: param(params, "b", DEFAULT_DECK_ID),
  };
}

/**
 * The E2E injection, unchecked: `window.__jackiohE2E`, or the localStorage copy that survives a
 * reload. Nothing at all in a production build.
 */
function readInjection(): unknown {
  if (!DEV_ONLY || typeof window === "undefined") return undefined;

  const raw: unknown = window.__jackiohE2E;
  if (raw !== undefined && raw !== null) return raw;
  try {
    const stored = getStored(E2E_DECKS_KEY);
    return stored === null ? undefined : JSON.parse(stored);
  } catch {
    // A private window, blocked site data, or malformed JSON: there is simply no injection.
    return undefined;
  }
}

/**
 * The E2E deck injection. Untrusted: anything that is not a list of strings is dropped, and the
 * ids that remain are still checked against the catalog by `resolveDeck`.
 */
export function readInjectedDecks(): Record<string, string[]> | undefined {
  const raw = readInjection();
  if (typeof raw !== "object" || raw === null) return undefined;

  const decks = (raw as { decks?: unknown }).decks;
  if (typeof decks !== "object" || decks === null) return undefined;

  const out: Record<string, string[]> = {};
  for (const [id, cards] of Object.entries(decks as Record<string, unknown>)) {
    if (Array.isArray(cards) && cards.every((card) => typeof card === "string")) {
      out[id] = [...(cards as string[])];
    }
  }
  return Object.keys(out).length === 0 ? undefined : out;
}

/**
 * One seat's injected handicap, if it has the SHAPE of one (R180; `heroHealth` is R290). Whether
 * the numbers are legal is the engine's call (`validateHandicap`, R184). The copy carries those
 * fields and nothing else, so no stray key reaches the engine.
 */
function handicapShape(raw: unknown): Handicap | undefined {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return undefined;
  const { deckSize, manaBonus, manaCap, extraOpeningCards, extraDrawsPerTurn, heroHealth } = raw as Record<
    string,
    unknown
  >;
  if (
    typeof deckSize !== "number" ||
    typeof manaBonus !== "number" ||
    typeof manaCap !== "number" ||
    typeof extraOpeningCards !== "number" ||
    typeof extraDrawsPerTurn !== "number"
  ) {
    return undefined;
  }
  if (heroHealth !== undefined && typeof heroHealth !== "number") return undefined;
  return {
    deckSize,
    manaBonus,
    manaCap,
    extraOpeningCards,
    extraDrawsPerTurn,
    ...(heroHealth === undefined ? {} : { heroHealth }),
  };
}

/**
 * The E2E injection's per-seat handicaps (R180), fixture A's as p1's and B's as p2's: the only way a
 * browser reaches R315's fatigue or R80's full library in a few turns. A misshapen seat is dropped;
 * `undefined` means no seat has one, and the game is created without handicaps.
 */
export function readInjectedHandicaps(): SeatHandicaps | undefined {
  const raw = readInjection();
  if (typeof raw !== "object" || raw === null) return undefined;
  const handicaps = (raw as { handicaps?: unknown }).handicaps;
  if (typeof handicaps !== "object" || handicaps === null) return undefined;

  const out: SeatHandicaps = {};
  for (const seat of SEATS) {
    const handicap = handicapShape((handicaps as Record<string, unknown>)[seat]);
    if (handicap !== undefined) out[seat] = handicap;
  }
  return Object.keys(out).length === 0 ? undefined : out;
}

/**
 * The E2E injection's `preview` (R1420): sets the engine treats as shipped, so a spec can play a set
 * before it ships (#552's play-through). Only strings pass: whether each names a set is the engine's
 * call (`EnginePort.previewSets`).
 */
export function readInjectedPreview(): SetName[] {
  const raw = readInjection();
  if (typeof raw !== "object" || raw === null) return [];
  const preview = (raw as { preview?: unknown }).preview;
  if (!Array.isArray(preview)) return [];
  return preview.filter((set): set is SetName => typeof set === "string");
}

// startup

type Status =
  | { kind: "loading" }
  | { kind: "no-engine"; missing: readonly string[]; message: string }
  | { kind: "no-game"; message: string }
  /** `decks` and `handicaps` are kept for the dev handle's replay (ASSUMPTION A2; spec 25). */
  | {
      kind: "ready";
      session: HotseatSession;
      defs: CardDefs | null;
      decks: [string[], string[]];
      handicaps: SeatHandicaps;
      /** R1341: both seats' emote hands, dealt from the seed by the port; null from a port that deals none. */
      hands: EmoteHands | null;
    };

function catalogOf(port: EnginePort): CardDefs | null {
  if (port.catalog === undefined) return null;
  try {
    return port.catalog();
  } catch {
    // `registeredCatalog()` throws when no catalog has been registered (M4-T2 has not landed).
    return null;
  }
}

function startSession(port: EnginePort, params: HotseatParams): Status {
  const defs = catalogOf(port);
  const handicaps = readInjectedHandicaps();
  const preview = readInjectedPreview();
  // R184: a handicapped seat's deck holds its handicap's size, so the readable pre-check agrees.
  const sizes: [number, number] = [handicaps?.p1?.deckSize ?? DECK_SIZE, handicaps?.p2?.deckSize ?? DECK_SIZE];
  const resolved = resolveDecks(params.a, params.b, defs ?? {}, readInjectedDecks(), sizes, preview);
  if ("error" in resolved) return { kind: "no-game", message: resolved.error };

  try {
    // R1420: the injection's sets open before the game is created, so `createGame` takes their cards.
    if (preview.length > 0) {
      if (port.previewSets === undefined) return { kind: "no-game", message: `this engine cannot preview ${preview.join(", ")} (R1420)` };
      port.previewSets(preview);
    }
    const session = createHotseat({
      seed: params.seed,
      decks: resolved.decks,
      engine: port,
      ...(defs === null ? {} : { catalog: defs }),
      ...(handicaps === undefined ? {} : { handicaps }),
    });
    // R1341: each seat holds the hand the game's seed deals it, as a match on that seed would.
    const deal = port.dealEmoteHand;
    const hands = deal === undefined ? null : { p1: deal(params.seed, "p1"), p2: deal(params.seed, "p2") };
    return { kind: "ready", session, defs, decks: resolved.decks, handicaps: handicaps ?? {}, hands };
  } catch (cause) {
    // `createGame` throws on an illegal deck (§2.6) or handicap (R184) — the engine's ruling,
    // printed as given.
    return { kind: "no-game", message: messageOf(cause) };
  }
}

// panels

function EnginePanel({ missing, message }: { missing: readonly string[]; message: string }) {
  return (
    <div className="app-shell">
      <h1>JackiOh — hotseat</h1>
      <p className="notice">
        The client is waiting on M3. It renders <code>viewFor</code> and nothing else (SPEC §10.8),
        so it cannot start a game until <code>@jackioh/engine</code> exports it.
        {missing.length > 0 ? (
          <>
            {" "}
            Missing: <code>{missing.join(", ")}</code>.
          </>
        ) : null}
      </p>
      <p className="notice">
        <code>{message}</code>
      </p>
    </div>
  );
}

function GamePanel({ message, params }: { message: string; params: HotseatParams }) {
  return (
    <div className="app-shell">
      <h1>JackiOh — hotseat</h1>
      <p className="notice">
        No game for <code>?seed={params.seed}&amp;a={params.a}&amp;b={params.b}</code>: {message}
      </p>
      <p className="notice">
        Dev decks: <code>{DECK_IDS.join(", ")}</code>.
      </p>
    </div>
  );
}

// the loop

/**
 * Re-render on every session notification. The session is external mutable state, so the version
 * counter — not the view object, which is rebuilt on each `viewFor` call — is the snapshot.
 */
function useSessionVersion(session: HotseatSession): void {
  const store = useMemo(() => {
    let version = 0;
    return {
      subscribe: (onChange: () => void) =>
        session.subscribe(() => {
          version += 1;
          onChange();
        }),
      snapshot: () => version,
    };
  }, [session]);
  useSyncExternalStore(store.subscribe, store.snapshot, store.snapshot);
}

function Hotseat({
  session,
  defs,
  decks,
  handicaps,
  hands,
}: {
  session: HotseatSession;
  defs: CardDefs | null;
  decks: [string[], string[]];
  handicaps: SeatHandicaps;
  hands: EmoteHands | null;
}) {
  useSessionVersion(session);
  const [error, setError] = useState<string | null>(null);

  const dispatch = useCallback(
    (body: ActionBody): DispatchResult => {
      const result = session.dispatch(body);
      setError(result.error ?? null);
      return result;
    },
    [session],
  );

  /** Handing the device over drops the previous seat's refusal: it was never this player's. */
  const switchTo = useCallback(
    (seat: PlayerId) => {
      setError(null);
      session.setSeat(seat);
    },
    [session],
  );

  // BUILD M5-T3: `{ state, dispatch, seed }` is the contract; the rest drives two seats and checks
  // the replay hash (ASSUMPTION A2 in e2e/support/types.ts). Getters, so a spec that re-reads
  // `handle.state` sees the current state.
  useEffect(() => {
    if (!DEV_ONLY) return;
    const handle: HotseatDevHandle = {
      get state() {
        return session.state();
      },
      get seed() {
        return session.seed;
      },
      get seat() {
        return session.seat;
      },
      get log() {
        return session.log();
      },
      get decks() {
        return decks;
      },
      get handicaps() {
        return handicaps;
      },
      dispatch: (action: DevAction) => {
        // `createHotseat` re-stamps `playerId` with the seat, so the extra key is overwritten.
        if (action.playerId !== undefined) switchTo(action.playerId);
        return dispatch(action);
      },
      view: () => session.view(),
      legal: () => session.legal(),
      hash: () => session.hash(),
      setSeat: switchTo,
    };
    window.__jackioh = handle;
    return () => {
      if (window.__jackioh === handle) delete window.__jackioh;
    };
  }, [session, dispatch, switchTo, decks, handicaps]);

  const view = session.view();
  const legal = session.legal();
  const next = otherSeat(session.seat);
  const lookup = useMemo(() => (defs === null ? null : lookupFromDefs(defs)), [defs]);

  // R642–R644: hotseat emotes run locally with the same shared gate. The dev decks carry no
  // portrait, so both seats read `vanilla`; the device mute applies to the seat not holding the
  // device. R1341: each seat's hand is dealt from the seed (`startSession`).
  const globalMuteEmotes = useSetting("muteOpponentEmotes");
  const emotes = useEmotes({
    hands,
    engine: getAudioEngine(),
    globalMute: globalMuteEmotes,
    you: view.viewer,
    // MD-D29, R1127: an `Emote` action through the engine, only while the view hears emotes.
    emit: (emote) => {
      if (view.emotesHeard === true) dispatch({ type: "emote", emote });
    },
  });

  // A finished game's ways on (Result.tsx): the same seed and decks again, or back to the start.
  const resultActions = (
    <>
      <button type="button" data-testid="result-play-again" onClick={() => window.location.reload()}>
        Play again
      </button>
      <button type="button" data-testid="result-back" onClick={() => navigate(paths.landing)}>
        Back
      </button>
    </>
  );
  const game = <Game view={view} legal={legal} onAction={dispatch} error={error} resultActions={resultActions} emotes={emotes} wearCosmetics />;

  return (
    <div className="app-shell app-shell--wide">
      <header className="hotseat-bar">
        {/* A way out, as every other screen has (nav.tsx); the board's own gear holds the settings. */}
        <button type="button" className="link-button" data-testid={navTestid.back} onClick={() => navigate(paths.landing)}>
          ← Back
        </button>
        <span>
          seed <code>{session.seed}</code> · seat <code>{session.seat}</code> · turn {view.turn} ·
          active <code>{view.active}</code>
        </span>
        <button type="button" data-testid={testid.seatSwitch} onClick={() => switchTo(next)}>
          Hand over to {next}
        </button>
      </header>
      {lookup === null ? game : <CatalogContext.Provider value={lookup}>{game}</CatalogContext.Provider>}
    </div>
  );
}

export function HotseatRoute() {
  const params = useMemo(() => readParams(window.location.search), []);
  const [status, setStatus] = useState<Status>({ kind: "loading" });

  useEffect(() => {
    let canceled = false;
    loadEnginePort()
      .then((port) => {
        if (!canceled) setStatus(startSession(port, params));
      })
      .catch((cause: unknown) => {
        if (canceled) return;
        const error = cause instanceof EngineUnavailableError ? cause : null;
        setStatus({
          kind: "no-engine",
          missing: error?.missing ?? [],
          message: messageOf(cause),
        });
      });
    return () => {
      canceled = true;
    };
  }, [params]);

  if (status.kind === "loading") {
    return (
      <div className="app-shell">
        <h1>JackiOh — hotseat</h1>
        <p className="notice">Loading the engine…</p>
      </div>
    );
  }
  if (status.kind === "no-engine") {
    return <EnginePanel missing={status.missing} message={status.message} />;
  }
  if (status.kind === "no-game") {
    return <GamePanel message={status.message} params={params} />;
  }

  return (
    <Hotseat
      session={status.session}
      defs={status.defs}
      decks={status.decks}
      handicaps={status.handicaps}
      hands={status.hands}
    />
  );
}

export default HotseatRoute;
