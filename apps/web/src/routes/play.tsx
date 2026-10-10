// Play lobby (SPEC §9.5; R104, R253, R257, R258, R259, R264, R330, R505, R612, R661, R765, R767,
// R1372). Verdicts are UX only (CLAUDE.md rule 7); the server owns validation and matchmaking.
// Room links only prefill, queue counts stay on mode tiles, and rank never exposes its hidden rating.

import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type ReactElement } from "react";

import { DECK_SIZE } from "@jackioh/engine/config";
import {
  validateDeck,
  validateTrio,
  type CatalogSnapshot,
  type Collection,
  type LoadoutDeck,
  type LoadoutResult,
} from "@jackioh/validator";

import { MATCH_FOUND_NAV_DELAY_MS, SERIES_POLL_SECONDS } from "@jackioh/server-config";
import {
  ApiRequestError,
  createRoom,
  dequeue,
  enqueue,
  getCatalog,
  getCollection,
  getDecks,
  getMe,
  getOwnRank,
  getPopulation,
  joinRoom,
  roomModeOf,
  type ModeChoice,
  type OwnRankResponse,
  type PopulationResponse,
  type QueueMode,
  type SavedDeck,
  type SavedTrio,
} from "../net/api.ts";
import { LeanNewestToggle, readPlayLeanNewest, writePlayLeanNewest } from "../game/LeanNewest.tsx";
import { MATCH_FOUND_STATUS, forgetQueued, liveGameOf, readQueued, rememberQueued } from "../net/liveGame.ts";
import { navigate, paths } from "../net/navigate.ts";
import { forgetRoomLink, readRoomLink, roomLinkUrl, sendRoomLink } from "../net/roomLink.ts";
import { rankWords } from "../rank/rank.ts";
import { BackLink, followInApp } from "./nav.tsx";
import "../auth/tavern.css";
import "./lobby.css";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

/** Browser-test contract; `e2e/support/testids.ts` mirrors these strings. */
export const playTestid = {
  queue: "play-queue",
  leaveQueue: "play-leave-queue",
  createRoom: "play-create-room",
  roomCode: "play-room-code",
  roomMode: "play-room-mode",
  joinForm: "play-join-form",
  joinInput: "play-join-code",
  joinSubmit: "play-join-submit",
  status: "play-status",
  error: "play-error",
  practice: "play-practice",
  modeBo1: "play-mode-bo1",
  modeBo3: "play-mode-bo3",
  modeRandom: "play-mode-random",
  deckSelect: "play-deck-select",
  trioSelect: "play-trio-select",
  verdict: "play-choice-verdict",
  /** R505: queue counts belong on the mode tiles. */
  population: "play-population",
  decksLink: "play-decks-link",
  seriesLink: "play-series-link",
  copyRoomCode: "play-copy-room-code",
  copyRoomLink: "play-copy-room-link",
  searching: "play-searching",
  rank: "play-rank",
  leaderboard: "play-leaderboard",
  leanNewest: "play-lean-newest",
} as const;

export const QUEUE_MODES: readonly QueueMode[] = ["bo1", "bo3", "random"];

const MODE_TESTID: Readonly<Record<QueueMode, string>> = {
  bo1: playTestid.modeBo1,
  bo3: playTestid.modeBo3,
  random: playTestid.modeRandom,
};

export function playModeTestid(mode: QueueMode): string {
  return MODE_TESTID[mode];
}

/** R330 calls wire mode `bo3` Conquest. */
export const MODE_LABEL: Readonly<Record<QueueMode, string>> = {
  bo1: "Best of 1",
  bo3: "Conquest",
  random: "All Random",
};

export const PLAY_CHOICE_KEY = "jackioh.play.choice";

/** `crates/server/src/api/http.rs`'s code for a choice the validator refused (R253). */
const LOADOUT_INVALID = "loadout_invalid";

// Remembered choice

type StoredChoice = { mode?: QueueMode; deckId?: string; trioId?: string };

function isQueueMode(value: unknown): value is QueueMode {
  return value === "bo1" || value === "bo3" || value === "random";
}

function readStoredChoice(): StoredChoice {
  try {
    const raw = window.localStorage.getItem(PLAY_CHOICE_KEY);
    if (raw === null) return {};
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return {};
    const { mode, deckId, trioId } = parsed as { mode?: unknown; deckId?: unknown; trioId?: unknown };
    return {
      ...(isQueueMode(mode) ? { mode } : {}),
      ...(typeof deckId === "string" ? { deckId } : {}),
      ...(typeof trioId === "string" ? { trioId } : {}),
    };
  } catch {
    return {};
  }
}

function writeStoredChoice(choice: StoredChoice): void {
  try {
    window.localStorage.setItem(PLAY_CHOICE_KEY, JSON.stringify(choice));
  } catch {
    // Remembering the choice is a convenience; a refusal costs nothing.
  }
}

// Lobby reads

type LobbyData = {
  decks: readonly SavedDeck[];
  trios: readonly SavedTrio[];
  /** A missing catalog skips the verdict. */
  catalog: CatalogSnapshot | null;
  /** A missing collection skips the verdict (L5). */
  collection: Collection | null;
};

type LobbyLoad = { kind: "loading" } | { kind: "ready"; data: LobbyData } | { kind: "failed"; message: string };

function collectionOf(entries: readonly { cardId: string; quantity: number }[]): Collection {
  const owned: Record<string, number> = {};
  for (const entry of entries) owned[entry.cardId] = entry.quantity;
  return owned;
}

function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}

/** Catalog and collection failures skip only the client verdict. */
function useLobbyData(token: string): LobbyLoad {
  const [load, setLoad] = useState<LobbyLoad>({ kind: "loading" });
  useEffect(() => {
    let cancelled = false;
    Promise.all([
      attempt(() => getDecks(token)),
      attempt(() => getCatalog())
        .then((catalog): CatalogSnapshot | null => ({ version: catalog.version, cards: catalog.defs }))
        .catch(() => null),
      attempt(() => getCollection(token))
        .then((collection): Collection | null => collectionOf(collection.entries))
        .catch(() => null),
    ]).then(
      ([decks, catalog, collection]) => {
        if (cancelled) return;
        setLoad({ kind: "ready", data: { decks: decks.decks, trios: decks.trios, catalog, collection } });
      },
      (cause: unknown) => {
        if (!cancelled) setLoad({ kind: "failed", message: messageOf(cause) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token]);
  return load;
}

/** The remembered deck if it is still saved, else the first complete deck, else the first deck. */
export function defaultDeck(decks: readonly SavedDeck[], remembered: string | null): SavedDeck | null {
  return (
    decks.find((deck) => deck.id === remembered) ??
    decks.find((deck) => deck.cards.length === DECK_SIZE) ??
    decks[0] ??
    null
  );
}

/** The remembered trio if it is still saved, else the first trio with three decks, else the first. */
export function defaultTrio(trios: readonly SavedTrio[], remembered: string | null): SavedTrio | null {
  return (
    trios.find((trio) => trio.id === remembered) ??
    trios.find((trio) => trio.deckIds.every((id) => id !== null)) ??
    trios[0] ??
    null
  );
}

/** Queue/room choice; null when its required pick is absent. */
export function choiceFor(
  mode: QueueMode,
  deck: SavedDeck | null,
  trio: SavedTrio | null,
  leanNewest = false,
): ModeChoice | null {
  switch (mode) {
    case "random":
      return leanNewest ? { mode: "random", leanNewest: true } : { mode: "random" };
    case "bo1":
      return deck === null ? null : { mode: "bo1", deckId: deck.id };
    case "bo3":
      return trio === null ? null : { mode: "bo3", trioId: trio.id };
  }
}

/** R253 client-only verdict; missing input has no verdict. */
export function verdictFor(
  mode: QueueMode,
  deck: SavedDeck | null,
  trio: SavedTrio | null,
  data: LobbyData,
): LoadoutResult | null {
  const { catalog, collection } = data;
  if (catalog === null || collection === null) return null;
  if (mode === "bo1") {
    if (deck === null) return null;
    return validateDeck({ deck: { name: deck.name, cards: deck.cards }, catalog, collection });
  }
  if (mode === "bo3") {
    if (trio === null) return null;
    // Missing trio slots are omitted (L1, R253).
    const decks: LoadoutDeck[] = [];
    for (const id of trio.deckIds) {
      const deck = id === null ? undefined : data.decks.find((saved) => saved.id === id);
      if (deck !== undefined) decks.push({ name: deck.name, cards: deck.cards });
    }
    return validateTrio({ decks, catalog, collection });
  }
  return null;
}

// Refusals

function messageOf(cause: unknown): string {
  if (cause instanceof ApiRequestError) return cause.message;
  return cause instanceof Error ? cause.message : String(cause);
}

type LobbyError = {
  message: string;
  issues: readonly string[];
  seriesId: string | null;
};

/** Keeps validator sentences verbatim. */
export function issueMessagesOf(details: unknown): string[] {
  if (!Array.isArray(details)) return [];
  const messages: string[] = [];
  for (const entry of details as unknown[]) {
    if (typeof entry !== "object" || entry === null) return [];
    const message = (entry as { message?: unknown }).message;
    if (typeof message !== "string") return [];
    messages.push(message);
  }
  return messages;
}

function seriesIdOfError(cause: unknown): string | null {
  if (!(cause instanceof ApiRequestError)) return null;
  const details = cause.details;
  if (typeof details !== "object" || details === null) return null;
  const seriesId = (details as { seriesId?: unknown }).seriesId;
  return typeof seriesId === "string" && seriesId.length > 0 ? seriesId : null;
}

function errorOf(cause: unknown): LobbyError {
  return {
    message: messageOf(cause),
    issues:
      cause instanceof ApiRequestError && cause.code === LOADOUT_INVALID ? issueMessagesOf(cause.details) : [],
    seriesId: seriesIdOfError(cause),
  };
}

export function roomModeMessage(mode: QueueMode): string {
  if (mode === "random") return `This room plays ${MODE_LABEL[mode]}: join again.`;
  return `This room plays ${MODE_LABEL[mode]}: pick a ${mode === "bo3" ? "trio" : "deck"} and join again.`;
}

export function roomLinkStatus(mode: QueueMode): string {
  if (mode === "random") return `No deck needed for ${MODE_LABEL[mode]}: press Join.`;
  return `Pick a ${mode === "bo3" ? "trio" : "deck"} for ${MODE_LABEL[mode]}, then press Join.`;
}

// Pairing watch

/** Uses `liveGameOf` so pairing navigation matches the rest of the client. */
export function pairTargetOf(me: {
  currentMatchId: string | null;
  currentSeriesId?: string | null;
}): string | null {
  return liveGameOf(me)?.path ?? null;
}

export { MATCH_FOUND_STATUS };

/** Polls while waiting and reports a pairing once before navigating. */
function useLobbyWatch(token: string, waiting: boolean, onTick: () => void, onPair: (target: string) => void): void {
  // Prevents a slow response from navigating twice after a re-render.
  const navigated = useRef(false);
  const tick = useRef(onTick);
  tick.current = onTick;
  const paired = useRef(onPair);
  paired.current = onPair;

  useEffect(() => {
    let cancelled = false;

    const check = (): void => {
      attempt(() => getMe(token))
        .then((me) => {
          if (cancelled || navigated.current) return;
          const target = pairTargetOf(me);
          if (target !== null) {
            navigated.current = true;
            paired.current(target);
          }
        })
        .catch(() => undefined);
    };

    // Check every mount so a reload after pairing cannot strand the player.
    check();
    if (!waiting) {
      return () => {
        cancelled = true;
      };
    }
    const handle = setInterval(() => {
      check();
      tick.current();
    }, SERIES_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, [token, waiting]);
}

// UI pieces

function DecksLink({ children }: { children: string }): ReactElement {
  return (
    <a href={paths.decks} data-testid={playTestid.decksLink} onClick={followInApp(paths.decks)}>
      {children}
    </a>
  );
}

function Verdict({ result }: { result: LoadoutResult | null }): ReactElement | null {
  if (result === null) return null;
  if (result.ok) {
    return (
      <p className="lobby-verdict" data-testid={playTestid.verdict} data-ready="true">
        <span className="lobby-verdict__mark" aria-hidden="true">
          ✓
        </span>
        Ready to queue
      </p>
    );
  }
  return (
    <div className="lobby-verdict" data-testid={playTestid.verdict} data-ready="false">
      <p className="lobby-verdict__lead">Not ready yet. The server checks again when you queue:</p>
      <ul>
        {result.errors.map((error, index) => (
          // The same message can repeat for two cards; the index keeps the keys apart.
          <li key={`${String(index)}-${error.message}`}>{error.message}</li>
        ))}
      </ul>
    </div>
  );
}

function ModeIcon({ mode }: { mode: QueueMode }): ReactElement {
  const common = { viewBox: "0 0 32 32", "aria-hidden": true, className: "play-mode-tile__icon" } as const;
  switch (mode) {
    case "bo1":
      return (
        <svg {...common}>
          <path d="M22 4h6v6L14 24l-6-6z" />
          <path d="M9 19l4 4-3 3-1.5-1.5L5 28l-1-1 3.5-3.5L6 22z" />
        </svg>
      );
    case "bo3":
      return (
        <svg {...common}>
          <path d="M4 7l6-2 6 2v6c0 4-2.6 6.7-6 8-3.4-1.3-6-4-6-8z" />
          <path d="M16 11l6-2 6 2v6c0 4-2.6 6.7-6 8-3.4-1.3-6-4-6-8z" />
          <path d="M10 15l6-2 6 2v6c0 4-2.6 6.7-6 8-3.4-1.3-6-4-6-8z" />
        </svg>
      );
    case "random":
      return (
        <svg {...common}>
          <rect x="5" y="5" width="22" height="22" rx="5" />
          <circle cx="11" cy="11" r="2" className="play-mode-tile__pip" />
          <circle cx="16" cy="16" r="2" className="play-mode-tile__pip" />
          <circle cx="21" cy="21" r="2" className="play-mode-tile__pip" />
        </svg>
      );
  }
}

function DeckSummary({ deck }: { deck: SavedDeck }): ReactElement {
  const full = deck.cards.length === DECK_SIZE;
  return (
    <p className="play-pick-summary" data-full={full ? "true" : "false"}>
      {String(deck.cards.length)} / {String(DECK_SIZE)} cards
    </p>
  );
}

function TrioSummary({ trio, decks }: { trio: SavedTrio; decks: readonly SavedDeck[] }): ReactElement {
  return (
    <ul className="play-trio-decks" aria-label={`Decks in ${trio.name}`}>
      {trio.deckIds.map((id, slot) => {
        const deck = id === null ? undefined : decks.find((saved) => saved.id === id);
        return (
          // Slot identity distinguishes duplicate empty entries.
          <li key={`${String(slot)}-${id ?? "empty"}`} className="play-trio-deck" data-empty={deck === undefined}>
            {deck === undefined ? "Empty slot" : deck.name}
          </li>
        );
      })}
    </ul>
  );
}

function Searching({ mode }: { mode: QueueMode }): ReactElement {
  return (
    <div className="play-search" data-testid={playTestid.searching} data-mode={mode}>
      <span className="play-search__beacon" aria-hidden="true">
        <span className="play-search__ring" />
        <span className="play-search__ring play-search__ring--late" />
      </span>
      <span className="play-search__text">Looking for a {MODE_LABEL[mode]} opponent…</span>
    </div>
  );
}

function RoomTicket({ room }: { room: Room }): ReactElement {
  const [copied, setCopied] = useState(false);
  const [linkSent, setLinkSent] = useState<"shared" | "copied" | null>(null);
  function copy(): void {
    // Copy failures stay silent because the code remains visible.
    attempt(() => navigator.clipboard.writeText(room.code)).then(
      () => {
        setCopied(true);
      },
      () => undefined,
    );
  }
  function sendLink(): void {
    // R767: sharing failures stay silent, like copying failures.
    attempt(() => sendRoomLink(roomLinkUrl(room.code, room.mode))).then(setLinkSent, () => undefined);
  }
  return (
    <div className="play-ticket">
      <span className="play-ticket__label">Room code</span>
      <code className="play-ticket__code" data-testid={playTestid.roomCode}>
        {room.code}
      </code>
      <span className="lobby-room-mode" data-testid={playTestid.roomMode} data-mode={room.mode}>
        {MODE_LABEL[room.mode]}
      </span>
      <button type="button" className="play-ticket__copy" data-testid={playTestid.copyRoomCode} onClick={copy}>
        {copied ? "Copied" : "Copy"}
      </button>
      <button
        type="button"
        className="play-ticket__copy play-ticket__copy--link"
        data-testid={playTestid.copyRoomLink}
        onClick={sendLink}
      >
        {linkSent === "shared" ? "Link shared" : linkSent === "copied" ? "Link copied" : "Copy invite link"}
      </button>
    </div>
  );
}

function RankPanel({ rank }: { rank: OwnRankResponse | null }): ReactElement {
  return (
    <section className="lobby-card play-panel play-panel--rank" aria-labelledby="play-rank-heading">
      <h2 id="play-rank-heading" className="play-panel__heading">
        Your rank
      </h2>
      {rank === null ? null : (
        <p data-testid={playTestid.rank}>
          {rankWords(rank.rank)} · Season {rank.season}
        </p>
      )}
      <a href={paths.leaderboard} data-testid={playTestid.leaderboard} onClick={followInApp(paths.leaderboard)}>
        Leaderboard →
      </a>
    </section>
  );
}

// Route

export type PlayRouteProps = { token: string };

type Room = { code: string; mode: QueueMode };

/** R505 queue status carries no count. */
export function queuedStatus(mode: QueueMode): string {
  return `In the ${MODE_LABEL[mode]} queue. You will be taken to the game as soon as someone is found. You can leave this screen while you wait.`;
}

export default function PlayRoute({ token }: PlayRouteProps): ReactElement {
  /** R765 resumes a queue left from another screen. */
  const [rejoined] = useState(readQueued);
  /** R767 room link, consumed below. */
  const [link] = useState(readRoomLink);
  const stored = useMemo(readStoredChoice, []);
  // Queue state takes precedence over a link hint and the saved choice.
  const [mode, setMode] = useState<QueueMode>(rejoined ?? link?.mode ?? stored.mode ?? "bo1");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(() => {
    if (rejoined !== null) return queuedStatus(rejoined);
    return link === null ? null : roomLinkStatus(mode);
  });
  const [error, setError] = useState<LobbyError | null>(null);
  const [room, setRoom] = useState<Room | null>(null);
  const [joinCode, setJoinCode] = useState(link?.code ?? "");
  const [waiting, setWaiting] = useState(rejoined !== null);
  const [population, setPopulation] = useState<PopulationResponse | null>(null);
  const [rank, setRank] = useState<OwnRankResponse | null>(null);

  const [deckId, setDeckId] = useState<string | null>(stored.deckId ?? null);
  const [trioId, setTrioId] = useState<string | null>(stored.trioId ?? null);
  const [leanNewest, setLeanNewest] = useState(readPlayLeanNewest);

  const lobby = useLobbyData(token);
  const data = lobby.kind === "ready" ? lobby.data : null;
  const deck = data === null ? null : defaultDeck(data.decks, deckId);
  const trio = data === null ? null : defaultTrio(data.trios, trioId);
  const choice = choiceFor(mode, deck, trio, leanNewest);
  const verdict = data === null ? null : verdictFor(mode, deck, trio, data);

  // Population is informational, never a blocker.
  const refreshPopulation = useCallback((): void => {
    attempt(() => getPopulation(token)).then(setPopulation, () => undefined);
  }, [token]);
  // Keep setup locked during the found-status beat to avoid racing navigation.
  const [found, setFound] = useState(false);
  const foundTarget = useRef<string | null>(null);
  const foundTimer = useRef<number | null>(null);
  useEffect(
    () => () => {
      if (foundTimer.current !== null) window.clearTimeout(foundTimer.current);
    },
    [],
  );
  // Delay navigation so status paints; watcher and response share one target.
  const goFound = useCallback((target: string): void => {
    if (foundTarget.current !== null) return;
    foundTarget.current = target;
    forgetQueued();
    setFound(true);
    setStatus(MATCH_FOUND_STATUS);
    foundTimer.current = window.setTimeout(() => {
      navigate(target);
    }, MATCH_FOUND_NAV_DELAY_MS);
  }, []);
  useLobbyWatch(token, waiting, refreshPopulation, goFound);
  useEffect(() => {
    refreshPopulation();
  }, [refreshPopulation]);

  useEffect(() => {
    let cancelled = false;
    attempt(() => getOwnRank(token)).then(
      (next) => {
        if (!cancelled) setRank(next);
      },
      () => undefined,
    );
    return () => {
      cancelled = true;
    };
  }, [token]);

  // Persist the displayed IDs so stale saved choices are replaced.
  const deckShown = deck?.id;
  const trioShown = trio?.id;
  useEffect(() => {
    if (data === null) return;
    writeStoredChoice({
      mode,
      ...(deckShown === undefined ? {} : { deckId: deckShown }),
      ...(trioShown === undefined ? {} : { trioId: trioShown }),
    });
  }, [data, mode, deckShown, trioShown]);

  // R767 link hints apply only once.
  useEffect(() => {
    forgetRoomLink();
  }, []);
  // R767 focuses the link's requested pick once it is available.
  const deckSelect = useRef<HTMLSelectElement>(null);
  const trioSelect = useRef<HTMLSelectElement>(null);
  const joinSubmit = useRef<HTMLButtonElement>(null);
  const focusPending = useRef(link !== null && rejoined === null);
  useEffect(() => {
    if (!focusPending.current) return;
    if (mode !== "random" && lobby.kind === "loading") return;
    focusPending.current = false;
    if (mode === "bo1") deckSelect.current?.focus();
    else if (mode === "bo3") trioSelect.current?.focus();
    else joinSubmit.current?.focus();
  }, [lobby.kind, mode]);

  function run(work: () => Promise<void>): void {
    if (busy) return;
    setBusy(true);
    setError(null);
    work()
      .catch((cause: unknown) => {
        setError(errorOf(cause));
      })
      .finally(() => {
        setBusy(false);
      });
  }

  /** Routes immediate pair answers through the found-status flow. */
  function follow(answer: { matchId: string | null; seriesId: string | null }): boolean {
    if (typeof answer.matchId === "string" && answer.matchId.length > 0) {
      goFound(paths.match(answer.matchId));
      return true;
    }
    if (typeof answer.seriesId === "string" && answer.seriesId.length > 0) {
      goFound(paths.series(answer.seriesId));
      return true;
    }
    return false;
  }

  function onEnqueue(): void {
    if (choice === null) return;
    const chosen = choice;
    run(async () => {
      const result = await enqueue(token, chosen);
      if (follow(result)) return;
      setWaiting(true);
      // R765 queue state follows the player across routes.
      rememberQueued(chosen.mode);
      refreshPopulation();
      // R505 status names the queue but never its population.
      setStatus(queuedStatus(chosen.mode));
    });
  }

  function onLeaveQueue(): void {
    run(async () => {
      await dequeue(token);
      forgetQueued();
      setWaiting(false);
      setStatus("Left the queue.");
    });
  }

  function onCreateRoom(): void {
    if (choice === null) return;
    const chosen = choice;
    run(async () => {
      const created = await createRoom(token, chosen);
      setRoom({ code: created.code, mode: created.mode });
      setWaiting(true);
      setStatus("Give your opponent this code. You will be taken to the game when they join.");
    });
  }

  function dropLinkStatus(named: QueueMode): void {
    setStatus((current) => (current === roomLinkStatus(named) ? null : current));
  }

  function onJoin(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    if (choice === null) return;
    const chosen = choice;
    run(async () => {
      try {
        // R104 matches server-side room-code canonicalization.
        const joined = await joinRoom(token, joinCode.trim().toUpperCase(), chosen);
        follow(joined);
      } catch (cause: unknown) {
        // R264 switches to the room's mode while retaining its code.
        const roomMode = roomModeOf(cause);
        if (roomMode === null) throw cause;
        setMode(roomMode);
        dropLinkStatus(chosen.mode);
        setError({ message: roomModeMessage(roomMode), issues: [], seriesId: null });
      }
    });
  }

  const noChoice = choice === null;
  const locked = waiting || found;
  const queued = waiting && room === null && !found;
  const byMode = population?.byMode;

  return (
    <div className="app-shell tavern lobby play-screen" data-waiting={locked ? "true" : "false"}>
      <BackLink />

      <header className="play-hero">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <p className="play-hero__title">Play online</p>
        <p className="play-hero__lead">Pick a mode, bring a deck, and find someone to play.</p>
      </header>

      {status !== null ? (
        <p
          className={status === MATCH_FOUND_STATUS ? "notice play-status--found" : "notice"}
          data-testid={playTestid.status}
          role="status"
        >
          {status}
        </p>
      ) : null}
      {error !== null ? (
        <div className="notice" data-testid={playTestid.error} role="alert">
          {error.issues.length === 0 ? (
            <p>{error.message}</p>
          ) : (
            <ul className="lobby-issues">
              {error.issues.map((issue, index) => (
                <li key={`${String(index)}-${issue}`}>{issue}</li>
              ))}
            </ul>
          )}
          {error.seriesId === null ? null : (
            <a
              href={paths.series(error.seriesId)}
              data-testid={playTestid.seriesLink}
              onClick={followInApp(paths.series(error.seriesId))}
            >
              Go to your series →
            </a>
          )}
        </div>
      ) : null}
      <div className="play-layout">
        <section className="lobby-card play-panel play-panel--setup" aria-labelledby="play-setup-heading">
          <h2 id="play-setup-heading" className="play-panel__heading">
            <span className="play-step" aria-hidden="true">
              1
            </span>
            How do you want to play?
          </h2>
          <fieldset
            className="lobby-modes"
            data-testid={playTestid.population}
            data-bo1={byMode?.bo1}
            data-bo3={byMode?.bo3}
            data-random={byMode?.random}
          >
            <legend className="lobby-modes__legend">Mode</legend>
            {QUEUE_MODES.map((option) => (
              <label
                key={option}
                className="lobby-mode play-mode-tile"
                data-selected={option === mode ? "true" : "false"}
                data-mode={option}
              >
                <input
                  type="radio"
                  name="play-mode"
                  value={option}
                  checked={option === mode}
                  data-testid={playModeTestid(option)}
                  disabled={locked}
                  onChange={() => {
                    dropLinkStatus(mode);
                    setMode(option);
                  }}
                />
                <ModeIcon mode={option} />
                <span className="lobby-mode__text">
                  <span className="lobby-mode__name">{MODE_LABEL[option]}</span>
                </span>
                {byMode === undefined ? null : (
                  <span className="play-mode-tile__waiting">{String(byMode[option])} waiting</span>
                )}
              </label>
            ))}
          </fieldset>

          <h2 className="play-panel__heading play-panel__heading--sub">
            <span className="play-step" aria-hidden="true">
              2
            </span>
            {mode === "bo3" ? "Bring a trio" : mode === "bo1" ? "Bring a deck" : "No deck to bring"}
          </h2>

          {lobby.kind === "loading" && mode !== "random" ? (
            <p className="lobby-note" role="status">
              Loading your decks…
            </p>
          ) : null}
          {lobby.kind === "failed" && mode !== "random" ? (
            <p className="notice" role="alert">
              Your decks could not be loaded: {lobby.message}
            </p>
          ) : null}

          {data !== null && mode === "bo1" ? (
            data.decks.length === 0 ? (
              <p className="lobby-note">
                You have no saved decks yet. <DecksLink>Build one in Decks</DecksLink>.
              </p>
            ) : (
              <div className="play-pick">
                <label htmlFor="play-deck">Your deck</label>
                <select
                  id="play-deck"
                  ref={deckSelect}
                  className="lobby-select"
                  data-testid={playTestid.deckSelect}
                  value={deck?.id ?? ""}
                  disabled={locked}
                  onChange={(event) => {
                    setDeckId(event.target.value);
                  }}
                >
                  {data.decks.map((saved) => (
                    <option key={saved.id} value={saved.id}>
                      {saved.name}
                    </option>
                  ))}
                </select>
                {deck === null ? null : <DeckSummary deck={deck} />}
              </div>
            )
          ) : null}

          {data !== null && mode === "bo3" ? (
            data.trios.length === 0 ? (
              <p className="lobby-note">
                You have no saved trios yet. <DecksLink>Build one in Decks</DecksLink>.
              </p>
            ) : (
              <div className="play-pick">
                <label htmlFor="play-trio">Your trio</label>
                <select
                  id="play-trio"
                  ref={trioSelect}
                  className="lobby-select"
                  data-testid={playTestid.trioSelect}
                  value={trio?.id ?? ""}
                  disabled={locked}
                  onChange={(event) => {
                    setTrioId(event.target.value);
                  }}
                >
                  {data.trios.map((saved) => (
                    <option key={saved.id} value={saved.id}>
                      {saved.name}
                    </option>
                  ))}
                </select>
                {trio === null ? null : <TrioSummary trio={trio} decks={data.decks} />}
              </div>
            )
          ) : null}

          {mode === "random" ? (
            <>
              <p className="lobby-note">No deck needed: the server deals both of you one when the game starts.</p>
              <LeanNewestToggle
                checked={leanNewest}
                disabled={locked}
                testid={playTestid.leanNewest}
                onChange={(on) => {
                  setLeanNewest(on);
                  writePlayLeanNewest(on);
                }}
              />
            </>
          ) : null}

          <Verdict result={verdict} />
        </section>

        <div className="play-side">
          <section className="lobby-card play-panel play-panel--queue" aria-labelledby="play-queue-heading">
            <h2 id="play-queue-heading" className="play-panel__heading">
              <span className="play-step" aria-hidden="true">
                3
              </span>
              Find a match
            </h2>
            {queued ? <Searching mode={mode} /> : null}
            <button
              type="button"
              className="button-primary play-cta"
              data-testid={playTestid.queue}
              disabled={busy || noChoice || locked}
              onClick={onEnqueue}
            >
              Find a match
            </button>
            <button
              type="button"
              className="link-button play-leave"
              data-testid={playTestid.leaveQueue}
              disabled={busy || found}
              onClick={onLeaveQueue}
            >
              Leave the queue
            </button>
          </section>

          <RankPanel rank={rank} />

          <section className="lobby-card play-panel play-panel--room" aria-labelledby="play-room-heading">
            <h2 id="play-room-heading" className="play-panel__heading">
              Play a friend
            </h2>
            <p className="lobby-note">A room plays the mode it was made with; the joiner picks for the same mode.</p>
            <button
              type="button"
              className="play-room-create"
              data-testid={playTestid.createRoom}
              disabled={busy || noChoice || locked}
              onClick={onCreateRoom}
            >
              Create a room
            </button>
            {room === null ? null : <RoomTicket room={room} />}

            <form className="play-join" data-testid={playTestid.joinForm} onSubmit={onJoin}>
              <label htmlFor="play-join-code">Join a room</label>
              <div className="play-join__row">
                <input
                  id="play-join-code"
                  data-testid={playTestid.joinInput}
                  value={joinCode}
                  autoComplete="off"
                  spellCheck={false}
                  placeholder="Code"
                  disabled={locked}
                  onChange={(event) => {
                    setJoinCode(event.target.value);
                  }}
                />
                <button
                  type="submit"
                  ref={joinSubmit}
                  data-testid={playTestid.joinSubmit}
                  disabled={busy || noChoice || locked}
                >
                  Join
                </button>
              </div>
            </form>
          </section>

          <section className="lobby-card play-panel play-panel--practice" aria-labelledby="play-practice-heading">
            <h2 id="play-practice-heading" className="play-panel__heading">
              Practice
            </h2>
            <p>A game against the AI, right here in your browser: no queue, no rating, three difficulties.</p>
            <a href={paths.practice} data-testid={playTestid.practice}>
              Practice against the AI →
            </a>
          </section>
        </div>
      </div>

    </div>
  );
}
