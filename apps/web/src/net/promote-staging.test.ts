// scripts/promote-staging.sh, the logic of .github/workflows/promote-staging.yml (#3275): every two
// hours `staging` fast-forwards to main's newest commit whose push-to-main CI run passed, never
// backwards, and deploy-staging.yml deploys each push of it, except while the super run on staging
// is red (the freeze, the owners' call of 2026-11-10). Each case runs the real script in a fresh clone
// of a throwaway "origin", with a stand-in for `gh` that answers the CI and super runs, the issues and
// the repository's activity (when staging was last pushed) from a JSON file, and records the issues it
// opens and the runs it starts.

import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it, vi } from "vitest";

vi.setConfig({ testTimeout: 60_000 });

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = join(ROOT, "scripts/promote-staging.sh");
const WORKFLOW = readFileSync(join(ROOT, ".github/workflows/promote-staging.yml"), "utf8");
const DEPLOY = readFileSync(join(ROOT, ".github/workflows/deploy-staging.yml"), "utf8");
const SUPER = readFileSync(join(ROOT, ".github/workflows/super.yml"), "utf8");
const NOW = Date.parse("2026-11-06T12:00:00Z") / 1000;

const FAKE_GH = String.raw`#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const cp = require("child_process");
const file = path.join(process.env.FAKE_GH_DIR, "state.json");
const st = JSON.parse(fs.readFileSync(file, "utf8"));
const argv = process.argv.slice(2);
const VALUE = new Set(["--workflow", "--branch", "--event", "--status", "--limit", "--json", "--jq", "--commit", "--created", "--state", "--search", "--title", "--body", "--ref", "--add-assignee"]);
const pos = [];
const flag = {};
const labels = [];
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === "--add-label") labels.push(argv[++i]);
  else if (VALUE.has(argv[i])) flag[argv[i]] = argv[++i];
  else pos.push(argv[i]);
}
st.log.push(argv.join(" "));
const save = () => fs.writeFileSync(file, JSON.stringify(st));
save();
function out(data) {
  const r = cp.spawnSync("jq", ["-r", flag["--jq"]], { input: JSON.stringify(data), encoding: "utf8" });
  if (r.status !== 0) { process.stderr.write(r.stderr); process.exit(1); }
  process.stdout.write(r.stdout);
}
if (pos[0] === "run" && pos[1] === "list" && flag["--workflow"] === "ci.yml") {
  if (flag["--branch"] !== "main" || flag["--event"] !== "push" || flag["--status"] !== "success") { process.stderr.write("unexpected run list\n"); process.exit(2); }
  // Newest run first, as GitHub lists them; --created ">=<time>" keeps the runs created since then.
  const since = flag["--created"] === undefined ? "" : flag["--created"].replace(/^>=/, "");
  const runs = [...st.green].reverse()
    .filter((sha) => flag["--commit"] === undefined || sha === flag["--commit"])
    .filter((sha) => since === "" || st.created[sha] >= since);
  out(runs.map((headSha) => ({ headSha })));
} else if (pos[0] === "run" && pos[1] === "list" && flag["--workflow"] === "super.yml") {
  if (flag["--branch"] !== "staging") { process.stderr.write("unexpected super run list\n"); process.exit(2); }
  out(st.super);
} else if (pos[0] === "issue" && pos[1] === "list") {
  if (flag["--state"] !== "all" || !flag["--search"].includes('in:title "CI: daily super run failed"')) { process.stderr.write("unexpected issue list\n"); process.exit(2); }
  out(st.issues);
} else if (pos[0] === "issue" && pos[1] === "create") {
  const number = 100 + st.issues.length;
  st.issues.push({ number, title: flag["--title"], state: "OPEN", closedAt: null, body: flag["--body"] });
  save();
  process.stdout.write("https://github.com/acme/game/issues/" + number + "\n");
} else if (pos[0] === "issue" && pos[1] === "edit") {
  st.edits.push({ number: Number(pos[2]), assignee: flag["--add-assignee"], labels });
  save();
} else if (pos[0] === "workflow" && pos[1] === "run") {
  if (st.startFails) { process.stderr.write("HTTP 403\n"); process.exit(1); }
  st.started.push(pos[2] + " --ref " + flag["--ref"]);
  save();
} else if (pos[0] === "api" && /^repos\/acme\/game\/activity\?ref=refs\/heads\/staging&per_page=1$/.test(pos[1])) {
  if (st.activityFails) { process.stderr.write("HTTP 403\n"); process.exit(1); }
  out(st.pushedAt ? [{ timestamp: st.pushedAt, activity_type: "push", ref: "refs/heads/staging" }] : []);
} else { process.stderr.write("fake gh: unsupported " + argv.join(" ") + "\n"); process.exit(2); }
`;

interface SuperRun { status: string; conclusion: string | null; headSha: string; updatedAt: string; url: string }
interface Issue { number: number; title: string; state: string; closedAt: string | null; body?: string }
interface Gh {
  green: string[];
  /** When each commit's push-to-main CI run was created. */
  created: Record<string, string>;
  pushedAt: string;
  activityFails: boolean;
  /** The super runs on staging, newest first. */
  super: SuperRun[];
  issues: Issue[];
  edits: { number: number; assignee: string; labels: string[] }[];
  /** The workflow runs it started. */
  started: string[];
  startFails: boolean;
  log: string[];
}
interface World { root: string; origin: string; work: string; bin: string; sha: Record<string, string> }
const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

const git = (cwd: string, ...args: string[]): string =>
  execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", ...args], { cwd, encoding: "utf8" }).trim();
const stateFile = (w: World): string => join(w.root, "gh", "state.json");
const gh = (w: World): Gh => JSON.parse(readFileSync(stateFile(w), "utf8")) as Gh;
const edit = (w: World, change: (state: Gh) => void): void => {
  const state = gh(w);
  change(state);
  writeFileSync(stateFile(w), JSON.stringify(state));
};
const staging = (w: World): string => git(w.origin, "rev-parse", "refs/heads/staging");
/** The time `minutes` before NOW, as GitHub writes one. */
const ago = (minutes: number): string => new Date((NOW - minutes * 60) * 1000).toISOString().replace(/\.\d+Z$/u, "Z");

/** When staging was last pushed, `minutes` before NOW. */
const pushedAgo = (w: World, minutes: number): void =>
  edit(w, (s) => {
    s.pushedAt = ago(minutes);
  });

/** A commit on main, pushed `minutes` before NOW, and recorded green unless `green` is false. */
function commit(w: World, name: string, green = true, minutes = 0): string {
  writeFileSync(join(w.work, "app.txt"), `${name}\n`);
  git(w.work, "add", "-A");
  git(w.work, "commit", "-q", "-m", `change ${name}`);
  git(w.work, "push", "-q", "origin", "main");
  const sha = git(w.work, "rev-parse", "HEAD");
  w.sha[name] = sha;
  edit(w, (s) => {
    s.created[sha] = ago(minutes);
    if (green) s.green.push(sha);
  });
  return sha;
}

/** origin with main at c3 (all green; c1, c2 and c3 pushed 60, 50 and 40 minutes ago) and staging at
 * c1, pushed three hours ago. */
function world(): World {
  const root = mkdtempSync(join(tmpdir(), "promote-staging-"));
  roots.push(root);
  const w: World = { root, origin: join(root, "origin.git"), work: join(root, "work"), bin: join(root, "bin"), sha: {} };
  mkdirSync(w.bin);
  mkdirSync(join(root, "gh"));
  writeFileSync(join(w.bin, "gh"), FAKE_GH);
  chmodSync(join(w.bin, "gh"), 0o755);
  writeFileSync(
    stateFile(w),
    JSON.stringify({ green: [], created: {}, pushedAt: "", activityFails: false, super: [], issues: [], edits: [], started: [], startFails: false, log: [] } satisfies Gh),
  );
  git(root, "init", "-q", "--bare", "-b", "main", w.origin);
  git(root, "init", "-q", "-b", "main", w.work);
  git(w.work, "remote", "add", "origin", w.origin);
  commit(w, "c1", true, 60);
  git(w.work, "push", "-q", "origin", `${w.sha.c1}:refs/heads/staging`);
  commit(w, "c2", true, 50);
  commit(w, "c3", true, 40);
  pushedAgo(w, 180);
  return w;
}

interface Run { status: number | null; text: string; output: string }
let runs = 0;
function run(w: World, event: string, extra: Record<string, string> = {}): Run {
  const runner = join(w.root, `runner-${++runs}`);
  const output = join(w.root, `output-${runs}`);
  writeFileSync(output, "");
  git(w.root, "clone", "-q", w.origin, runner);
  const result = spawnSync("bash", [SCRIPT], {
    cwd: runner,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${w.bin}:${process.env.PATH ?? ""}`,
      GITHUB_REPOSITORY: "acme/game",
      GITHUB_STEP_SUMMARY: "/dev/null",
      GITHUB_OUTPUT: output,
      GH_TOKEN: "x",
      EVENT: event,
      NOW: String(NOW),
      INPUT_SHA: "",
      FAKE_GH_DIR: join(w.root, "gh"),
      ...extra,
    },
  });
  return { status: result.status, text: `${result.stdout}${result.stderr}`, output: readFileSync(output, "utf8") };
}
const ok = (r: Run): Run => {
  expect(r.status, r.text).toBe(0);
  return r;
};

describe("the move", () => {
  it("fast-forwards staging to main's newest green commit once its last move is two hours old", () => {
    const w = world();
    const r = ok(run(w, "schedule"));
    expect(staging(w)).toBe(w.sha.c3);
    expect(r.output).toBe(`moved=true\nsha=${w.sha.c3}\n`);
    expect(r.text).toContain(`Moved staging from ${w.sha.c1?.slice(0, 12)} to ${w.sha.c3?.slice(0, 12)} (2 commit(s))`);
    // A fast-forward: the old head is in the new one's history.
    expect(git(w.origin, "merge-base", "--is-ancestor", w.sha.c1 as string, "refs/heads/staging")).toBe("");
  });

  it("skips main's newer commits whose CI has not passed", () => {
    const w = world();
    commit(w, "c4", false);
    ok(run(w, "workflow_run"));
    expect(staging(w)).toBe(w.sha.c3);
  });

  it("waits, on the schedule and on a green CI run alike, while its last move is under two hours old", () => {
    const w = world();
    pushedAgo(w, 119);
    for (const event of ["schedule", "workflow_run"]) {
      const r = ok(run(w, event));
      expect(r.output).toBe("");
      expect(r.text).toContain("staging moved 119 minute(s) ago; it moves again once it is 120 old");
      expect(staging(w)).toBe(w.sha.c1);
    }
    pushedAgo(w, 120);
    expect(ok(run(w, "workflow_run")).output).toContain("moved=true");
    expect(staging(w)).toBe(w.sha.c3);
  });

  it("takes its pace from STAGING_EVERY_MINUTES", () => {
    const w = world();
    pushedAgo(w, 30);
    expect(ok(run(w, "schedule", { STAGING_EVERY_MINUTES: "20" })).output).toContain("moved=true");
  });

  it("moves when when staging was last pushed cannot be read, and says so", () => {
    const w = world();
    pushedAgo(w, 5);
    edit(w, (s) => {
      s.activityFails = true;
    });
    const r = ok(run(w, "schedule"));
    expect(r.text).toContain("::notice::When staging was last pushed could not be read; moving it now.");
    expect(staging(w)).toBe(w.sha.c3);
  });

  it("moves at once by hand, to the commit named or the newest green one", () => {
    const w = world();
    pushedAgo(w, 1);
    ok(run(w, "workflow_dispatch", { INPUT_SHA: w.sha.c2 as string }));
    expect(staging(w)).toBe(w.sha.c2);
    ok(run(w, "workflow_dispatch"));
    expect(staging(w)).toBe(w.sha.c3);
  });

  it("never moves staging backwards: a commit it already has is a no-op", () => {
    const w = world();
    ok(run(w, "schedule"));
    for (const sha of [w.sha.c3, w.sha.c2] as string[]) {
      const r = ok(run(w, "workflow_dispatch", { INPUT_SHA: sha }));
      expect(r.output).toBe("");
      expect(r.text).toContain("nothing to move");
      expect(staging(w)).toBe(w.sha.c3);
    }
    // A CI run passing again on an old commit makes it the newest green run.
    edit(w, (s) => s.green.push(w.sha.c2 as string));
    ok(run(w, "schedule"));
    expect(staging(w)).toBe(w.sha.c3);
  });
});

describe("the freeze: staging holds still while the super run on it is red (the owners' call of 2026-11-10)", () => {
  const GATE = "CI: daily super run failed";
  const URL = "https://github.com/acme/game/actions/runs/9";
  /** A finished super run on staging's commit `sha`, ended `minutes` before NOW (55: before every close below). */
  const finished = (sha: string, conclusion: string, minutes = 55): SuperRun => ({ status: "completed", conclusion, headSha: sha, updatedAt: ago(minutes), url: URL });
  const frozen = (w: World, issues: Issue[] = [{ number: 7, title: GATE, state: "OPEN", closedAt: null }]): void =>
    edit(w, (s) => {
      s.super = [finished(w.sha.c1 as string, "failure")];
      s.issues = issues;
    });

  it("holds on the schedule and on green CI alike while the red run's issue is open, and a manual run moves it all the same", () => {
    const w = world();
    frozen(w);
    for (const event of ["schedule", "workflow_run"]) {
      const r = ok(run(w, event));
      expect(r.output).toBe("");
      expect(r.text).toContain(`staging holds still: the super run on it ended failure on \`${w.sha.c1?.slice(0, 12)}\` (${URL}), so production waits on a fix: #7. It moves once #7 closes.`);
      expect(staging(w)).toBe(w.sha.c1);
    }
    expect(gh(w).started).toEqual([]);
    ok(run(w, "workflow_dispatch"));
    expect(staging(w)).toBe(w.sha.c3);
  });

  it("moves as ever when the newest finished run passed, a cancelled run newer than it included, or there is none", () => {
    const w = world();
    edit(w, (s) => {
      s.super = [finished(w.sha.c1 as string, "cancelled", 5), finished(w.sha.c1 as string, "success")];
    });
    expect(ok(run(w, "schedule")).output).toContain("moved=true");
    expect(gh(w).started).toEqual([]);
    const fresh = world();
    expect(ok(run(fresh, "schedule")).output).toContain("moved=true");
  });

  it("holds while a super run on staging is under way, even with the red run's issue closed", () => {
    const w = world();
    frozen(w, [{ number: 7, title: GATE, state: "CLOSED", closedAt: ago(45) }]);
    edit(w, (s) => s.super.unshift({ status: "in_progress", conclusion: null, headSha: w.sha.c1 as string, updatedAt: ago(1), url: URL }));
    const r = ok(run(w, "workflow_run"));
    expect(r.text).toContain("so production waits on a fix, and a super run on staging is under way.");
    expect(staging(w)).toBe(w.sha.c1);
    expect(gh(w).started).toEqual([]);
  });

  it("once the issue closes after the red run, moves at once to main's newest green commit pushed since and starts the super run on it", () => {
    const w = world();
    // c2 (50 minutes ago) predates the close and may lack the fix; c3 (40) came after it.
    frozen(w, [{ number: 7, title: GATE, state: "CLOSED", closedAt: ago(45) }]);
    pushedAgo(w, 5);
    const r = ok(run(w, "workflow_run"));
    expect(r.text).toContain(`#7 closed at ${ago(45)}, after the red run: staging moves now to test the fix.`);
    expect(r.output).toBe(`moved=true\nsha=${w.sha.c3}\n`);
    expect(staging(w)).toBe(w.sha.c3);
    expect(gh(w).started).toEqual(["super.yml --ref staging"]);
    expect(r.text).toContain(`Started the super run on staging (\`${w.sha.c3?.slice(0, 12)}\`).`);
    expect(gh(w).log).toContain(`run list --workflow ci.yml --branch main --event push --status success --limit 1 --created >=${ago(45)} --json headSha --jq .[0].headSha // ""`);
  });

  it("waits for a green commit pushed after the close, never one from before it", () => {
    const w = world();
    frozen(w, [{ number: 7, title: GATE, state: "CLOSED", closedAt: ago(30) }]);
    expect(ok(run(w, "schedule")).text).toContain(`#7 closed at ${ago(30)}; it moves to main's first commit after that whose CI passes.`);
    expect(staging(w)).toBe(w.sha.c1);
    commit(w, "c4", false, 20);
    ok(run(w, "workflow_run"));
    expect(staging(w)).toBe(w.sha.c1);
    edit(w, (s) => s.green.push(w.sha.c4 as string));
    expect(ok(run(w, "workflow_run")).output).toBe(`moved=true\nsha=${w.sha.c4}\n`);
    expect(staging(w)).toBe(w.sha.c4);
    expect(gh(w).started).toEqual(["super.yml --ref staging"]);
  });

  it("starts the super run on a staging that moved since the red run (by hand) with nothing testing it, and holds", () => {
    const w = world();
    frozen(w);
    ok(run(w, "workflow_dispatch", { INPUT_SHA: w.sha.c2 as string }));
    pushedAgo(w, 180);
    const r = ok(run(w, "schedule"));
    expect(r.text).toContain(`staging has moved since, to \`${w.sha.c2?.slice(0, 12)}\`, and nothing is testing it.`);
    expect(staging(w)).toBe(w.sha.c2);
    expect(gh(w).started).toEqual(["super.yml --ref staging"]);
  });

  it("a super run that does not start is a warning, and the next run starts it", () => {
    const w = world();
    frozen(w, [{ number: 7, title: GATE, state: "CLOSED", closedAt: ago(45) }]);
    edit(w, (s) => {
      s.startFails = true;
    });
    const r = ok(run(w, "workflow_run"));
    expect(r.text).toContain("::warning::Could not start the super run on staging; the next run of this tries again.");
    expect(staging(w)).toBe(w.sha.c3);
    edit(w, (s) => {
      s.startFails = false;
    });
    ok(run(w, "workflow_run"));
    expect(gh(w).started).toEqual(["super.yml --ref staging"]);
  });

  it("opens the issue a red run's report never opened, once REPORT_GRACE_MINUTES have passed, and hands it to the night bot", () => {
    const w = world();
    // The newest issue closed before the run ended, so it is not this run's.
    frozen(w, [{ number: 7, title: GATE, state: "CLOSED", closedAt: ago(90) }]);
    edit(w, (s) => {
      s.super = [finished(w.sha.c1 as string, "failure", 10)];
    });
    let r = ok(run(w, "schedule"));
    expect(r.text).toContain("and its issue is not open yet.");
    expect(gh(w).issues).toHaveLength(1);
    r = ok(run(w, "schedule", { REPORT_GRACE_MINUTES: "5" }));
    expect(r.text).toContain("and no issue was opened for it.");
    const opened = gh(w).issues[1];
    expect(opened?.title).toBe(GATE);
    expect(opened?.body).toContain(`[The super run](${URL}) on \`staging\` (\`${w.sha.c1?.slice(0, 7)}\`) failed, and no issue was opened for it`);
    expect(gh(w).edits).toEqual([{ number: opened?.number, assignee: "jgoetzmann-bot", labels: ["bot:build", "priority:high", "difficulty:hard"] }]);
    expect(staging(w)).toBe(w.sha.c1);
    // Open now: it holds on that issue.
    expect(ok(run(w, "schedule")).text).toContain(`It moves once #${opened?.number} closes.`);
    expect(gh(w).edits).toHaveLength(1);
  });

  it("reads the super run's issue by the title super.yml's report gives it", () => {
    const script = readFileSync(SCRIPT, "utf8");
    expect(/: "\$\{GATE_TITLE:=([^}]+)\}"/u.exec(script)?.[1]).toBe(GATE);
    expect(SUPER).toContain(`DAILY_TITLE: "${GATE}"`);
    expect(/: "\$\{BOT_LOGIN:=([^}]+)\}"/u.exec(script)?.[1]).toBe(/BOT_LOGIN: (\S+)/u.exec(SUPER)?.[1]);
  });
});

describe("what it refuses", () => {
  it("a commit whose CI has not passed, one not on main, and one that is not a commit", () => {
    const w = world();
    const red = commit(w, "c4", false);
    let r = run(w, "workflow_dispatch", { INPUT_SHA: red });
    expect(r.status).not.toBe(0);
    expect(r.text).toContain(`\`${red.slice(0, 12)}\` has no passing push-to-main CI run.`);

    git(w.work, "checkout", "-q", "-b", "side");
    writeFileSync(join(w.work, "side.txt"), "x\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "side");
    git(w.work, "push", "-q", "origin", "side");
    const side = git(w.work, "rev-parse", "HEAD");
    edit(w, (s) => s.green.push(side));
    r = run(w, "workflow_dispatch", { INPUT_SHA: side });
    expect(r.status).not.toBe(0);
    expect(r.text).toContain(`\`${side.slice(0, 12)}\` is not on main.`);

    r = run(w, "workflow_dispatch", { INPUT_SHA: "abcdef1234" });
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("`abcdef1234` is not a commit.");
    expect(staging(w)).toBe(w.sha.c1);
  });

  it("a staging that holds something main does not, which it leaves for a person", () => {
    const w = world();
    git(w.work, "checkout", "-q", "-b", "hand", w.sha.c1 as string);
    writeFileSync(join(w.work, "hand.txt"), "x\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "by hand");
    git(w.work, "push", "-q", "origin", "hand:staging");
    const before = staging(w);
    const r = run(w, "schedule");
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("is not an ancestor of");
    expect(staging(w)).toBe(before);
  });

  it("a repository with no staging branch, saying how to make one", () => {
    const w = world();
    git(w.origin, "update-ref", "-d", "refs/heads/staging");
    const r = run(w, "schedule");
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("The staging branch does not exist.");
  });

  it("an event it does not know", () => {
    const w = world();
    const r = run(w, "push");
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("does not handle the push event");
  });
});

describe("the workflows", () => {
  it("runs every two hours, on each green push to main and by hand, one at a time", () => {
    expect(WORKFLOW).toMatch(/^ {2}schedule:\n {4}- cron: "5 \*\/2 \* \* \*"\n/mu);
    expect(WORKFLOW).toMatch(/^ {2}workflow_run:\n {4}workflows: \[CI\]\n {4}types: \[completed\]\n {4}branches: \[main\]\n/mu);
    expect(WORKFLOW).toContain("(github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.event == 'push')");
    expect(WORKFLOW).toMatch(/^ {6}sha:\n/mu);
    expect(WORKFLOW).toContain('          STAGING_EVERY_MINUTES: "120"\n');
    expect(WORKFLOW).toMatch(/^ {4}concurrency:\n {6}group: promote-staging\n {6}cancel-in-progress: false$/mu);
  });

  it("may start the super run and open an issue, for the freeze", () => {
    expect(WORKFLOW).toMatch(/^ {4}permissions:\n {6}contents: read\n {6}actions: write\n {6}issues: write\n {4}outputs:\n/mu);
  });

  it("pushes with the bot's token, so the push may carry workflow files and starts the deploy", () => {
    expect(WORKFLOW).toContain("          fetch-depth: 0\n          token: ${{ secrets.BOT_GITHUB_TOKEN || github.token }}\n");
  });

  it("hands a failed move to Squishy, and closes it when a move goes through", () => {
    expect(WORKFLOW).toContain('TITLE: "CI: staging promotion failed"');
    expect(WORKFLOW).toContain('--add-assignee "$SQUISHY_LOGIN" --add-label "$SQUISHY_QUEUE"');
    expect(WORKFLOW).toContain("needs.promote.result == 'failure' || needs.promote.outputs.moved == 'true'");
  });

  it("deploys staging on each push of the staging branch, not on main's CI", () => {
    expect(DEPLOY).toMatch(/^on:\n {2}push:\n {4}branches: \[staging\]\n {2}workflow_dispatch:\n/mu);
    expect(DEPLOY).not.toContain("workflow_run");
    expect(DEPLOY).toContain('compare/$sha...staging');
  });
});
