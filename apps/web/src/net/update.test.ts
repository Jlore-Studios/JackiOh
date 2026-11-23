// R4605000: a tab left open across a deploy notices the next build when it wakes.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  UPDATE_CHECK_MIN_MS,
  UPDATE_RELOAD_KEY,
  checkForUpdate,
  noteReloadedEntry,
  readDeployedEntry,
  resetUpdateForTests,
  runningEntry,
  updateWaiting,
} from "./update.ts";

const RUNNING = "/assets/index-old.js";
const NEWER = "/assets/index-new.js";

let script: HTMLScriptElement | null = null;

function deployed(entry: unknown, ok = true): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => ({
      ok,
      json: async () => entry,
    })),
  );
}

function fetchCalls(): number {
  return (fetch as unknown as ReturnType<typeof vi.fn>).mock.calls.length;
}

beforeEach(() => {
  resetUpdateForTests();
  window.sessionStorage.clear();
  script = document.createElement("script");
  script.type = "module";
  script.src = RUNNING;
  document.head.append(script);
});

afterEach(() => {
  script?.remove();
  script = null;
  vi.unstubAllGlobals();
  resetUpdateForTests();
  window.sessionStorage.clear();
});

describe("R4605000 the tab checks the deployed build", () => {
  it("R4605000 a newer version.json sets the update as waiting", async () => {
    deployed({ entry: NEWER });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(true);
    expect(updateWaiting()).toBe(true);
    expect(fetchCalls()).toBe(1);
  });

  it("R4605000 the same entry sets nothing waiting", async () => {
    deployed({ entry: RUNNING });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    expect(updateWaiting()).toBe(false);
  });

  it("R4605000 a failed fetch sets nothing waiting", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("Failed to fetch");
      }),
    );
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    expect(updateWaiting()).toBe(false);
  });

  it("R4605000 a non-ok answer sets nothing waiting", async () => {
    deployed({ entry: NEWER }, false);
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    expect(updateWaiting()).toBe(false);
  });

  it("R4605000 an answer naming no entry sets nothing waiting", async () => {
    for (const body of [{}, { entry: "" }, { entry: 42 }, null, []]) {
      vi.unstubAllGlobals();
      resetUpdateForTests();
      deployed(body);
      expect(await readDeployedEntry()).toBeNull();
      expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    }
  });

  it("R4605000 a check inside the interval sends no request", async () => {
    deployed({ entry: RUNNING });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    expect(fetchCalls()).toBe(1);
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS + UPDATE_CHECK_MIN_MS - 1, "production")).toBe(false);
    expect(fetchCalls()).toBe(1);
  });

  it("R4605000 a check past the interval sends again and sees the newer build", async () => {
    deployed({ entry: RUNNING });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    vi.unstubAllGlobals();
    deployed({ entry: NEWER });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS * 2, "production")).toBe(true);
    expect(updateWaiting()).toBe(true);
  });

  it("R4605000 outside a production build no request is sent and nothing waits", async () => {
    deployed({ entry: NEWER });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "development")).toBe(false);
    expect(updateWaiting()).toBe(false);
    expect(fetchCalls()).toBe(0);
  });

  it("R4605000 a tab whose running entry cannot be read waits on nothing", async () => {
    script?.remove();
    script = null;
    expect(runningEntry()).toBeNull();
    deployed({ entry: NEWER });
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(false);
    expect(updateWaiting()).toBe(false);
  });

  it("R4605000 the running entry is the document's module script", () => {
    expect(runningEntry()).toBe(RUNNING);
  });
});

describe("R4605000 a reloaded entry is remembered for the tab", () => {
  it("R4605000 the first failure for an entry reloads, its repeat does not", () => {
    expect(noteReloadedEntry(NEWER)).toBe(false);
    expect(window.sessionStorage.getItem(UPDATE_RELOAD_KEY)).toBe(NEWER);
    expect(noteReloadedEntry(NEWER)).toBe(true);
    expect(noteReloadedEntry("/assets/index-newer.js")).toBe(false);
  });
});
