// #5174: what holds the card statistics' check in place. `/stats` stopped loading three times in four days
// (#5129, #5174), each time found by a person, because no deploy asked for a statistics route.
// .github/actions/check-stats asks for the page's first read within the client's timeout, and both deploys
// run it once their server reports its commit, while the deploy has the container awake. No schedule runs
// it: a scheduled check would wake each sleeping server, and the owners asked that the check add no cost.
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { API_REQUEST_TIMEOUT_SECONDS } from "@jackioh/server-config";
import { describe, expect, it } from "vitest";

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
