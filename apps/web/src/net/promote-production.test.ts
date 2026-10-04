// .github/workflows/promote-production.yml: when `production` moves. The deciding shell script is
// read out of the workflow and run against throwaway git repositories, with a stand-in `gh` that
// calls every commit's CI green, so each case here is the answer one trigger would get. `production`
// may only ever fast-forward, a catalog bump moves it at once, and the daily cron's fallback (the
// first green CI run on main a day after production's commit) moves it when GitHub never started
// the cron.

import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const WORKFLOW = join(HERE, "../../../../.github/workflows/promote-production.yml");

/** The step's `run: |` block, dedented: it is the workflow's last step, so it runs to the end of the file. */
function promoteScript(): string {
  const text = readFileSync(WORKFLOW, "utf8");
  const marker = "        run: |\n";
  const at = text.indexOf(marker, text.indexOf("pick the candidate and promote it"));
  expect(at, "the promote step's run block").toBeGreaterThan(-1);
  const body = text
    .slice(at + marker.length)
    .split("\n")
    .map((line) => (line.startsWith("          ") ? line.slice(10) : line))
    .join("\n");
  expect(body).toContain('git push origin "$candidate:refs/heads/production"');
  return body;
}

const START = Date.parse("2026-10-01T00:00:00Z") / 1000;
const HOUR = 3600;

let root = "";
let seq = 0;

type Rig = { origin: string; work: string; summary: string; shas: Record<string, string> };

function git(cwd: string, date: number | undefined, ...args: string[]): string {
  const stamp = date === undefined ? {} : { GIT_AUTHOR_DATE: `${date} +0000`, GIT_COMMITTER_DATE: `${date} +0000` };
  const quiet = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "-c", "push.negotiate=false"];
  return execFileSync("git", [...quiet, ...args], {
    cwd,
    encoding: "utf8",
    env: { ...process.env, ...stamp },
  }).trim();
}

const renderYaml = (version: string): string =>
  `services:\n  - type: web\n    envVars:\n      - key: CATALOG_VERSION\n        value: ${version}\n`;

/**
 * A repository whose `main` has one commit per entry of `commits` (hours after the start, and the
 * catalog version render.yaml names there), cloned from a bare origin the way the workflow's
 * checkout is. `production` starts at the commit named by `productionAt`, or does not exist.
 */
function rig(commits: { name: string; hours: number; catalog: string }[], productionAt: string | null): Rig {
  seq += 1;
  const origin = join(root, `origin-${seq}.git`);
  const work = join(root, `work-${seq}`);
  mkdirSync(origin);
  git(origin, undefined, "init", "-q", "--bare", "-b", "main");
  mkdirSync(work);
  git(work, undefined, "init", "-q", "-b", "main");
  git(work, undefined, "remote", "add", "origin", origin);
  const shas: Record<string, string> = {};
  for (const commit of commits) {
    writeFileSync(join(work, "render.yaml"), renderYaml(commit.catalog));
    writeFileSync(join(work, "note.txt"), `${commit.name}\n`);
    git(work, START + commit.hours * HOUR, "add", "-A");
    git(work, START + commit.hours * HOUR, "commit", "-q", "-m", commit.name);
    shas[commit.name] = git(work, undefined, "rev-parse", "HEAD");
  }
  git(work, undefined, "push", "-q", "origin", "main");
  if (productionAt !== null) git(work, undefined, "push", "-q", "origin", `${shas[productionAt] ?? ""}:refs/heads/production`);
  git(work, undefined, "fetch", "-q", "origin");
  return { origin, work, summary: join(root, `summary-${seq}.md`), shas };
}

let stubDir = "";

/** Runs the workflow's script as `event` would, with `candidate` as the CI run's commit (workflow_run) or the newest green one. */
function promote(r: Rig, event: string, candidate: string): { status: number; out: string } {
  writeFileSync(r.summary, "");
  const run = spawnSync("bash", ["-c", promoteScript()], {
    cwd: r.work,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${stubDir}:${process.env["PATH"] ?? ""}`,
      GH_TOKEN: "unused",
      EVENT: event,
      RUN_SHA: event === "workflow_run" ? (r.shas[candidate] ?? "") : "",
      INPUT_SHA: "",
      STUB_NEWEST: r.shas[candidate] ?? "",
      GITHUB_STEP_SUMMARY: r.summary,
      MAX_LAG_HOURS: "24",
    },
  });
  return { status: run.status ?? -1, out: `${run.stdout}${run.stderr}` };
}

const productionOf = (r: Rig): string | null => {
  const sha = git(r.origin, undefined, "rev-parse", "-q", "--verify", "refs/heads/production").trim();
  return sha === "" ? null : sha;
};

function productionName(r: Rig): string | null {
  const sha = (() => {
    try {
      return productionOf(r);
    } catch {
      return null;
    }
  })();
  if (sha === null) return null;
  return Object.entries(r.shas).find(([, value]) => value === sha)?.[0] ?? sha;
}

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), "promote-production-"));
  stubDir = join(root, "bin");
  mkdirSync(stubDir);
  // `gh run list` is the only call: `--commit X` asks whether X's push CI passed (always), and
  // without it the newest green commit, which the test names in STUB_NEWEST.
  writeFileSync(
    join(stubDir, "gh"),
    '#!/bin/sh\nwhile [ $# -gt 0 ]; do\n  if [ "$1" = --commit ]; then echo "$2"; exit 0; fi\n  shift\ndone\necho "$STUB_NEWEST"\n',
  );
  chmodSync(join(stubDir, "gh"), 0o755);
});

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
});

describe("promote-production.yml", () => {
  it("leaves production alone on a green push that changes nothing about the catalog and is not a day ahead", () => {
    const r = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 2, catalog: "v0.2.4" }], "a");
    const result = promote(r, "workflow_run", "b");
    expect(result.status, result.out).toBe(0);
    expect(result.out).toContain("waits for the daily promotion");
    expect(productionName(r)).toBe("a");
  });

  it("promotes on a catalog bump at once, however recent production is", () => {
    const r = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 1, catalog: "v0.2.5" }], "a");
    const result = promote(r, "workflow_run", "b");
    expect(result.status, result.out).toBe(0);
    expect(productionName(r)).toBe("b");
  });

  it("stands in for a daily run that never started: a green push a day past production's commit promotes it", () => {
    const r = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 25, catalog: "v0.2.4" }], "a");
    const result = promote(r, "workflow_run", "b");
    expect(result.status, result.out).toBe(0);
    expect(result.out).toContain("stands in");
    expect(productionName(r)).toBe("b");
  });

  it("treats exactly MAX_LAG_HOURS as a day, and a minute short of it as not yet", () => {
    const day = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 24, catalog: "v0.2.4" }], "a");
    expect(promote(day, "workflow_run", "b").status).toBe(0);
    expect(productionName(day)).toBe("b");

    const short = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 23.98, catalog: "v0.2.4" }], "a");
    expect(promote(short, "workflow_run", "b").status).toBe(0);
    expect(productionName(short)).toBe("a");
  });

  it("promotes the newest green commit when the daily run (or a person) asks, whatever the lag", () => {
    for (const event of ["schedule", "workflow_dispatch"]) {
      const r = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 1, catalog: "v0.2.4" }], "a");
      const result = promote(r, event, "b");
      expect(result.status, `${event}: ${result.out}`).toBe(0);
      expect(productionName(r), event).toBe("b");
    }
  });

  it("creates production on the first run", () => {
    const r = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }], null);
    const result = promote(r, "workflow_dispatch", "a");
    expect(result.status, result.out).toBe(0);
    expect(result.out).toContain("this run creates it");
    expect(productionName(r)).toBe("a");
  });

  it("never moves production backwards or sideways: a promotion of an older commit is a no-op, a diverged production an error", () => {
    const behind = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 30, catalog: "v0.2.4" }], "b");
    const older = promote(behind, "workflow_dispatch", "a");
    expect(older.status, older.out).toBe(0);
    expect(older.out).toContain("already past");
    expect(productionName(behind)).toBe("b");

    const diverged = rig([{ name: "a", hours: 0, catalog: "v0.2.4" }, { name: "b", hours: 1, catalog: "v0.2.4" }], "a");
    // production moved by hand to a commit that is not on main
    writeFileSync(join(diverged.work, "side.txt"), "by hand\n");
    git(diverged.work, START, "checkout", "-q", "-b", "side", diverged.shas["a"] ?? "");
    git(diverged.work, START, "add", "-A");
    git(diverged.work, START, "commit", "-q", "-m", "by hand");
    const side = git(diverged.work, undefined, "rev-parse", "HEAD");
    git(diverged.work, undefined, "push", "-q", "--force", "origin", `${side}:refs/heads/production`);
    git(diverged.work, undefined, "fetch", "-q", "origin");
    const result = promote(diverged, "workflow_dispatch", "b");
    expect(result.status).toBe(1);
    expect(result.out).toContain("not an ancestor");
    expect(productionOf(diverged)).toBe(side);
  });
});
