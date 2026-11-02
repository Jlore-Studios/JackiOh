// B13 (DR14 part D, #2705): `/dungeon` opens the Dungeon Run and `/dungeon/stats` its stats, both in the
// route table (a title, a chunk the engine's module comes with) and in the shell. The Play hub's and the
// command menu's links to it are proved beside theirs (`routes/play.test.tsx`, `routes/command-menu.test.tsx`).
// The run's bindings are the fixtures' scripted ones; the screens of parts E, F and G are stand-ins.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { chunkAt, prefetchRoute, resetRoutePrefetchForTests, routeChunks } from "../routes/chunks.tsx";
import { canonicalUrlFor, documentTitleFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { jsonResponse, at } from "../test/stubServer.ts";
import { __resetDungeonForTests } from "./store.ts";
import { dungeonTestid } from "./testids.ts";
import { resetBindings } from "./test/fixtures.ts";

vi.mock("../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("./test/fixtures.ts")).wasmBindings,
}));
vi.mock("./fight/FightScreen.tsx", () => ({ FightScreen: (): null => null }));
vi.mock("./map/RunScreen.tsx", () => ({ RunScreen: (): null => null }));
vi.mock("./stats/DungeonStats.tsx", () => ({
  DungeonStats: () => <p data-testid="stand-in-stats">Dungeon stats</p>,
}));

const { App } = await import("../main.tsx");

beforeEach(() => {
  window.localStorage.clear();
  __resetDungeonForTests();
  resetBindings();
  resetRoutePrefetchForTests();
  // A visitor with no session: the Sandbox needs none.
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.resolve(jsonResponse(401, { error: { code: "unauthorized", message: "Sign in first." } }))),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  at("/");
});

describe("B13 the Dungeon Run's route", () => {
  it("B13 /dungeon and /dungeon/stats open their chunks and have non-404 titles", async () => {
    expect(paths.dungeon).toBe("/dungeon");
    expect(paths.dungeonStats).toBe("/dungeon/stats");

    // Each path is a route chunk of its own, the engine's module loaded with it.
    expect(chunkAt(paths.dungeon)).toBe("dungeon");
    expect(chunkAt(`${paths.dungeon}/`)).toBe("dungeon");
    expect(chunkAt(paths.dungeonStats)).toBe("dungeonStats");
    expect(chunkAt(`${paths.dungeonStats}?x=1`)).toBe("dungeonStats");
    const load = vi.spyOn(routeChunks.dungeon, "load");
    prefetchRoute(paths.dungeon);
    expect(load).toHaveBeenCalledTimes(1);

    // Neither is a page that does not exist.
    expect(documentTitleFor(paths.dungeon)).toBe("Dungeon Run · JackiOh");
    expect(documentTitleFor(paths.dungeonStats)).toBe("Dungeon Run stats · JackiOh");
    expect(canonicalUrlFor(paths.dungeon, "https://jackioh.example")).toBe("https://jackioh.example/dungeon");
    expect(canonicalUrlFor(paths.dungeonStats, "https://jackioh.example")).toBe("https://jackioh.example/dungeon/stats");
    expect(documentTitleFor("/dungeon/extra")).toContain("Page not found");
  });

  it("B13 /dungeon opens the Dungeon Run screen, with no account", async () => {
    at(paths.dungeon);
    render(<App />);

    expect(await screen.findByTestId(dungeonTestid.start)).toBeInTheDocument();
    expect(screen.getByTestId(dungeonTestid.screen)).toBeInTheDocument();
    expect(window.location.pathname).toBe("/dungeon");
    expect(document.title).toBe("Dungeon Run · JackiOh");
  });

  it("B13 /dungeon/stats opens the Dungeon Run's stats, with no account", async () => {
    at(paths.dungeonStats);
    render(<App />);

    expect(await screen.findByTestId("stand-in-stats")).toBeInTheDocument();
    expect(screen.queryByTestId(dungeonTestid.start)).toBeNull();
    expect(window.location.pathname).toBe("/dungeon/stats");
    expect(document.title).toBe("Dungeon Run stats · JackiOh");
  });
});
