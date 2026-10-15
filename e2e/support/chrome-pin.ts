// The Chrome the e2e run launches as `chrome`, when the one Cypress found is a major known to break
// under it. Node-side only: cypress.config.ts's setupNodeEvents calls `pinChrome`.
//
// Cypress 16 runs Chrome on its native network path: every request the page makes, at both its
// request and its response stage, is paused and answered over the DevTools protocol (`Fetch.enable`,
// packages/server/lib/network-runtime.ts and browsers/cdp-protocol/cdp-fetch-transport.ts in
// cypress-io/cypress v16.1.0). `forceHttp1: true` is the only way back to the HTTP/1 proxy, and that
// path never delivered the engine's .wasm. Chrome 155 made a request id reused under DevTools
// request interception fatal (Chromium CL 8359730), and Chrome legitimately reuses one when a
// request restarts, so now and then the browser kills its own network service ("DevTools:
// Duplicate request ID", then "Network service crashed or was terminated"). Every fetch in flight
// fails at once and every socket drops, while `cy.request`, which runs in Node, never notices. CL
// 8411428 (crbug.com/559770861) fixed it in Chrome 156; 154 has neither change
// (github.com/blassaut/dappress/pull/37 measured it). GitHub's ubuntu-24.04 image ubuntu24/20261012
// ships Google Chrome 155.0.8059.39, on which `e2e smoke` failed 8 runs in 15 against none in 7 on
// 154: a lobby showing "Couldn't reach JackiOh", a series banner that never came, a guest left on
// /play.
//
// So when the `chrome` entry is a major in BROKEN_CHROME_MAJORS and the run is on CI (or
// E2E_CHROME_PIN=1 asks), that entry is pointed at Chrome for Testing PINNED_CHROME_MILESTONE,
// downloaded once into the repository's .cache/ (gitignored). Its name stays `chrome`, so
// `cypress run --browser chrome` launches it; Cypress picks the highest version of a name, so the
// entry is replaced, not joined. Any other Chrome, and every other browser, is left exactly as
// found. A failed download leaves the system Chrome in place and says why. Remove 155 from
// BROKEN_CHROME_MAJORS once the runner image ships 156, and this file with it.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** Google Chrome majors whose network service dies under Cypress's DevTools interception. */
export const BROKEN_CHROME_MAJORS: readonly number[] = [155];

/** The Chrome for Testing milestone launched in their place: the last major this suite passed on. */
export const PINNED_CHROME_MILESTONE = "154";

/** Chrome for Testing's index of the newest build of every milestone, with its downloads. */
const CFT_MILESTONES_URL =
  "https://googlechromelabs.github.io/chrome-for-testing/latest-versions-per-milestone-with-downloads.json";

/** The download for Linux x64, which is what CI runs, and the binary inside its archive. */
const CFT_PLATFORM = "linux64";
const CFT_BINARY = join("chrome-linux64", "chrome");

type Build = { version: string; url: string };

type MilestoneIndex = {
  milestones?: Record<string, { version?: unknown; downloads?: { chrome?: { platform?: unknown; url?: unknown }[] } }>;
};

function majorOf(browser: Cypress.Browser): number {
  return Number.parseInt(String(browser.majorVersion), 10);
}

/** E2E_CHROME_PIN=1 pins anywhere, 0 never; unset, only on CI, which GitHub marks with CI=true. */
function pinWanted(env: NodeJS.ProcessEnv): boolean {
  if (env.E2E_CHROME_PIN === "1") return true;
  if (env.E2E_CHROME_PIN === "0") return false;
  return env.CI === "true" || env.CI === "1";
}

async function pinnedBuild(): Promise<Build> {
  const response = await fetch(CFT_MILESTONES_URL);
  if (!response.ok) throw new Error(`${CFT_MILESTONES_URL} answered ${String(response.status)}`);
  const index = (await response.json()) as MilestoneIndex;
  const milestone = index.milestones?.[PINNED_CHROME_MILESTONE];
  const download = milestone?.downloads?.chrome?.find((entry) => entry.platform === CFT_PLATFORM);
  if (typeof milestone?.version !== "string" || typeof download?.url !== "string") {
    throw new Error(`the index lists no ${CFT_PLATFORM} build for milestone ${PINNED_CHROME_MILESTONE}`);
  }
  return { version: milestone.version, url: download.url };
}

/** The binary of `build`, downloaded and unpacked into `cacheDir` the first time it is asked for. */
async function installed(build: Build, cacheDir: string): Promise<string> {
  const home = join(cacheDir, build.version);
  const binary = join(home, CFT_BINARY);
  if (existsSync(binary)) return binary;
  mkdirSync(cacheDir, { recursive: true });
  // Unpacked beside its place and renamed into it, so a run stopped halfway, or a second run on the
  // same machine, never finds half a browser.
  const scratch = mkdtempSync(join(cacheDir, `.${build.version}-`));
  try {
    const response = await fetch(build.url);
    if (!response.ok) throw new Error(`${build.url} answered ${String(response.status)}`);
    const archive = join(scratch, "chrome.zip");
    writeFileSync(archive, Buffer.from(await response.arrayBuffer()));
    const unpacked = join(scratch, "unpacked");
    execFileSync("unzip", ["-q", archive, "-d", unpacked]);
    try {
      renameSync(unpacked, home);
    } catch (cause) {
      // Another run on this machine got there first.
      if (!existsSync(binary)) throw cause;
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  return binary;
}

/**
 * `browsers` as Cypress found them, with a Google Chrome of a broken major pointed at Chrome for
 * Testing PINNED_CHROME_MILESTONE (unpacked under `repoRoot`/.cache) when the run wants the pin.
 */
export async function pinChrome(
  browsers: readonly Cypress.Browser[],
  repoRoot: string,
  env: NodeJS.ProcessEnv = process.env,
): Promise<Cypress.Browser[]> {
  const broken = browsers.filter((browser) => browser.name === "chrome" && BROKEN_CHROME_MAJORS.includes(majorOf(browser)));
  if (broken.length === 0) return [...browsers];
  const found = broken.map((browser) => browser.version).join(", ");
  if (!pinWanted(env)) {
    console.warn(
      `[e2e] Chrome ${found} drops the page's requests under Cypress now and then (support/chrome-pin.ts); ` +
        `E2E_CHROME_PIN=1 runs Chrome for Testing ${PINNED_CHROME_MILESTONE} as \`chrome\` instead.`,
    );
    return [...browsers];
  }
  if (process.platform !== "linux" || process.arch !== "x64") {
    console.warn(`[e2e] Chrome for Testing is pinned for Linux x64 only; keeping Chrome ${found}.`);
    return [...browsers];
  }
  try {
    const build = await pinnedBuild();
    const path = await installed(build, join(repoRoot, ".cache", "chrome-for-testing"));
    console.log(`[e2e] Chrome ${found} is a broken major (support/chrome-pin.ts); \`chrome\` is Chrome for Testing ${build.version}, ${path}.`);
    return browsers.map((browser) =>
      broken.includes(browser)
        ? { ...browser, displayName: "Chrome for Testing", version: build.version, majorVersion: Number.parseInt(build.version, 10), path }
        : browser,
    );
  } catch (cause) {
    const reason = cause instanceof Error ? cause.message : String(cause);
    console.warn(`[e2e] could not set up Chrome for Testing ${PINNED_CHROME_MILESTONE} (${reason}); keeping Chrome ${found}.`);
    return [...browsers];
  }
}
