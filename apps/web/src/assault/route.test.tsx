import { cleanup, render, screen } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, expect, it, vi } from "vitest";

import { paths } from "../net/navigate.ts";
import { chunkAt, prefetchRoute, resetRoutePrefetchForTests, routeChunks } from "../routes/chunks.tsx";
import { at, jsonResponse } from "../test/stubServer.ts";
import { __resetAssaultForTests } from "./store.ts";
import { assaultTestid } from "./testids.ts";
import { bindingFixtures, resetBindings } from "./test/fixtures.ts";

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
