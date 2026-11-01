// A dungeon run's fight (DR5, DR14 part E, #2706): while the run stands on a fight (`RunPhase`
// `fight`), the page plays it as a practice game against the AI (§9.9, R2706000). The run writes the
// game's setup (`runFightSetup`: the run deck, its HP and its Radiant and buffed cards against the
// enemy's deck and strength), the practice worker plays it with the AI as the enemy, whose name the
// board's rail shows beside the run's timer (B22, B24), and each side's relics sit in a tray beside its
// hero (B23). There is no Play again: once the game is over the page asks the worker for its report,
// the run's `FightOver` (B25), and Continue sends it to the run, which a lost fight ends (B26).
//
// The board is the practice route's (`routes/practice.tsx`): the same controller, pacing, emotes and
// `Game`. The worker holds the game; the page holds snapshots (CLAUDE.md rule 7). A fight is never
// saved (R2706000): after a reload the run, still on its fight, deals it again from its seed.

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

import type { ActionBody } from "@jackioh/shared";
import type { RunAction } from "@jackioh/run";

import "../../auth/tavern.css";
import { customCardsOfDeck, gameCustomCards, namesCustomCard, useCustomCards } from "../../creator/useCustomCards.ts";
import Game from "../../game/Game.tsx";
import { reducedMotionNow } from "../../game/animations.ts";
import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import { t } from "../../i18n/i18n.ts";
import { useAccount } from "../../net/gate.ts";
import { PRACTICE_PACING, PRACTICE_PACING_REDUCED, type PracticePacing } from "../../practice/config.ts";
import {
  PRACTICE_IDLE_STATE,
  createPracticeController,
  type PracticeController,
  type PracticeControllerState,
} from "../../practice/controller.ts";
import { usePracticeEmotes } from "../../practice/emotes.ts";
import { createPracticeHost, type PracticeHost } from "../../practice/host.ts";
import { ThinkIndicator } from "../../practice/ThinkIndicator.tsx";
import "../../practice/practice.css";
import { useSetting } from "../../settings/store.ts";
import { runFightSetup } from "../../wasm/index.ts";
import type { Dungeon } from "../store.ts";
import { ArtifactTray, type TrayItem } from "./ArtifactTray.tsx";
import { FIGHT_DIFFICULTY } from "./config.ts";
import { RunTimer } from "./RunTimer.tsx";
import { fightTestid } from "./testids.ts";
import "./fight.css";

export type FightScreenProps = {
  dungeon: Dungeon;
  /** Tests: the worker's host; default `createPracticeHost`. */
  hostFactory?: () => PracticeHost;
  /** Tests: the AI's pacing; default practice's, or its reduced one under reduced motion. */
  pacing?: PracticePacing;
};

/** The fight's report as the worker answered it: the run's action, or why there is none. */
type Report = { controller: PracticeController } & ({ action: RunAction } | { error: string });

function defaultHostFactory(): PracticeHost {
  return createPracticeHost();
}

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

const noSubscribe = (): (() => void) => () => {};
const idleState = (): PracticeControllerState => PRACTICE_IDLE_STATE;

/** The controller's state, as the practice route reads it. */
function useControllerState(controller: PracticeController | null): PracticeControllerState {
  return useSyncExternalStore(
    controller === null ? noSubscribe : controller.subscribe,
    controller === null ? idleState : controller.getState,
    controller === null ? idleState : controller.getState,
  );
}

/**
 * The AI's next step waits while the board animates, as in the practice route: an element under
 * `root` carries `data-animating` for exactly as long as the board holds a view back.
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

/** The setup, loading and failure screens are practice's reading column; the fight is its full-width board. */
function Shell({ variant, children }: { variant: "lobby" | "game"; children: ReactNode }): ReactElement {
  const shell = variant === "game" ? "app-shell app-shell--wide" : "app-shell tavern one-screen";
  return (
    <div className={`${shell} practice practice--${variant}`} data-testid={variant === "game" ? fightTestid.screen : undefined}>
      {children}
    </div>
  );
}

export function FightScreen({ dungeon, hostFactory, pacing }: FightScreenProps): ReactElement {
  const run = dungeon.run;
  const content = dungeon.content;
  // R2706000: the run writes the game; null outside a fight. Every effect keys on its seed, never on
  // the object, which a new render of the run may build again.
  const setup = useMemo(() => (run === null ? null : runFightSetup(run, content)), [run, content]);
  const seed = setup?.seed ?? null;
  const setupRef = useRef(setup);
  setupRef.current = setup;

  // §12.9, R915006: in the Sandbox the run's pool holds the creator's custom cards (DR1), which the
  // game must carry when either deck names one; an active account's are read, as practice reads them.
  const account = useAccount();
  const token = account.kind === "ready" && account.me.profile.status === "active" ? account.token : null;
  const decks = useMemo(() => (setup === null ? [] : [...setup.humanDeck, ...setup.enemyDeck]), [setup]);
  const needs = namesCustomCard(decks);
  const custom = useCustomCards(needs ? token : null);
  const ready = !needs || (account.kind !== "loading" && custom.state.kind !== "loading");
  const customCards = custom.state.kind === "ready" ? custom.state.cards : null;
  const cards = useMemo(() => gameCustomCards(customCardsOfDeck(decks, customCards ?? []).cards), [decks, customCards]);
  const cardsRef = useRef(cards);
  cardsRef.current = cards;
  /** The seed whose fight may start: once its cards are read it stays startable, so a renewed token never ends it. */
  const [readyFor, setReadyFor] = useState<string | null>(null);
  if (seed !== null && ready && readyFor !== seed) setReadyFor(seed);
  const startable = seed !== null && readyFor === seed;

  // Read when a fight starts, so a prop change mid-game does not tear it down.
  const factory = useRef(hostFactory ?? defaultHostFactory);
  factory.current = hostFactory ?? defaultHostFactory;
  const pace = useRef(pacing ?? (reducedMotionNow() ? PRACTICE_PACING_REDUCED : PRACTICE_PACING));
  pace.current = pacing ?? (reducedMotionNow() ? PRACTICE_PACING_REDUCED : PRACTICE_PACING);

  /** Bumped by the failure screen's Try again: the same fight, dealt again from its seed. */
  const [restart, setRestart] = useState(0);
  const [controller, setController] = useState<PracticeController | null>(null);
  useEffect(() => {
    const fight = setupRef.current;
    if (!startable || fight === null) return;
    const next = createPracticeController({ host: factory.current(), pacing: pace.current });
    setController(next);
    const carried = cardsRef.current;
    // B22: the AI plays the enemy at p2; `run` replaces the deck, the difficulty's handicap and the seed.
    void next.start({
      seed: fight.seed,
      difficulty: FIGHT_DIFFICULTY,
      humanSeat: "p1",
      deck: { kind: "random" },
      run: fight,
      ...(carried.length === 0 ? {} : { customCards: carried }),
    });
    return () => {
      next.dispose();
      setController((current) => (current === next ? null : current));
    };
  }, [startable, seed, restart]);

  const state = useControllerState(controller);
  const [boardRoot, setBoardRoot] = useState<HTMLDivElement | null>(null);
  useBoardBusy(boardRoot, controller);
  const emotes = usePracticeEmotes(state.config, state.snapshot, state.aiSeat, controller);
  const lethalOn = useSetting("lethalHint");
  const defs = state.defs;
  const lookup = useMemo(() => (defs === null ? null : lookupFromDefs(defs)), [defs]);

  // B25: once the game is over, the worker's report of it, the run's `FightOver`.
  const [report, setReport] = useState<Report | null>(null);
  const [reportAttempt, setReportAttempt] = useState(0);
  const over = state.phase === "over";
  useEffect(() => {
    if (controller === null || !over) return;
    let canceled = false;
    controller.runReport().then(
      (action) => {
        if (!canceled) setReport({ controller, action });
      },
      (cause: unknown) => {
        if (!canceled) setReport({ controller, error: messageOf(cause) });
      },
    );
    return () => {
      canceled = true;
    };
  }, [controller, over, reportAttempt]);
  const shownReport = report !== null && report.controller === controller ? report : null;
  const action = shownReport !== null && "action" in shownReport ? shownReport.action : null;

  /** What the run said to the report Continue sent it: its refusal, or null once it took it. */
  const [sent, setSent] = useState<{ controller: PracticeController; refusal: string | null } | null>(null);
  const answer = sent !== null && sent.controller === controller ? sent : null;
  const onContinue = useCallback(() => {
    if (action === null || controller === null) return;
    setSent({ controller, refusal: dungeon.act(action) });
  }, [action, controller, dungeon]);
  const onRetryReport = useCallback(() => {
    setReport(null);
    setReportAttempt((attempt) => attempt + 1);
  }, []);
  const onRetryFight = useCallback(() => {
    setRestart((count) => count + 1);
  }, []);
  const onAction = useCallback(
    (body: ActionBody) => {
      controller?.act(body);
    },
    [controller],
  );

  if (setup === null) {
    return (
      <Shell variant="lobby">
        <div className="practice-stage">
          <p className="notice" data-testid={fightTestid.noSetup} role="alert">
            {t("This fight could not be set up.")}
          </p>
        </div>
      </Shell>
    );
  }

  if (state.phase === "failed") {
    return (
      <Shell variant="lobby">
        <div className="practice-stage">
          <h1 className="practice-stage__title">{t("The fight stopped")}</h1>
          <p className="notice" data-testid={fightTestid.failed} role="alert">
            {t(state.failure ?? "Something went wrong while the game was running.")}
          </p>
          <button type="button" className="practice-play button-primary" data-testid={fightTestid.retry} onClick={onRetryFight}>
            {t("Try again")}
          </button>
        </div>
      </Shell>
    );
  }

  const snapshot = state.snapshot;
  if (controller === null || snapshot === null) {
    return (
      <Shell variant="lobby">
        <div className="practice-stage">
          <div className="practice-shuffle" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
          <p className="practice-stage__status" data-testid={fightTestid.loading} role="status">
            {t("Shuffling the decks…")}
          </p>
        </div>
      </Shell>
    );
  }

  const enemy = t(setup.enemy.name);
  // B23: the run's relics, then its blessings, beside the player's hero; the enemy's beside its own.
  const ownItems: TrayItem[] = [
    ...(run?.artifacts ?? []).map((artifact) => ({ id: artifact.id, level: artifact.level })),
    ...(run?.blessings ?? []).map((id) => ({ id, level: null })),
  ];
  const enemyItems: TrayItem[] = setup.enemy.artifacts.map((id) => ({ id, level: null }));

  const resultActions = (
    <>
      {shownReport !== null && "error" in shownReport ? (
        <>
          <p className="result-overlay__note" data-testid={fightTestid.reportError} role="alert">
            {t("The fight's result could not be read: {reason}", { reason: t(shownReport.error) })}
          </p>
          <button type="button" data-testid={fightTestid.reportRetry} onClick={onRetryReport}>
            {t("Retry")}
          </button>
        </>
      ) : (
        <button
          type="button"
          className="button-primary"
          data-testid={fightTestid.continue}
          // Once the run took the report it moves on (part F's screens); a refusal may be sent again.
          disabled={action === null || (answer !== null && answer.refusal === null)}
          onClick={onContinue}
        >
          {t("Continue")}
        </button>
      )}
      {answer === null || answer.refusal === null ? null : (
        <p className="result-overlay__note" data-testid={fightTestid.actRefusal} role="alert">
          {t(answer.refusal)}
        </p>
      )}
    </>
  );

  const board = (
    <Game
      view={snapshot.view}
      legal={snapshot.legal}
      onAction={onAction}
      error={snapshot.error}
      emotes={emotes}
      // A fight counts in the run's own stats (DR9), not the device's (R639).
      trackStats={false}
      wearCosmetics
      // R2291000: the lethal hint, as in every game against the AI, which the setting turns off.
      lethalHint={lethalOn && snapshot.lethal === true}
      matchStatus={
        <span className="dungeon-fight__status">
          <strong className="dungeon-fight__enemy" data-testid={fightTestid.enemy}>
            {enemy}
          </strong>
          <RunTimer startedAt={dungeon.startedAt} />
          <ThinkIndicator thinking={state.thinking} />
        </span>
      }
      heroAside={{
        own: <ArtifactTray side="own" items={ownItems} passives={content.passives} />,
        enemy: <ArtifactTray side="enemy" items={enemyItems} passives={content.passives} />,
      }}
      resultActions={resultActions}
    />
  );

  return (
    <Shell variant="game">
      {/* The board has no heading of its own; this names the screen for a screen reader, and draws nothing. */}
      <h1 className="tutorial-sr-only">{t("Fight against {enemy}", { enemy })}</h1>
      {/* The wrapper hands the board the screen's height (practice.css) and is the root `useBoardBusy` watches. */}
      <div className="practice-board practice-table" ref={setBoardRoot} data-thinking={state.thinking ? "true" : "false"}>
        {lookup === null ? board : <CatalogContext.Provider value={lookup}>{board}</CatalogContext.Provider>}
      </div>
    </Shell>
  );
}
