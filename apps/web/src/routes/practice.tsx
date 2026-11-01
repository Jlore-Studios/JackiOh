// `/practice`: a game against the AI, with no account and no server (SPEC §9.9, R187). The engine
// and the AI run in a Web Worker (`practice/host.ts`); this route holds only what the worker answers
// (CLAUDE.md rule 7). The tutorial lives here too (SPEC §9.10): a lesson is a practice game whose
// config names it (`tutorial/start.ts`).
//
// The route is NOT gated. It asks for the account only to offer an active player's saved decks, to
// keep its tutorial progress on the account (R320, R321) and to earn with a free game; an anonymous
// visitor makes no request at all.
//
// URL params, all optional and dropped when invalid: `?seed=`, `?seat=`, `?difficulty=` and `?deck=`
// (a `random` or `preset:` deck with a difficulty starts at once), `?lesson=` (on its own seed and
// seat, which `?seed=` and `?seat=` never override), `?try=<custom id>` (#921, R915005), and, outside
// a production build only, `?pace=fast` and `?aipool=fixture` (#1113, e2e).
//
// With no param that starts a game, a free game left in progress picks up where it was (R668); R765:
// one left with Save and leave waits on the setup's banner instead. Custom cards (SPEC §12): a game
// with any carries them (R915006), runs on the Luau module (R915009) and records no statistics (§12.10).
// Earning (SPEC §14.8, R1303000): an active account's free game from the setup or Play again plays
// on a server seed, with the last board the server kept (R1303003). Lessons, `?try=`, `?seed=` and
// the other URL games never ask.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactElement,
  type ReactNode,
} from "react";

import { FIXTURE_BANNED_IDS } from "@jackioh/ai";
import { CUSTOM_ID_PREFIX, DIFFICULTIES, type Difficulty } from "@jackioh/engine/config";
import { ECONOMY_LIVE } from "@jackioh/server-config";
import { LEAN_NEWEST, isLeanPick } from "@jackioh/shared";
import type { ActionBody, CardDefs, LeanPick, PlayerId, PlayerView, SandboxOp } from "@jackioh/shared";

import { deckCosmeticsOf } from "../cards/cosmetics.ts";
import { FullArtAllowed } from "../cards/fullArt.ts";
import { customDefsOf, gameCustomCards, useCustomCards } from "../creator/useCustomCards.ts";
import { announceWallet } from "../economy/wallet.ts";
import { parseGameEconomy } from "../economy/xp.ts";
import Game from "../game/Game.tsx";
import { reducedMotionNow } from "../game/animations.ts";
import { getFxSettings } from "../fx/settings.ts";
import { t } from "../i18n/i18n.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { ApiRequestError, ApiUnreachableError, getDecks, getMe, type DecksResponse, type MeResponse } from "../net/api.ts";
import { getCustomCards, type CustomCardsResponse } from "../net/customCards.ts";
import { PRACTICE_EARNING_API, type PracticeEarningApi } from "../net/practice.ts";
import { readFailureOf, useAccount, type Account } from "../net/gate.ts";
import { useLiveGame } from "../net/liveGame.ts";
import { navigate, paths } from "../net/navigate.ts";
import { GameBanner, LiveGameBanner, gameBannerTestid } from "./GameBanner.tsx";
import { BackLink } from "./nav.tsx";
import {
  PRACTICE_DEFAULT_DIFFICULTY,
  PRACTICE_PACING,
  PRACTICE_PACING_FAST,
  PRACTICE_PACING_REDUCED,
  PRACTICE_SEED_BYTES,
  PRACTICE_SEED_MAX_LENGTH,
  PRACTICE_SETUP_KEY,
  PRACTICE_SHOWCASE_HOLD_MAX_MS,
  PRACTICE_VOICE_HOLD_MAX_MS,
  type PracticePacing,
} from "../practice/config.ts";
import {
  PRACTICE_IDLE_STATE,
  createPracticeController,
  type PracticeController,
  type PracticeControllerState,
} from "../practice/controller.ts";
import { deckChoiceFromValue, deckChoiceValue, isAutostartDeckValue, isDeckValue } from "../practice/decks.ts";
import { earningText, seedRequestOf, type PracticeEarning } from "../practice/earning.ts";
import { createPracticeHost, type PracticeHost } from "../practice/host.ts";
import { usePracticeEmotes } from "../practice/emotes.ts";
import type { PracticeDebug, PracticeDeckChoice, PracticeStartConfig } from "../practice/protocol.ts";
import { clearPracticeResume, readPracticeResumeState, writePracticeResume } from "../practice/resume.ts";
import { ModifierList } from "../game/ModifierList.tsx";
import { SANDBOX_HOLD, sandboxAction } from "../practice/sandbox.ts";
import { SandboxPanel } from "../practice/SandboxPanel.tsx";
import { PracticeLeave, type PracticeLeaveTo } from "../practice/PracticeLeave.tsx";
import { OUTCOME_TITLE, PracticeResult, outcomeOf } from "../practice/PracticeResult.tsx";
import ReplayPlayer from "../replay/ReplayPlayer.tsx";
import { PracticeSetup, type PracticeSetupChoice, type SavedDecks } from "../practice/PracticeSetup.tsx";
import { practiceTestid } from "../practice/testids.ts";
import { ThinkIndicator } from "../practice/ThinkIndicator.tsx";
import { DIFFICULTY_LABEL, TierCrest } from "../practice/Tier.tsx";
import "../auth/tavern.css";
import "../practice/practice.css";
import { useSettingsAccountSync } from "../settings/accountSync.ts";
import { useSetting } from "../settings/store.ts";
import { useTutorialAccountSync, type TutorialAccountApi } from "../tutorial/accountSync.ts";
import { Coach } from "../tutorial/Coach.tsx";
import { missedOf, type LessonScript } from "../tutorial/coach.ts";
import { useTutorialDevHandle } from "../tutorial/devHandle.ts";
import { lessonById, nextLessonOf, type TutorialLesson } from "../tutorial/lessons.ts";
import { fieldGuideOf, lessonStatus, markFound, markLessonComplete, readTutorialProgress } from "../tutorial/progress.ts";
import { scriptFor } from "../tutorial/scripts/index.ts";
import { lessonStartConfig } from "../tutorial/start.ts";
import { createCoachTracker, silentScript, type CoachTracker } from "../tutorial/tracker.ts";
import { TutorialHud } from "../tutorial/TutorialHud.tsx";
import { TutorialPath } from "../tutorial/TutorialPath.tsx";
import { TutorialResult } from "../tutorial/TutorialResult.tsx";

/** The dev handle exists only outside a production build, like `window.__jackioh`. */
const DEV_ONLY = import.meta.env.MODE !== "production";

// the dev handle (spec 13's replay check)

type PracticeDevHandle = {
  snapshot(): Promise<PracticeDebug>;
  readonly aiSeat: PlayerId | null;
  readonly thinking: boolean;
  readonly view: PlayerView | null;
};

declare global {
  interface Window {
    /** Set only when MODE !== "production". */
    __jackiohPractice?: PracticeDevHandle;
  }
}

// URL params

type PracticeParams = {
  seed?: string;
  difficulty?: Difficulty;
  deck?: string;
  seat?: PlayerId;
  /** A tutorial lesson's id (`tutorial/lessons.ts`). */
  lesson?: string;
  /** #921: a custom card's id, to try on a random deck around it. */
  try?: string;
  pace?: "fast";
  /** Issue #1113, e2e only: the seeded fixtures' pool. Dropped in a production build. */
  aipool?: "fixture";
};

function isDifficulty(value: unknown): value is Difficulty {
  return typeof value === "string" && (DIFFICULTIES as readonly string[]).includes(value);
}

/** Invalid values dropped. */
export function readPracticeParams(search: string): PracticeParams {
  const params = new URLSearchParams(search);
  const out: PracticeParams = {};

  const seed = params.get("seed");
  if (seed !== null && seed.length > 0 && seed.length <= PRACTICE_SEED_MAX_LENGTH) out.seed = seed;

  const difficulty = params.get("difficulty");
  if (isDifficulty(difficulty)) out.difficulty = difficulty;

  const deck = params.get("deck");
  if (deck !== null && isDeckValue(deck)) out.deck = deck;

  const seat = params.get("seat");
  if (seat === "p1" || seat === "p2") out.seat = seat;

  const lesson = params.get("lesson");
  if (lesson !== null && lessonById(lesson) !== undefined) out.lesson = lesson;

  const tried = params.get("try");
  if (tried !== null && tried.startsWith(CUSTOM_ID_PREFIX)) out.try = tried;

  if (params.get("pace") === "fast") out.pace = "fast";

  // Read at call time, not DEV_ONLY, so a test can stub MODE to production and watch it drop.
  if (import.meta.env.MODE !== "production" && params.get("aipool") === "fixture") out.aipool = "fixture";

  return out;
}

// seeds, seats, pacing and the remembered setup

function randomBytes(count: number): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(count));
}

function randomPracticeSeed(): string {
  return [...randomBytes(PRACTICE_SEED_BYTES)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function randomSeat(): PlayerId {
  const [byte = 0] = randomBytes(1);
  return (byte & 1) === 0 ? "p1" : "p2";
}

function pacingFor(params: PracticeParams): PracticePacing {
  if (DEV_ONLY && params.pace === "fast") return PRACTICE_PACING_FAST;
  // The media query or either reduce setting: nothing animates, so there is nothing to wait for.
  return reducedMotionNow() ? PRACTICE_PACING_REDUCED : PRACTICE_PACING;
}

/**
 * `leanSet` is the "More cards from" pick (R1373, R924000), kept whichever deck was picked. A setup
 * kept before the picker holds R1373's `leanNewest`, read as "newest" when on.
 */
type StoredSetup = { difficulty?: Difficulty; deck?: string; leanSet?: LeanPick };

function readStoredSetup(): StoredSetup {
  try {
    const raw = window.localStorage.getItem(PRACTICE_SETUP_KEY);
    if (raw === null) return {};
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return {};
    const { difficulty, deck, leanSet, leanNewest } = parsed as {
      difficulty?: unknown;
      deck?: unknown;
      leanSet?: unknown;
      leanNewest?: unknown;
    };
    const lean = isLeanPick(leanSet) ? leanSet : leanNewest === true ? LEAN_NEWEST : undefined;
    return {
      ...(isDifficulty(difficulty) ? { difficulty } : {}),
      ...(typeof deck === "string" && isDeckValue(deck) ? { deck } : {}),
      ...(lean === undefined ? {} : { leanSet: lean }),
    };
  } catch {
    // A private window, blocked site data or malformed JSON: nothing is remembered.
    return {};
  }
}

function writeStoredSetup(difficulty: Difficulty, deck: PracticeDeckChoice, lean: LeanPick | null): void {
  try {
    window.localStorage.setItem(
      PRACTICE_SETUP_KEY,
      // R1373, R924000: the pick only when there is one; a setup without it reads as none.
      JSON.stringify({ difficulty, deck: deckChoiceValue(deck), ...(lean === null ? {} : { leanSet: lean }) }),
    );
  } catch {
    // Remembering the setup is a convenience; a refusal costs nothing.
  }
}

function configFor(choice: PracticeSetupChoice, params: PracticeParams): PracticeStartConfig {
  return {
    seed: params.seed ?? randomPracticeSeed(),
    difficulty: choice.difficulty,
    humanSeat: params.seat ?? randomSeat(),
    deck: choice.deck,
    // `EngineGate` (main.tsx) holds this screen until the module is loaded, so the binding is set.
    ...(params.aipool === "fixture" ? { aiBanned: [...FIXTURE_BANNED_IDS] } : {}),
    // R923000: a sandbox game.
    ...(choice.sandbox === true ? { sandbox: true } : {}),
    // R915006: the custom cards the deck names.
    ...(choice.customCards === undefined || choice.customCards.length === 0 ? {} : { customCards: choice.customCards }),
  };
}

/** `?lesson=`, or `?difficulty=` with a `random` or `preset:` `?deck=`, starts a game with no click. */
function autostartConfig(params: PracticeParams): PracticeStartConfig | null {
  const lesson = params.lesson === undefined ? undefined : lessonById(params.lesson);
  if (lesson !== undefined) return lessonStartConfig(lesson);
  if (params.difficulty === undefined || params.deck === undefined) return null;
  if (!isAutostartDeckValue(params.deck)) return null;
  const deck = deckChoiceFromValue(params.deck, null);
  if (deck === null) return null;
  return configFor({ difficulty: params.difficulty, deck }, params);
}

// hooks

/** An active account's saved decks (R250), or why there are none, so a signed-in player is never told to sign in. */
function useSavedDecks(account: Account, loadDecks: (token: string) => Promise<DecksResponse>): SavedDecks {
  const [loaded, setLoaded] = useState<{ token: string; saved: SavedDecks } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const token = account.kind === "ready" && account.me.profile.status === "active" ? account.token : null;
  const load = useRef(loadDecks);
  load.current = loadDecks;

  useEffect(() => {
    if (token === null) return;
    let canceled = false;
    let pending: Promise<DecksResponse>;
    try {
      pending = load.current(token);
    } catch (cause) {
      pending = Promise.reject(cause instanceof Error ? cause : new Error(String(cause)));
    }
    pending.then(
      (response) => {
        if (canceled) return;
        const decks = response.decks;
        const saved: SavedDecks =
          Array.isArray(decks) && decks.length > 0
            ? {
                kind: "ready",
                // R1297000: a deck's own playmat, backdrop and card back ride with it.
                decks: decks.map((deck) => {
                  const cosmetics = deckCosmeticsOf(deck);
                  return {
                    name: deck.name,
                    cards: [...deck.cards],
                    portrait: deck.portrait ?? null,
                    ...(cosmetics === null ? {} : { cosmetics }),
                  };
                }),
              }
            : { kind: "none" };
        setLoaded({ token, saved });
      },
      (cause: unknown) => {
        // A failed read offers no saved decks, and says so; Try again reads them again (#1691).
        if (canceled) return;
        const { retryAfterMs } = readFailureOf(cause);
        setLoaded({
          token,
          saved: retryAfterMs === undefined ? { kind: "unavailable" } : { kind: "unavailable", retryAfterMs },
        });
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, attempt]);

  const retry = useCallback((): void => {
    setLoaded(null);
    setAttempt((before) => before + 1);
  }, []);

  switch (account.kind) {
    case "loading":
      return { kind: "checking" };
    case "anonymous":
      return { kind: "anonymous" };
    case "error":
      return account.retry === undefined
        ? { kind: "unavailable" }
        : { kind: "unavailable", retry: account.retry, retryAfterMs: account.retryAfterMs };
    case "ready":
      if (token === null) return { kind: "inactive" };
      if (loaded === null || loaded.token !== token) return { kind: "checking" };
      return loaded.saved.kind === "unavailable" ? { ...loaded.saved, retry } : loaded.saved;
  }
}

/**
 * The card data the setup previews decks with, asked of a host of its own (no game exists yet) and
 * closed once it answers; a started game brings the same catalog, so a later setup needs no second ask.
 */
function useLobbyCatalog(
  wanted: boolean,
  known: CardDefs | null,
  factory: () => PracticeHost,
): { defs: CardDefs | null; failed: boolean; retry: () => void } {
  const [fetched, setFetched] = useState<CardDefs | null>(null);
  const [failed, setFailed] = useState(false);
  const make = useRef(factory);
  make.current = factory;
  const have = known ?? fetched;

  useEffect(() => {
    if (!wanted || have !== null || failed) return;
    let opened: PracticeHost;
    let canceled = false;
    try {
      opened = make.current();
    } catch {
      setFailed(true);
      return;
    }
    opened.request({ type: "catalog" }).then(
      (response) => {
        opened.dispose();
        if (canceled) return;
        if (response.type === "catalog") setFetched(response.defs);
        else setFailed(true);
      },
      () => {
        opened.dispose();
        if (!canceled) setFailed(true);
      },
    );
    return () => {
      canceled = true;
      opened.dispose();
    };
  }, [wanted, have, failed]);

  /** #1691: Try again, which asks a host for the catalog once more. */
  const retry = useCallback((): void => {
    setFailed(false);
  }, []);

  return { defs: have, failed: have === null && failed, retry };
}

/**
 * Tell the controller whether the board is still animating, so the AI's next step waits for the
 * player to see the last one. An element carries `data-animating` for exactly as long as the runner
 * holds a view back (BUILD M5-T4), which is also what `cy.settled()` waits on.
 */
function useBoardBusy(root: HTMLElement | null, controller: PracticeController | null): void {
  useEffect(() => {
    if (controller === null || root === null || typeof MutationObserver !== "function") return;
    const report = (): void => {
      controller.setBoardBusy(root.querySelector("[data-animating]") !== null);
    };
    const observer = new MutationObserver(report);
    observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-animating"] });
    report();
    return () => {
      observer.disconnect();
    };
  }, [root, controller]);
}

/**
 * Whether the board under `root` carries `data-animating`. A voice line (`data-speaking`) is left
 * out on purpose, so the victory line never holds the result dialog back.
 */
export function useBoardAnimating(root: HTMLElement | null): boolean {
  const [animating, setAnimating] = useState(false);
  useEffect(() => {
    if (root === null || typeof MutationObserver !== "function") return;
    const report = (): void => {
      setAnimating(root.querySelector("[data-animating]") !== null);
    };
    const observer = new MutationObserver(report);
    observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-animating"] });
    report();
    return () => {
      observer.disconnect();
    };
  }, [root]);
  return animating;
}

/**
 * Hold the AI's next step while an element anywhere in the page carries `attribute` (SPEC §9.9: AI
 * turns are paced so the player can follow them), for at most `maxMs` per mark, so a stuck mark can
 * slow a turn but never stop the game; it is ignored from then on until the page drops it.
 */
function useMarkHold(controller: PracticeController | null, reason: string, attribute: string, maxMs: number): void {
  useEffect(() => {
    if (controller === null || typeof MutationObserver !== "function") return;
    const root = document.documentElement;
    const selector = `[${attribute}]`;
    let cap: ReturnType<typeof setTimeout> | null = null;
    /** The cap ran out on this mark; it is ignored until the page drops it. */
    let expired = false;
    const release = (): void => {
      if (cap !== null) clearTimeout(cap);
      cap = null;
      controller.setHold(reason, false);
    };
    const report = (): void => {
      const marked = root.matches(selector) || root.querySelector(selector) !== null;
      if (!marked) {
        expired = false;
        release();
        return;
      }
      if (expired || cap !== null) return;
      controller.setHold(reason, true);
      cap = setTimeout(() => {
        expired = true;
        release();
      }, maxMs);
    };
    const observer = new MutationObserver(report);
    observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: [attribute] });
    report();
    return () => {
      observer.disconnect();
      release();
    };
  }, [controller, reason, attribute, maxMs]);
}

/** Hold the AI's next step while a voice line plays (`data-speaking`, polish task 2). */
function useVoiceHold(controller: PracticeController | null): void {
  useMarkHold(controller, "voice", "data-speaking", PRACTICE_VOICE_HOLD_MAX_MS);
}

/** Hold the AI's next step while its last card is held up (`data-showcase`), so it never plays over it. */
function useShowcaseHold(controller: PracticeController | null): void {
  useMarkHold(controller, "showcase", "data-showcase", PRACTICE_SHOWCASE_HOLD_MAX_MS);
}

const noSubscribe = (): (() => void) => () => {};
const idleState = (): PracticeControllerState => PRACTICE_IDLE_STATE;

function useControllerState(controller: PracticeController | null): PracticeControllerState {
  return useSyncExternalStore(
    controller === null ? noSubscribe : controller.subscribe,
    controller === null ? idleState : controller.getState,
    controller === null ? idleState : controller.getState,
  );
}

// the route

export type PracticeRouteProps = {
  /** default createPracticeHost */
  hostFactory?: () => PracticeHost;
  /** default by ?pace / reduced motion */
  pacing?: PracticePacing;
  /** default useAccount() */
  account?: Account;
  /** default getDecks */
  loadDecks?: (token: string) => Promise<DecksResponse>;
  /** default `GET`/`PUT /api/tutorial` (R320, R321) */
  tutorialAccount?: TutorialAccountApi;
  /** default the lessons' own coach scripts (`tutorial/scripts`) */
  coachScript?: (lessonId: string) => LessonScript | undefined;
  /** default `GET /api/auth/me`: R765's re-read of the player's live online game, for its banner */
  loadMe?: (token: string) => Promise<MeResponse>;
  /** default `GET /api/custom-cards` (§12.9): the custom cards a saved deck or `?try=` names */
  loadCustomCards?: (token: string) => Promise<CustomCardsResponse>;
  /** R1303000: the seed and claim calls; default `PRACTICE_EARNING_API` while `ECONOMY_LIVE`, else none (null) */
  practiceEarning?: PracticeEarningApi | null;
};

type ScreenProps = Omit<PracticeRouteProps, "account"> & { account: Account };

function defaultHostFactory(): PracticeHost {
  return createPracticeHost();
}

/** The lobby screens are a centered column in the tavern, like `/play`; the game is the full-width board. */
function Shell({ variant, children }: { variant: "lobby" | "game"; children: ReactNode }): ReactElement {
  const shell = variant === "game" ? "app-shell app-shell--wide" : "app-shell tavern one-screen";
  return <div className={`${shell} practice practice--${variant}`}>{children}</div>;
}

/** The deal on its way: a game starting in the worker, or (R1303000) its seed being asked for. */
function LoadingScreen({ onBack }: { onBack: () => void }): ReactElement {
  return (
    <Shell variant="lobby">
      {/* A worker that never answers must not trap the page: back to the setup, which has its own
          way out (integration: every screen has a way back). */}
      <BackLink onPress={onBack} />
      <div className="practice-stage">
        <div className="practice-shuffle" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <p className="practice-stage__status" data-testid={practiceTestid.loading} role="status">
          {t("Shuffling the decks…")}
        </p>
      </div>
    </Shell>
  );
}

function PracticeScreen({
  account,
  hostFactory,
  pacing,
  loadDecks,
  tutorialAccount,
  coachScript,
  loadMe,
  loadCustomCards,
  practiceEarning,
}: ScreenProps): ReactElement {
  const params = useMemo(() => readPracticeParams(window.location.search), []);
  const saved = useSavedDecks(account, loadDecks ?? getDecks);
  // §12.9: an active account's custom cards, which its saved decks and `?try=` name.
  const customToken = account.kind === "ready" && account.me.profile.status === "active" ? account.token : null;
  const custom = useCustomCards(customToken, loadCustomCards ?? getCustomCards);
  // R1303000: the seed and claim calls, read when they are made, so a prop change mid-game changes nothing.
  const earningApi = useRef<PracticeEarningApi | null>(null);
  earningApi.current = practiceEarning === undefined ? (ECONOMY_LIVE ? PRACTICE_EARNING_API : null) : practiceEarning;
  const earningToken = useRef(customToken);
  earningToken.current = customToken;
  // R321: an active account's tutorial progress is merged with this device's and kept level with it.
  useTutorialAccountSync(account, tutorialAccount);
  // R634: and so are its game settings.
  useSettingsAccountSync(account);

  const initial = useMemo(() => {
    const stored = readStoredSetup();
    return {
      difficulty: params.difficulty ?? stored.difficulty ?? PRACTICE_DEFAULT_DIFFICULTY,
      deck: params.deck ?? stored.deck ?? "random",
      leanSet: stored.leanSet ?? null,
    };
  }, [params]);

  /** R668: the free game this device left in progress, read once; a URL that starts a game wins. */
  const [arrival] = useState(() => {
    const autostart = autostartConfig(params);
    // #921: `?try=` starts a game too, once the card is read, so it picks up no game left before.
    const kept = autostart === null && params.try === undefined ? readPracticeResumeState() : null;
    return { autostart, kept };
  });
  /** The game being picked up again (R668, R765); compared by identity with `game`. */
  const resumeFrom = useRef<PracticeStartConfig | null>(
    arrival.kept !== null && !arrival.kept.saved ? arrival.kept.config : null,
  );
  /** The game to play, fixed (seed and seat included) when it is chosen; null shows the setup. */
  const [game, setGame] = useState<PracticeStartConfig | null>(() => arrival.autostart ?? resumeFrom.current);
  /** R765: the game kept with Save and leave, which the setup's banner offers back; null when none. */
  const [savedGame, setSavedGame] = useState<PracticeStartConfig | null>(() =>
    arrival.kept !== null && arrival.kept.saved ? arrival.kept.config : null,
  );
  /** R765: the banner's Resume found nothing to fold, and the setup says so. */
  const [resumeLost, setResumeLost] = useState(false);
  /** #921: "pending" until the account's custom cards are read, then null (its game starts) or "missing". */
  const [tryState, setTryState] = useState<"pending" | "missing" | null>(() =>
    params.try !== undefined && arrival.autostart === null ? "pending" : null,
  );
  /** The live online game, for the banner beside the practice one (R765). */
  const liveGame = useLiveGame(account, loadMe ?? getMe);
  const [controller, setController] = useState<PracticeController | null>(null);
  /** A lesson's coach, fed by the controller from its first snapshot on; null for a practice game. */
  const [tracker, setTracker] = useState<CoachTracker | null>(null);

  // Read when a game starts, so a prop change mid-game does not tear the game down.
  const factory = useRef(hostFactory ?? defaultHostFactory);
  factory.current = hostFactory ?? defaultHostFactory;
  const scripts = useRef(coachScript ?? scriptFor);
  scripts.current = coachScript ?? scriptFor;

  /** The last catalog a game brought, so the next setup screen previews decks without asking. */
  const [gameDefs, setGameDefs] = useState<CardDefs | null>(null);
  // #921: a `?try=` still finding its card shows no setup, so it asks for no catalog yet.
  const lobbyCatalog = useLobbyCatalog(game === null && tryState !== "pending", gameDefs, hostFactory ?? defaultHostFactory);
  const pace = useRef(pacing ?? pacingFor(params));
  pace.current = pacing ?? pacingFor(params);

  useEffect(() => {
    if (game === null) return;
    const next = createPracticeController({ host: factory.current(), pacing: pace.current });
    // Subscribed before `start` is sent, so the coach reads every snapshot (tutorial/tracker.ts).
    const lessonId = game.lesson;
    const coach =
      lessonId === undefined
        ? null
        : createCoachTracker(next, scripts.current(lessonId) ?? silentScript(lessonId), {
            known: fieldGuideOf(readTutorialProgress()),
            onFound: markFound,
          });
    setController(next);
    setTracker(coach);
    if (resumeFrom.current === game) void next.resume(game);
    else void next.start(game);
    return () => {
      coach?.dispose();
      next.dispose();
      setController((current) => (current === next ? null : current));
      setTracker((current) => (current === coach ? null : current));
    };
  }, [game]);

  const state = useControllerState(controller);
  // R668: a save the worker will not fold is no game to show a failure for: the player lands on the
  // setup. A failure once a resumed board is shown is reported like any other.
  const resumeFailed = state.phase === "failed" && state.snapshot === null && game !== null && resumeFrom.current === game;
  /** R765: whether the resume in flight was asked for by the banner's Resume, which then says it failed. */
  const resumeAsked = useRef(false);
  useEffect(() => {
    if (!resumeFailed) return;
    resumeFrom.current = null;
    setResumeLost(resumeAsked.current);
    resumeAsked.current = false;
    setGame(null);
  }, [resumeFailed]);
  const startedDefs = state.defs;
  useEffect(() => {
    if (startedDefs !== null) setGameDefs(startedDefs);
  }, [startedDefs]);
  // A callback ref held in state, so the watch starts when the board first renders, not before.
  const [boardRoot, setBoardRoot] = useState<HTMLDivElement | null>(null);
  useBoardBusy(boardRoot, controller);
  // `?pace=fast` is e2e pacing: holding the AI for every voice line would only slow the spec.
  const held = DEV_ONLY && params.pace === "fast" ? null : controller;
  useVoiceHold(held);
  useShowcaseHold(held);

  // The dev handle for spec 13's replay check; never in a production build.
  useEffect(() => {
    if (!DEV_ONLY || controller === null) return;
    const handle: PracticeDevHandle = {
      snapshot: () => controller.debug(),
      get aiSeat() {
        return controller.getState().aiSeat;
      },
      get thinking() {
        return controller.getState().thinking;
      },
      get view() {
        return controller.getState().snapshot?.view ?? null;
      },
    };
    window.__jackiohPractice = handle;
    return () => {
      if (window.__jackiohPractice === handle) delete window.__jackiohPractice;
    };
  }, [controller]);
  // The tutorial's handle for the e2e lesson spec, under the same rule.
  useTutorialDevHandle(tracker, game?.lesson ?? null);

  /** R2379000: the seed on its way back to the server, which the next seed request waits for. */
  const releasing = useRef<Promise<unknown>>(Promise.resolve());
  /** An earning game given up unclaimed gives its seed back; one that fails lapses after `PRACTICE_SEED_TTL_SECONDS`. */
  const releaseSeedOf = useCallback((left: PracticeStartConfig | null) => {
    const api = earningApi.current;
    const token = earningToken.current;
    if (left === null || left.earning !== true || api === null || token === null) return;
    releasing.current = api.release(token, left.seed).catch(() => undefined);
  }, []);

  /** R765: starting any game from here ends the one kept with Save and leave. */
  const dropSavedGame = useCallback(() => {
    releaseSeedOf(savedGame);
    clearPracticeResume();
    setSavedGame(null);
    setResumeLost(false);
  }, [savedGame, releaseSeedOf]);

  /** R1303000: the setup awaiting its server seed. Asks are numbered, so one given up or redone never starts. */
  const [seeding, setSeeding] = useState<PracticeStartConfig | null>(null);
  const seedAsks = useRef(0);
  /**
   * On a server seed when the account may earn (`seedRequestOf`) and the URL fixes no seed. A seed
   * the server will not issue, or that never comes, leaves the game to play as before, earning nothing.
   */
  const launch = useCallback(
    (config: PracticeStartConfig) => {
      seedAsks.current += 1;
      const api = earningApi.current;
      const token = earningToken.current;
      const request = params.seed === undefined ? seedRequestOf(config) : null;
      if (api === null || token === null || request === null) {
        setSeeding(null);
        setGame(config);
        return;
      }
      const ask = seedAsks.current;
      setSeeding(config);
      // R2379000: after any seed on its way back, so the place it frees counts for this one.
      releasing.current
        .then(() => api.requestSeed(token, request))
        .then(
          (issued) => {
            if (seedAsks.current !== ask) return;
            setSeeding(null);
            setGame({
              ...config,
              seed: issued.seed,
              earning: true,
              // R1303003: the board the server kept from the last paid game, which the claim folds with.
              ...(issued.lastBoard.length === 0 ? {} : { lastBoard: issued.lastBoard }),
            });
          },
          (error: unknown) => {
            if (seedAsks.current !== ask) return;
            setSeeding(null);
            setGame(config);
            // R2379000: the game says from its start that it earns nothing, in the server's words.
            const reason = error instanceof ApiRequestError || error instanceof ApiUnreachableError ? error.message : null;
            setEarning({ game: config, earning: { kind: "unearned", reason } });
          },
        );
    },
    [params],
  );
  /** Back while a seed is asked for gives the ask up. */
  const onBackFromSeeding = useCallback(() => {
    seedAsks.current += 1;
    setSeeding(null);
    setGame(null);
  }, []);

  const onStart = useCallback(
    (choice: PracticeSetupChoice) => {
      writeStoredSetup(choice.difficulty, choice.deck, choice.leanSet ?? null);
      dropSavedGame();
      setTryState(null);
      launch(configFor(choice, params));
    },
    [params, dropSavedGame, launch],
  );

  /** A lesson always plays on its own seed and seat, from the path, the URL or its result dialog. */
  const onStartLesson = useCallback(
    (lesson: TutorialLesson) => {
      dropSavedGame();
      setTryState(null);
      setGame(lessonStartConfig(lesson));
    },
    [dropSavedGame],
  );

  // #921: a random deck around the card (R915005), on the remembered difficulty.
  const tryId = params.try;
  const customState = custom.state;
  useEffect(() => {
    if (tryState !== "pending" || tryId === undefined) return;
    if (account.kind === "loading" || customState.kind === "loading") return;
    const card = customState.kind === "ready" ? customState.cards.find((candidate) => candidate.id === tryId) : undefined;
    if (card === undefined) {
      setTryState("missing");
      return;
    }
    setTryState(null);
    setGame(
      configFor(
        { difficulty: initial.difficulty, deck: { kind: "try", card: card.id }, customCards: gameCustomCards([card]) },
        params,
      ),
    );
  }, [tryState, tryId, account.kind, customState, initial.difficulty, params]);

  /** R765: the banner's Resume: the saved game picked up as a reload would pick it up (R668). */
  const onResume = useCallback(() => {
    if (savedGame === null) return;
    resumeFrom.current = savedGame;
    resumeAsked.current = true;
    setSavedGame(null);
    setResumeLost(false);
    setGame(savedGame);
  }, [savedGame]);

  /** "New game" or "Menu" mid-game asks first; the question belongs to its game, so no other inherits it. */
  const [leaveAskedFor, setLeaveAskedFor] = useState<{ game: PracticeStartConfig; to: PracticeLeaveTo } | null>(null);
  const onAskNewGame = useCallback(() => {
    if (game !== null) setLeaveAskedFor({ game, to: "setup" });
  }, [game]);
  const onAskMenu = useCallback(() => {
    if (game !== null) setLeaveAskedFor({ game, to: "menu" });
  }, [game]);
  const onStay = useCallback(() => {
    setLeaveAskedFor(null);
  }, []);

  // Leaving a game without saving it gives it up (R668, R765): no reload and no banner brings it back.
  const onNewGame = useCallback(() => {
    setLeaveAskedFor(null);
    clearPracticeResume();
    setGame(null);
  }, []);

  const onMenu = useCallback(() => {
    setLeaveAskedFor(null);
    clearPracticeResume();
    navigate(paths.landing);
  }, []);

  /**
   * R765: Save and leave keeps the game (the worker's save is already its last answer) and marks it
   * left on purpose, so the next `/practice` shows the setup with its banner instead of picking it up.
   */
  const onSaveAndLeave = useCallback(() => {
    if (leaveAskedFor === null) return;
    const { game: left, to } = leaveAskedFor;
    setLeaveAskedFor(null);
    writePracticeResume(left, { saved: true });
    if (to === "menu") {
      navigate(paths.landing);
      return;
    }
    setSavedGame(left);
    setGame(null);
  }, [leaveAskedFor]);

  /** R2379000: Leave without saving gives the game up, and with it the seed it would have claimed on. */
  const onLeaveWithoutSaving = useCallback(() => {
    if (leaveAskedFor === null) return;
    releaseSeedOf(leaveAskedFor.game);
    if (leaveAskedFor.to === "menu") onMenu();
    else onNewGame();
  }, [leaveAskedFor, releaseSeedOf, onMenu, onNewGame]);

  /** Back from loading gives a start up; a resume is not the player's to lose (R765), so it is kept as saved. */
  const onBackFromLoading = useCallback(() => {
    if (game === null || resumeFrom.current !== game) {
      onNewGame();
      return;
    }
    resumeFrom.current = null;
    resumeAsked.current = false;
    writePracticeResume(game, { saved: true });
    setSavedGame(game);
    setGame(null);
  }, [game, onNewGame]);

  /** "Exit tutorial" in the middle of a lesson asks first, as "New game" does. */
  const onAskExitLesson = useCallback(() => {
    if (game !== null) setLeaveAskedFor({ game, to: "lessons" });
  }, [game]);

  /** "Play a practice game" after the last lesson: the lobby, scrolled to the practice setup. */
  const [scrollToSetup, setScrollToSetup] = useState(false);
  const onPlayPractice = useCallback(() => {
    setLeaveAskedFor(null);
    clearPracticeResume();
    setScrollToSetup(true);
    setGame(null);
  }, []);
  const practiceHeader = useRef<HTMLElement>(null);
  useEffect(() => {
    if (!scrollToSetup || game !== null) return;
    practiceHeader.current?.scrollIntoView?.({ block: "start" });
    setScrollToSetup(false);
  }, [scrollToSetup, game]);

  // A free game comes back after a reload (R668), but a lesson does not (its coach reads the game
  // from the deal on), so leaving the page mid-lesson makes the browser ask first.
  const inProgress = game?.lesson !== undefined && state.phase === "playing";
  useEffect(() => {
    if (!inProgress) return;
    const onBeforeUnload = (event: BeforeUnloadEvent): void => {
      event.preventDefault();
      // Older engines show the prompt only when returnValue is set.
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
    };
  }, [inProgress]);

  /** The same difficulty and deck again, with a fresh seed and seat (unless the URL fixes them). */
  const onPlayAgain = useCallback(() => {
    if (game === null) return;
    launch(
      configFor(
        {
          difficulty: game.difficulty,
          deck: game.deck,
          ...(game.sandbox === true ? { sandbox: true } : {}),
          // R915006: a game with custom cards deals them again.
          ...(game.customCards === undefined ? {} : { customCards: game.customCards }),
        },
        params,
      ),
    );
  }, [game, params, launch]);

  /** The game whose result dialog the player closed to read the final board; null shows it. */
  const [resultClosedFor, setResultClosedFor] = useState<PracticeStartConfig | null>(null);
  /**
   * The game whose result dialog may open: it waits out the board's own game-over sequence rather
   * than cover it, and with no effects to watch (reduced motion, effects off, e2e pacing) opens at once.
   */
  const [resultReadyFor, setResultReadyFor] = useState<PracticeStartConfig | null>(null);
  // This game's own end: a Play again renders once with the old controller's finished state.
  const finished = state.config === game && state.snapshot?.view.result != null;
  const boardAnimating = useBoardAnimating(boardRoot);
  const lethalHintOn = useSetting("lethalHint");
  useEffect(() => {
    // The dialog opens once the board has settled, then `delay` more for Victory or Defeat.
    if (!finished || game === null || boardAnimating) return;
    const effectsOff = reducedMotionNow() || getFxSettings().intensity === "off";
    const delay = effectsOff ? 0 : (pace.current.resultDelayMs ?? 0);
    // Never open on this render's word alone: the board's queue starts in Game's layout effect, after
    // this one, so the timer checks the board itself (a mark found re-runs this via `boardAnimating`).
    const timer = setTimeout(() => {
      if (boardRoot?.querySelector("[data-animating]") != null) return;
      setResultReadyFor(game);
    }, delay);
    return () => {
      clearTimeout(timer);
    };
  }, [finished, game, boardAnimating, boardRoot]);
  /** R1303000: what a game's claim earned, held by its game, so a Play again shows nothing of the last's. */
  const [earning, setEarning] = useState<{ game: PracticeStartConfig; earning: PracticeEarning } | null>(null);
  /** The game whose end was claimed, so each game is claimed once. */
  const claimedFor = useRef<PracticeStartConfig | null>(null);
  useEffect(() => {
    if (!finished || game === null || game.earning !== true || controller === null || claimedFor.current === game) return;
    claimedFor.current = game;
    const claimed = game;
    const settleTo = (next: PracticeEarning): void => {
      setEarning((current) => (current?.game === claimed ? { game: claimed, earning: next } : current));
    };
    setEarning({ game: claimed, earning: { kind: "pending" } });
    const api = earningApi.current;
    const token = earningToken.current;
    if (api === null || token === null) {
      settleTo({ kind: "unreachable" });
      return;
    }
    // Any failure is "unreachable": the dialog stays whole either way.
    controller
      .claim()
      .then((claim) => api.claim(token, claim))
      .then(
        (answer) => {
          if (!answer.paid) {
            settleTo({ kind: "refused", reason: answer.reason });
            return;
          }
          // #1664: the game's Jlore is in the wallet now, and its tracks and quests are for the dialog to play.
          announceWallet();
          const economy = parseGameEconomy(answer);
          settleTo({
            kind: "paid",
            jlore: answer.jlore,
            xp: answer.xp,
            jloreCapped: answer.jloreCapped,
            ...(economy === null ? {} : { tracks: economy.tracks, quests: economy.quests }),
          });
        },
        () => {
          settleTo({ kind: "unreachable" });
        },
      );
  }, [finished, game, controller]);
  const onViewBoard = useCallback(() => {
    setResultClosedFor(game);
  }, [game]);
  const onShowResult = useCallback(() => {
    setResultClosedFor(null);
    setResultReadyFor(game);
  }, [game]);

  /** R768000: the replay watched from the result dialog (`share` opens it at the last turn's GIF). */
  const [watching, setWatching] = useState<{ game: PracticeStartConfig; share: boolean } | null>(null);
  const kept = state.config === game ? state.snapshot?.replay : undefined;
  const keptGame = kept?.game;
  const keptSteps = kept?.steps;
  const replaySource = useMemo(
    () =>
      controller === null || keptGame === undefined || keptSteps === undefined
        ? null
        : controller.replaySource({ game: keptGame, steps: keptSteps }),
    [controller, keptGame, keptSteps],
  );
  const onWatchReplay = useCallback(() => {
    if (game !== null) setWatching({ game, share: false });
  }, [game]);
  const onShareLastTurn = useCallback(() => {
    if (game !== null) setWatching({ game, share: true });
  }, [game]);
  const onLeaveReplay = useCallback(() => {
    setWatching(null);
  }, []);

  const onAction = useCallback(
    (body: ActionBody) => {
      controller?.act(body);
    },
    [controller],
  );

  // R923000: a sandbox game's "Hold the AI's turn", released with the game.
  const [sandboxHeld, setSandboxHeld] = useState(false);
  const onSandboxHold = useCallback(
    (held: boolean) => {
      setSandboxHeld(held);
      controller?.setHold(SANDBOX_HOLD, held);
    },
    [controller],
  );
  useEffect(() => {
    setSandboxHeld(false);
    controller?.setHold(SANDBOX_HOLD, false);
  }, [controller, state.config]);
  const onSandboxTool = useCallback(
    (op: SandboxOp) => {
      controller?.act(sandboxAction(op));
    },
    [controller],
  );

  // R642–R645: the dealt portraits, the AI's persona and its replies; with no game it deals nothing.
  const emotes = usePracticeEmotes(state.config, state.snapshot, state.aiSeat, controller);

  const defs = state.defs;
  const lookup = useMemo(() => (defs === null ? null : lookupFromDefs(defs)), [defs]);
  // R962000: the game's own custom cards, which a sandbox game's panel offers beside the catalog.
  const gameCustoms = state.config?.customCards;
  const sandboxCustoms = useMemo(() => customDefsOf(gameCustoms ?? []), [gameCustoms]);

  // A lesson won is completed on this device (R294) the moment the view says so, whether or not its
  // result dialog is ever seen.
  const lesson = game?.lesson === undefined ? undefined : lessonById(game.lesson);
  const lessonResult = lesson !== undefined && state.config === game ? (state.snapshot?.view ?? null) : null;
  const wonLesson =
    lesson !== undefined && lessonResult?.result != null && lessonResult.result.winner === lessonResult.viewer
      ? lesson.id
      : null;
  useEffect(() => {
    if (wonLesson !== null) markLessonComplete(wonLesson);
  }, [wonLesson]);
  /** The path as it stood when this game started, so the result can say what the win opened. */
  const progressAtStart = useMemo(() => (game === null ? null : readTutorialProgress()), [game]);

  if (seeding !== null) return <LoadingScreen onBack={onBackFromSeeding} />;

  if (game === null && tryState === "pending") {
    return (
      <Shell variant="lobby">
        <BackLink to={paths.play} />
        <div className="practice-stage">
          <p className="practice-stage__status" data-testid={practiceTestid.tryLoading} role="status">
            {t("Finding your custom card…")}
          </p>
        </div>
      </Shell>
    );
  }

  if (game === null) {
    return (
      <Shell variant="lobby">
        <BackLink to={paths.play} />
        <div className="one-screen__body one-screen__stack">
          {/* The page's one H1, first in the outline and read, not drawn: Back and the path say where
              the player is. */}
          <h1 className="tutorial-sr-only">{t("Practice")}</h1>
          {savedGame === null && liveGame === null && !resumeLost ? null : (
            <div className="practice-banners">
              {savedGame === null ? null : (
                <GameBanner
                  testId={gameBannerTestid.practice}
                  title="Game in progress"
                  action={{ label: "Resume", testId: gameBannerTestid.practiceResume, onPress: onResume }}
                  data={{ difficulty: savedGame.difficulty }}
                >
                  {t("Your {difficulty} game against the AI is saved where you left it. Starting a new game ends it.", {
                    difficulty: t(DIFFICULTY_LABEL[savedGame.difficulty]),
                  })}
                </GameBanner>
              )}
              {resumeLost ? (
                <p className="notice" data-testid={practiceTestid.resumeLost} role="status">
                  {t("Your saved game could not be picked up again, so it has been dropped.")}
                </p>
              ) : null}
              <LiveGameBanner game={liveGame} />
            </div>
          )}
          {tryState === "missing" ? (
            <p className="notice" data-testid={practiceTestid.tryMissing} role="status">
              {t("That custom card could not be found: it may have been deleted, or you may need to sign in.")}
            </p>
          ) : null}
          <TutorialPath onStart={onStartLesson} />
          <header className="practice-lobby__header" ref={practiceHeader}>
            <h2 className="practice-lobby__title">{t("Practice against the AI")}</h2>
          </header>
          <PracticeSetup
            saved={saved}
            initial={initial}
            defs={lobbyCatalog.defs}
            defsFailed={lobbyCatalog.failed}
            custom={custom.state}
            onRetryCustom={custom.reload}
            onRetryDefs={lobbyCatalog.retry}
            onStart={onStart}
          />
        </div>
      </Shell>
    );
  }

  if (state.phase === "failed") {
    return (
      <Shell variant="lobby">
        <div className="practice-stage">
          <h1 className="practice-stage__title">{t("The practice game stopped")}</h1>
          <p className="notice" data-testid={practiceTestid.error} role="alert">
            {t(state.failure ?? "Something went wrong while the game was running.")}
          </p>
          <button
            type="button"
            className="practice-play button-primary"
            data-testid={practiceTestid.newGame}
            onClick={onNewGame}
          >
            {t("New game")}
          </button>
        </div>
      </Shell>
    );
  }

  const snapshot = state.snapshot;
  const config = state.config;
  if (controller === null || snapshot === null || config === null || state.phase === "idle" || state.phase === "starting") {
    return <LoadingScreen onBack={onBackFromLoading} />;
  }

  if (watching !== null && watching.game === game && replaySource !== null) {
    const player = (
      <ReplayPlayer
        key={watching.share ? "share" : "watch"}
        source={replaySource}
        start={watching.share ? "last" : "first"}
        shareOnOpen={watching.share}
        onLeave={onLeaveReplay}
      />
    );
    return (
      <Shell variant="game">
        {lookup === null ? player : <CatalogContext.Provider value={lookup}>{player}</CatalogContext.Provider>}
      </Shell>
    );
  }

  const board = (
    // The result panel is practice's own dialog (PracticeResult), so the board keeps to its chip.
    // R345: a lesson keeps R82's automatic turn end on, since its coach is written around it, and its
    // mulligan picker the engine's instruction, which the lesson teaches.
    // R1108000: and no card on a lesson's board prints full art, whatever the Full-art cards setting.
    <FullArtAllowed allowed={config.lesson === undefined}>
      <Game
        view={snapshot.view}
        legal={snapshot.legal}
        onAction={onAction}
        error={snapshot.error}
        resultForm="chip"
        emotes={emotes}
        // R639: a practice game is logged; a lesson is not, nor (R923000) a sandbox game, nor
        // (R915009, §12.10) a game with custom cards.
        trackStats={config.lesson === undefined && config.sandbox !== true && (config.customCards?.length ?? 0) === 0}
        wearCosmetics
        // R1297000: a saved deck brings its own playmat, backdrop and card back.
        deckCosmetics={config.deck.kind === "saved" ? (config.deck.cosmetics ?? null) : null}
        {...(config.lesson === undefined ? {} : { autoEndTurn: true, lesson: true })}
        // R2291000: in every game against the AI (R2102000), unless the Lethal hint setting is off.
        lethalHint={lethalHintOn && snapshot.lethal === true}
      />
    </FullArtAllowed>
  );
  const result = snapshot.view.result;
  const outcome = result === null ? null : outcomeOf(result, snapshot.view.viewer);
  // The controller's config, so a screen that just asked for the next lesson labels the one it shows.
  const playing = config.lesson === undefined ? undefined : lessonById(config.lesson);
  const coached = playing !== undefined && tracker !== null ? { lesson: playing, tracker } : null;
  const nextLesson = coached === null ? undefined : nextLessonOf(coached.lesson.id);

  return (
    <Shell variant="game">
      {/* The board has no heading of its own; this names the screen for a screen reader's heading
          list, and draws nothing. */}
      <h1 className="tutorial-sr-only">
        {playing === undefined
          ? t("Practice game")
          : t("Lesson {number}: {title}", { number: String(playing.number), title: t(playing.title) })}
      </h1>
      {coached !== null ? (
        <TutorialHud
          lesson={coached.lesson}
          tracker={coached.tracker}
          view={snapshot.view}
          humanSeat={config.humanSeat}
          aiSeat={state.aiSeat}
          thinking={state.thinking}
          outcome={outcome}
          onShowResult={onShowResult}
          onExit={result === null ? onAskExitLesson : onNewGame}
        />
      ) : (
        <header
          className="practice-hud"
          data-testid={practiceTestid.hud}
          data-difficulty={config.difficulty}
          data-human-seat={config.humanSeat}
          data-ai-seat={state.aiSeat ?? ""}
          data-thinking={state.thinking ? "true" : "false"}
        >
          <span className="practice-hud__tier">
            <TierCrest tier={config.difficulty} size="sm" />
            <span className="practice-hud__label">
              <span className="practice-hud__mode">{config.sandbox === true ? t("Sandbox") : t("Practice")}</span>
              <strong>{t(DIFFICULTY_LABEL[config.difficulty])}</strong>
            </span>
          </span>
          {/* SPEC §2.1 step 5: p1 takes the first turn. */}
          <span className="practice-hud__seat">{config.humanSeat === "p1" ? t("You go first") : t("You go second")}</span>
          <span className="practice-hud__status">
            <ModifierList view={snapshot.view} opponentLabel={t("AI")} />
            <ThinkIndicator thinking={state.thinking} />
            {outcome === null ? null : (
              <button
                type="button"
                className="practice-hud__outcome"
                data-testid={practiceTestid.outcome}
                data-outcome={outcome}
                onClick={onShowResult}
              >
                {t(OUTCOME_TITLE[outcome])}
              </button>
            )}
          </span>
          {/* A game in progress is one tap from gone, so leaving it asks first; a finished one does not. */}
          <button
            type="button"
            className="practice-hud__new"
            data-testid={practiceTestid.newGame}
            aria-haspopup={result === null ? "dialog" : undefined}
            onClick={result === null ? onAskNewGame : onNewGame}
          >
            {t("New game")}
          </button>
          <button
            type="button"
            className="practice-hud__menu"
            data-testid={practiceTestid.menu}
            aria-haspopup={result === null ? "dialog" : undefined}
            aria-label={t("Main menu")}
            title={t("Main menu")}
            onClick={result === null ? onAskMenu : onMenu}
          >
            {t("Menu")}
          </button>
        </header>
      )}
      {earning !== null && earning.game === game && earning.earning.kind === "unearned" ? (
        <p className="notice practice-unearned" data-testid={practiceTestid.unearned} role="status">
          {earningText(earning.earning)}
        </p>
      ) : null}
      {/* A lesson's coach comes between the HUD and the board: on a phone it is a panel in the page
          there, and the board takes the height that is left (tutorial/Coach.tsx); elsewhere it
          floats over the board, and this is only where the keyboard and a screen reader meet it. */}
      {coached === null ? null : <Coach tracker={coached.tracker} boardRoot={boardRoot} />}
      {/* `practice-table` holds the board and hands it the screen's height (practice.css, "the game
          screen"); the wrapper is also the root `useBoardBusy` watches. */}
      <div
        className="practice-board practice-table"
        ref={setBoardRoot}
        data-difficulty={config.difficulty}
        data-thinking={state.thinking ? "true" : "false"}
      >
        {lookup === null ? board : <CatalogContext.Provider value={lookup}>{board}</CatalogContext.Provider>}
      </div>
      {/* R923000: a sandbox game's dev tools, over the board's edge; while the game runs only. */}
      {config.sandbox === true && result === null ? (
        <SandboxPanel view={snapshot.view} defs={defs} customs={sandboxCustoms} held={sandboxHeld} onHold={onSandboxHold} onTool={onSandboxTool} />
      ) : null}
      {leaveAskedFor !== null && leaveAskedFor.game === game && result === null ? (
        <PracticeLeave
          to={leaveAskedFor.to}
          onStay={onStay}
          onLeave={onLeaveWithoutSaving}
          // R765: a free game may be kept; a lesson is never kept (R668), so it keeps its own flow.
          {...(leaveAskedFor.to === "lessons" ? {} : { onSaveAndLeave })}
        />
      ) : null}
      {result === null || resultClosedFor === game || resultReadyFor !== game ? null : coached !== null ? (
        <TutorialResult
          result={result}
          viewer={snapshot.view.viewer}
          lesson={coached.lesson}
          next={nextLesson}
          unlockedNow={
            nextLesson !== undefined && progressAtStart !== null && lessonStatus(progressAtStart, nextLesson) === "locked"
          }
          missed={missedOf(coached.tracker.script, coached.tracker.getState().coach)}
          onNext={() => {
            if (nextLesson !== undefined) onStartLesson(nextLesson);
          }}
          onRetry={() => {
            onStartLesson(coached.lesson);
          }}
          onBack={onNewGame}
          onPlayPractice={onPlayPractice}
          onViewBoard={onViewBoard}
        />
      ) : (
        <PracticeResult
          result={result}
          viewer={snapshot.view.viewer}
          difficulty={config.difficulty}
          onPlayAgain={onPlayAgain}
          onChangeSetup={onNewGame}
          onViewBoard={onViewBoard}
          {...(replaySource === null ? {} : { onWatchReplay, onShareLastTurn })}
          {...(earning === null || earning.game !== game ? {} : { earning: earning.earning })}
        />
      )}
    </Shell>
  );
}

function PracticeWithAccount(props: Omit<PracticeRouteProps, "account">): ReactElement {
  const account = useAccount();
  return <PracticeScreen {...props} account={account} />;
}

export default function PracticeRoute(props: PracticeRouteProps): ReactElement {
  // A component per source rather than a conditional hook: an injected account never touches the
  // session or the network.
  if (props.account !== undefined) return <PracticeScreen {...props} account={props.account} />;
  return <PracticeWithAccount {...props} />;
}
