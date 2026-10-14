// The replay player (#511, SPEC §9.3, §10.8, R768): a finished game, a step at a time. It reads a
// `ReplaySource` and nothing else (CLAUDE.md rule 7), draws each step's view with `Game` as a board
// to look at (`replay`: no legal action, glow or prompt, and a click changes nothing) and holds the
// controls in the board's notice slot.
//
// A step forward hands `Game` the next view with the same key, so its animation queue, effects and
// sound run as live play's do (R201's speed included). Any other move, a step back or a jump, gives
// `Game` a fresh key: the board mounts on that view with nothing animating, and its log history,
// which reads a window that does not overlap the last as a gap, starts over (`useLogHistory`). The
// last step is `Game`'s own result chip, with both hands (R434).
//
// R768000: a result screen opens it at step 0 ("Watch the replay") or at the last step with the
// last turn's GIF already being made ("Share the last turn").

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactElement } from "react";

import type { ReplayStep } from "@jackioh/engine";
import type { ActionBody } from "@jackioh/shared";

import { useFxSettings } from "../fx/settings.ts";
import { scaleForSpeed } from "../game/animations.ts";
import Game from "../game/Game.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { REPLAY_PLAY_GAP_MS } from "./config.ts";
import { ReplayFailure, replayFailureOf, replayFailureText } from "./source.ts";
import type { ReplaySource } from "./source.ts";
import { ShareGif, useGifExport } from "./ShareGif.tsx";
import { createStepCache } from "./steps.ts";
import "./replay.css";

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
} as const;

export type ReplayPlayerProps = {
  /** One replay. A different replay is a different player: the route keys this by what it opens. */
  source: ReplaySource;
  /** Back: the route's own way out. */
  onLeave: () => void;
  /** R768000: the step it opens at: the first (the default) or the last. */
  start?: "first" | "last";
  /** R768000: once the opening step is on the board, open "Share as GIF" and make its turn's GIF. */
  shareOnOpen?: boolean;
};

/** The step on the board, and which mount of the board shows it (`Game`'s key). */
type Shown = { step: ReplayStep; board: number };

/** A board to look at is offered nothing, and sends nothing. */
const NOTHING_LEGAL: readonly ActionBody[] = [];
const IGNORE = (): void => undefined;

/** What marks the board as still playing something: an animation's entry, or a voice line (R203). */
const BUSY = "[data-animating], [data-speaking]";

/**
 * Whether the board under `root` is animating or speaking. It reads the board's own marks, as
 * practice's pacing does (`useBoardBusy` in `routes/practice.tsx`), so Play never starts a step over
 * the last one's animation.
 */
function useBoardBusy(root: HTMLElement | null): boolean {
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (root === null || typeof MutationObserver !== "function") return;
    const report = (): void => {
      setBusy(root.querySelector(BUSY) !== null);
    };
    const observer = new MutationObserver(report);
    observer.observe(root, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["data-animating", "data-speaking"],
    });
    report();
    return () => {
      observer.disconnect();
    };
  }, [root]);
  return busy;
}

/** A key the arrow keys must leave to its own control: a text field, a select, the scrubber. */
function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")
  );
}

/** The control inside the player that has focus, by test id: a fresh board takes its element away. */
function focusedControl(): string | null {
  const active = document.activeElement;
  if (!(active instanceof HTMLElement) || active.closest(`[data-testid="${replayTestid.player}"]`) === null) return null;
  return active.dataset.testid ?? null;
}

export default function ReplayPlayer({ source, onLeave, start = "first", shareOnOpen = false }: ReplayPlayerProps): ReactElement {
  useLocale();
  const cache = useMemo(() => createStepCache(source), [source]);
  const total = cache.total;
  const last = Math.max(0, total - 1);
  const [at, setAt] = useState<Shown | null>(null);
  const [failure, setFailure] = useState<ReplayFailure | null>(null);
  const [playing, setPlaying] = useState(false);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
  /** Where the scrubber is while it is held: the board moves when it is let go, not at every tick. */
  const [scrub, setScrub] = useState<number | null>(null);
  /** The newest move asked for: an answer from an older one (a slow page) is dropped. */
  const wanted = useRef(0);
  /** The step on the board as the newest move left it, which a held-back answer has not reached yet. */
  const shownAt = useRef<number | null>(null);
  /** The control that had focus when the board was mounted afresh, to give it back to the new one. */
  const refocus = useRef<string | null>(null);
  const [fx] = useFxSettings();
  const busy = useBoardBusy(root);
  const current = at?.step.step ?? 0;
  const gif = useGifExport(cache, current);

  /** Shows step `index` when its page is in, now if it is already held. */
  const show = useCallback(
    (index: number): void => {
      wanted.current += 1;
      const ticket = wanted.current;
      const apply = (step: ReplayStep): void => {
        if (wanted.current !== ticket) return;
        // Only the next step carries the board on; any other is a fresh board that animates nothing.
        const forward = shownAt.current !== null && step.step === shownAt.current + 1;
        shownAt.current = step.step;
        if (!forward) refocus.current = focusedControl();
        setAt((prev) => ({ step, board: forward && prev !== null ? prev.board : (prev?.board ?? 0) + 1 }));
        if (step.step >= last) setPlaying(false);
        cache.prefetch(step.step);
      };
      const held = cache.get(index);
      if (held !== undefined) {
        apply(held);
        return;
      }
      cache.load(index).then(apply, (cause: unknown) => {
        if (wanted.current !== ticket) return;
        setPlaying(false);
        setFailure(replayFailureOf(cause));
      });
    },
    [cache, last],
  );

  /** A move the viewer made: Play stops, and the step is clamped to the replay. */
  function goTo(index: number): void {
    setPlaying(false);
    const target = Math.min(Math.max(index, 0), last);
    if (target !== current) show(target);
  }

  /** A turn move: the step is found first, which may fetch pages to see where the turn changes. */
  function goTurn(direction: 1 | -1): void {
    setPlaying(false);
    wanted.current += 1;
    const ticket = wanted.current;
    (direction === 1 ? cache.nextTurn(current) : cache.previousTurn(current)).then(
      (target) => {
        if (wanted.current === ticket && target !== current) show(target);
      },
      (cause: unknown) => {
        if (wanted.current === ticket) setFailure(replayFailureOf(cause));
      },
    );
  }

  // The scrubber let go, wherever the pointer is: the board goes to where it was left. Until then it
  // stays, since a board mounted afresh at every tick would take the scrubber from under the pointer.
  const dropScrub = useRef<() => void>(IGNORE);
  dropScrub.current = () => {
    if (scrub === null) return;
    setScrub(null);
    goTo(scrub);
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

  const keys = useRef<(event: KeyboardEvent) => void>(IGNORE);
  keys.current = (event) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    if (event.altKey || event.ctrlKey || event.metaKey || event.defaultPrevented || isTyping(event.target)) return;
    event.preventDefault();
    const direction = event.key === "ArrowRight" ? 1 : -1;
    if (event.shiftKey) goTurn(direction);
    else goTo(current + direction);
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

  const opening = start === "last" ? last : 0;
  useEffect(() => {
    show(opening);
  }, [show, opening]);

  // "Share the last turn": once, when the opening step is the one on the board, the GIF of its turn.
  const shared = useRef(false);
  const onOpening = at !== null && at.step.step === opening;
  useEffect(() => {
    if (!shareOnOpen || shared.current || !onOpening) return;
    shared.current = true;
    if (!gif.open) gif.toggle();
    gif.make();
  }, [shareOnOpen, onOpening, gif]);

  // A fresh board is new elements, so the control that was pressed has gone: focus its successor.
  const board = at?.board ?? 0;
  useLayoutEffect(() => {
    const id = refocus.current;
    refocus.current = null;
    if (id !== null) root?.querySelector<HTMLElement>(`[data-testid="${id}"]`)?.focus({ preventScroll: true });
  }, [board, root]);

  // Play: the next step comes a gap after the board has settled, the gap shortened by the table's
  // speed (R201), and any move or a failure stops it.
  useEffect(() => {
    if (!playing || busy || at === null || current >= last) return;
    const timer = window.setTimeout(() => {
      show(current + 1);
    }, scaleForSpeed(REPLAY_PLAY_GAP_MS, fx.speed));
    return () => {
      window.clearTimeout(timer);
    };
  }, [playing, busy, at, current, last, fx.speed, show]);

  const back = (
    <button type="button" className="link-button" data-testid={replayTestid.back} onClick={onLeave}>
      {t("← Back")}
    </button>
  );

  if (failure !== null) {
    return (
      <div className="app-shell">
        {back}
        <p className="notice" role="alert" data-testid={replayTestid.failure}>
          {replayFailureText(failure)}
        </p>
      </div>
    );
  }
  if (at === null) {
    return (
      <div className="app-shell">
        {back}
        <p className="notice" role="status" data-testid={replayTestid.loading}>
          {t("Loading the replay…")}
        </p>
      </div>
    );
  }

  const atStart = current <= 0;
  const atEnd = current >= last;
  const controls = (
    <div className="replay-controls" role="toolbar" aria-label={t("Replay controls")} data-testid={replayTestid.controls}>
      <button type="button" data-testid={replayTestid.start} aria-label={t("To the start")} disabled={atStart} onClick={() => goTo(0)}>
        ⏮
      </button>
      <button type="button" data-testid={replayTestid.backTurn} aria-label={t("Back a turn")} disabled={atStart} onClick={() => goTurn(-1)}>
        ⏪
      </button>
      <button type="button" data-testid={replayTestid.backStep} aria-label={t("Back a step")} disabled={atStart} onClick={() => goTo(current - 1)}>
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
      <button type="button" data-testid={replayTestid.forwardStep} aria-label={t("Forward a step")} disabled={atEnd} onClick={() => goTo(current + 1)}>
        ›
      </button>
      <button type="button" data-testid={replayTestid.forwardTurn} aria-label={t("Forward a turn")} disabled={atEnd} onClick={() => goTurn(1)}>
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
        value={scrub ?? current}
        onPointerDown={() => {
          setPlaying(false);
          setScrub(current);
        }}
        onChange={(event) => {
          const value = Number(event.currentTarget.value);
          if (scrub === null) goTo(value);
          else setScrub(value);
        }}
      />
      <span className="replay-readout" data-testid={replayTestid.readout}>
        {t("Step {step} of {steps} · Turn {turn}", { step: current + 1, steps: total, turn: at.step.turn })}
      </span>
    </div>
  );

  return (
    <div
      className="app-shell app-shell--wide replay-player"
      ref={setRoot}
      data-testid={replayTestid.player}
      data-step={current}
      data-board={at.board}
    >
      <Game
        key={at.board}
        view={at.step.view}
        legal={NOTHING_LEGAL}
        onAction={IGNORE}
        replay
        resultForm="chip"
        boardRail={back}
        boardNotice={
          <>
            {controls}
            <ShareGif gif={gif} />
          </>
        }
      />
    </div>
  );
}
