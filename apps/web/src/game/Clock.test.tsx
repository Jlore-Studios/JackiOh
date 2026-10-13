// BUILD M7-T1's client half. R79 fixes the behaviour and the actor runs it; these tests assert that the
// component RENDERS it rather than deciding it: a paused turn clock is `turnDeadline: null` in the frame,
// and remaining time is `deadline - now` against a monotonic delta (`protocol.rs`), never the wall clock.
// Every duration comes from `crates/server/src/config.rs`; no test spells a number of seconds.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import {
  DISCONNECT_GRACE_MS,
  MATCH_CEILING_MS,
  MULLIGAN_CLOCK_MS,
  PROMPT_CLOCK_MS,
  TURN_CLOCK_MS,
} from "@jackioh/server-config";
import Clock, {
  formatClock,
  readClock,
  turnKeyOf,
  useFrameFor,
  type ClockFrame,
  type ClockLine,
  type ClockProps,
} from "./Clock.tsx";
import { TURN_CLOCK_FINAL_MS, TURN_CLOCK_LAST_MS } from "./clockConstants.ts";
import { NO_URGENCY, fuseGeometry, readUrgency, turnClockUrgency } from "./clockUrgency.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";

afterEach(cleanup);

const NOW = 1_700_000_000_000;

function frame(over: Partial<ClockFrame["clocks"]> = {}, now = NOW): ClockFrame {
  return {
    now,
    clocks: {
      turnDeadline: now + TURN_CLOCK_MS,
      promptDeadline: null,
      graceDeadline: { p1: null, p2: null },
      ceilingAt: now + MATCH_CEILING_MS,
      ...over,
    },
  };
}

/** A monotonic source the test drives by hand, so no timer is involved. */
function monotonicAt(value: { ms: number }): () => number {
  return () => value.ms;
}

function props(over: Partial<ClockProps> = {}): ClockProps {
  return { youMs: null, opponentMs: null, viewer: "p1", activePlayer: "p1", ...over };
}

// R79: whose clock is running

describe("R79 — the turn clock belongs to the active player", () => {
  it("counts down for the active player and is idle for the other", () => {
    const readout = readClock(props({ frame: frame(), activePlayer: "p1", viewer: "p1" }), 0);
    expect(readout.you.kind).toBe("turn");
    expect(readout.you.remainingMs).toBe(TURN_CLOCK_MS);
    expect(readout.you.totalMs).toBe(TURN_CLOCK_MS);
    expect(readout.opponent.kind).toBe("idle");
    expect(readout.turn).toBe(readout.you);
  });

  it("follows the viewer's seat, not p1", () => {
    const readout = readClock(props({ frame: frame(), activePlayer: "p1", viewer: "p2" }), 0);
    expect(readout.opponent.kind).toBe("turn");
    expect(readout.you.kind).toBe("idle");
    expect(readout.turn).toBe(readout.opponent);
  });

  it("arms no prompt clock while nobody holds a prompt", () => {
    expect(readClock(props({ frame: frame() }), 0).prompt).toBe(null);
  });
});

describe("R79 — a prompt held by the non-active player pauses the turn clock", () => {
  const paused = props({
    // The server stopped the turn clock and armed the prompt's own: that is what the frame says.
    frame: frame({ turnDeadline: null, promptDeadline: NOW + PROMPT_CLOCK_MS }),
    activePlayer: "p1",
    promptHolder: "p2",
    viewer: "p1",
    youMs: 30_000,
    opponentMs: null,
  });

  it("shows the active player's clock as paused", () => {
    const readout = readClock(paused, 0);
    expect(readout.you.kind).toBe("turn");
    expect(readout.you.paused).toBe(true);
    // Paused means stopped, so what is left is whatever the last view carried.
    expect(readout.you.remainingMs).toBe(30_000);
  });

  it("shows the prompt holder's own clock, over PROMPT_CLOCK_MS", () => {
    const readout = readClock(paused, 0);
    expect(readout.opponent.kind).toBe("prompt");
    expect(readout.opponent.remainingMs).toBe(PROMPT_CLOCK_MS);
    expect(readout.opponent.totalMs).toBe(PROMPT_CLOCK_MS);
    expect(readout.prompt).toBe(readout.opponent);
  });

  it("does not run the paused clock down as time passes, and does run the prompt's", () => {
    const early = readClock(paused, 0);
    const later = readClock(paused, 5_000);
    expect(later.you.remainingMs).toBe(early.you.remainingMs);
    expect(later.opponent.remainingMs).toBe((early.opponent.remainingMs ?? 0) - 5_000);
  });

  it("does not pause when the prompt is the active player's own", () => {
    // R79 arms the separate clock only for a prompt held by the *non-active* player.
    const readout = readClock(
      props({
        frame: frame({ promptDeadline: NOW + PROMPT_CLOCK_MS }),
        activePlayer: "p1",
        promptHolder: "p1",
      }),
      0,
    );
    expect(readout.you.paused).toBe(false);
    expect(readout.you.kind).toBe("prompt");
  });

  it("decides nothing itself: no frame means no pause is claimed", () => {
    const readout = readClock(
      props({ frame: null, activePlayer: "p1", promptHolder: "p2", youMs: 30_000 }),
      0,
    );
    expect(readout.you.paused).toBe(false);
    expect(readout.you.remainingMs).toBe(30_000);
  });
});

// The monotonic delta

describe("remaining time is deadline - now against a monotonic delta", () => {
  it("subtracts the elapsed monotonic time from the server's own now", () => {
    const readout = readClock(props({ frame: frame() }), 12_000);
    expect(readout.you.remainingMs).toBe(TURN_CLOCK_MS - 12_000);
  });

  it("is unaffected by the browser's wall clock being wrong", () => {
    // `frame.now` is the server's clock; a client 10 minutes off still reads the same remainder.
    const skewed = frame({}, NOW + 600_000);
    expect(readClock(props({ frame: skewed }), 0).you.remainingMs).toBe(TURN_CLOCK_MS);
  });

  it("never shows a negative number of seconds", () => {
    expect(formatClock(-5_000)).toBe("0s");
    expect(formatClock(null)).toBe("—");
    expect(formatClock(1_400)).toBe("2s");
  });
});

// Grace and the ceiling (§9.5, R79)

describe("grace and the ceiling", () => {
  it("maps the frame's per-player grace onto the viewer's sides", () => {
    const readout = readClock(
      props({
        frame: frame({ graceDeadline: { p1: null, p2: NOW + DISCONNECT_GRACE_MS } }),
        viewer: "p1",
      }),
      0,
    );
    expect(readout.grace.you).toBe(null);
    expect(readout.grace.opponent).toBe(DISCONNECT_GRACE_MS);
  });

  it("falls back to the graceMs prop when no frame has arrived", () => {
    const readout = readClock(props({ frame: null, graceMs: { you: 1_000, opponent: null } }), 0);
    expect(readout.grace.you).toBe(1_000);
  });

  it("counts the wall-clock ceiling down from the frame", () => {
    expect(readClock(props({ frame: frame() }), 1_000).ceilingMs).toBe(MATCH_CEILING_MS - 1_000);
  });
});

// What reaches the DOM

describe("the rendered clock", () => {
  it("renders both sides, the turn clock and the ceiling", () => {
    const at = { ms: 0 };
    render(<Clock {...props({ frame: frame(), monotonic: monotonicAt(at) })} />);
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-kind", "turn");
    expect(screen.getByTestId("clock-opponent")).toHaveAttribute("data-kind", "idle");
    expect(screen.getByTestId("turn-clock")).toHaveAttribute("data-total-ms", String(TURN_CLOCK_MS));
    expect(screen.getByTestId("match-ceiling")).toHaveAttribute(
      "data-total-ms",
      String(MATCH_CEILING_MS),
    );
    expect(screen.queryByTestId("prompt-clock")).toBeNull();
    expect(screen.queryByTestId("grace-you")).toBeNull();
  });

  it("marks the turn clock paused and shows the prompt clock when R79 says so", () => {
    const at = { ms: 0 };
    render(
      <Clock
        {...props({
          frame: frame({ turnDeadline: null, promptDeadline: NOW + PROMPT_CLOCK_MS }),
          activePlayer: "p1",
          promptHolder: "p2",
          youMs: 30_000,
          monotonic: monotonicAt(at),
        })}
      />,
    );
    expect(screen.getByTestId("turn-clock")).toHaveAttribute("data-paused", "true");
    const prompt = screen.getByTestId("prompt-clock");
    expect(prompt).toHaveAttribute("data-side", "opponent");
    expect(prompt).toHaveAttribute("data-total-ms", String(PROMPT_CLOCK_MS));
  });

  it("renders grace-<side> only for the side that is actually away", () => {
    const at = { ms: 0 };
    render(
      <Clock
        {...props({
          frame: frame({ graceDeadline: { p1: NOW + DISCONNECT_GRACE_MS, p2: null } }),
          monotonic: monotonicAt(at),
        })}
      />,
    );
    const grace = screen.getByTestId("grace-you");
    expect(grace).toHaveAttribute("data-total-ms", String(DISCONNECT_GRACE_MS));
    expect(grace).toHaveAttribute("data-remaining-ms", String(DISCONNECT_GRACE_MS));
    expect(screen.queryByTestId("grace-opponent")).toBeNull();
  });

  it("still renders both sides from PlayerView.clockMs when no frame has arrived", () => {
    render(<Clock {...props({ youMs: 42_000, opponentMs: null, frame: null })} />);
    expect(screen.getByTestId("clock-you")).toHaveTextContent("42s");
    expect(screen.getByTestId("clock-opponent")).toHaveTextContent("—");
    expect(screen.queryByTestId("turn-clock")).toBeInTheDocument();
    expect(screen.queryByTestId("match-ceiling")).toBeNull();
  });

  it("ticks down as the monotonic clock advances, without a new frame", () => {
    const at = { ms: 0 };
    const pinned = props({ frame: frame(), monotonic: monotonicAt(at) });
    const view = render(<Clock {...pinned} />);
    expect(screen.getByTestId("clock-you")).toHaveTextContent(formatClock(TURN_CLOCK_MS));
    at.ms = 10_000;
    view.rerender(<Clock {...pinned} />);
    expect(screen.getByTestId("clock-you")).toHaveTextContent(formatClock(TURN_CLOCK_MS - 10_000));
  });
});

// R268: the mulligan window runs one clock for both seats

describe("R268 — while both mulligans are open, one mulligan clock for both seats", () => {
  /** The server's frame in the window: the turn clock paused, the mulligan deadline as the prompt's. */
  function mulliganFrame(): ClockFrame {
    return frame({ turnDeadline: null, promptDeadline: NOW + MULLIGAN_CLOCK_MS });
  }

  it("R268 both sides count the one deadline down over MULLIGAN_CLOCK_MS, and no turn clock runs", () => {
    const readout = readClock(props({ frame: mulliganFrame(), mulligan: true }), 5_000);
    for (const line of [readout.you, readout.opponent]) {
      expect(line.kind).toBe("mulligan");
      expect(line.remainingMs).toBe(MULLIGAN_CLOCK_MS - 5_000);
      expect(line.totalMs).toBe(MULLIGAN_CLOCK_MS);
      expect(line.paused).toBe(false);
    }
    expect(readout.turn, "setup is nobody's turn").toBeNull();
    expect(readout.prompt).toEqual({
      side: null,
      kind: "mulligan",
      remainingMs: MULLIGAN_CLOCK_MS - 5_000,
      totalMs: MULLIGAN_CLOCK_MS,
      paused: false,
    });
  });

  it("R268 the seat that has already answered sees the same countdown as the one still choosing", () => {
    // Answered: the view names the other seat as the one a prompt waits on.
    const ready = readClock(props({ frame: mulliganFrame(), mulligan: true, viewer: "p1", promptHolder: "p2" }), 0);
    const owing = readClock(props({ frame: mulliganFrame(), mulligan: true, viewer: "p2", promptHolder: "p2" }), 0);
    expect(ready.you).toEqual(owing.you);
    expect(ready.opponent).toEqual(owing.opponent);
    expect(ready.you.remainingMs).toBe(MULLIGAN_CLOCK_MS);
  });

  it("outside the window the same deadline is an ordinary prompt's, over PROMPT_CLOCK_MS", () => {
    const readout = readClock(props({ frame: mulliganFrame(), promptHolder: "p2" }), 0);
    expect(readout.prompt?.kind).toBe("prompt");
    expect(readout.prompt?.totalMs).toBe(PROMPT_CLOCK_MS);
  });

  it("R268 with no frame yet, both sides read PlayerView.clockMs as the mulligan clock", () => {
    const readout = readClock(props({ youMs: 30_000, opponentMs: 30_000, mulligan: true, frame: null }), 0);
    expect(readout.you).toEqual({ side: "you", kind: "mulligan", remainingMs: 30_000, totalMs: MULLIGAN_CLOCK_MS, paused: false });
    expect(readout.opponent.kind).toBe("mulligan");
    expect(readout.prompt).toBeNull();
  });

  it("R268 renders both sides and the prompt clock as the mulligan's, with no turn clock", () => {
    const at = { ms: 0 };
    render(<Clock {...props({ frame: mulliganFrame(), mulligan: true, monotonic: monotonicAt(at) })} />);
    for (const id of ["clock-you", "clock-opponent", "prompt-clock"]) {
      const line = screen.getByTestId(id);
      expect(line, id).toHaveAttribute("data-kind", "mulligan");
      expect(line, id).toHaveAttribute("data-total-ms", String(MULLIGAN_CLOCK_MS));
      expect(line, id).toHaveTextContent(formatClock(MULLIGAN_CLOCK_MS));
    }
    expect(screen.queryByTestId("turn-clock")).toBeNull();
  });
});

// R439: the last 30 seconds of a turn clock

describe("R439 — the last 30 seconds of a turn clock", () => {
  afterEach(() => {
    setReducedMotion(false);
    window.localStorage.clear();
    __resetSettingsForTests();
  });

  function turnLine(remainingMs: number | null, over: Partial<ClockLine> = {}): ClockLine {
    return { side: "you", kind: "turn", remainingMs, totalMs: TURN_CLOCK_MS, paused: false, ...over };
  }

  function finalFrame(ms: number): ClockFrame {
    return frame({ turnDeadline: NOW + ms });
  }

  it("R439 the thresholds: urgent from 30 seconds, sharper from 10, as the readout rounds them", () => {
    expect(turnClockUrgency(turnLine(TURN_CLOCK_FINAL_MS + 1))).toBe("none");
    expect(formatClock(TURN_CLOCK_FINAL_MS + 1), "one more millisecond still reads 31s").not.toBe(formatClock(TURN_CLOCK_FINAL_MS));
    expect(turnClockUrgency(turnLine(TURN_CLOCK_FINAL_MS))).toBe("final");
    expect(turnClockUrgency(turnLine(TURN_CLOCK_LAST_MS + 1))).toBe("final");
    expect(turnClockUrgency(turnLine(TURN_CLOCK_LAST_MS))).toBe("last10");
    expect(turnClockUrgency(turnLine(0))).toBe("last10");
    expect(turnClockUrgency(turnLine(-500)), "past the deadline, until the server ends the turn").toBe("last10");
    expect(TURN_CLOCK_FINAL_MS).toBeLessThan(TURN_CLOCK_MS);
    expect(TURN_CLOCK_LAST_MS).toBeLessThan(TURN_CLOCK_FINAL_MS);
  });

  it("R439 only a running turn clock has a final stretch: never a prompt's, the mulligan's, an idle side or no number", () => {
    const ms = TURN_CLOCK_LAST_MS;
    expect(turnClockUrgency(turnLine(ms, { kind: "prompt", totalMs: PROMPT_CLOCK_MS }))).toBe("none");
    expect(turnClockUrgency(turnLine(ms, { kind: "mulligan", totalMs: MULLIGAN_CLOCK_MS }))).toBe("none");
    expect(turnClockUrgency(turnLine(ms, { kind: "idle" }))).toBe("none");
    expect(turnClockUrgency(turnLine(null))).toBe("none");
    expect(turnClockUrgency(null)).toBe("none");

    const mulligan = readClock(props({ frame: frame({ turnDeadline: null, promptDeadline: NOW + ms }), mulligan: true }), 0);
    expect(readUrgency(mulligan)).toEqual(NO_URGENCY);
    render(
      <Clock
        {...props({
          frame: frame({ turnDeadline: null, promptDeadline: NOW + ms }),
          activePlayer: "p2",
          promptHolder: "p1",
          monotonic: monotonicAt({ ms: 0 }),
        })}
      />,
    );
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-kind", "prompt");
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-urgency", "none");
    expect(document.querySelector(".clock")).toHaveAttribute("data-clock-urgency", "none");
  });

  it("R439 a paused clock (a null turn deadline) shows no urgency, whatever was left on it", () => {
    const paused = props({
      frame: frame({ turnDeadline: null, promptDeadline: NOW + PROMPT_CLOCK_MS }),
      activePlayer: "p1",
      promptHolder: "p2",
      youMs: TURN_CLOCK_LAST_MS,
      monotonic: monotonicAt({ ms: 0 }),
    });
    const readout = readClock(paused, 0);
    expect(readout.you.paused).toBe(true);
    expect(readout.you.remainingMs).toBe(TURN_CLOCK_LAST_MS);
    expect(readUrgency(readout)).toEqual(NO_URGENCY);

    render(<Clock {...paused} />);
    const root = document.querySelector(".clock");
    expect(root).toHaveAttribute("data-clock-urgency", "none");
    expect(root).toHaveAttribute("data-clock-side", "");
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-urgency", "none");
    expect(screen.queryByTestId("turn-clock-fuse")).toBeNull();
  });

  it("R439 whose clock: the viewer's own final stretch is the urgent readout and the fuse", () => {
    const left = TURN_CLOCK_FINAL_MS - 8_000;
    render(<Clock {...props({ frame: finalFrame(left), activePlayer: "p1", viewer: "p1", monotonic: monotonicAt({ ms: 0 }) })} />);

    const root = document.querySelector(".clock");
    expect(root).toHaveAttribute("data-clock-urgency", "final");
    expect(root).toHaveAttribute("data-clock-side", "you");
    expect(root).toHaveAttribute("data-motion", "full");

    const line = screen.getByTestId("clock-you");
    expect(line).toHaveAttribute("data-urgency", "final");
    // Read by words and shape, not colour alone: a timer with a name, the words and the gauge.
    expect(line).toHaveAttribute("role", "timer");
    expect(line).toHaveAttribute("aria-label", `Your turn ends in ${String(left / 1000)} seconds`);
    expect(line).toHaveTextContent("Your turn");
    expect(line).toHaveTextContent(formatClock(left));
    expect(line.querySelector("svg.clock-gauge")).not.toBeNull();
    expect(screen.getByTestId("clock-opponent")).toHaveAttribute("data-urgency", "none");

    const fuse = screen.getByTestId("turn-clock-fuse");
    expect(fuse).toHaveAttribute("data-urgency", "final");
    expect(fuse).toHaveAttribute("aria-hidden", "true");
    expect(fuse.querySelectorAll(".clock-fuse-edge")).toHaveLength(4);
    expect(screen.getByTestId("turn-clock")).toHaveAttribute("data-urgency", "final");
  });

  it("R439 whose clock: the opponent's final stretch is the quieter readout, with no fuse", () => {
    const left = TURN_CLOCK_FINAL_MS - 1_000;
    render(<Clock {...props({ frame: finalFrame(left), activePlayer: "p2", viewer: "p1", monotonic: monotonicAt({ ms: 0 }) })} />);
    const root = document.querySelector(".clock");
    expect(root).toHaveAttribute("data-clock-urgency", "final");
    expect(root).toHaveAttribute("data-clock-side", "opponent");
    const line = screen.getByTestId("clock-opponent");
    expect(line).toHaveAttribute("data-urgency", "final");
    expect(line).toHaveTextContent("Their turn");
    expect(line).toHaveAttribute("aria-label", `Their turn ends in ${String(left / 1000)} seconds`);
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-urgency", "none");
    expect(screen.queryByTestId("turn-clock-fuse")).toBeNull();
  });

  it("R439 the stretch starts as the clock ticks into it, and sharpens in the last 10, with no new frame", () => {
    const at = { ms: 0 };
    const pinned = props({ frame: finalFrame(TURN_CLOCK_FINAL_MS + 2_000), activePlayer: "p1", monotonic: monotonicAt(at) });
    const view = render(<Clock {...pinned} />);
    const root = (): Element | null => document.querySelector(".clock");
    expect(root()).toHaveAttribute("data-clock-urgency", "none");
    expect(screen.queryByTestId("turn-clock-fuse")).toBeNull();

    at.ms = 2_000;
    view.rerender(<Clock {...pinned} />);
    expect(root()).toHaveAttribute("data-clock-urgency", "final");
    const fuse = screen.getByTestId("turn-clock-fuse");
    // At the start of the stretch the whole fuse is left.
    for (const edge of Array.from(fuse.querySelectorAll<HTMLElement>(".clock-fuse-edge"))) {
      expect(edge.style.getPropertyValue("--lit")).toBe("1");
    }

    at.ms = 2_000 + TURN_CLOCK_FINAL_MS - TURN_CLOCK_LAST_MS;
    view.rerender(<Clock {...pinned} />);
    expect(root()).toHaveAttribute("data-clock-urgency", "last10");
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-urgency", "last10");
    expect(screen.getByTestId("turn-clock-fuse")).toHaveAttribute("data-urgency", "last10");
    expect(screen.getByTestId("clock-you")).toHaveTextContent(formatClock(TURN_CLOCK_LAST_MS));
  });

  it("R439 under reduced motion (the panel's switch or the media query) nothing moves and the urgent readout stays", () => {
    for (const reduce of [() => writeSettings({ reduceMotion: true }), () => setReducedMotion(true)]) {
      reduce();
      render(<Clock {...props({ frame: finalFrame(TURN_CLOCK_LAST_MS), activePlayer: "p1", monotonic: monotonicAt({ ms: 0 }) })} />);
      const root = document.querySelector(".clock");
      expect(root).toHaveAttribute("data-motion", "reduced");
      expect(root).toHaveAttribute("data-clock-urgency", "last10");
      expect(screen.queryByTestId("turn-clock-fuse")).toBeNull();
      const line = screen.getByTestId("clock-you");
      expect(line).toHaveAttribute("data-urgency", "last10");
      expect(line).toHaveTextContent("Your turn");
      expect(line.querySelector("svg.clock-gauge")).not.toBeNull();
      cleanup();
      setReducedMotion(false);
      window.localStorage.clear();
      __resetSettingsForTests();
    }

    // clock.css: every motion of the stretch stops under both switches, and the fuse goes.
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "clock.css"), "utf8").replace(/\s+/g, " ");
    expect(css).toContain('.clock[data-motion="reduced"] *, .clock[data-motion="reduced"] .clock-line { animation: none !important; transition: none !important; }');
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\) \{ \.clock \*, \.clock \.clock-line \{ animation: none !important; transition: none !important; \} \.clock-fuse \{ display: none; \}/);
    expect(css).toContain(':root[data-reduce-motion="true"] .clock-fuse, :root[data-reduce-motion="true"] .hand-you::before { display: none; }');
    // The heartbeat only runs under full motion, and the fuse never takes the pointer.
    expect(css).toContain('.clock[data-motion="full"][data-clock-side="you"]');
    expect(css).toMatch(/\.clock-fuse \{ position: fixed; inset: 0; z-index: 37; pointer-events: none; \}/);
  });

  it("R439 the fuse burns clockwise from the top-left corner, one edge per quarter", () => {
    expect(fuseGeometry(1)).toEqual({ edges: [1, 1, 1, 1], spark: { x: 0, y: 0 } });
    const half = fuseGeometry(0.5);
    expect(half.edges).toEqual([0, 0, 1, 1]);
    expect(half.spark, "half burnt: at the bottom-right corner").toEqual({ x: 100, y: 100 });
    const eighth = fuseGeometry(7 / 8);
    expect(eighth.edges).toEqual([0.5, 1, 1, 1]);
    expect(eighth.spark).toEqual({ x: 50, y: 0 });
    expect(fuseGeometry(3 / 8).spark).toEqual({ x: 50, y: 100 });
    expect(fuseGeometry(1 / 8).spark).toEqual({ x: 0, y: 50 });
    expect(fuseGeometry(0).edges).toEqual([0, 0, 0, 0]);
    expect(fuseGeometry(2).edges, "clamped").toEqual([1, 1, 1, 1]);
  });

  it("R439 a turn clock no seat holds warns nobody", () => {
    // A frame with a deadline but no active player named (the readout's `side: null`).
    const readout = readClock(props({ frame: finalFrame(TURN_CLOCK_LAST_MS), activePlayer: null }), 0);
    expect(readout.turn?.side).toBeNull();
    expect(readUrgency(readout)).toEqual(NO_URGENCY);
  });

  it("R439 hands on a frame only while the view is on the turn it arrived in", () => {
    const first = finalFrame(TURN_CLOCK_LAST_MS);
    const next = finalFrame(TURN_CLOCK_MS);
    type HookProps = { f: ClockFrame | null; key: string | null };
    const start: HookProps = { f: first, key: "p1:3" };
    const { result, rerender } = renderHook(({ f, key }: HookProps) => useFrameFor(f, key), { initialProps: start });
    expect(result.current).toBe(first);
    // The next turn's view lands before its frame: the last turn's deadline is not read against it.
    rerender({ f: first, key: "p2:4" });
    expect(result.current).toBeNull();
    rerender({ f: next, key: "p2:4" });
    expect(result.current).toBe(next);
    rerender({ f: null, key: "p2:4" });
    expect(result.current).toBeNull();
    expect(turnKeyOf({ active: "p2", turn: 4, result: null })).toBe("p2:4");
    expect(turnKeyOf({ active: "p2", turn: 4, result: { winner: "p1" } })).toBeNull();
    expect(turnKeyOf(null)).toBeNull();
  });
});
