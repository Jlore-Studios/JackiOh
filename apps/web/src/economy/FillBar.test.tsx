// The bar that fills (#1658): drawn at its value without a start, the sweeps, the burst and the fill
// reported when it bursts, the rest at the server's value, reduced motion under either switch, and
// the stylesheet that holds the motion to transform and opacity behind one gate. Fake timers move
// the bar; nothing here waits in real time.

import { act, cleanup, render, screen } from "@testing-library/react";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import { resetFxSettingsForTests } from "../fx/settings.ts";
import { REDUCED_MOTION_QUERY } from "../game/animations.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { BURST_MS, segmentMs } from "./bars.ts";
import FillBar from "./FillBar.tsx";

beforeEach(() => {
  vi.useFakeTimers();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  window.localStorage.clear();
});

const ID = "bar";

/** The sweep from 18 to the top of a bar of 20, and from 0 to 3. */
const TO_FULL_MS = segmentMs({ from: 18, to: 20, burst: true }, 20);
const TO_VALUE_MS = segmentMs({ from: 0, to: 3, burst: false }, 20);

function bar(): HTMLElement {
  return screen.getByTestId(ID);
}

function progress(): HTMLProgressElement {
  return bar().querySelector("progress")!;
}

function fillTransform(): string {
  return bar().querySelector<HTMLElement>(".fill-bar__fill")!.style.transform;
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** The system's reduced-motion switch, on for the test. */
function systemReducesMotion(): void {
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        media: query,
        matches: query === REDUCED_MOTION_QUERY,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
      }) as unknown as MediaQueryList,
  );
}

describe("#1658 a bar drawn at its value", () => {
  it("without a start it rests at the value, its caption {value} / {size}, and never moves", () => {
    const onFill = vi.fn<(fill: number) => void>();
    const onDone = vi.fn<() => void>();
    render(<FillBar label="Card token bar" size={20} value={7} testid={ID} onFill={onFill} onDone={onDone} />);

    expect(bar()).toHaveTextContent("Card token bar");
    expect(bar()).toHaveTextContent("7 / 20");
    expect(progress().value).toBe(7);
    expect(progress().max).toBe(20);
    expect(progress()).toHaveAccessibleName("Card token bar");
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(fillTransform()).toBe("scaleX(0.35)");
    expect(bar().querySelector(".fill-bar__fill--sweep")).toBeNull();

    advance(60_000);
    expect(onFill).not.toHaveBeenCalled();
    expect(onDone).not.toHaveBeenCalled();
    expect(fillTransform()).toBe("scaleX(0.35)");
  });

  it("a caption replaces the default words", () => {
    render(<FillBar label="Level" size={100} value={40} caption="60 to go" testid={ID} />);
    expect(bar()).toHaveTextContent("60 to go");
    expect(bar()).not.toHaveTextContent("40 / 100");
    expect(progress().value).toBe(40);
  });

  it("an empty bar and a full one are drawn at 0 and at the whole track", () => {
    const { rerender } = render(<FillBar label="Bar" size={20} value={0} testid={ID} />);
    expect(fillTransform()).toBe("scaleX(0)");
    rerender(<FillBar label="Bar" size={20} value={20} testid={ID} />);
    expect(fillTransform()).toBe("scaleX(1)");
  });
});

describe("#1658 a bar that fills", () => {
  it("from 18 to 3 of 20 with one fill, it bursts once, reports the fill and rests at 3", () => {
    const onFill = vi.fn<(fill: number) => void>();
    const onDone = vi.fn<() => void>();
    render(<FillBar label="Card token bar" size={20} value={3} from={18} fills={1} testid={ID} onFill={onFill} onDone={onDone} />);

    // The native progress rests at the server's value from the start: the picture moves, it does not.
    expect(bar()).toHaveAttribute("data-state", "filling");
    expect(bar()).toHaveAttribute("data-bursts", "0");
    expect(progress().value).toBe(3);
    expect(fillTransform()).toBe("scaleX(0.9)");

    advance(TO_FULL_MS - 1);
    expect(onFill).not.toHaveBeenCalled();
    expect(bar()).toHaveAttribute("data-state", "filling");

    advance(1);
    expect(onFill).toHaveBeenCalledTimes(1);
    expect(onFill).toHaveBeenLastCalledWith(1);
    expect(bar()).toHaveAttribute("data-state", "bursting");
    expect(bar()).toHaveAttribute("data-bursts", "1");
    expect(bar().querySelectorAll(".fill-bar__burst")).toHaveLength(1);
    expect(fillTransform()).toBe("scaleX(1)");
    expect(onDone).not.toHaveBeenCalled();

    // The burst holds, then the bar restarts at 0 and sweeps to the value.
    advance(BURST_MS);
    expect(bar()).toHaveAttribute("data-state", "filling");
    expect(bar().querySelector(".fill-bar__burst")).toBeNull();
    expect(fillTransform()).toBe("scaleX(0)");
    expect(onDone).not.toHaveBeenCalled();

    advance(TO_VALUE_MS);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(onFill).toHaveBeenCalledTimes(1);
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(bar()).toHaveAttribute("data-bursts", "1");
    expect(progress().value).toBe(3);
    expect(fillTransform()).toBe("scaleX(0.15)");

    advance(60_000);
    expect(onFill).toHaveBeenCalledTimes(1);
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("each of several fills bursts and is reported in turn, and the bar is done after the last", () => {
    const calls: string[] = [];
    render(
      <FillBar
        label="Bar"
        size={20}
        value={3}
        from={18}
        fills={2}
        testid={ID}
        onFill={(fill) => calls.push(`fill ${String(fill)}`)}
        onDone={() => calls.push("done")}
      />,
    );

    advance(TO_FULL_MS);
    expect(calls).toEqual(["fill 1"]);
    advance(BURST_MS);
    advance(segmentMs({ from: 0, to: 20, burst: true }, 20));
    expect(calls).toEqual(["fill 1", "fill 2"]);
    expect(bar()).toHaveAttribute("data-state", "bursting");
    advance(BURST_MS);
    advance(TO_VALUE_MS);
    expect(calls).toEqual(["fill 1", "fill 2", "done"]);
    expect(bar()).toHaveAttribute("data-bursts", "2");
  });

  it("a bar that paid nothing sweeps from where it was to where it is and bursts never", () => {
    const onFill = vi.fn<(fill: number) => void>();
    const onDone = vi.fn<() => void>();
    render(<FillBar label="Bar" size={20} value={12} from={5} testid={ID} onFill={onFill} onDone={onDone} />);
    expect(fillTransform()).toBe("scaleX(0.25)");

    advance(segmentMs({ from: 5, to: 12, burst: false }, 20));
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(onFill).not.toHaveBeenCalled();
    expect(fillTransform()).toBe("scaleX(0.6)");
  });

  it("a new answer plays from its own start, and an unmounted bar reports nothing more", () => {
    const onFill = vi.fn<(fill: number) => void>();
    const { rerender, unmount } = render(
      <FillBar label="Bar" size={20} value={3} from={18} fills={1} testid={ID} onFill={onFill} />,
    );
    advance(TO_FULL_MS);
    expect(onFill).toHaveBeenCalledTimes(1);

    rerender(<FillBar label="Bar" size={20} value={4} from={19} fills={1} testid={ID} onFill={onFill} />);
    expect(bar()).toHaveAttribute("data-state", "filling");
    expect(bar()).toHaveAttribute("data-bursts", "0");
    advance(segmentMs({ from: 19, to: 20, burst: true }, 20));
    expect(onFill).toHaveBeenCalledTimes(2);

    unmount();
    advance(60_000);
    expect(onFill).toHaveBeenCalledTimes(2);
  });

  it("calls the latest callbacks, not the ones it first rendered with", () => {
    const first = vi.fn<(fill: number) => void>();
    const latest = vi.fn<(fill: number) => void>();
    const { rerender } = render(<FillBar label="Bar" size={20} value={3} from={18} fills={1} testid={ID} onFill={first} />);
    rerender(<FillBar label="Bar" size={20} value={3} from={18} fills={1} testid={ID} onFill={latest} />);
    advance(TO_FULL_MS);
    expect(first).not.toHaveBeenCalled();
    expect(latest).toHaveBeenCalledWith(1);
  });
});

describe("#1658 reduced motion", () => {
  /** Whichever switch is on, the end state is drawn at once and every callback has run. */
  function expectEndStateAtOnce(onFill: Mock<(fill: number) => void>, onDone: Mock<() => void>): void {
    expect(progress().value).toBe(3);
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(bar()).toHaveAttribute("data-bursts", "1");
    expect(bar().querySelector(".fill-bar__fill--sweep")).toBeNull();
    expect(bar().querySelector(".fill-bar__burst")).toBeNull();
    expect(fillTransform()).toBe("scaleX(0.15)");
    expect(onFill).toHaveBeenCalledTimes(1);
    expect(onFill).toHaveBeenCalledWith(1);
    expect(onDone).toHaveBeenCalledTimes(1);
  }

  it("the settings' Reduce motion shows 3 at once, with onFill and onDone called", () => {
    writeSettings({ reduceMotion: true });
    const onFill = vi.fn<(fill: number) => void>();
    const onDone = vi.fn<() => void>();
    render(<FillBar label="Bar" size={20} value={3} from={18} fills={1} testid={ID} onFill={onFill} onDone={onDone} />);

    expectEndStateAtOnce(onFill, onDone);
    advance(60_000);
    expectEndStateAtOnce(onFill, onDone);
  });

  it("the system's reduced motion shows 3 at once, with onFill and onDone called", () => {
    systemReducesMotion();
    const onFill = vi.fn<(fill: number) => void>();
    const onDone = vi.fn<() => void>();
    render(<FillBar label="Bar" size={20} value={3} from={18} fills={1} testid={ID} onFill={onFill} onDone={onDone} />);

    expectEndStateAtOnce(onFill, onDone);
  });

  it("every fill is reported once, in order, even when the page renders twice", () => {
    writeSettings({ reduceMotion: true });
    const calls: string[] = [];
    const { rerender } = render(
      <StrictMode>
        <FillBar
          label="Bar"
          size={20}
          value={3}
          from={18}
          fills={3}
          testid={ID}
          onFill={(fill) => calls.push(`fill ${String(fill)}`)}
          onDone={() => calls.push("done")}
        />
      </StrictMode>,
    );
    rerender(
      <StrictMode>
        <FillBar
          label="Bar"
          size={20}
          value={3}
          from={18}
          fills={3}
          testid={ID}
          onFill={(fill) => calls.push(`fill ${String(fill)}`)}
          onDone={() => calls.push("done")}
        />
      </StrictMode>,
    );

    expect(calls).toEqual(["fill 1", "fill 2", "fill 3", "done"]);
  });

  it("a switch turned on while the bar fills finishes it at once, reporting the fills still to come", () => {
    const onFill = vi.fn<(fill: number) => void>();
    const onDone = vi.fn<() => void>();
    render(<FillBar label="Bar" size={20} value={3} from={18} fills={2} testid={ID} onFill={onFill} onDone={onDone} />);
    advance(TO_FULL_MS);
    expect(onFill).toHaveBeenCalledTimes(1);

    act(() => {
      writeSettings({ reduceMotion: true });
    });

    expect(onFill).toHaveBeenCalledTimes(2);
    expect(onFill).toHaveBeenLastCalledWith(2);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(fillTransform()).toBe("scaleX(0.15)");
    advance(60_000);
    expect(onFill).toHaveBeenCalledTimes(2);
    expect(onDone).toHaveBeenCalledTimes(1);
  });
});

/* ------------------------------------------------------------------------ the stylesheet --- */

type Block = { prelude: string; body: string };

/** The top-level `prelude { body }` blocks of a stylesheet's comment-free text. */
function blocks(text: string): Block[] {
  const out: Block[] = [];
  let depth = 0;
  let start = 0;
  let open = 0;
  for (let at = 0; at < text.length; at += 1) {
    const char = text[at];
    if (char === "{") {
      if (depth === 0) open = at;
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        out.push({ prelude: text.slice(start, open).trim(), body: text.slice(open + 1, at) });
        start = at + 1;
      }
    }
  }
  return out;
}

/** The property names a declaration list holds. */
function properties(body: string): string[] {
  return body
    .split(";")
    .map((declaration) => declaration.split(":")[0]!.trim())
    .filter((name) => name !== "");
}

/** The stylesheet as text: vitest stubs CSS imports, so the file is read from the test's working directory. */
const sheet: string = (() => {
  for (const candidate of ["src/economy/economy.css", "apps/web/src/economy/economy.css"]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  }
  throw new Error(`economy.css not found from ${process.cwd()}`);
})();
const top = blocks(sheet);
const keyframes = top.filter((block) => block.prelude.startsWith("@keyframes"));
const gates = top.filter((block) => block.prelude.startsWith("@media"));
const rules = top.filter((block) => !block.prelude.startsWith("@"));

const GATE = "@media (prefers-reduced-motion: no-preference)";
const NOT_REDUCED = ':root:not([data-reduce-motion="true"])';

describe("#1658 economy.css", () => {
  it("its keyframes animate only transform and opacity", () => {
    expect(keyframes.map((block) => block.prelude)).toEqual([
      "@keyframes fill-bar-sweep",
      "@keyframes fill-bar-burst",
      "@keyframes reward-reveal-fly",
    ]);
    for (const block of keyframes) {
      const animated = blocks(block.body).flatMap((stop) => properties(stop.body));
      expect(animated.length, block.prelude).toBeGreaterThan(0);
      for (const name of animated) expect(["transform", "opacity"], `${block.prelude}: ${name}`).toContain(name);
    }
  });

  it("reduce motion stills them: every animation sits in one gate of both switches, and no rule outside moves", () => {
    for (const rule of rules) {
      for (const name of properties(rule.body)) {
        expect(name, `${rule.prelude}: ${name}`).not.toMatch(/^(animation|transition)/);
      }
    }
    expect(gates.map((gate) => gate.prelude)).toEqual([GATE]);

    const gated = blocks(gates[0]!.body);
    const used = new Set<string>();
    for (const rule of gated) {
      expect(rule.prelude.startsWith(NOT_REDUCED), rule.prelude).toBe(true);
      const animation = /animation:\s*([\w-]+)/.exec(rule.body);
      expect(animation, rule.prelude).not.toBeNull();
      used.add(animation![1]!);
    }
    // Every keyframes is played somewhere in the gate, and only there.
    expect([...used].sort()).toEqual(keyframes.map((block) => block.prelude.replace("@keyframes ", "")).sort());
  });

  it("the animation lengths come from the components' constants, not from numbers in the sheet", () => {
    const gated = blocks(gates[0]!.body);
    for (const rule of gated) {
      expect(/animation:\s*[\w-]+\s+var\(--[\w-]+-ms\)/.test(rule.body), rule.prelude).toBe(true);
    }
  });
});
