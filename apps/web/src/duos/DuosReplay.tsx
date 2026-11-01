// A finished co-op game, a step at a time (#965, SPEC §13.9, B41), read once (`openDuosReplay`). A
// failed read that can pass (`reason: "error"`) offers Try again (#1691); an expired or refused one
// offers none. CLAUDE.md rule 7: only the server's views are drawn, on `DuosBoard`'s `replay` mode.
// The controls are the 1v1 replay's (`replay/ReplayToolbar.tsx`), a round standing for a turn (§13.3).
//
// A step forward keeps the board's key, so its animations and sounds run as live play's do (R201's
// speed included); any other move gives a fresh key, mounting with nothing animating. The controls sit
// outside the board, so a remount never takes the scrubber from under the pointer, and the scrubber
// held moves the board only when it is let go (#2025). Download JSON (R1242000) waits for the steps.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactElement, type ReactNode } from "react";

import { useFxSettings } from "../fx/settings.ts";
import { scaleForSpeed } from "../game/animations.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { ApiRequestError } from "../net/api.ts";
import { getDuosReplay, type ReceivedPing } from "../net/duos.ts";
import { REPLAY_OPEN_ATTEMPTS, REPLAY_OPEN_RETRY_MS, REPLAY_PLAY_GAP_MS } from "../replay/config.ts";
import { downloadFile } from "../replay/download.ts";
import { replayTestid, useBoardBusy } from "../replay/ReplayPlayer.tsx";
import { ReplayToolbar, isTyping } from "../replay/ReplayToolbar.tsx";
import { ReplayFailure, replayFailureOf, replayFailureText } from "../replay/source.ts";
import { onlineFailure } from "../replay/sources.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { DuosChromeContext } from "./context.ts";
import { duosReplayJsonFile } from "./download.ts";
import { DuosBoard } from "./DuosBoard.tsx";
import { duosTestid } from "./testids.ts";
import "../replay/replay.css";

/** A co-op replay's steps: the server's answer always holds step 0. */
export type DuosReplaySteps = readonly [DuosView, ...DuosView[]];

/** A board to look at is offered nothing, hears no ping and sends nothing. */
const NOTHING: readonly DuosActionBody[] = [];
const NO_PINGS: readonly ReceivedPing[] = [];
function ignore(): void {}

/** The step on the board, and which mount of the board shows it (`DuosBoard`'s key). */
type Shown = { at: number; board: number };

/**
 * The replay of `matchId`, read once. The actor sends a match's last view before it writes its end
 * (`on_terminal`, `crates/server/src/duos/actor.rs`), so the server may still answer 409: it is asked
 * again, `REPLAY_OPEN_ATTEMPTS` times in all, as `openOnlineMatchReplay` asks (R768000). Any refusal
 * rejects with a `ReplayFailure`, as a 1v1 replay's does.
 */
export async function openDuosReplay(
  matchId: string,
  wait: (ms: number) => Promise<void> = (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
): Promise<DuosReplaySteps> {
  for (let attempt = 1; ; attempt += 1) {
    try {
      const [first, ...rest] = (await getDuosReplay(matchId)).steps;
      if (first === undefined) throw new ReplayFailure("error", "the co-op replay holds no step");
      return [first, ...rest];
    } catch (cause) {
      const finishing = cause instanceof ApiRequestError && cause.status === 409;
      if (!finishing || attempt >= REPLAY_OPEN_ATTEMPTS) throw onlineFailure(cause);
      await wait(REPLAY_OPEN_RETRY_MS);
    }
  }
}

/** The first step after `at` in another round, or the last step when `at` is in the last round. */
export function nextRound(steps: DuosReplaySteps, at: number): number {
  for (let next = at + 1; next < steps.length; next += 1) {
    if (steps[next]?.round !== steps[at]?.round) return next;
  }
  return steps.length - 1;
}

/** The first step of the round before `at`'s (or of `at`'s own round, when `at` is past its first step). */
export function previousRound(steps: DuosReplaySteps, at: number): number {
  if (at <= 0) return 0;
  const round = steps[at - 1]?.round;
  let first = at - 1;
  while (first > 0 && steps[first - 1]?.round === round) first -= 1;
  return first;
}

/** The control inside the co-op player that has focus, by test id: a fresh board takes its element away. */
function focusedDuosControl(): string | null {
  const active = document.activeElement;
  if (!(active instanceof HTMLElement) || active.closest(`[data-testid="${duosTestid.replay}"]`) === null) return null;
  return active.dataset.testid ?? null;
}

export function DuosReplayPlayer({ steps, back, actions }: { steps: DuosReplaySteps; back: ReactNode; actions?: ReactNode }): ReactElement {
  useLocale();
  const last = steps.length - 1;
  const [shown, setShown] = useState<Shown>({ at: 0, board: 0 });
  const [playing, setPlaying] = useState(false);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
  /** The control that had focus when the board was mounted afresh, to give it back to the new one. */
  const refocus = useRef<string | null>(null);
  const [fx] = useFxSettings();
  const busy = useBoardBusy(root);
  const view = steps[shown.at] ?? steps[0];

  /** Shows step `index`, clamped to the replay; only the next step carries the board on. */
  const show = useCallback(
    (index: number): void => {
      setShown((prev) => {
        const at = Math.min(Math.max(index, 0), last);
        if (at === prev.at) return prev;
        return { at, board: at === prev.at + 1 ? prev.board : prev.board + 1 };
      });
    },
    [last],
  );

  /** A move the viewer made: Play stops. */
  function goTo(index: number): void {
    setPlaying(false);
    const target = Math.min(Math.max(index, 0), last);
    if (target !== shown.at + 1) refocus.current = focusedDuosControl();
    show(target);
  }

  function goRound(direction: 1 | -1): void {
    goTo(direction === 1 ? nextRound(steps, shown.at) : previousRound(steps, shown.at));
  }

  // Play: the next step comes a gap after the board has settled, the gap shortened by the table's
  // speed (R201); any move stops it, and so does the last step.
  useEffect(() => {
    if (!playing || busy || shown.at >= last) return undefined;
    const timer = window.setTimeout(() => {
      show(shown.at + 1);
      if (shown.at + 1 >= last) setPlaying(false);
    }, scaleForSpeed(REPLAY_PLAY_GAP_MS, fx.speed));
    return () => {
      window.clearTimeout(timer);
    };
  }, [playing, busy, shown.at, last, fx.speed, show]);

  const keys = useRef<(event: KeyboardEvent) => void>(ignore);
  keys.current = (event) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    if (event.altKey || event.ctrlKey || event.metaKey || event.defaultPrevented || isTyping(event.target)) return;
    event.preventDefault();
    const direction = event.key === "ArrowRight" ? 1 : -1;
    if (event.shiftKey) goRound(direction);
    else goTo(shown.at + direction);
  };
  useEffect(() => {
    const listener = (event: KeyboardEvent): void => {
      keys.current(event);
    };
    window.addEventListener("keydown", listener);
    return () => {
      window.removeEventListener("keydown", listener);
    };
  }, []);

  // A fresh board is new elements, so the control that was pressed has gone: focus its successor.
  const board = shown.board;
  useLayoutEffect(() => {
    const id = refocus.current;
    refocus.current = null;
    if (id !== null) root?.querySelector<HTMLElement>(`[data-testid="${id}"]`)?.focus({ preventScroll: true });
  }, [board, root]);

  return (
    <div className="app-shell app-shell--wide" ref={setRoot} data-testid={duosTestid.replay} data-step={shown.at} data-board={shown.board}>
      <ReplayToolbar
        at={shown.at}
        last={last}
        playing={playing}
        onPlaying={setPlaying}
        onGoTo={goTo}
        onTurn={goRound}
        readout={t("Step {step} of {steps} · Round {round}", { step: shown.at + 1, steps: steps.length, round: view.round })}
        actions={actions}
      />
      <DuosChromeContext.Provider value={{ back }}>
        {/* Replay steps carry no economy: no seat is ever capped. */}
        <DuosBoard key={shown.board} view={view} legal={NOTHING} onAction={ignore} pings={NO_PINGS} onPing={ignore} jloreCapped={false} replay />
      </DuosChromeContext.Provider>
    </div>
  );
}

export function DuosReplayBack({ onLeave }: { onLeave: () => void }): ReactElement {
  useLocale();
  return (
    <button type="button" className="link-button" data-testid={replayTestid.back} onClick={onLeave}>
      {t("← Back")}
    </button>
  );
}

export default function DuosReplay({ matchId, onLeave }: { matchId: string; onLeave: () => void }): ReactElement {
  useLocale();
  const [steps, setSteps] = useState<DuosReplaySteps | null>(null);
  const [failure, setFailure] = useState<ReplayFailure | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let canceled = false;
    openDuosReplay(matchId).then(
      (loaded) => {
        if (!canceled) setSteps(loaded);
      },
      (cause: unknown) => {
        if (!canceled) setFailure(replayFailureOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
    // One load per mount and per Try again (#1691).
  }, [matchId, attempt]);

  const back = <DuosReplayBack onLeave={onLeave} />;
  if (steps !== null) {
    return (
      <DuosReplayPlayer
        steps={steps}
        back={back}
        actions={
          <button
            type="button"
            data-testid={duosTestid.replayDownload}
            onClick={() => {
              downloadFile(duosReplayJsonFile(steps));
            }}
          >
            {t("Download JSON")}
          </button>
        }
      />
    );
  }
  return (
    <div className="app-shell">
      {back}
      {failure === null ? (
        <Skeleton variant="panel" label={t("Loading the replay…")} testId={replayTestid.loading} />
      ) : (
        <>
          <p className="notice" role="alert" data-testid={replayTestid.failure}>
            {replayFailureText(failure)}
          </p>
          {failure.reason === "error" ? (
            <div className="row">
              <button
                type="button"
                className="button-primary"
                data-testid={replayTestid.retry}
                onClick={() => {
                  setFailure(null);
                  setAttempt((before) => before + 1);
                }}
              >
                {t("Try again")}
              </button>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
