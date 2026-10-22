// The bar that fills (#1658, part of #1651, SPEC §14.5): a track and a fill, shared by the token
// bars, the pack opening's two-bar animation and the store's and XP's bars.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). `size` and `value` are the server's, and so is `fills`; the
// bar only plays them. Without `from` it is drawn at `value` and never moves. With `from` it sweeps
// to `size`, bursts, reports the fill, restarts at 0 for each further fill, then sweeps to `value`
// and reports it is done (`bars.ts::segments` lays the sweeps out).
//
// THE MOTION is transform and opacity only: the fill is a `scaleX` (`economy.css` holds the
// keyframes, each taking its duration from `bars.ts`' constants through a custom property), and the
// timers here move the bar from one sweep to the next. REDUCED MOTION (the panel's setting or the
// system's) draws the end state at once and reports every fill, then `onDone`, straight away.
//
// A native `<progress>` always rests at `value` for screen readers (and the e2e specs), visually
// hidden: the track beside it is the picture, and it is hidden from them.

import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { useFxSettings } from "../fx/settings.ts";
import { reducedMotionNow } from "../game/animations.ts";
import { useOsReducedMotion } from "../game/useOsReducedMotion.ts";
import { useSetting } from "../settings/store.ts";
import { BURST_MS, fraction, segmentMs, segments } from "./bars.ts";
import "./economy.css";

export type FillBarProps = {
  /** The bar's name, already through `t()`. */
  label: string;
  /** What a full bar holds, the server's. */
  size: number;
  /** Where the bar rests, the server's. */
  value: number;
  /** Where the bar starts, when it is to play from there; without it the bar does not move. */
  from?: number;
  /** How many times the bar fills and restarts on its way from `from` to `value`; 0 when absent. */
  fills?: number;
  /** The small text beside the label; `{value} / {size}` when absent. */
  caption?: string;
  /** Called with the fill's number (from 1) as the bar bursts. */
  onFill?: (fill: number) => void;
  /** Called once the bar rests at `value`. */
  onDone?: () => void;
  testid?: string;
};

/** Where the bar is in its plan: a sweep, the burst that ends a full one, or at rest. */
type Position = { step: number; phase: "sweep" | "burst" | "done" };

/** Whether motion is off: the panel's switch, the system's (followed live) or the effects' own. */
export function useReducedMotion(): boolean {
  const [fx] = useFxSettings();
  const panel = useSetting("reduceMotion");
  const system = useOsReducedMotion();
  return panel || system || reducedMotionNow(fx);
}

/** A bar's sweep, as the custom properties `economy.css`' keyframes read. */
function sweepStyle(from: number, to: number, ms: number): CSSProperties {
  return {
    transform: `scaleX(${String(from)})`,
    "--fill-from": from,
    "--fill-to": to,
    "--fill-ms": `${String(ms)}ms`,
  } as CSSProperties;
}

/**
 * The bar. A new `size`, `value`, `from` or `fills` starts it afresh: a bar given a new answer plays
 * that answer, not the rest of the last one.
 */
export default function FillBar(props: FillBarProps): ReactElement {
  const { size, value, from, fills = 0 } = props;
  return <Bar key={`${String(size)}:${String(value)}:${String(from ?? "-")}:${String(fills)}`} {...props} />;
}

function Bar({ label, size, value, from, fills = 0, caption, onFill, onDone, testid }: FillBarProps): ReactElement {
  const reduced = useReducedMotion();
  const plan = useMemo(() => (from === undefined ? [] : segments(size, value, from, fills)), [size, value, from, fills]);
  const [position, setPosition] = useState<Position>({ step: 0, phase: from === undefined ? "done" : "sweep" });
  const told = useRef(0);
  const finished = useRef(false);
  const callbacks = useRef({ onFill, onDone });
  useEffect(() => {
    callbacks.current = { onFill, onDone };
  });

  // Motion off: every fill reported and the bar done at once, once however often it re-renders.
  useEffect(() => {
    if (from === undefined || !reduced) return;
    for (let fill = told.current + 1; fill < plan.length; fill += 1) {
      told.current = fill;
      callbacks.current.onFill?.(fill);
    }
    if (!finished.current) {
      finished.current = true;
      callbacks.current.onDone?.();
    }
    setPosition({ step: plan.length - 1, phase: "done" });
  }, [from, reduced, plan]);

  // Motion on: each sweep and each burst waits its time, then the bar moves to the next.
  useEffect(() => {
    if (from === undefined || reduced || position.phase === "done") return undefined;
    const segment = plan[position.step]!;
    const wait = position.phase === "sweep" ? segmentMs(segment, size) : BURST_MS;
    const timer = window.setTimeout(() => {
      if (position.phase === "burst") {
        setPosition({ step: position.step + 1, phase: "sweep" });
      } else if (segment.burst) {
        told.current = Math.max(told.current, position.step + 1);
        callbacks.current.onFill?.(position.step + 1);
        setPosition({ step: position.step, phase: "burst" });
      } else {
        finished.current = true;
        setPosition({ step: position.step, phase: "done" });
        callbacks.current.onDone?.();
      }
    }, wait);
    return () => {
      window.clearTimeout(timer);
    };
  }, [from, reduced, plan, position, size]);

  // The segment on screen; none when the bar is at rest, from the start or at the end.
  const segment = from === undefined || reduced || position.phase === "done" ? undefined : plan[position.step];
  const sweeping = segment !== undefined && position.phase === "sweep";
  const bursting = segment !== undefined && position.phase === "burst";
  const bursts = segment === undefined ? Math.max(0, plan.length - 1) : position.step + (bursting ? 1 : 0);

  const fillStyle: CSSProperties = sweeping
    ? sweepStyle(fraction(segment.from, size), fraction(segment.to, size), segmentMs(segment, size))
    : { transform: `scaleX(${String(fraction(bursting ? segment.to : value, size))})` };

  return (
    <div
      className="fill-bar"
      data-testid={testid}
      data-state={sweeping ? "filling" : bursting ? "bursting" : "resting"}
      data-bursts={bursts}
    >
      <span className="fill-bar__label">{label}</span>
      <div className="fill-bar__track" aria-hidden="true">
        <div className="fill-bar__well">
          <div key={position.step} className={sweeping ? "fill-bar__fill fill-bar__fill--sweep" : "fill-bar__fill"} style={fillStyle} />
        </div>
        {bursting ? (
          <span
            key={position.step}
            className="fill-bar__burst"
            style={{ "--burst-ms": `${String(BURST_MS)}ms` } as CSSProperties}
          />
        ) : null}
      </div>
      <progress className="fill-bar__progress" max={size} value={value} aria-label={label} />
      <span className="fill-bar__caption">{caption ?? `${String(value)} / ${String(size)}`}</span>
    </div>
  );
}
