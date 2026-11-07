// R3348001 (#3348): corner play and a tile's answer are the viewer's own. They never reach the net layer or
// the log, never send an action, and the net layer and the replay never reach them.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "../../audio/engine.ts";
import { fakeAudioEngine } from "../../audio/test/fakeAudio.ts";
import { fullBoardView } from "../../test/fixtures.ts";
import { testid } from "../contract.ts";
import Game from "../Game.tsx";
import { CORNER_ESCALATE_TAPS } from "./config.ts";
import { setArenaRandomForTests } from "./play.ts";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.(ts|tsx)$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
  });
}

const read = (path: string): string => readFileSync(path, "utf8");
const importsOf = (code: string): string[] => [...code.matchAll(/from\s+"([^"]+)"/g)].map((match) => match[1] ?? "");

describe("R3348001 the arena is the viewer's own", () => {
  it("R3348001 nothing in arena/ or a tile's answer imports the net layer, the wire or the replay, or opens a connection", () => {
    const files = [...sources(join(SRC, "game", "arena")), join(SRC, "game", "SandSurface.tsx")];
    expect(files.length).toBeGreaterThan(7);
    for (const file of files) {
      const code = read(file);
      for (const specifier of importsOf(code)) {
        expect(specifier, `${relative(SRC, file)} imports ${specifier}`).not.toMatch(/(^|\/)net(\/|\.ts)|\/wire\/|(^|\/)replay\//);
      }
      expect(code, relative(SRC, file)).not.toMatch(/\b(fetch|WebSocket|sendBeacon|EventSource)\b/);
    }
  });

  it("R3348001 the net layer and the replay import nothing from arena/", () => {
    const files = [join(SRC, "game", "net.ts"), ...sources(join(SRC, "net")), ...sources(join(SRC, "replay")), ...sources(join(SRC, "wire"))];
    expect(files.length).toBeGreaterThan(10);
    for (const file of files) {
      for (const specifier of importsOf(read(file))) expect(specifier, `${relative(SRC, file)} imports ${specifier}`).not.toMatch(/arena\//);
    }
  });

  it("R3348001 the corner layer takes no view and no action: it reads only a mat id", () => {
    const code = read(join(SRC, "game", "arena", "CornerProps.tsx"));
    expect(code).not.toMatch(/PlayerView|onAction|ActionBody|@jackioh\/shared/);
  });
});

describe("R3348001 tiles and props change nothing the game hears of", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setAudioEngineForTests(fakeAudioEngine());
    setArenaRandomForTests(() => 0.5);
  });

  afterEach(() => {
    cleanup();
    setAudioEngineForTests(null);
    setArenaRandomForTests(null);
    vi.useRealTimers();
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  it("R3348001 pressing tiles and tapping props sends no action, opens no socket, calls no fetch and leaves the log as it was", () => {
    const onAction = vi.fn();
    const fetchSpy = vi.fn();
    const socketSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    vi.stubGlobal("WebSocket", socketSpy);
    const beacon = vi.fn();
    Object.defineProperty(navigator, "sendBeacon", { configurable: true, value: beacon });

    render(<Game view={fullBoardView()} legal={[]} onAction={onAction} wearCosmetics deckCosmetics={{ table: "table-tavern", field: null, cardBack: null }} />);
    // The game's own mount may fetch (its music); what matters is what the interactions below add.
    fetchSpy.mockClear();
    const log = screen.getByTestId(testid.log).textContent;
    const field = document.querySelector<HTMLElement>(".field");
    if (field === null) throw new Error("the board draws a field");
    Object.defineProperty(field, "getBoundingClientRect", { configurable: true, value: () => ({ left: 0, top: 0, width: 100, height: 100 }) });

    for (const zone of document.querySelectorAll(".zone")) {
      fireEvent.pointerDown(zone, { clientX: 30, clientY: 70, pointerType: "touch" });
      fireEvent.click(zone);
    }
    expect(document.querySelectorAll(".tile-answer").length).toBeGreaterThan(0);

    const props = [...document.querySelectorAll<HTMLElement>("button.corner-prop")];
    expect(props).toHaveLength(2);
    for (const prop of props) {
      for (let count = 0; count < CORNER_ESCALATE_TAPS.max + 1; count += 1) {
        fireEvent.pointerDown(prop, { pointerType: "touch" });
        fireEvent.click(prop);
      }
    }
    expect(props.some((prop) => prop.dataset.state !== undefined)).toBe(true);
    act(() => {
      vi.advanceTimersByTime(10_000);
    });

    expect(onAction).not.toHaveBeenCalled();
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(socketSpy).not.toHaveBeenCalled();
    expect(beacon).not.toHaveBeenCalled();
    expect(screen.getByTestId(testid.log).textContent).toBe(log);
    Reflect.deleteProperty(navigator, "sendBeacon");
  });
});
