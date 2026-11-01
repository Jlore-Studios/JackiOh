// The Chrome the e2e run launches as `chrome`, when the one Cypress found is a major known to break
// under it. Node-side only: cypress.config.ts's setupNodeEvents calls `pinChrome`.
//
// Cypress 16 answers every request the page makes over the DevTools protocol (`Fetch.enable`;
// `forceHttp1: true` never delivered the engine's .wasm). Chrome 155 made a request id reused under
// that interception fatal (Chromium CL 8359730), and a restarted request reuses one, so now and then
// the browser kills its network service: every fetch and socket fails while Node's `cy.request` never
// notices. CL 8411428 (crbug.com/559770861) fixed it in 156; on 155 `e2e smoke` failed 8 runs in 15.
//
// So on CI (or with E2E_CHROME_PIN=1) such a `chrome` entry is replaced, keeping its name since Cypress
// picks a name's highest version, by Chrome for Testing PINNED_CHROME_MILESTONE cached in .cache/; a
// failed download keeps the system Chrome and says why. Remove 155 from BROKEN_CHROME_MAJORS once the
// runner image ships 156, and this file with it.

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
