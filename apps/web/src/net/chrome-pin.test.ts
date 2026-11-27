// @vitest-environment node
// e2e/support/chrome-pin.ts: the Chrome for Testing pin retries its fetches, and a pin that cannot be set up stops
// the run instead of keeping the broken Chrome (#5178).

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const PIN = join(HERE, "../../../../e2e/support/chrome-pin.ts");

interface Browser {
  name: string;
  family: string;
  channel: string;
  displayName: string;
  version: string;
  majorVersion: number;
  path: string;
  isHeaded: boolean;
  isHeadless: boolean;
}

interface PinModule {
  PINNED_CHROME_MILESTONE: string;
  CFT_ATTEMPTS: number;
  CFT_RETRY_DELAY_MS: number;
  pinChrome: (browsers: Browser[], repoRoot: string, env?: Record<string, string>) => Promise<Browser[]>;
}

/** A Chrome of a major the pin replaces, as Cypress finds it on the runner. */
const BROKEN: Browser = {
  name: "chrome",
  family: "chromium",
  channel: "stable",
  displayName: "Chrome",
  version: "158.0.8127.36",
  majorVersion: 158,
  path: "/usr/bin/google-chrome",
  isHeaded: true,
  isHeadless: false,
};

/** The build the stubbed index lists for the pinned milestone. */
const PINNED_VERSION = "154.0.8037.92";

/** The environment of a CI run, which wants the pin. */
const ON_CI = { CI: "true" };

describe.runIf(process.platform === "linux" && process.arch === "x64")("chrome-pin", () => {
  let pin: PinModule;
  let root: string;
  let binary: string;

  beforeAll(async () => {
    pin = (await import(pathToFileURL(PIN).href)) as PinModule;
  });

  beforeEach(() => {
    vi.useFakeTimers();
    root = mkdtempSync(join(tmpdir(), "jackioh-chrome-pin-"));
    // The build already unpacked, so only the index is fetched.
    binary = join(root, ".cache", "chrome-for-testing", PINNED_VERSION, "chrome-linux64", "chrome");
    mkdirSync(dirname(binary), { recursive: true });
    writeFileSync(binary, "");
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    rmSync(root, { recursive: true, force: true });
  });

  function index(): Response {
    return new Response(
      JSON.stringify({
        milestones: {
          [pin.PINNED_CHROME_MILESTONE]: {
            version: PINNED_VERSION,
            downloads: { chrome: [{ platform: "linux64", url: "https://example.invalid/chrome-linux64.zip" }] },
          },
        },
      }),
    );
  }

  it("#5178 a fetch that fails once is tried again, and the run gets Chrome for Testing", async () => {
    const fetchMock = vi.fn().mockRejectedValueOnce(new TypeError("fetch failed")).mockResolvedValueOnce(index());
    vi.stubGlobal("fetch", fetchMock);
    const pinned = pin.pinChrome([BROKEN], root, ON_CI);
    await vi.advanceTimersByTimeAsync(pin.CFT_RETRY_DELAY_MS);
    const browsers = await pinned;
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(browsers).toHaveLength(1);
    expect(browsers[0]?.path).toBe(binary);
    expect(browsers[0]?.displayName).toBe("Chrome for Testing");
    expect(browsers[0]?.majorVersion).toBe(Number(pin.PINNED_CHROME_MILESTONE));
  });

  it("#5178 a pin that cannot be set up stops the run with the reason, rather than keep the broken Chrome", async () => {
    const fetchMock = vi.fn().mockRejectedValue(new TypeError("fetch failed"));
    vi.stubGlobal("fetch", fetchMock);
    const settled = expect(pin.pinChrome([BROKEN], root, ON_CI)).rejects.toThrow(
      `Chrome for Testing ${pin.PINNED_CHROME_MILESTONE} could not be set up`,
    );
    await vi.advanceTimersByTimeAsync(pin.CFT_RETRY_DELAY_MS * (pin.CFT_ATTEMPTS - 1));
    await settled;
    expect(fetchMock).toHaveBeenCalledTimes(pin.CFT_ATTEMPTS);
  });
});
