import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest";

import type { AssaultContent, AssaultFile } from "@jackioh/run";

import { paths } from "../net/navigate.ts";
import { chunkAt, prefetchRoute, resetRoutePrefetchForTests, routeChunks } from "../routes/chunks.tsx";
import { at, jsonResponse } from "../test/stubServer.ts";
import { catalogVersion } from "../wasm/index.ts";
import { ASSAULT_ATTEMPT_KEY, ASSAULT_RECORDS_KEY, ASSAULT_STORAGE_VERSION } from "./config.ts";
import { __resetAssaultForTests } from "./store.ts";
import { assaultTestid } from "./testids.ts";
import { CONTENT, RECORD, bindingFixtures, resetBindings } from "./test/fixtures.ts";

vi.mock("../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("./test/fixtures.ts")).bindingFixtures,
}));

const { App } = await import("../main.tsx");

beforeAll(() => routeChunks.assault.load());

beforeEach(() => {
  at("/");
  window.localStorage.clear();
  __resetAssaultForTests();
  resetBindings();
  resetRoutePrefetchForTests();
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.resolve(jsonResponse(401, { error: { code: "unauthorized", message: "Sign in first." } }))),
  );
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

afterAll(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  __resetAssaultForTests();
  at("/");
});

it("/assault opens the assault screen with no account", async () => {
  at(paths.assault);
  render(<App />);

  expect(await screen.findByTestId(assaultTestid.page)).toBeInTheDocument();
  expect(screen.getByTestId(assaultTestid.start)).toBeInTheDocument();
  expect(window.location.pathname).toBe("/assault");
});

it("chunkAt maps /assault and prefetch starts its chunk", () => {
  expect(chunkAt(paths.assault)).toBe("assault");
  expect(chunkAt(`${paths.assault}?day=1`)).toBe("assault");
  const load = vi.spyOn(routeChunks.assault, "load");
  prefetchRoute(paths.assault);
  expect(load).toHaveBeenCalledTimes(1);
  expect(bindingFixtures.assaultContent).not.toHaveBeenCalled();
});

it("R4581000 Try again after midnight starts today's attempt on the current content", async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-11-23T00:05:00Z"));
  const olderContent: AssaultContent = { ...CONTENT, doctrines: [] };
  bindingFixtures.assaultContentAt.mockReturnValue(olderContent);
  const lost: AssaultFile = {
    version: "an-older-catalog",
    setup: { date: "2026-11-22", deck: ["core-003", "core-004"], source: "dayDeck", startLevel: 1, attempt: 1 },
    record: { date: "2026-11-22", best: 4, checkpoints: [2, 4], attempts: 0, repelledOn: null },
    actions: [{ kind: "fight" }, { kind: "fightOver", won: false, turns: 3 }],
  };
  const today = { ...RECORD, date: "2026-11-23", checkpoints: [2], attempts: 2 };
  window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: ASSAULT_STORAGE_VERSION, file: lost }));
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, JSON.stringify({ v: ASSAULT_STORAGE_VERSION, days: { [today.date]: today } }));
  at(paths.assault);
  render(<App />);

  fireEvent.click(await screen.findByRole("button", { name: "Try again" }));
  expect(bindingFixtures.assaultDay, "the lost attempt folded on its own day and content").toHaveBeenCalledWith("2026-11-22", olderContent);

  expect(screen.getByTestId(assaultTestid.briefing)).toBeInTheDocument();
  expect(bindingFixtures.assaultDay).toHaveBeenLastCalledWith("2026-11-23", CONTENT);
  expect(bindingFixtures.assaultDayDeck).toHaveBeenLastCalledWith("2026-11-23");
  const kept = JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile };
  expect(kept.file).toEqual({
    version: catalogVersion(),
    setup: { date: "2026-11-23", deck: ["core-001", "core-002"], source: "dayDeck", startLevel: 2, attempt: 3 },
    record: today,
    actions: [],
  });
});
