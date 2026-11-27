// An Infinite Assault level's fight (issue #4582, IA7): while the attempt stands on a fight
// (`AssaultPhase` `fight`), the page plays it as a practice game against the AI (§9.9, R4582000).
// The Assault writes the game's setup (`assaultFightSetup`: the seed, the player's deck and
// handicap against the enemy's deck, handicap and deck marks), the practice worker plays it with
// the AI as the enemy, whose name and level the board's rail shows beside the chip that lists the
// live modifiers (R3912000, R4582002), and the enemy's upgrades sit in a tray beside its hero,
// with a Commander's name on it (R4582002). There is no Play again: once the game is over the page
// asks the worker for its report, the attempt's `fightOver`, and Continue sends it to the attempt,
// which a lost fight ends.
//
// The board is the practice route's (`routes/practice.tsx`): the same controller, pacing
// (`practice/aiTable.ts`: the AI waits on `data-animating`, `data-showcase` and `data-speaking`,
// #2925), emotes and `Game`, with no Offer draw (R188) and the lethal hint as in every game
// against the AI (R2102000). The worker holds the game; the page holds snapshots (CLAUDE.md
// rule 7). A fight left unfinished waits where it stood (R3476000, R4582001): after a reload the
// attempt, still on its fight, resumes the kept fight where it was left; Try again deals it fresh
// from its seed instead (R2972000). Once over, the fight is kept on the device as a free game is,
// and its result offers Watch the replay beside Continue (R5125000).

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
} from "react";

import type { ActionBody } from "@jackioh/shared";
import type { AssaultAction, AssaultDay, AssaultState } from "@jackioh/run";

import "../../auth/tavern.css";
import Game from "../../game/Game.tsx";
import { reducedMotionNow } from "../../game/animations.ts";
import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import { MainMenuButton } from "../../game/MainMenuButton.tsx";
import { ModifierList } from "../../game/ModifierList.tsx";
import { t } from "../../i18n/i18n.ts";
import { PRACTICE_PACING, PRACTICE_PACING_REDUCED, type PracticePacing } from "../../practice/config.ts";
import { useBoardBusy, useControllerState, useShowcaseHold, useVoiceHold } from "../../practice/aiTable.ts";
import { createPracticeController, type PracticeController } from "../../practice/controller.ts";
import { usePracticeEmotes } from "../../practice/emotes.ts";
import { createPracticeHost, type PracticeHost } from "../../practice/host.ts";
import type { PracticeStartConfig } from "../../practice/protocol.ts";
import { ThinkIndicator } from "../../practice/ThinkIndicator.tsx";
import { FightMenuButton, FightShell, fightNoSetup, fightShuffling, fightStopped } from "../../practice/FightStages.tsx";
import { BackLayer } from "../../net/backLayers.ts";
import { navigate, paths, useLeaveGuard } from "../../net/navigate.ts";
import { messageOf } from "../../net/errors.ts";
import "../../practice/practice.css";
import ReplayPlayer from "../../replay/ReplayPlayer.tsx";
import { useSetting } from "../../settings/store.ts";
import { assaultFightSetup } from "../../wasm/index.ts";
import ConfirmLeave from "./ConfirmLeave.tsx";
import { UpgradeTray } from "./UpgradeTray.tsx";
import { ASSAULT_FIGHT_DIFFICULTY } from "./config.ts";
import { assaultFightTestid } from "./testids.ts";
import "./fight.css";

export type AssaultFightScreenProps = {
  state: AssaultState;
  day: AssaultDay;
  onAction: (action: AssaultAction) => void;
  /** R4733000: the store's hold for a finished fight's report, called the moment it arrives. */
  onHold: (action: AssaultAction) => void;
  /** R4582001: the page's way back to its day screen, the attempt still on its fight; absent, Menu leaves to the Play menu. */
  onLeave?: () => void;
  /** Tests: the worker's host; default `createPracticeHost`. */
  hostFactory?: () => PracticeHost;
  /** Tests: the AI's pacing; default practice's, or its reduced one under reduced motion. */
  pacing?: PracticePacing;
};

/** The fight's report as the worker answered it: the attempt's action, or why there is none. */
type Report = { controller: PracticeController } & ({ action: AssaultAction } | { error: string });

function defaultHostFactory(): PracticeHost {
  return createPracticeHost();
}

export function AssaultFightScreen({ state, day, onAction, onHold, onLeave, hostFactory, pacing }: AssaultFightScreenProps): ReactElement {
  // R4582000: the Assault writes the game; null outside a fight. Every effect keys on its seed,
  // never on the object, which a new render of the attempt may build again.
  const setup = useMemo(() => assaultFightSetup(state, day), [state, day]);
  const seed = setup?.seed ?? null;
  const setupRef = useRef(setup);
  setupRef.current = setup;

  // Read when a fight starts, so a prop change mid-game does not tear it down.
  const factory = useRef(hostFactory ?? defaultHostFactory);
  factory.current = hostFactory ?? defaultHostFactory;
  const pace = useRef(pacing ?? (reducedMotionNow() ? PRACTICE_PACING_REDUCED : PRACTICE_PACING));
  pace.current = pacing ?? (reducedMotionNow() ? PRACTICE_PACING_REDUCED : PRACTICE_PACING);
  const onActionRef = useRef(onAction);
  onActionRef.current = onAction;
  const onHoldRef = useRef(onHold);
  onHoldRef.current = onHold;

  /** Bumped by the failure screen's Try again: the same fight, dealt again from its seed. */
  const [restart, setRestart] = useState(0);
  const [controller, setController] = useState<PracticeController | null>(null);
  useEffect(() => {
    const fight = setupRef.current;
    if (fight === null) return;
    const next = createPracticeController({ host: factory.current(), pacing: pace.current });
    setController(next);
    // R4582000: the AI plays the enemy at p2; `assault` replaces the deck, the difficulty's handicap and the seed.
    const config: PracticeStartConfig = {
      seed: fight.seed,
      difficulty: ASSAULT_FIGHT_DIFFICULTY,
      humanSeat: "p1",
      deck: { kind: "random" },
      assault: fight,
    };
    if (restart === 0) {
      // R4582001: a kept fight is picked up where it stood; one that does not fold is dealt fresh.
      void next.resume(config).then(() => {
        if (next.getState().phase === "failed" && next.getState().snapshot === null) void next.start(config);
      });
    } else {
      // R2972000: Try again deals the same fight fresh from its seed.
      void next.start(config);
    }
    return () => {
      next.dispose();
      setController((current) => (current === next ? null : current));
    };
  }, [seed, restart]);

  const controllerState = useControllerState(controller);
  const [boardRoot, setBoardRoot] = useState<HTMLDivElement | null>(null);
  useBoardBusy(boardRoot, controller);
  useVoiceHold(controller);
  useShowcaseHold(controller);
  const emotes = usePracticeEmotes(controllerState.config, controllerState.snapshot, controllerState.aiSeat, controller);
  const lethalOn = useSetting("lethalHint");
  const defs = controllerState.defs;
  const lookup = useMemo(() => (defs === null ? null : lookupFromDefs(defs)), [defs]);

  // R4582000: once the game is over, the worker's report of it, the attempt's `fightOver`.
  const [report, setReport] = useState<Report | null>(null);
  const [reportAttempt, setReportAttempt] = useState(0);
  const over = controllerState.phase === "over";
  useEffect(() => {
    if (controller === null || !over) return;
    let canceled = false;
    controller.assaultReport().then(
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

  // R4582000: Continue sends the report to the attempt once, and then no more.
  // R4733000: the report goes once per fight, however the player leaves: Continue, Menu or unmount.
  const [sent, setSent] = useState<PracticeController | null>(null);
  const sentRef = useRef<PracticeController | null>(null);
  const sendOnce = useCallback(() => {
    if (action === null || controller === null || sentRef.current === controller) return;
    sentRef.current = controller;
    setSent(controller);
    onActionRef.current(action);
  }, [action, controller]);
  // R4733000: the report is held beside the kept attempt the moment it arrives, so leaving before
  // Continue keeps the result. Holding changes no screen: the board keeps showing its end.
  const heldFor = useRef<PracticeController | null>(null);
  useEffect(() => {
    if (action !== null && controller !== null && heldFor.current !== controller) {
      heldFor.current = controller;
      onHoldRef.current(action);
    }
  }, [action, controller]);
  // R4733000: leaving with a held report the screen never sent applies it, as Continue would.
  const sendRef = useRef(sendOnce);
  sendRef.current = sendOnce;
  useEffect(
    () => () => {
      sendRef.current();
    },
    [controller],
  );
  const onRetryReport = useCallback(() => {
    setReport(null);
    setReportAttempt((attempt) => attempt + 1);
  }, []);
  const onRetryFight = useCallback(() => {
    setRestart((count) => count + 1);
  }, []);
  const onAct = useCallback(
    (body: ActionBody) => {
      controller?.act(body);
    },
    [controller],
  );

  const snapshot = controllerState.snapshot;

  // R2972000: while the fight is live, every in-app move off it asks first (R1754000), as every
  // way off a live online match does (R2479000). These hooks sit before the screens below: a
  // live-to-failed transition re-renders this same screen, and hooks after a return would change
  // the hook count. The stopped screen holds nothing: Try again deals the same fight fresh from
  // its seed (R4582001), so there is no kept fight to pick up. The cleanup releases the guard when
  // the store moves screens on its own (Continue sends the report and unmounts this screen), and
  // `!over` releases it once the fight has its result.
  const atStake =
    setup !== null && controller !== null && snapshot !== null && !over && controllerState.phase !== "failed";
  const [leaving, setLeaving] = useLeaveGuard(atStake);

  const onMenu = useCallback(() => {
    if (atStake && onLeave !== undefined) {
      setLeaving({ leave: onLeave });
      return;
    }
    if (!atStake) sendOnce(); // R4582001: a finished fight's report goes to the attempt first
    if (onLeave === undefined) navigate(paths.play); // mid-fight the leave guard asks (R2972000)
    else onLeave();
  }, [atStake, onLeave, sendOnce, setLeaving]);

  // R5125000: the finished fight, kept on the device as a free game is (R768), watched again from its
  // result. Keyed by the controller, so a fight dealt again never opens the last one's replay; watching
  // sends nothing to the attempt, and Back returns to the result.
  const [watching, setWatching] = useState<PracticeController | null>(null);
  const keptGame = snapshot?.replay?.game;
  const keptSteps = snapshot?.replay?.steps;
  const replaySource = useMemo(
    () =>
      controller === null || keptGame === undefined || keptSteps === undefined
        ? null
        : controller.replaySource({ game: keptGame, steps: keptSteps, portraits: null }),
    [controller, keptGame, keptSteps],
  );
  const onWatchReplay = useCallback(() => {
    setWatching(controller);
  }, [controller]);
  const onLeaveReplay = useCallback(() => {
    setWatching(null);
  }, []);

  if (setup === null) return fightNoSetup(assaultFightTestid);

  if (controllerState.phase === "failed") return fightStopped(assaultFightTestid, controllerState.failure, onRetryFight);

  if (controller === null || snapshot === null) return fightShuffling(assaultFightTestid);

  const enemy = t(setup.level.enemy);

  if (watching !== null && watching === controller && replaySource !== null) {
    // The enemy's name on its hero, as the replay list opens it (R5125000).
    const player = <ReplayPlayer source={replaySource} onLeave={onLeaveReplay} opponentLabel={enemy} />;
    return (
      <FightShell variant="game" testids={assaultFightTestid}>
        {/* R4619000: Back means the replay's corner arrow; first, so what the replay opens is a layer above it. */}
        <BackLayer onClose={onLeaveReplay} />
        {lookup === null ? player : <CatalogContext.Provider value={lookup}>{player}</CatalogContext.Provider>}
      </FightShell>
    );
  }

  const resultActions = (
    <>
      {shownReport !== null && "error" in shownReport ? (
        <>
          <p className="result-overlay__note" data-testid={assaultFightTestid.reportError} role="alert">
            {t("The fight's result could not be read: {reason}", { reason: t(shownReport.error) })}
          </p>
          <button type="button" data-testid={assaultFightTestid.reportRetry} onClick={onRetryReport}>
            {t("Retry")}
          </button>
        </>
      ) : (
        <button
          type="button"
          className="button-primary"
          data-testid={assaultFightTestid.continue}
          // Once the attempt took the report this screen is gone; until then Continue sends it once.
          disabled={action === null || sent === controller}
          onClick={sendOnce}
        >
          {t("Continue")}
        </button>
      )}
      {/* #4742, R4742000: after Continue, which keeps the focus and the primary look: the board's Menu, once the fight is over. */}
      <MainMenuButton testid={assaultFightTestid.resultMenu} onClick={onMenu} />
      {/* R5125000: once the worker kept the fight, as practice's result offers it (R768000). */}
      {replaySource === null ? null : (
        <button type="button" data-testid={assaultFightTestid.watchReplay} onClick={onWatchReplay}>
          {t("Watch the replay")}
        </button>
      )}
    </>
  );

  // R4582001: out to the day screen, or the Play menu with no way back; mid-fight it asks first,
  // and leaves once the fight has its result.
  const menuButton = <FightMenuButton testids={assaultFightTestid} over={over} onClick={onMenu} />;

  const board = (
    <Game
      view={snapshot.view}
      legal={snapshot.legal}
      onAction={onAct}
      error={snapshot.error}
      // R3477000: the practice snapshot's forecast of the human's attacks.
      forecast={snapshot.forecast}
      emotes={emotes}
      // An Assault fight counts in the attempt's own record, not the device's (R639, R4582000).
      trackStats={false}
      wearCosmetics
      // §9.9, R188: the AI declines every draw offer, so the board draws no Offer draw, as practice's.
      showOfferDraw={false}
      // R2102000: the lethal hint, as in every game against the AI, which the setting turns off.
      lethalHint={lethalOn && snapshot.lethal === true}
      controlExtras={menuButton}
      matchStatus={
        <>
          {/* R3912000, R4582002: every live modifier in full, as every other table's rail lists them
              (R169), the enemy's name over its list. Beside the text, not inside it: the chip is 32px
              tall and the text wraps, so inside it a phone's one-row control bar would grow a line taller. */}
          <ModifierList view={snapshot.view} opponentLabel={enemy} />
          <span className="assault-fight__status">
            <strong className="assault-fight__enemy" data-testid={assaultFightTestid.enemy}>
              {enemy}
            </strong>
            <span className="assault-fight__level" data-testid={assaultFightTestid.level}>
              {t("Level {level}", { level: String(setup.level.level) })}
            </span>
            <ThinkIndicator thinking={controllerState.thinking} />
          </span>
        </>
      }
      // R4582002: a Commander's name on its hero; a normal enemy's hero keeps the default label.
      enemyLabel={setup.level.commander === null ? undefined : enemy}
      heroAside={{
        enemy: <UpgradeTray upgrades={setup.level.upgrades} />,
      }}
      resultActions={resultActions}
      // R4582001: a lost fight ends the attempt, so the Concede dialog says so.
      concedeBody={t("Your opponent wins, and this attempt ends here.")}
    />
  );

  return (
    <FightShell variant="game" testids={assaultFightTestid}>
      {/* The board has no heading of its own; this names the screen for a screen reader, and draws nothing. */}
      <h1 className="tutorial-sr-only">{t("Fight against {enemy}", { enemy })}</h1>
      {/* The wrapper hands the board the screen's height (practice.css) and is the root `useBoardBusy` watches. */}
      <div className="practice-board practice-table" ref={setBoardRoot} data-thinking={controllerState.thinking ? "true" : "false"}>
        {lookup === null ? board : <CatalogContext.Provider value={lookup}>{board}</CatalogContext.Provider>}
      </div>
      {leaving === null ? null : (
        <ConfirmLeave
          onStay={() => {
            setLeaving(null);
          }}
          onLeave={() => {
            const go = leaving.leave;
            setLeaving(null);
            go();
          }}
        />
      )}
    </FightShell>
  );
}
