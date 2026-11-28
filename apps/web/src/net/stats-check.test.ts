// #5174: what holds the card statistics' check in place. `/stats` stopped loading three times in four days
// (#5129, #5174), each time found by a person, because no deploy asked for a statistics route.
// .github/actions/check-stats asks for the page's first read within the client's timeout, and both deploys
// run it once their server reports its commit, while the deploy has the container awake. No schedule runs
// it: a scheduled check would wake each sleeping server, and the owners asked that the check add no cost.
// The action's own script runs here too, under `bash -e` as GitHub runs it, against a stand-in curl: a
// request curl gave up on once ended the step with no message and no second attempt (deploy-staging run
// 46066268550).
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { API_REQUEST_TIMEOUT_SECONDS } from "@jackioh/server-config";
import { afterEach, describe, expect, it } from "vitest";

import { cardStatsPath } from "./api.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const read = (path: string): string => readFileSync(join(ROOT, path), "utf8");
const ACTION = read(".github/actions/check-stats/action.yml");
const STAGING = read(".github/workflows/deploy-staging.yml");
const PRODUCTION = read(".github/workflows/promote-production.yml");

/** The action's `env:` value named `name`. */
function setting(name: string): string {
  return new RegExp(`^ {8}${name}: "?([^"\\n]+)"?$`, "mu").exec(ACTION)?.[1] ?? "";
}

describe("the card statistics' check (#5174)", () => {
  it("asks for the Cards tab's first read, on the page's default match type", () => {
    expect(read("apps/web/src/routes/stats.tsx")).toMatch(/^const DEFAULT_MODE: StatsMode = "all";$/mu);
    expect(setting("STATS_PATH")).toBe(cardStatsPath({ mode: "all" }));
  });

  it("holds the server to the client's own timeout, parsed from the file the client reads it from", () => {
    const config = setting("CONFIG");
    expect(config).toBe("apps/web/src/wire/serverConfig.ts");
    // The action's sed: `export const API_REQUEST_TIMEOUT_SECONDS = <n>;`, underscores dropped.
    const parsed = /^export const API_REQUEST_TIMEOUT_SECONDS = ([0-9_]+);/mu.exec(read(config))?.[1]?.replaceAll("_", "");
    expect(Number(parsed)).toBe(API_REQUEST_TIMEOUT_SECONDS);
    expect(ACTION).toContain("s/^export const API_REQUEST_TIMEOUT_SECONDS = ([0-9_]+);.*/\\1/p");
  });

  it("runs on both deploys once their server reports the deployed commit", () => {
    for (const workflow of [STAGING, PRODUCTION]) {
      const polled = workflow.indexOf("uses: ./.github/actions/poll-live");
      const checked = workflow.indexOf("uses: ./.github/actions/check-stats");
      expect(polled).toBeGreaterThan(-1);
      expect(checked).toBeGreaterThan(polled);
    }
    expect(STAGING).toMatch(
      /uses: \.\/\.github\/actions\/check-stats\n {8}with:\n {10}site: \$\{\{ env\.SITE \}\}\n {10}access-client-id: \$\{\{ secrets\.CF_ACCESS_CLIENT_ID \}\}\n/u,
    );
    expect(PRODUCTION).toMatch(/uses: \.\/\.github\/actions\/check-stats\n {8}with:\n {10}site: \$\{\{ env\.SITE \}\}\n/u);
  });

  it("runs on no schedule, which would wake a sleeping server and add its time to the bill", () => {
    const scheduled = readdirSync(join(ROOT, ".github/workflows"))
      .filter((file) => file.endsWith(".yml"))
      .filter((file) => {
        const workflow = read(`.github/workflows/${file}`);
        return /^ {2}schedule:$/mu.test(workflow) && workflow.includes("./.github/actions/check-stats");
      });
    expect(scheduled).toEqual([]);
  });
});

/** The action's script: its `run: |` block, the file's last, unindented. */
const RUN = "      run: |\n";
const SCRIPT = ACTION.slice(ACTION.indexOf(RUN) + RUN.length)
  .split("\n")
  .map((line) => line.slice(8))
  .join("\n");

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

/**
 * The action's script run as GitHub runs a composite step's `shell: bash` (`-e`, `-o pipefail`), with a
 * stand-in curl that answers each request for the statistics with the next of `answers`: `gave-up` as curl
 * answers a request it gave up on (its -w line with status 000, an error, exit 52), or an HTTP status,
 * with one card for 200 and none for any other. It answers each ask of `/api/stats/loading` (R5186000)
 * with the next of `loading`, `false` once they run out: `true` or `false` as the server's JSON, `down` as
 * the Worker's plain 500 for a container it cannot reach, `gave-up` as a request curl gave up on, and
 * `missing` as a server without the route.
 */
function check(answers: string[], loading: string[] = []): { status: number | null; out: string; loadingAsks: number } {
  const root = mkdtempSync(join(tmpdir(), "stats-check-"));
  roots.push(root);
  writeFileSync(join(root, "answers"), `${answers.join("\n")}\n`);
  writeFileSync(join(root, "asked"), "0");
  writeFileSync(join(root, "loading"), `${loading.join("\n")}\n`);
  writeFileSync(join(root, "loading-asked"), "0");
  writeFileSync(join(root, "check.sh"), SCRIPT);
  const curl = join(root, "curl");
  writeFileSync(
    curl,
    `#!/bin/sh
body=""
url=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = -o ]; then body=$2; fi
  url=$1
  shift
done
case $url in
  */api/stats/loading)
    loaded=$(( $(cat "${root}/loading-asked") + 1 ))
    echo "$loaded" > "${root}/loading-asked"
    said=$(sed -n "$loaded"p "${root}/loading" | grep . || echo false)
    case $said in
      down) printf 'Internal Server Error' > "$body"; printf '500' ;;
      gave-up) printf '000'; exit 28 ;;
      missing) printf '{"error":"not found"}' > "$body"; printf '404' ;;
      *) printf '{"loading":%s}' "$said" > "$body"; printf '200' ;;
    esac
    exit 0
    ;;
esac
asked=$(( $(cat "${root}/asked") + 1 ))
echo "$asked" > "${root}/asked"
answer=$(sed -n "$asked"p "${root}/answers")
case $answer in
  gave-up) echo "curl: (52) Empty reply from server" >&2; printf '000 62.0'; exit 52 ;;
  200) printf '{"cards":[{"card":"core-001"}]}' > "$body" ;;
  *) printf '{"error":"down"}' > "$body" ;;
esac
printf '%s 1.5' "$answer"
`,
  );
  chmodSync(curl, 0o755);
  const result = spawnSync("bash", ["--noprofile", "--norc", "-e", "-o", "pipefail", join(root, "check.sh")], {
    cwd: ROOT,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${root}:${process.env.PATH ?? ""}`,
      SITE: "https://staging.example",
      ACCESS_ID: "",
      ACCESS_SECRET: "",
      CONFIG: setting("CONFIG"),
      STATS_PATH: setting("STATS_PATH"),
      ATTEMPTS: setting("ATTEMPTS"),
      PAUSE_SECONDS: "0",
      LOADING_PATH: setting("LOADING_PATH"),
      LOAD_WAIT_SECONDS: setting("LOAD_WAIT_SECONDS"),
      LOAD_POLL_SECONDS: "0",
      GITHUB_STEP_SUMMARY: join(root, "summary"),
    },
  });
  return {
    status: result.status,
    out: `${result.stdout}${result.stderr}`,
    loadingAsks: Number(readFileSync(join(root, "loading-asked"), "utf8")),
  };
}

describe("the card statistics' check, run as GitHub runs it (bash -e)", () => {
  it("reads the action's whole script", () => {
    expect(SCRIPT.startsWith("set -uo pipefail\n")).toBe(true);
    expect(SCRIPT.trimEnd().endsWith("exit 1")).toBe(true);
  });

  it("takes a request curl gave up on as a failed attempt, says so, and asks again", () => {
    const { status, out } = check(["gave-up", "200"]);
    expect(status).toBe(0);
    expect(out).toContain(
      "::warning::https://staging.example/api/stats/cards?mode=all, attempt 1 of 2: HTTP 000 after 62.0s, no body; curl: (52) Empty reply from server",
    );
    expect(out).toContain("https://staging.example/api/stats/cards?mode=all: 1 cards in 1.5s (attempt 2 of 2)");
  });

  it("R5186000 follows a load the server reports before its next attempt, as the page does", () => {
    const { status, out, loadingAsks } = check(["gave-up", "200"], ["true", "true", "false"]);
    expect(status).toBe(0);
    expect(setting("LOADING_PATH")).toBe("/api/stats/loading");
    expect(out).toContain("https://staging.example: the card statistics are still loading, 0s waited of 600s");
    expect(out).toContain("1 cards in 1.5s (attempt 2 of 2)");
    expect(loadingAsks).toBe(3);
  });

  it("waits out a server that does not answer the probe, as a rollout's draining container does, until it says the load is done", () => {
    // deploy-staging run 46126109815: the first attempt reached the old image as Cloudflare rolled it, and the
    // probe's plain 500 ended the wait, so the second attempt waited on the new server's first load.
    const { status, out, loadingAsks } = check(["500", "200"], ["down", "gave-up", "true", "false"]);
    expect(status).toBe(0);
    expect(out).toContain(
      "https://staging.example: the server did not say whether the card statistics are loading (HTTP 500), so it may be restarting; 0s waited of 600s",
    );
    expect(out).toContain("did not say whether the card statistics are loading (HTTP 000)");
    expect(out).toContain("the card statistics are still loading");
    expect(out).toContain("1 cards in 1.5s (attempt 2 of 2)");
    expect(loadingAsks).toBe(4);
  });

  it("stops waiting at once for a server without the loading route", () => {
    const { status, loadingAsks } = check(["500", "200"], ["missing", "true"]);
    expect(status).toBe(0);
    expect(loadingAsks).toBe(1);
  });

  it("names the last failure when every attempt fails, with an error from curl or without one", () => {
    const refused = check(["500", "500"]);
    expect(refused.status).toBe(1);
    expect(refused.out).toContain("::warning::https://staging.example/api/stats/cards?mode=all, attempt 1 of 2: HTTP 500");
    expect(refused.out).toContain("in 2 attempts; last: HTTP 500 after 1.5s, no cards list.");
    const gaveUp = check(["gave-up", "gave-up"]);
    expect(gaveUp.status).toBe(1);
    expect(gaveUp.out).toContain("last: HTTP 000 after 62.0s, no body; curl: (52) Empty reply from server.");
  });
});
