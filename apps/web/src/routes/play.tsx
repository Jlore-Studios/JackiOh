// Play hub (SPEC §9.5; R253, R257, R258, R259, R264, R330, R505, R612, R661, R765, R1372, R640002, R3207000,
// R3207003, R3729000, R3729001, R3792000). Verdicts are UX only (CLAUDE.md rule 7); the server owns validation and
// matchmaking. Rank never exposes its hidden rating. R1578000, R3729000: /play is the hub every play mode starts
// from, four groups of cards (`PlayHub.tsx`, the words `playModes.ts`'s); an Online card opens one panel
// (`QueuePanel.tsx`) with the deck or trio and Find a match, and the Private lobby card the private lobby's panels
// (`lobby/LobbyPanel.tsx`, R3730000). R3729001: while queued, the search shows in the shell's banner.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from "react";

import { DECK_SIZE } from "@jackioh/engine/config";
import {
  validateDeck,
  validateTrio,
  type CatalogSnapshot,
  type Collection,
  type LoadoutDeck,
  type LoadoutResult,
} from "@jackioh/validator";

import {
  MATCH_FOUND_NAV_DELAY_MS,
  QUEUE_ENDED_AFTER_READS,
  QUEUE_POPULATION_POLL_SECONDS,
  SERIES_POLL_SECONDS,
} from "@jackioh/server-config";
import type { LeanPick } from "@jackioh/shared";
import {
  ApiRequestError,
  ApiUnreachableError,
  dequeue,
  enqueue,
  getCollection,
  getDecks,
  getMe,
  getOwnRank,
  ladderOfMode,
  type Ladder,
  type MeResponse,
  getPopulation,
  isQueueMode,
  type ModeChoice,
  type OwnRankResponse,
  type PopulationResponse,
  type QueueMode,
  type SavedDeck,
  type SavedTrio,
} from "../net/api.ts";
import { Dialog } from "../game/Dialog.tsx";
import { currentCatalog } from "../game/currentCatalog.ts";
import { pageWake } from "../game/net.ts";
import { readPlayLean, writePlayLean } from "../game/LeanNewest.tsx";
import { MATCH_FOUND_STATUS, MODE_LABEL, forgetQueued, liveGameOf, readQueued, rememberQueued, useQueued } from "../net/liveGame.ts";
import { isFreshAccount } from "../net/gate.ts";
import { matchIdOf, navigate, paths } from "../net/navigate.ts";
import { LobbyPanel } from "../lobby/LobbyPanel.tsx";
import { useMenuShell } from "../shell/context.ts";
import { BackLink, followInApp } from "./nav.tsx";
import { ArrowIcon } from "./ArrowIcon.tsx";
import { OpponentLine } from "./GameBanner.tsx";
import { PlayHub } from "./PlayHub.tsx";
import { playCards } from "./playModes.ts";
import { QueuePanel, Verdict, type QueuePicks } from "./QueuePanel.tsx";
import { playTestid, playWaitTestid } from "./playTestids.ts";
import { Wordmark } from "./Wordmark.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { waitWords } from "../net/queueReadout.ts";
import { getStored, setStored } from "../net/storage.ts";
import { useSocialReads } from "../together/socialPoll.ts";
import "../auth/tavern.css";
import "./lobby.css";
import "./play-hub.css";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

/** R3792000: account reads of a wait failing in a row (the server unreachable, or answering 5xx, R3846000) before the screen says so; at SERIES_POLL_SECONDS apart it is inside the server's ticket timeout (R897000). */
export const LOBBY_UNREACHABLE_AFTER_READS = 3;

/** R3846000: the first status of an answer that says the server was not reached: in production the edge Worker answers for a server that is down with a 500, 502, 503 or 504 (`apps/edge/src/index.ts`, `containerFetch`). */
const FIRST_UNREACHABLE_STATUS = 500;

/** Browser-test contract (`playTestids.ts`, which has no imports); `e2e/support/testids.ts` mirrors these strings. */
export { playTestid, playWaitTestid };

export const QUEUE_MODES: readonly QueueMode[] = ["bo1", "bo3", "random"];

const MODE_TESTID: Readonly<Record<QueueMode, string>> = {
  bo1: playTestid.modeBo1,
  bo3: playTestid.modeBo3,
  random: playTestid.modeRandom,
};

export function playModeTestid(mode: QueueMode): string {
  return MODE_TESTID[mode];
}

/** `net/queueReadout.ts` words the expected wait, for the shell's banner too (R3729001); kept here for this file's importers. */
export { waitWords };

/** R640002: the games that ended in the last hour, bot games among them. */
export function recentGamesWords(games: number): string {
  return games === 1
    ? t("1 game played in the last hour")
    : t("{count} games played in the last hour", { count: String(games) });
}

/** `net/liveGame.ts` names the modes, for the shell's queue banner too (R3728002); kept here for this file's importers. */
export { MODE_LABEL };

export const PLAY_CHOICE_KEY = "jackioh.play.choice";

/** `crates/server/src/api/http.rs`'s code for a choice the validator refused (R253). */
const LOADOUT_INVALID = "loadout_invalid";

// Remembered choice

type StoredChoice = { mode?: QueueMode; deckId?: string; trioId?: string };

function readStoredChoice(): StoredChoice {
  try {
    const raw = getStored(PLAY_CHOICE_KEY);
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
  // Remembering the choice is a convenience; a refusal costs nothing.
  setStored(PLAY_CHOICE_KEY, JSON.stringify(choice));
}

// Lobby reads

export type LobbyData = {
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

/** A collection failure skips only the client verdict; the catalog is the one the client ships (game/currentCatalog.ts). */
export function useLobbyData(token: string): [LobbyLoad, () => void] {
  const [load, setLoad] = useState<LobbyLoad>({ kind: "loading" });
  const [round, setRound] = useState(0);
  useEffect(() => {
    let canceled = false;
    Promise.all([
      attempt(() => getDecks(token)),
      attempt(() => getCollection(token))
        .then((collection): Collection | null => collectionOf(collection.entries))
        .catch(() => null),
    ]).then(
      ([decks, collection]) => {
        if (canceled) return;
        setLoad({ kind: "ready", data: { decks: decks.decks, trios: decks.trios, catalog: currentCatalog(), collection } });
      },
      (cause: unknown) => {
        if (!canceled) setLoad({ kind: "failed", message: messageOf(cause) });
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, round]);
  const retry = useCallback((): void => {
    setLoad({ kind: "loading" });
    setRound((before) => before + 1);
  }, []);
  return [load, retry];
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

/** The remembered trio if it is still saved, else the first with three decks, else the first trio. */
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
  lean: LeanPick | null = null,
): ModeChoice | null {
  switch (mode) {
    case "random":
      return lean === null ? { mode: "random" } : { mode: "random", leanSet: lean };
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

type LobbyError = {
  message: string;
  issues: readonly string[];
  seriesId: string | null;
  /** R1566000: the live co-op match an `already_in_match` refusal names (R1065000). */
  duosMatchId: string | null;
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

/** The id a refusal's `details` names under `key` (R264's series, R1065000's co-op match), or null. */
function detailIdOfError(cause: unknown, key: "seriesId" | "duosMatchId"): string | null {
  if (!(cause instanceof ApiRequestError)) return null;
  const details = cause.details;
  if (typeof details !== "object" || details === null) return null;
  const id = (details as Record<string, unknown>)[key];
  return typeof id === "string" && id.length > 0 ? id : null;
}

function errorOf(cause: unknown): LobbyError {
  return {
    message: messageOf(cause),
    issues:
      cause instanceof ApiRequestError && cause.code === LOADOUT_INVALID ? issueMessagesOf(cause.details) : [],
    seriesId: detailIdOfError(cause, "seriesId"),
    duosMatchId: detailIdOfError(cause, "duosMatchId"),
  };
}

// The status line

/** The line under the header, worded at render so a language switch rewords it. */
type Status =
  | { kind: "queued"; mode: QueueMode }
  | { kind: "queueEnded" }
  | { kind: "found" }
  | { kind: "left" };

// Pairing watch

/** `liveGameOf`, as the rest of the client. R1566000: a co-op game is no pairing; the refusal links it. */
export function pairTargetOf(me: {
  currentMatchId: string | null;
  currentSeriesId?: string | null;
}): string | null {
  return liveGameOf(me)?.path ?? null;
}

export { MATCH_FOUND_STATUS, Verdict };

/** R3846000: a read that did not reach the game server: a transport failure or timeout, or the edge's 5xx answer for a server that is down. A 4xx proves the server was reached. */
function isUnreachableRead(cause: unknown): boolean {
  return (
    cause instanceof ApiUnreachableError ||
    (cause instanceof ApiRequestError && cause.status >= FIRST_UNREACHABLE_STATUS)
  );
}

/**
 * Polls while waiting and reports a pairing once before navigating. R897000: a queue ticket the
 * server calls gone `QUEUE_ENDED_AFTER_READS` reads in a row ends the wait once instead. R3207003:
 * `known` is the account the gate read for this screen, which the first run takes as its mount read
 * when it is fresh and names no game. R3792000: returns whether the wait's reads have failed unreachable (a transport failure, a timeout or a 5xx answer)
 * `LOBBY_UNREACHABLE_AFTER_READS` times in a row; the first read the server answered clears it.
 */
function useLobbyWatch(
  token: string,
  waiting: boolean,
  onPair: (target: string) => void,
  onQueueEnded: () => void,
  queueWait: boolean,
  known: MeResponse | undefined,
): boolean {
  // Prevents a slow response from navigating twice after a re-render.
  const navigated = useRef(false);
  // R897000: like `navigated`, but for the queue ending under the wait.
  const ended = useRef(false);
  /** R3207003: spent by the first run, so a later run (the wait starting or ending) reads for itself. */
  const gateMe = useRef(known);
  const paired = useRef(onPair);
  paired.current = onPair;
  const queueEnded = useRef(onQueueEnded);
  queueEnded.current = onQueueEnded;
  /** R897000: consecutive reads saying the ticket is gone. */
  const endedReads = useRef(0);
  /** R3792000: consecutive reads that could not reach the server. */
  const failedReads = useRef(0);
  const [unreachable, setUnreachable] = useState(false);

  useEffect(() => {
    let canceled = false;
    if (!waiting) {
      // Idle again: a later wait judges its own reads from nothing.
      ended.current = false;
      endedReads.current = 0;
      failedReads.current = 0;
      setUnreachable(false);
    }

    // A wake while a read is out would only repeat it; each effect run counts its own.
    let reading = false;
    const check = (): void => {
      if (reading) return;
      reading = true;
      attempt(() => getMe(token))
        .then((me) => {
          if (canceled) return;
          // R3792000: an answer of any kind proves the server was reached.
          failedReads.current = 0;
          setUnreachable(false);
          if (navigated.current || ended.current) return;
          const target = pairTargetOf(me);
          if (target !== null) {
            navigated.current = true;
            paired.current(target);
            return;
          }
          if (!waiting) return;
          // Explicit `false` only (`undefined` never ends a wait), and only of a wait this tab
          // remembers (R765).
          if (me.queued === false && queueWait) {
            endedReads.current += 1;
            if (endedReads.current >= QUEUE_ENDED_AFTER_READS) {
              ended.current = true;
              queueEnded.current();
            }
          } else {
            endedReads.current = 0;
          }
        })
        .catch((cause: unknown) => {
          if (canceled || !waiting) return;
          if (isUnreachableRead(cause)) {
            failedReads.current += 1;
            if (failedReads.current >= LOBBY_UNREACHABLE_AFTER_READS) setUnreachable(true);
          } else {
            // The server answered (a refusal: 4xx): reached, so the count starts again.
            failedReads.current = 0;
            setUnreachable(false);
          }
        })
        .finally(() => {
          reading = false;
        });
    };

    // R3205000: this wait keeps reading in a hidden tab, unlike the polls on `pollWhileVisible`: a queue
    // wait's reads keep its ticket alive (R897000).
    // Check every mount so a reload after pairing cannot strand the player.
    // R3207003: the first run starts from the account the gate read for this screen, when that answer is
    // fresh (inside READ_CACHE_FRESH_MS) and names no game. A kept answer the gate opened on may be of any
    // age, and one that names a game may have ended since (it must not send the player back into it): both
    // are read again.
    const handed = gateMe.current;
    gateMe.current = undefined;
    if (handed === undefined || pairTargetOf(handed) !== null || !isFreshAccount(handed)) check();
    if (!waiting) {
      return () => {
        canceled = true;
      };
    }
    const handle = setInterval(check, SERIES_POLL_SECONDS * MS_PER_SECOND);
    // A throttled tab's interval stalls; waking it reads the wait again at once.
    const stopWake = pageWake(check);
    return () => {
      canceled = true;
      stopWake();
      clearInterval(handle);
    };
  }, [token, waiting, queueWait]);
  return waiting && unreachable;
}

// UI pieces

function Searching({ mode }: { mode: QueueMode }): ReactElement {
  return (
    <div className="play-search" data-testid={playTestid.searching} data-mode={mode}>
      <span className="play-search__beacon" aria-hidden="true">
        <span className="play-search__ring" />
        <span className="play-search__ring play-search__ring--late" />
      </span>
      <span className="play-search__text">
        {t("Looking for a {mode} opponent…", { mode: t(MODE_LABEL[mode]) })}
      </span>
    </div>
  );
}

function PlayHero(): ReactElement {
  return (
    <header className="play-hero">
      <Wordmark />
    </header>
  );
}

// Route

/** R1578000: `null` is a visitor with no account, who reads nothing and is offered sign-in. */
export type PlayRouteProps = {
  token: string | null;
  /** R3207003: the account the gate read for this screen, so the lobby opens without reading it again. */
  me?: MeResponse;
};

/** R505 queue status carries no count. */
export function queuedStatus(mode: QueueMode): string {
  return t(
    "In the {mode} queue. You will be taken to the game as soon as someone is found. You can leave this screen while you wait.",
    { mode: t(MODE_LABEL[mode]) },
  );
}

function statusWords(status: Status): string {
  switch (status.kind) {
    case "queued":
      return queuedStatus(status.mode);
    case "queueEnded":
      return t("Your queue expired. Find a match to queue again.");
    case "found":
      return t(MATCH_FOUND_STATUS);
    case "left":
      return t("Left the queue.");
  }
}

/** Both ladders, whose ranks the hub's cards show (R2778000). */
const LADDERS: readonly Ladder[] = ["constructed", "random"];

const NO_RANKS: Readonly<Record<Ladder, OwnRankResponse | null>> = { constructed: null, random: null };

/** A refusal, in the server's words, with the way to the series or the co-op game it names. */
function Refusal({ error }: { error: LobbyError }): ReactElement {
  return (
    <div className="notice" data-testid={playTestid.error} role="alert">
      {error.issues.length === 0 ? (
        <p>{t(error.message)}</p>
      ) : (
        <ul className="lobby-issues">
          {error.issues.map((issue, index) => (
            <li key={`${String(index)}-${issue}`}>{t(issue)}</li>
          ))}
        </ul>
      )}
      {error.seriesId === null ? null : (
        <a
          href={paths.series(error.seriesId)}
          data-testid={playTestid.seriesLink}
          onClick={followInApp(paths.series(error.seriesId))}
        >
          {t("Go to your series")}
          <ArrowIcon direction="forward" />
        </a>
      )}
      {error.duosMatchId === null ? null : (
        <a
          href={paths.duos(error.duosMatchId)}
          data-testid={playTestid.duosLink}
          onClick={followInApp(paths.duos(error.duosMatchId))}
        >
          {t("Go to your co-op game")}
          <ArrowIcon direction="forward" />
        </a>
      )}
    </div>
  );
}

function SignedOutPlay(): ReactElement {
  useLocale();
  return (
    <div className="app-shell tavern lobby play-screen play-hub" data-testid={playTestid.screen}>
      <BackLink title={t("Play")} />
      <PlayHero />
      <div className="play-body">
        <section
          className="play-online play-online--signed-out"
          data-testid={playTestid.online}
          aria-labelledby="play-online-heading"
        >
          <h2 id="play-online-heading" className="play-online__heading">
            {t("Play online")}
          </h2>
          <p>{t("Online play is invite-only for now: you'll need an invite code after signing up.")}</p>
          <a
            className="button-primary play-cta"
            href={paths.login}
            data-testid={playTestid.signIn}
            onClick={followInApp(paths.login)}
          >
            {t("Sign in")}
          </a>
        </section>
        <PlayHub
          cards={playCards({ signedIn: false, dailyWaiting: 0 })}
          lastMode={null}
          ranks={NO_RANKS}
          expectedWait={undefined}
          dailyWaiting={0}
          locked={false}
          onPick={() => undefined}
          onLobby={() => undefined}
        />
      </div>
    </div>
  );
}

export default function PlayRoute({ token, me }: PlayRouteProps): ReactElement {
  return token === null ? <SignedOutPlay /> : <OnlinePlay token={token} me={me} />;
}

/** R3729000: what the hub has open over it: an Online card's panel (by mode), the Private lobby's, or nothing. */
type Panel = QueueMode | "lobby" | null;

function OnlinePlay({ token, me }: { token: string; me: MeResponse | undefined }): ReactElement {
  useLocale();
  /** R3729001: inside the menus' shell the banner shows the search, so this screen draws none of its own. */
  const shell = useMenuShell();
  /** R765 resumes a queue left from another screen. */
  const [rejoined] = useState(readQueued);
  const stored = useMemo(readStoredChoice, []);
  // Queue state takes precedence over the saved choice.
  const [mode, setMode] = useState<QueueMode>(rejoined ?? stored.mode ?? "bo1");
  /** R3729000: the card marked "Last used": the mode the player queued or picked last, none for a player with no choice kept. */
  const [lastMode, setLastMode] = useState<QueueMode | null>(rejoined ?? stored.mode ?? null);
  const [panel, setPanel] = useState<Panel>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<Status | null>(() =>
    rejoined === null ? null : { kind: "queued", mode: rejoined },
  );
  const [error, setError] = useState<LobbyError | null>(null);
  const [waiting, setWaiting] = useState(rejoined !== null);
  const [population, setPopulation] = useState<PopulationResponse | null>(null);
  const [ranks, setRanks] = useState<Readonly<Record<Ladder, OwnRankResponse | null>>>(NO_RANKS);
  // #3727: the social read's count of the daily games waiting on the player (none signed out).
  const { dailyWaiting } = useSocialReads();

  const [deckId, setDeckId] = useState<string | null>(stored.deckId ?? null);
  const [trioId, setTrioId] = useState<string | null>(stored.trioId ?? null);
  const [lean, setLean] = useState(readPlayLean);

  const [lobby, retryLobby] = useLobbyData(token);
  const data = lobby.kind === "ready" ? lobby.data : null;
  const deck = data === null ? null : defaultDeck(data.decks, deckId);
  const trio = data === null ? null : defaultTrio(data.trios, trioId);
  const choice = choiceFor(mode, deck, trio, lean);
  const verdict = data === null ? null : verdictFor(mode, deck, trio, data);

  // The readout is informational, never a blocker.
  const refreshPopulation = useCallback((): void => {
    attempt(() => getPopulation(token)).then(setPopulation, () => undefined);
  }, [token]);
  // Keep setup locked during the found-status beat to avoid racing navigation.
  const [foundPath, setFoundPath] = useState<string | null>(null);
  const found = foundPath !== null;
  const foundTarget = useRef<string | null>(null);
  const foundTimer = useRef<number | null>(null);
  useEffect(
    () => () => {
      if (foundTimer.current !== null) window.clearTimeout(foundTimer.current);
    },
    [],
  );
  /** R3729001: set by the code that forgets the queue itself, so the wait it ends is not taken for one ended elsewhere. */
  const ownForget = useRef(false);
  // Delay navigation so status paints; watcher and response share one target.
  const goFound = useCallback((target: string): void => {
    if (foundTarget.current !== null) return;
    foundTarget.current = target;
    ownForget.current = true;
    forgetQueued();
    setPanel(null);
    setFoundPath(target);
    setStatus({ kind: "found" });
    foundTimer.current = window.setTimeout(() => {
      navigate(target);
    }, MATCH_FOUND_NAV_DELAY_MS);
  }, []);
  // R897000: Find-a-match is the way back into a queue the server no longer holds. Only a wait this
  // tab remembers (R765) can end.
  const queued = useQueued();
  const queueWait = queued !== null;
  const onQueueEnded = useCallback((): void => {
    ownForget.current = true;
    forgetQueued();
    setWaiting(false);
    setStatus({ kind: "queueEnded" });
  }, []);
  const unreachable = useLobbyWatch(token, waiting, goFound, onQueueEnded, queueWait, me);
  // R3729001: Cancel on the shell's banner forgets the queue from outside this screen, which ends the wait here
  // too. A wait this tab never remembered (blocked storage) has no banner to cancel, so it is never taken for one.
  const sawQueued = useRef(rejoined !== null);
  useEffect(() => {
    if (queued !== null) {
      sawQueued.current = true;
      return;
    }
    if (!sawQueued.current) return;
    sawQueued.current = false;
    if (ownForget.current) {
      ownForget.current = false;
      return;
    }
    if (!waiting || foundTarget.current !== null) return;
    setWaiting(false);
    setStatus({ kind: "left" });
  }, [queued, waiting]);
  useEffect(() => {
    refreshPopulation();
  }, [refreshPopulation]);
  // R3207000: while waiting, the readout is read again at its own pace; the read above is the mount's.
  // The wait's account read keeps SERIES_POLL_SECONDS (R897000), and this one holds no ticket.
  useEffect(() => {
    if (!waiting) return;
    const handle = setInterval(refreshPopulation, QUEUE_POPULATION_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      clearInterval(handle);
    };
  }, [refreshPopulation, waiting]);

  // R2778000, R3729000: both ladders' ranks, read at mount, for the cards and the panel.
  useEffect(() => {
    let canceled = false;
    for (const ladder of LADDERS) {
      attempt(() => getOwnRank(token, ladder)).then(
        (next) => {
          if (!canceled) setRanks((before) => ({ ...before, [ladder]: next }));
        },
        () => undefined,
      );
    }
    return () => {
      canceled = true;
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

  function onPick(picked: QueueMode): void {
    setMode(picked);
    setLastMode(picked);
    setError(null);
    setPanel(picked);
  }

  function onEnqueue(): void {
    if (choice === null) return;
    const chosen = choice;
    run(async () => {
      const result = await enqueue(token, chosen);
      // The panel's work is done once the server took the ticket: the search shows in the banner (R3729001).
      setPanel(null);
      if (follow(result)) return;
      ownForget.current = false;
      setWaiting(true);
      rememberQueued(chosen.mode);
      refreshPopulation();
      // R505, R640002: the status names the queue, never its readout.
      setStatus({ kind: "queued", mode: chosen.mode });
    });
  }

  function onLeaveQueue(): void {
    run(async () => {
      await dequeue(token);
      ownForget.current = true;
      forgetQueued();
      setWaiting(false);
      setStatus({ kind: "left" });
    });
  }

  const noChoice = choice === null;
  const locked = waiting || found;
  const searching = waiting && !found;
  const expectedWait = population?.expectedWaitSeconds;
  const games = population?.gamesLastHour ?? 0;
  const picks: QueuePicks =
    lobby.kind === "ready" && data !== null
      ? { kind: "ready", decks: data.decks, trios: data.trios, deck, trio }
      : lobby.kind === "failed"
        ? { kind: "failed", message: lobby.message, retry: retryLobby }
        : { kind: "loading" };
  const closePanel = (): void => {
    setPanel(null);
  };
  const refusal = error === null ? null : <Refusal error={error} />;
  const foundMatch = foundPath === null ? null : matchIdOf(foundPath);

  return (
    <div
      className="app-shell tavern lobby play-screen play-hub"
      data-testid={playTestid.screen}
      data-waiting={locked ? "true" : "false"}
    >
      <BackLink title={t("Play")} />
      <PlayHero />

      {status !== null ? (
        <p
          className={status.kind === "found" ? "notice play-status--found" : "notice"}
          data-testid={playTestid.status}
          role="status"
        >
          {statusWords(status)}
          {status.kind === "found" && foundMatch !== null ? <OpponentLine matchId={foundMatch} /> : null}
        </p>
      ) : null}
      {unreachable ? (
        <p className="notice" data-testid={playTestid.unreachable} role="status">
          {t("Can't reach the server. Still trying…")}
        </p>
      ) : null}
      {panel === null ? refusal : null}
      <div className="play-body">
        {searching && shell === null ? (
          <section className="play-searching" aria-label={t("Find a match")}>
            <Searching mode={mode} />
            <button
              type="button"
              className="link-button play-leave"
              data-testid={playTestid.leaveQueue}
              disabled={busy || found}
              onClick={onLeaveQueue}
            >
              {t("Leave the queue")}
            </button>
          </section>
        ) : null}
        {games === 0 ? null : (
          <p className="lobby-note play-recent" data-testid={playTestid.recentGames}>
            {recentGamesWords(games)}
          </p>
        )}
        <PlayHub
          cards={playCards({ signedIn: true, dailyWaiting })}
          lastMode={lastMode}
          ranks={ranks}
          expectedWait={expectedWait}
          dailyWaiting={dailyWaiting}
          locked={locked}
          onPick={onPick}
          onLobby={() => {
            setError(null);
            setPanel("lobby");
          }}
        />
      </div>

      {panel === "lobby" ? (
        <Dialog
          role="dialog"
          sheet
          title={t("Private lobby")}
          testid={playTestid.lobbyDialog}
          className="play-lobby-dialog"
          onClose={closePanel}
          actions={[]}
        >
          <LobbyPanel format="duel" mode={mode} />
          <LobbyPanel format="coop" />
        </Dialog>
      ) : null}
      {panel !== null && panel !== "lobby" ? (
        <QueuePanel
          mode={panel}
          ladder={ladderOfMode(panel)}
          rank={ranks[ladderOfMode(panel)]}
          picks={picks}
          lean={lean}
          verdict={verdict}
          locked={locked}
          busy={busy}
          noChoice={noChoice}
          notice={refusal}
          onDeck={setDeckId}
          onTrio={setTrioId}
          onLean={(pick) => {
            setLean(pick);
            writePlayLean(pick);
          }}
          onFind={onEnqueue}
          onClose={closePanel}
        />
      ) : null}
    </div>
  );
}
