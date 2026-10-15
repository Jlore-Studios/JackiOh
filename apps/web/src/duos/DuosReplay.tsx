// A finished co-op game, a step at a time (#965, SPEC §13.9, B41): the result panel's "Watch the
// replay". `GET /api/duos/matches/:matchId/replay` answers every step at once, the seat's own view
// after each, so it is read once (`openDuosReplay`) and the steps are held here.
//
// The controls are the replay player's (`replay/ReplayPlayer.tsx`), a round standing for a turn
// (§13.3: a `DuosView` carries no turn). CLAUDE.md rule 7: only the server's views are drawn, on
// `DuosBoard`'s `replay` mode, and nothing is computed from them.
//
// A step forward hands the board the next view with the same key, so its animations and sounds run
// as live play's do (R201's speed included). Any other move, a step back or a jump, gives the board
// a fresh key: it mounts on that view with nothing animating, and its portal shows the own duel again.
// The controls sit outside the board, so a remount never takes the scrubber from under the pointer.

import { useCallback, useEffect, useState, type ReactElement, type ReactNode } from "react";

import { useFxSettings } from "../fx/settings.ts";
import { scaleForSpeed } from "../game/animations.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { ApiRequestError } from "../net/api.ts";
import { getDuosReplay, type ReceivedPing } from "../net/duos.ts";
import { REPLAY_OPEN_ATTEMPTS, REPLAY_OPEN_RETRY_MS, REPLAY_PLAY_GAP_MS } from "../replay/config.ts";
import { replayTestid, useBoardBusy } from "../replay/ReplayPlayer.tsx";
import { ReplayFailure, replayFailureOf, replayFailureText } from "../replay/source.ts";
import { onlineFailure } from "../replay/sources.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { DuosChromeContext } from "./context.ts";
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
 * (`on_terminal`, `crates/server/src/duos/actor.rs`), so a result panel can open the replay while the
 * server still answers 409: it is asked again, `REPLAY_OPEN_ATTEMPTS` times in all, as
 * `openOnlineMatchReplay` asks (R768000). Any refusal rejects with a `ReplayFailure`, as a 1v1
 * replay's does.
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
      // A 409 is a match whose result is still being written: ask again, a little later.
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

function DuosReplayPlayer({ steps, back }: { steps: DuosReplaySteps; back: ReactNode }): ReactElement {
  useLocale();
  const last = steps.length - 1;
  const [shown, setShown] = useState<Shown>({ at: 0, board: 0 });
  const [playing, setPlaying] = useState(false);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
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
    show(index);
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

  const atStart = shown.at <= 0;
  const atEnd = shown.at >= last;

  return (
    <div className="app-shell app-shell--wide" ref={setRoot} data-testid={duosTestid.replay} data-step={shown.at} data-board={shown.board}>
      <div className="replay-controls" role="toolbar" aria-label={t("Replay controls")} data-testid={replayTestid.controls}>
        <button type="button" data-testid={replayTestid.start} aria-label={t("To the start")} disabled={atStart} onClick={() => goTo(0)}>
          ⏮
        </button>
        <button
          type="button"
          data-testid={replayTestid.backTurn}
          aria-label={t("Back a turn")}
          disabled={atStart}
          onClick={() => goTo(previousRound(steps, shown.at))}
        >
          ⏪
        </button>
        <button
          type="button"
          data-testid={replayTestid.backStep}
          aria-label={t("Back a step")}
          disabled={atStart}
          onClick={() => goTo(shown.at - 1)}
        >
          ‹
        </button>
        <button
          type="button"
          data-testid={replayTestid.play}
          aria-label={playing ? t("Pause") : t("Play")}
          aria-pressed={playing}
          disabled={atEnd}
          onClick={() => setPlaying(!playing)}
        >
          {playing ? "⏸" : "▶"}
        </button>
        <button
          type="button"
          data-testid={replayTestid.forwardStep}
          aria-label={t("Forward a step")}
          disabled={atEnd}
          onClick={() => goTo(shown.at + 1)}
        >
          ›
        </button>
        <button
          type="button"
          data-testid={replayTestid.forwardTurn}
          aria-label={t("Forward a turn")}
          disabled={atEnd}
          onClick={() => goTo(nextRound(steps, shown.at))}
        >
          ⏩
        </button>
        <button type="button" data-testid={replayTestid.end} aria-label={t("To the end")} disabled={atEnd} onClick={() => goTo(last)}>
          ⏭
        </button>
        <input
          type="range"
          data-testid={replayTestid.scrubber}
          aria-label={t("Step")}
          min={0}
          max={last}
          step={1}
          value={shown.at}
          onChange={(event) => {
            goTo(Number(event.currentTarget.value));
          }}
        />
        <span className="replay-readout" data-testid={replayTestid.readout}>
          {t("Step {step} of {steps} · Round {round}", { step: shown.at + 1, steps: steps.length, round: view.round })}
        </span>
      </div>
      <DuosChromeContext.Provider value={{ back }}>
        <DuosBoard key={shown.board} view={view} legal={NOTHING} onAction={ignore} pings={NO_PINGS} onPing={ignore} replay />
      </DuosChromeContext.Provider>
    </div>
  );
}

export default function DuosReplay({ matchId, onLeave }: { matchId: string; onLeave: () => void }): ReactElement {
  useLocale();
  const [steps, setSteps] = useState<DuosReplaySteps | null>(null);
  const [failure, setFailure] = useState<ReplayFailure | null>(null);

  useEffect(() => {
    let cancelled = false;
    openDuosReplay(matchId).then(
      (loaded) => {
        if (!cancelled) setSteps(loaded);
      },
      (cause: unknown) => {
        if (!cancelled) setFailure(replayFailureOf(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, [matchId]);

  const back = (
    <button type="button" className="link-button" data-testid={replayTestid.back} onClick={onLeave}>
      {t("← Back")}
    </button>
  );
  if (steps !== null) return <DuosReplayPlayer steps={steps} back={back} />;
  return (
    <div className="app-shell">
      {back}
      {failure === null ? (
        <p className="notice" role="status" data-testid={replayTestid.loading}>
          {t("Loading the replay…")}
        </p>
      ) : (
        <p className="notice" role="alert" data-testid={replayTestid.failure}>
          {replayFailureText(failure)}
        </p>
      )}
    </div>
  );
}
