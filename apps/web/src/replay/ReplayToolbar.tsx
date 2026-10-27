// The toolbar both replays share (#2025): the step and turn buttons, Play, the scrubber with
// its held state, a readout slot and an extra-actions slot. The 1v1 player puts it in its
// board's notice slot, so it is mounted afresh with each fresh board, and the co-op replay puts
// it above its board.
//
// It decides nothing: each replay hands it the step on the board, its own moves (a co-op turn
// is a round, §13.9) and its readout, and clamps the moves itself. While the scrubber is held
// nothing moves: the board goes to where it is let go (`pointerup` on the window), and
// `pointercancel` leaves it where it was. Moved without a pointer (its arrow keys, a `change`
// with no hold), it moves at once. Giving focus back after a fresh board is the replay's job.

import { useEffect, useRef, useState } from "react";
import type { ReactElement, ReactNode } from "react";

import { t, useLocale } from "../i18n/i18n.ts";

export const replayTestid = {
  player: "replay-player",
  controls: "replay-controls",
  start: "replay-start",
  backTurn: "replay-back-turn",
  backStep: "replay-back-step",
  play: "replay-play",
  forwardStep: "replay-forward-step",
  forwardTurn: "replay-forward-turn",
  end: "replay-end",
  scrubber: "replay-scrubber",
  readout: "replay-readout",
  back: "replay-back",
  loading: "replay-loading",
  failure: "replay-failure",
  /** #1568: a failed page offers Try again beside the alert. */
  retry: "replay-retry",
} as const;

/** A key the arrow keys must leave to its own control: a text field, a select, the scrubber. */
export function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")
  );
}

export type ReplayToolbarProps = {
  /** The step on the board. */
  at: number;
  /** The replay's last step. */
  last: number;
  playing: boolean;
  /** Play or Pause pressed; a hold of the scrubber stops Play. */
  onPlaying: (playing: boolean) => void;
  /** A move to step `index`: the replay clamps it. */
  onGoTo: (index: number) => void;
  /** A turn back or forward (a round, in a co-op replay). */
  onTurn: (direction: 1 | -1) => void;
  /** The readout's words: each replay says where it is its own way. */
  readout: ReactNode;
  /** More controls at the toolbar's end: the co-op replay's Download JSON. */
  actions?: ReactNode;
};

const IGNORE = (): void => undefined;

export function ReplayToolbar({ at, last, playing, onPlaying, onGoTo, onTurn, readout, actions }: ReplayToolbarProps): ReactElement {
  useLocale();
  /** Where the scrubber is while it is held: the board moves when it is let go, not at every tick. */
  const [scrub, setScrub] = useState<number | null>(null);

  // The scrubber let go, wherever the pointer is: the board goes to where it was left. Until then it
  // stays, since a board mounted afresh at every tick would take the scrubber from under the pointer.
  const dropScrub = useRef<() => void>(IGNORE);
  dropScrub.current = () => {
    if (scrub === null) return;
    setScrub(null);
    onGoTo(scrub);
  };
  const scrubbing = scrub !== null;
  useEffect(() => {
    if (!scrubbing) return;
    const drop = (): void => {
      dropScrub.current();
    };
    const cancel = (): void => {
      setScrub(null);
    };
    window.addEventListener("pointerup", drop);
    window.addEventListener("pointercancel", cancel);
    return () => {
      window.removeEventListener("pointerup", drop);
      window.removeEventListener("pointercancel", cancel);
    };
  }, [scrubbing]);

  const atStart = at <= 0;
  const atEnd = at >= last;
  return (
    <div className="replay-controls" role="toolbar" aria-label={t("Replay controls")} data-testid={replayTestid.controls}>
      <button type="button" data-testid={replayTestid.start} aria-label={t("To the start")} disabled={atStart} onClick={() => onGoTo(0)}>
        ⏮
      </button>
      <button type="button" data-testid={replayTestid.backTurn} aria-label={t("Back a turn")} disabled={atStart} onClick={() => onTurn(-1)}>
        ⏪
      </button>
      <button type="button" data-testid={replayTestid.backStep} aria-label={t("Back a step")} disabled={atStart} onClick={() => onGoTo(at - 1)}>
        ‹
      </button>
      <button
        type="button"
        data-testid={replayTestid.play}
        aria-label={playing ? t("Pause") : t("Play")}
        aria-pressed={playing}
        disabled={atEnd}
        onClick={() => onPlaying(!playing)}
      >
        {playing ? "⏸" : "▶"}
      </button>
      <button type="button" data-testid={replayTestid.forwardStep} aria-label={t("Forward a step")} disabled={atEnd} onClick={() => onGoTo(at + 1)}>
        ›
      </button>
      <button type="button" data-testid={replayTestid.forwardTurn} aria-label={t("Forward a turn")} disabled={atEnd} onClick={() => onTurn(1)}>
        ⏩
      </button>
      <button type="button" data-testid={replayTestid.end} aria-label={t("To the end")} disabled={atEnd} onClick={() => onGoTo(last)}>
        ⏭
      </button>
      <input
        type="range"
        data-testid={replayTestid.scrubber}
        aria-label={t("Step")}
        min={0}
        max={last}
        step={1}
        value={scrub ?? at}
        onPointerDown={() => {
          onPlaying(false);
          setScrub(at);
        }}
        onChange={(event) => {
          const value = Number(event.currentTarget.value);
          if (scrub === null) onGoTo(value);
          else setScrub(value);
        }}
      />
      <span className="replay-readout" data-testid={replayTestid.readout}>
        {readout}
      </span>
      {actions}
    </div>
  );
}
