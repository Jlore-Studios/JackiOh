// scripts/promote-production.sh, the logic of .github/workflows/promote-production.yml: production
// follows the super run. When the daily super run on main passes, the commit it tested merges into
// production at once (the daily promotion, the owners' call of 2026-11-03). `/promote` on the one
// production issue goes sooner: it opens a pull request of a commit of main into production, the
// super run tests it, and it merges once that run passed; `/skip-super` with a reason merges at once
// (#2750, the owner's calls of 2026-11-02). Each case runs the real
// script in a fresh clone of a throwaway "origin", with a stand-in for `gh` (below) that keeps issues,
// pull requests, comments, reactions and CI and super runs in a JSON file and does the merge for real
// with git, so what production ends up holding is what GitHub would leave it.

import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it, vi } from "vitest";

// A case runs the script a few times, and each run is a clone and a dozen short-lived processes.
vi.setConfig({ testTimeout: 60_000 });

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = join(ROOT, "scripts/promote-production.sh");
const WORKFLOW = readFileSync(join(ROOT, ".github/workflows/promote-production.yml"), "utf8");
const SUPER = readFileSync(join(ROOT, ".github/workflows/super.yml"), "utf8");
const LABEL = "production merge";
// Both people, and `human` so the night bot never picks these up.
const PEOPLE = ["jgoetzmann", "MaxGoetzmann"];

// The stand-in for `gh`

const FAKE_GH = String.raw`#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const cp = require("child_process");
const dir = process.env.FAKE_GH_DIR;
const origin = process.env.FAKE_GH_ORIGIN;
const file = path.join(dir, "state.json");
const st = JSON.parse(fs.readFileSync(file, "utf8"));
const argv = process.argv.slice(2);
const VALUE = new Set(["--title", "--label", "--body-file", "--body", "--base", "--head", "--state", "--limit", "--json", "--jq",
  "--reason", "--comment", "--color", "--description", "--workflow", "--branch", "--event", "--status", "--commit",
  "--match-head-commit", "--assignee", "-X", "-f"]);
const pos = [];
const flag = {};
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (VALUE.has(a)) (flag[a] = flag[a] || []).push(argv[++i]);
  else if (a.startsWith("-")) flag[a] = [true];
  else pos.push(a);
}
const f = (k) => (flag[k] ? flag[k][0] : undefined);
const iso = () => new Date(Number(process.env.NOW) * 1000).toISOString().replace(/\.\d+Z$/, "Z");
const git = (cwd, ...a) =>
  cp.execFileSync("git", ["-c", "user.name=gh", "-c", "user.email=gh@example.com", "-c", "commit.gpgsign=false", ...a], { cwd, encoding: "utf8" }).trim();
// What a call prints goes out only after its state is saved (the last line). The script reads the
// comments through a process substitution and reacts to each one while that call is still running,
// so a call that printed first and saved last overwrote the reaction saved in between.
let printed = "";
function out(data) {
  if (f("--jq") === undefined) return void (printed += JSON.stringify(data) + "\n");
  const r = cp.spawnSync("jq", ["-r", f("--jq")], { input: JSON.stringify(data), encoding: "utf8" });
  if (r.status !== 0) { process.stderr.write(r.stderr); process.exit(1); }
  printed += r.stdout;
}
const bot = (n, body) => st.issues.find((i) => i.number === n).comments.push({ id: st.nextComment++, login: "github-actions[bot]", type: "Bot", assoc: "NONE", at: iso(), body });
const issue = (n) => st.issues.find((i) => i.number === Number(n));
st.log.push(argv.join(" "));
if ((st.fail || []).some((prefix) => argv.join(" ").startsWith(prefix))) {
  process.stderr.write("HTTP 403: Resource not accessible by integration\n");
  process.exit(1);
}
const [a, b] = pos;
// Which token each call went out with, and what it was: "pr merge", "issue close", "api DELETE".
st.tokens.push(process.env.GH_TOKEN + " " + a + " " + (a === "api" ? (f("-X") || "GET") : b));
if (a === "run" && b === "list" && f("--workflow") === "super.yml" && f("--commit") === undefined) {
  // The super runs on main, newest first: the daily promotion's.
  out([...st.mainSuper].reverse().filter((r) => f("--status") === undefined || r.conclusion === f("--status")));
} else if (a === "run" && b === "list" && f("--workflow") === "super.yml") {
  // The super run on a promotion pull request's head commit, newest first.
  const r = st.super[f("--commit")];
  out(r ? [r] : []);
} else if (a === "run" && b === "list" && f("--status") === undefined && f("--commit") === undefined) {
  // Every push-to-main run, for the queue's CI column.
  out([
    ...st.green.map((headSha) => ({ headSha, status: "completed", conclusion: "success" })),
    ...st.failed.map((headSha) => ({ headSha, status: "completed", conclusion: "failure" })),
    ...st.running.map((headSha) => ({ headSha, status: "in_progress", conclusion: null })),
  ]);
} else if (a === "run" && b === "list") {
  const sha = f("--commit") !== undefined ? (st.green.includes(f("--commit")) ? f("--commit") : "") : st.newestGreen;
  out(sha ? [{ headSha: sha }] : []);
} else if (a === "label") {
  if (!st.labels.includes(pos[2])) st.labels.push(pos[2]);
} else if (a === "issue" && b === "list") {
  out(st.issues.filter((i) => i.state === "open" && i.labels.includes(f("--label"))).map((i) => ({ number: i.number })));
} else if (a === "issue" && b === "view") {
  out({ body: issue(pos[2]).body, title: issue(pos[2]).title });
} else if (a === "issue" && b === "create") {
  const number = st.next++;
  st.issues.push({ number, title: f("--title"), body: fs.readFileSync(f("--body-file"), "utf8"), labels: flag["--label"], assignees: (f("--assignee") || "").split(","), state: "open", reason: "", comments: [] });
  printed += "https://github.com/acme/game/issues/" + number + "\n";
} else if (a === "issue" && b === "edit") {
  const i = issue(pos[2]);
  if (f("--title") !== undefined) i.title = f("--title");
  if (f("--body-file") !== undefined) i.body = fs.readFileSync(f("--body-file"), "utf8");
} else if (a === "issue" && b === "close") {
  const i = issue(pos[2]);
  i.state = "closed";
  i.reason = f("--reason");
  if (f("--comment") !== undefined) bot(i.number, f("--comment"));
} else if (a === "issue" && b === "comment") {
  bot(Number(pos[2]), f("--body"));
} else if (a === "pr" && b === "list") {
  out(st.prs.filter((p) => p.state === "open" && p.base === f("--base") && (f("--head") === undefined || p.head === f("--head")))
    .map((p) => ({ number: p.number, headRefName: p.head, headRefOid: git(origin, "rev-parse", "refs/heads/" + p.head) })));
} else if (a === "pr" && b === "create") {
  const number = st.next++;
  st.prs.push({ number, head: f("--head"), base: f("--base"), title: f("--title"), body: fs.readFileSync(f("--body-file"), "utf8"), labels: flag["--label"], assignees: (f("--assignee") || "").split(","), state: "open", merged: false, comments: [] });
  printed += "https://github.com/acme/game/pull/" + number + "\n";
} else if (a === "pr" && b === "close") {
  const p = st.prs.find((x) => x.number === Number(pos[2]));
  p.state = "closed";
  if (f("--comment") !== undefined) p.comments.push(f("--comment"));
} else if (a === "pr" && b === "comment") {
  st.prs.find((x) => x.number === Number(pos[2])).comments.push(f("--body"));
} else if (a === "pr" && b === "merge") {
  const p = st.prs.find((x) => x.number === Number(pos[2]));
  if (st.failMerges > 0) { st.failMerges--; fs.writeFileSync(file, JSON.stringify(st)); process.stderr.write("Pull request is not mergeable\n"); process.exit(1); }
  const tmp = fs.mkdtempSync(path.join(dir, "merge-"));
  git(dir, "clone", "-q", origin, tmp);
  if (git(tmp, "rev-parse", "origin/" + p.head) !== f("--match-head-commit")) { process.stderr.write("head moved\n"); process.exit(1); }
  git(tmp, "checkout", "-q", p.base);
  try { git(tmp, "merge", "--no-ff", "-q", "-m", "Merge pull request #" + p.number + " from " + p.head, "origin/" + p.head); }
  catch (e) { process.stderr.write("merge conflict\n"); process.exit(1); }
  git(tmp, "push", "-q", "origin", p.base);
  p.state = "closed"; p.merged = true;
} else if (a === "api") {
  let m;
  if ((m = /^repos\/[^/]+\/[^/]+\/issues\/(\d+)\/comments$/.exec(b))) {
    out(issue(m[1]).comments.map((c) => ({ id: c.id, user: { login: c.login, type: c.type }, author_association: c.assoc, created_at: c.at, body: c.body })));
  } else if ((m = /^repos\/[^/]+\/[^/]+\/issues\/comments\/(\d+)\/reactions$/.exec(b))) {
    st.reactions.push({ comment: Number(m[1]), content: f("-f").replace(/^content=/, "") });
  } else if ((m = /^repos\/[^/]+\/[^/]+\/git\/refs\/heads\/(.+)$/.exec(b)) && f("-X") === "DELETE") {
    git(origin, "update-ref", "-d", "refs/heads/" + m[1]);
  } else { process.stderr.write("fake gh: unsupported api call " + argv.join(" ") + "\n"); process.exit(2); }
} else { process.stderr.write("fake gh: unsupported " + argv.join(" ") + "\n"); process.exit(2); }
fs.writeFileSync(file, JSON.stringify(st));
process.stdout.write(printed);
`;

interface Comment { id: number; login: string; type: string; assoc: string; at: string; body: string }
interface Issue { number: number; title: string; body: string; labels: string[]; assignees?: string[]; state: "open" | "closed"; reason: string; comments: Comment[] }
interface Pr { number: number; head: string; base: string; title: string; body: string; labels: string[]; assignees?: string[]; state: string; merged: boolean; comments: string[] }
interface SuperRun { databaseId: number; attempt: number; status: string; conclusion: string | null; url: string }
interface MainSuperRun { databaseId: number; headSha: string; conclusion: string; url: string }
interface Gh {
  next: number;
  nextComment: number;
  issues: Issue[];
  prs: Pr[];
  reactions: { comment: number; content: string }[];
  labels: string[];
  green: string[];
  failed: string[];
  running: string[];
  newestGreen: string;
  super: Record<string, SuperRun>;
  mainSuper: MainSuperRun[];
  failMerges: number;
  fail: string[];
  log: string[];
  tokens: string[];
}

// A world: origin, a working copy, the stand-in

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

/** A commit on main (pushed, and recorded green); `files` are written whole. */
function commit(w: World, name: string, files: Record<string, string>, green = true, subject = `change ${name}`): string {
  if (Object.keys(w.sha).length > 0) git(w.work, "checkout", "-q", "main");
  for (const [file, text] of Object.entries(files)) {
    mkdirSync(dirname(join(w.work, file)), { recursive: true });
    writeFileSync(join(w.work, file), text);
  }
  git(w.work, "add", "-A");
  git(w.work, "commit", "-q", "-m", subject);
  git(w.work, "push", "-q", "origin", "main");
  const sha = git(w.work, "rev-parse", "HEAD");
  w.sha[name] = sha;
  edit(w, (s) => {
    if (green) {
      s.green.push(sha);
      s.newestGreen = sha;
    }
  });
  return sha;
}

/** crates/cards/patches/patches.json, whose newest entry is the catalog version a commit ships. */
const PATCHES = "crates/cards/patches/patches.json";
const patches = (...versions: string[]): string => `${JSON.stringify(versions.map((version) => ({ version })), null, 2)}\n`;

/** origin with main at c3 (all green) and production at c1, as after a few days' work. */
function world(): World {
  const root = mkdtempSync(join(tmpdir(), "promote-"));
  roots.push(root);
  const w: World = { root, origin: join(root, "origin.git"), work: join(root, "work"), bin: join(root, "bin"), sha: {} };
  mkdirSync(w.bin);
  mkdirSync(join(root, "gh"));
  writeFileSync(join(w.bin, "gh"), FAKE_GH);
  chmodSync(join(w.bin, "gh"), 0o755);
  git(root, "init", "-q", "--bare", "-b", "main", w.origin);
  git(root, "init", "-q", "-b", "main", w.work);
  git(w.work, "remote", "add", "origin", w.origin);
  const empty: Gh = { next: 1, nextComment: 1000, issues: [], prs: [], reactions: [], labels: [], green: [], failed: [], running: [], newestGreen: "", super: {}, mainSuper: [], failMerges: 0, fail: [], log: [], tokens: [] };
  writeFileSync(stateFile(w), JSON.stringify(empty));
  commit(w, "c1", { [PATCHES]: patches("v0.2.0a", "v0.2.0b"), "app.txt": "one\n" });
  git(w.work, "push", "-q", "origin", `${w.sha.c1}:refs/heads/production`);
  commit(w, "c2", { "app.txt": "two\n" });
  commit(w, "c3", { "app.txt": "three\n" });
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
      GITHUB_RUN_ID: "",
      GH_TOKEN: "x",
      EVENT: event,
      NOW: String(Date.parse("2026-11-02T18:00:00Z") / 1000),
      MERGE_RETRY_SECONDS: "0",
      FAKE_GH_DIR: join(w.root, "gh"),
      FAKE_GH_ORIGIN: w.origin,
      INPUT_SHA: "",
      INPUT_SKIP_REASON: "",
      ACTOR: "runner",
      ...extra,
    },
  });
  return { status: result.status, text: `${result.stdout}${result.stderr}`, output: readFileSync(output, "utf8") };
}
const ok = (r: Run): Run => {
  expect(r.status, r.text).toBe(0);
  return r;
};

const issue = (w: World, number: number): Issue => {
  const found = gh(w).issues.find((i) => i.number === number);
  if (found === undefined) throw new Error(`no issue ${number}`);
  return found;
};
const pr = (w: World, number: number): Pr => {
  const found = gh(w).prs.find((p) => p.number === number);
  if (found === undefined) throw new Error(`no pull request ${number}`);
  return found;
};
const openIssues = (w: World): Issue[] => gh(w).issues.filter((i) => i.state === "open");
const openPrs = (w: World): Pr[] => gh(w).prs.filter((p) => p.state === "open");
const marker = (i: Issue): string => /promote-production (last=\d+ reported=[0-9.-]+)/u.exec(i.body)?.[1] ?? "";
const daily = (i: Issue): string => /promote-production last=\d+ reported=[0-9.-]+ daily=([0-9-]+) -->/u.exec(i.body)?.[1] ?? "";
const production = (w: World): string => git(w.origin, "rev-parse", "refs/heads/production");
const tree = (w: World, rev: string): string => git(w.origin, "rev-parse", `${rev}^{tree}`);
const head = (w: World, p: Pr): string => git(w.origin, "rev-parse", `refs/heads/${p.head}`);
const short = (sha: string | undefined, n = 7): string => (sha ?? "").slice(0, n);
const lastReply = (w: World): string => issue(w, 1).comments.at(-1)?.body ?? "";
const comment = (w: World, body: string, who: { login?: string; assoc?: string; type?: string } = {}): number => {
  let id = 0;
  edit(w, (s) => {
    id = s.nextComment++;
    s.issues.find((i) => i.number === 1)?.comments.push({ id, login: who.login ?? "maintainer", type: who.type ?? "User", assoc: who.assoc ?? "MEMBER", at: "2026-11-02T17:00:00Z", body });
  });
  return id;
};
const reactions = (w: World, id: number): string[] => gh(w).reactions.filter((r) => r.comment === id).map((r) => r.content);
/** The super run on commit `sha`, as `gh run list --workflow super.yml` reports it. */
const superRun = (w: World, sha: string, conclusion: string | null, attempt = 1, status = "completed"): void =>
  edit(w, (s) => {
    s.super[sha] = { databaseId: 77, attempt, status, conclusion, url: "https://github.com/acme/game/actions/runs/77" };
  });

/** A super run on main, as `gh run list --workflow super.yml --branch main` reports it. */
const mainRun = (w: World, sha: string, id: number, conclusion = "success"): void =>
  edit(w, (s) => {
    s.mainSuper.push({ databaseId: id, headSha: sha, conclusion, url: `https://github.com/acme/game/actions/runs/${id}` });
  });

/** The first run, which opens the production issue, #1; most cases start from there. */
function started(): World {
  const w = world();
  ok(run(w, "workflow_run"));
  expect(openIssues(w).map((i) => i.number)).toEqual([1]);
  return w;
}
/** A comment on the production issue, and the run it wakes. */
function say(w: World, body: string, who: { login?: string; assoc?: string; type?: string } = {}): { id: number; run: Run } {
  const id = comment(w, body, who);
  return { id, run: run(w, "issue_comment") };
}
/** `/promote` from a member: the promotion pull request of c3, #2. */
function promoted(): World {
  const w = started();
  ok(say(w, "/promote").run);
  expect(openPrs(w).map((p) => p.number)).toEqual([2]);
  return w;
}

describe("the production issue", () => {
  it("opens one on its first run, labeled and assigned, with the queue and the commands, and moves nothing", () => {
    const w = world();
    const before = production(w);
    const r = ok(run(w, "workflow_run"));
    const [only, ...rest] = openIssues(w);
    expect(rest).toEqual([]);
    expect(only).toMatchObject({ number: 1, title: "Production: 2 commits on main are not live yet", labels: [LABEL, "human"], assignees: PEOPLE });
    expect(marker(only as Issue)).toBe("last=0 reported=-");
    expect(daily(only as Issue)).toBe("-");
    for (const command of ["`/promote`", "`/promote <sha>`", "`/skip-super <reason>`", "`/cancel`"]) {
      expect(only?.body).toContain(`| ${command} |`);
    }
    expect(only?.body).toContain("**Production** promotes itself once a day: when the daily super run on `main` passes every job, the commit it tested merges into `production` and deploys.");
    expect(gh(w).labels).toEqual([LABEL]);
    expect(gh(w).prs).toEqual([]);
    expect(production(w)).toBe(before);
    expect(r.output).not.toContain("promoted=true");
  });

  it("keeps the oldest, the one that stays open, and closes any newer duplicate as not planned", () => {
    const w = started();
    edit(w, (s) => {
      s.issues.push({ number: 9, title: "dup", body: "", labels: [LABEL], state: "open", reason: "", comments: [] });
    });
    ok(run(w, "workflow_run"));
    expect(openIssues(w).map((i) => i.number)).toEqual([1]);
    expect(issue(w, 9)).toMatchObject({ state: "closed", reason: "not planned" });
  });

  it("takes over the countdown's issue: its old state line keeps `last`, so its old comments are not read again", () => {
    const w = world();
    edit(w, (s) => {
      s.issues.push({
        number: 1,
        title: "Merging to production: on hold (was due 2026-11-02 10:00 AM CT)",
        body: "<!-- promote-production due=1793632800 held=1 last=1000 -->\n**Production: on hold.**",
        labels: [LABEL, "human"],
        state: "open",
        reason: "",
        comments: [{ id: 1000, login: "maintainer", type: "User", assoc: "MEMBER", at: "2026-11-02T09:00:00Z", body: "/hold moving to Cloudflare" }],
      });
      s.nextComment = 1001;
      s.next = 2;
    });
    ok(run(w, "workflow_run"));
    expect(issue(w, 1).title).toBe("Production: 2 commits on main are not live yet");
    expect(marker(issue(w, 1))).toBe("last=1000 reported=-");
    expect(issue(w, 1).body).not.toContain("on hold");
    ok(say(w, "/promote").run);
    expect(reactions(w, 1000)).toEqual([]);
    expect(openPrs(w)).toHaveLength(1);
  });

  it("starts its state afresh when someone deleted the state line", () => {
    const w = started();
    edit(w, (s) => {
      const first = s.issues[0];
      if (first !== undefined) first.body = "edited by hand";
    });
    const r = ok(run(w, "workflow_run"));
    expect(r.text).toContain("has lost its state line");
    expect(marker(issue(w, 1))).toBe("last=0 reported=-");
  });
});

describe("a green CI run on main", () => {
  it("only brings the queue up to date", () => {
    const w = started();
    const before = production(w);
    commit(w, "c4", { "app.txt": "four\n" });
    const r = ok(run(w, "workflow_run"));
    expect(r.output).not.toContain("promoted=true");
    expect(production(w)).toBe(before);
    expect(gh(w).prs).toEqual([]);
    expect(issue(w, 1).title).toBe("Production: 3 commits on main are not live yet");
  });

  it("refuses a scheduled run, or any event it does not know: the daily promotion runs off super's own schedule", () => {
    const w = world();
    for (const event of ["schedule", "push"]) {
      const r = run(w, event);
      expect(r.status).not.toBe(0);
      expect(r.text).toContain(`does not handle the ${event} event: production follows a passing super run`);
    }
    expect(gh(w).issues).toEqual([]);
  });
});

describe("the daily promotion", () => {
  it("merges the commit a passing super run on main tested, at once, with no second super run, and starts the deploy", () => {
    const w = started();
    mainRun(w, w.sha.c3 as string, 501);
    edit(w, (s) => {
      s.tokens = [];
    });
    const r = ok(run(w, "workflow_run", { GH_TOKEN: "actions", PROMOTE_TOKEN: "bot" }));
    expect(r.output).toContain("promoted=true");
    const p = pr(w, 2);
    expect(p).toMatchObject({ base: "production", state: "closed", merged: true, labels: [LABEL, "human"], assignees: PEOPLE });
    expect(p.body).toContain("The daily promotion: the super run on `main` passed every job on this commit ([run](https://github.com/acme/game/actions/runs/501)), so this merges at once");
    expect(p.body).toContain("- Started by: the daily promotion\n");
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
    expect(git(w.origin, "for-each-ref", "refs/heads/promote")).toBe("");
    expect(lastReply(w)).toContain(
      `Merged to production in #2, the daily promotion: the super run on \`main\` passed every job on \`${short(w.sha.c3, 12)}\` ([run](https://github.com/acme/game/actions/runs/501)). 2 commit(s), catalog v0.2.0b; the deploy follows.`,
    );
    expect(daily(issue(w, 1))).toBe("501");
    expect(issue(w, 1).title).toBe("Production: everything on main is live");
    // Opened with the Actions token, whose pull request starts no workflow, so super does not run on
    // it again; moved with the bot's.
    const calls = (token: string): string[] =>
      gh(w).tokens.filter((c) => c.startsWith(`${token} `)).map((c) => c.slice(token.length + 1));
    expect(calls("actions")).toContain("pr create");
    expect(calls("bot")).toEqual(["pr merge", "api DELETE"]);
  });

  it("acts on each passing run once, so a /cancel or a newer commit never brings it back", () => {
    const w = started();
    mainRun(w, w.sha.c2 as string, 501);
    ok(run(w, "workflow_run"));
    expect(tree(w, "production")).toBe(tree(w, w.sha.c2 as string));
    commit(w, "c4", { "app.txt": "four\n" });
    const r = ok(run(w, "workflow_run"));
    expect(r.output).not.toContain("promoted=true");
    expect(gh(w).prs).toHaveLength(1);
    mainRun(w, w.sha.c4 as string, 502);
    expect(ok(run(w, "workflow_run")).output).toContain("promoted=true");
    expect(tree(w, "production")).toBe(tree(w, w.sha.c4 as string));
    expect(daily(issue(w, 1))).toBe("502");
  });

  it("ignores a red run on main, and a passing one on a commit production already has", () => {
    const w = started();
    mainRun(w, w.sha.c3 as string, 501, "failure");
    expect(ok(run(w, "workflow_run")).output).not.toContain("promoted=true");
    expect(gh(w).prs).toEqual([]);
    mainRun(w, w.sha.c1 as string, 502);
    expect(ok(run(w, "workflow_run")).output).not.toContain("promoted=true");
    expect(gh(w).prs).toEqual([]);
    expect(daily(issue(w, 1))).toBe("502");
  });

  it("says why when the commit it tested has not passed CI, and moves nothing", () => {
    const w = started();
    const red = commit(w, "c4", { "app.txt": "four\n" }, false);
    mainRun(w, red, 501);
    expect(ok(run(w, "workflow_run")).output).not.toContain("promoted=true");
    expect(gh(w).prs).toEqual([]);
    expect(lastReply(w)).toBe(`The super run on \`main\` passed on \`${short(red, 12)}\` ([run](https://github.com/acme/game/actions/runs/501)), but it is not promoted: \`${short(red, 12)}\` has no passing push-to-main CI run.`);
  });

  it("merges an open /promote of the same commit, leaves one of a newer commit to its own run, and supersedes an older one", () => {
    const same = promoted();
    mainRun(same, same.sha.c3 as string, 501);
    expect(ok(run(same, "workflow_run")).output).toContain("promoted=true");
    expect(pr(same, 2).merged).toBe(true);

    const newer = promoted();
    mainRun(newer, newer.sha.c2 as string, 501);
    expect(ok(run(newer, "workflow_run")).output).not.toContain("promoted=true");
    expect(pr(newer, 2).state).toBe("open");
    expect(daily(issue(newer, 1))).toBe("501");

    const older = started();
    ok(say(older, `/promote ${short(older.sha.c2, 12)}`).run);
    mainRun(older, older.sha.c3 as string, 501);
    expect(ok(run(older, "workflow_run")).output).toContain("promoted=true");
    expect(pr(older, 2)).toMatchObject({ state: "closed", merged: false });
    expect(pr(older, 3)).toMatchObject({ merged: true });
    expect(tree(older, "production")).toBe(tree(older, older.sha.c3 as string));
  });

  it("takes over an issue from before it, whose state line has no `daily`", () => {
    const w = started();
    edit(w, (s) => {
      const first = s.issues[0];
      if (first !== undefined) first.body = first.body.replace(/ daily=[0-9-]+/u, "");
    });
    mainRun(w, w.sha.c3 as string, 501);
    expect(ok(run(w, "workflow_run")).output).toContain("promoted=true");
    expect(daily(issue(w, 1))).toBe("501");
  });
});

describe("/promote", () => {
  it("opens a pull request of main's newest green commit into production, which the super run tests, and merges nothing yet", () => {
    const w = started();
    const before = production(w);
    const { id, run: r } = say(w, "/promote");
    expect(ok(r).output).not.toContain("promoted=true");
    const p = pr(w, 2);
    expect(p).toMatchObject({ base: "production", state: "open", merged: false, labels: [LABEL, "human"], assignees: PEOPLE });
    expect(p.head).toMatch(/^promote\/\d{8}-[0-9a-f]{7}$/u);
    expect(head(w, p)).toBe(w.sha.c3);
    expect(p.title).toBe(`Promote main to production: 2 commit(s) up to ${short(w.sha.c3)}`);
    expect(p.body).toContain("The daily super run (`super.yml`) tests this pull request.");
    expect(p.body).toContain("- Catalog: v0.2.0b\n");
    expect(p.body).toContain("- Started by: /promote from @maintainer\n");
    expect(p.body).toContain("- Production issue: #1\n");
    expect(p.body).toMatch(/~~~\n[0-9a-f]{7} change c3\n[0-9a-f]{7} change c2\n~~~/u);
    expect(production(w)).toBe(before);
    expect(reactions(w, id)).toEqual(["+1"]);
    expect(lastReply(w)).toContain(`Opened #2 (@maintainer): 2 commit(s) of \`main\` up to \`${short(w.sha.c3, 12)}\``);
    expect(issue(w, 1).title).toBe(`Production: the super run is testing ${short(w.sha.c3)} (#2)`);
    expect(issue(w, 1).body).toContain(`**Production:** #2 promotes \`main\` up to \`${short(w.sha.c3, 12)}\`, and the super run is testing it.`);
  });

  it("promotes the commit named, and /fast-forward says the same", () => {
    const w = started();
    ok(say(w, `/promote ${short(w.sha.c2, 9)}`).run);
    expect(head(w, pr(w, 2))).toBe(w.sha.c2);

    const other = started();
    ok(say(other, "/fast-forward").run);
    expect(head(other, pr(other, 2))).toBe(other.sha.c3);
  });

  it("refuses, with a 😕 and the reason, a commit that has not passed CI, is not on main or is not a commit", () => {
    const w = started();
    const red = commit(w, "c4", { "app.txt": "four\n" }, false);
    let { id } = say(w, `/promote ${short(red, 12)}`);
    expect(reactions(w, id)).toEqual(["confused"]);
    expect(lastReply(w)).toBe(`\`${short(red, 12)}\` has no passing push-to-main CI run.`);

    git(w.work, "checkout", "-q", "-b", "side");
    writeFileSync(join(w.work, "side.txt"), "x\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "side");
    git(w.work, "push", "-q", "origin", "side");
    const side = git(w.work, "rev-parse", "HEAD");
    ({ id } = say(w, `/promote ${short(side, 12)}`));
    expect(reactions(w, id)).toEqual(["confused"]);
    expect(lastReply(w)).toBe(`\`${short(side, 12)}\` is not on main.`);

    ({ id } = say(w, "/promote abcdef1234"));
    expect(reactions(w, id)).toEqual(["confused"]);
    expect(lastReply(w)).toBe("`abcdef1234` is not a commit.");
    expect(gh(w).prs).toEqual([]);
  });

  it("says there is nothing to merge when production has the commit already", () => {
    const w = started();
    ok(say(w, `/promote ${short(w.sha.c1, 12)}`).run);
    expect(lastReply(w)).toBe(`Nothing to merge: \`production\` already has \`${short(w.sha.c1, 12)}\`.`);
    expect(gh(w).prs).toEqual([]);
  });

  it("keeps the open pull request for the same commit, and closes it as superseded for a newer one", () => {
    const w = promoted();
    ok(say(w, "/promote").run);
    expect(gh(w).prs).toHaveLength(1);
    expect(lastReply(w)).toBe(`#2 already promotes \`${short(w.sha.c3, 12)}\`; the super run is on it.`);

    const old = pr(w, 2).head;
    commit(w, "c4", { "app.txt": "four\n" });
    ok(say(w, "/promote").run);
    expect(pr(w, 2)).toMatchObject({ state: "closed", merged: false });
    expect(pr(w, 2).comments).toEqual([`Superseded by the promotion of \`${short(w.sha.c4, 12)}\`, a newer commit of main.`]);
    expect(git(w.origin, "for-each-ref", `refs/heads/${old}`)).toBe("");
    expect(openPrs(w).map((p) => p.number)).toEqual([3]);
    expect(head(w, pr(w, 3))).toBe(w.sha.c4);
  });
});

describe("the super run", () => {
  it("merges the pull request once its run on the head commit passed, says so, and starts the deploy", () => {
    const w = promoted();
    superRun(w, w.sha.c3 as string, "success");
    const r = ok(run(w, "workflow_run"));
    expect(r.output).toContain("promoted=true");
    expect(pr(w, 2)).toMatchObject({ state: "closed", merged: true });
    // Production holds main's files, through a merge commit, and the promote branch is gone.
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
    expect(git(w.origin, "rev-list", "--parents", "-n1", "production").split(" ")).toHaveLength(3);
    expect(git(w.origin, "for-each-ref", "refs/heads/promote")).toBe("");
    expect(lastReply(w)).toContain(`Merged to production in #2: the super run passed every job on \`${short(w.sha.c3, 12)}\` ([run](https://github.com/acme/game/actions/runs/77)). 2 commit(s), catalog v0.2.0b`);
    expect(issue(w, 1)).toMatchObject({ state: "open", title: "Production: everything on main is live" });
    expect(gh(w).issues).toHaveLength(1);
  });

  it("waits while the run goes, and before it has started", () => {
    const w = promoted();
    ok(run(w, "workflow_run"));
    superRun(w, w.sha.c3 as string, null, 1, "in_progress");
    expect(ok(run(w, "workflow_run")).output).not.toContain("promoted=true");
    expect(pr(w, 2).state).toBe("open");
    expect(issue(w, 1).title).toBe(`Production: the super run is testing ${short(w.sha.c3)} (#2)`);
    expect(issue(w, 1).body).toContain("is testing it ([run](https://github.com/acme/game/actions/runs/77)).");
  });

  it("says once a run attempt that the run failed, leaves the pull request open, and merges after a re-run passes", () => {
    const w = promoted();
    const before = production(w);
    superRun(w, w.sha.c3 as string, "failure");
    expect(ok(run(w, "workflow_run")).output).not.toContain("promoted=true");
    expect(production(w)).toBe(before);
    expect(pr(w, 2).state).toBe("open");
    expect(lastReply(w)).toContain(`The super run ended \`failure\` on \`${short(w.sha.c3, 12)}\``);
    expect(lastReply(w)).toContain('the issue "CI: super run failed on a production candidate"');
    expect(issue(w, 1).title).toBe(`Production: the super run failed on ${short(w.sha.c3)} (#2)`);
    expect(marker(issue(w, 1))).toMatch(/reported=77\.1$/u);
    const said = issue(w, 1).comments.length;

    ok(run(w, "workflow_run"));
    expect(issue(w, 1).comments).toHaveLength(said);

    superRun(w, w.sha.c3 as string, "failure", 2);
    ok(run(w, "workflow_run"));
    expect(issue(w, 1).comments).toHaveLength(said + 1);

    superRun(w, w.sha.c3 as string, "success", 3);
    expect(ok(run(w, "workflow_run")).output).toContain("promoted=true");
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
  });

  it("counts only the run on the pull request's head commit", () => {
    const w = promoted();
    superRun(w, w.sha.c2 as string, "success");
    ok(run(w, "workflow_run"));
    expect(pr(w, 2).state).toBe("open");
  });

  it("is made up by the next run of any kind when GitHub dropped its own wake-up", () => {
    const w = promoted();
    superRun(w, w.sha.c3 as string, "success");
    expect(ok(say(w, "thanks, all").run).output).toContain("promoted=true");
    expect(pr(w, 2).merged).toBe(true);
  });
});

describe("/skip-super", () => {
  it("merges the open pull request at once, without the super run, and records who, why and which commit", () => {
    const w = promoted();
    const { id, run: r } = say(w, "/skip-super the fuzz wave is red on a seed this does not touch", { login: "jgoetzmann", assoc: "OWNER" });
    expect(ok(r).output).toContain("promoted=true");
    expect(reactions(w, id)).toEqual(["+1"]);
    expect(pr(w, 2)).toMatchObject({ state: "closed", merged: true });
    expect(pr(w, 2).comments).toEqual(["@jgoetzmann merges this without the super run, the emergency way (`/skip-super`): the fuzz wave is red on a seed this does not touch"]);
    expect(lastReply(w)).toContain(
      `Merged #2 into production without the super run, at @jgoetzmann's word: the fuzz wave is red on a seed this does not touch (2 commit(s) up to \`${short(w.sha.c3, 12)}\``,
    );
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
    expect(issue(w, 1).title).toBe("Production: everything on main is live");
  });

  it("with no pull request open, merges main's newest green commit through one", () => {
    const w = started();
    ok(say(w, "/skip-super sign-in is down").run);
    expect(gh(w).prs).toHaveLength(1);
    expect(pr(w, 2)).toMatchObject({ merged: true });
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
  });

  it("is refused without a reason", () => {
    const w = promoted();
    const { id } = say(w, "/skip-super");
    expect(reactions(w, id)).toEqual(["confused"]);
    expect(lastReply(w)).toContain("`/skip-super` needs a reason after it");
    expect(pr(w, 2).merged).toBe(false);
  });

  it("is the only way to create production when there is none", () => {
    const w = started();
    git(w.origin, "update-ref", "-d", "refs/heads/production");
    ok(say(w, "/promote").run);
    expect(lastReply(w)).toContain("`production` does not exist yet");
    expect(gh(w).prs).toEqual([]);
    expect(ok(say(w, "/skip-super first deploy").run).output).toContain("promoted=true");
    expect(production(w)).toBe(w.sha.c3);
    expect(gh(w).prs).toEqual([]);
    expect(lastReply(w)).toContain("Created `production`");
  });
});

describe("/cancel", () => {
  it("closes the open pull request and its branch, and says when there is none", () => {
    const w = promoted();
    const branch = pr(w, 2).head;
    ok(say(w, "/cancel").run);
    expect(pr(w, 2)).toMatchObject({ state: "closed", merged: false, comments: ["Cancelled by @maintainer on #1."] });
    expect(git(w.origin, "for-each-ref", `refs/heads/${branch}`)).toBe("");
    expect(lastReply(w)).toBe("Closed #2, as @maintainer asked. Nothing merges.");
    expect(issue(w, 1).title).toBe("Production: 2 commits on main are not live yet");

    ok(say(w, "/cancel").run);
    expect(lastReply(w)).toBe("No promotion pull request is open, so there is nothing to cancel.");
  });
});

describe("the commands", () => {
  it("are heard only from people with write access, and never from bots", () => {
    const w = started();
    for (const who of [{ assoc: "NONE", login: "passerby" }, { assoc: "CONTRIBUTOR", login: "drive-by" }, { type: "Bot", login: "some-bot" }]) {
      const { id, run: r } = say(w, "/promote", who);
      ok(r);
      expect(reactions(w, id)).toEqual(["confused"]);
    }
    expect(gh(w).prs).toEqual([]);
  });

  it("are read once, only at the start of a line, whatever their case", () => {
    const w = started();
    let { id } = say(w, "could someone /promote this later?");
    expect(reactions(w, id)).toEqual([]);
    expect(gh(w).prs).toEqual([]);
    ({ id } = say(w, "Looks good.\n  /PROMOTE"));
    expect(reactions(w, id)).toEqual(["+1"]);
    ok(run(w, "issue_comment"));
    expect(reactions(w, id)).toEqual(["+1"]);
    expect(gh(w).prs).toHaveLength(1);
  });

  it("answers /hold, /resume and /delay that there is nothing to hold: production follows the super run", () => {
    const w = started();
    for (const command of ["/hold", "/resume", "/delay 3h"]) {
      const { id } = say(w, command);
      expect(reactions(w, id)).toEqual(["confused"]);
      expect(lastReply(w)).toBe(
        "There is nothing to hold, resume or delay: production follows the super run, promoting itself when the daily run on `main` passes, or a `/promote` here once its own run passes. `/cancel` closes an open promotion pull request.",
      );
    }
    expect(gh(w).prs).toEqual([]);
  });
});

describe("by hand", () => {
  it("opens the pull request for the commit given, or main's newest green one, and leaves it to the super run", () => {
    const w = started();
    const r = ok(run(w, "workflow_dispatch", { INPUT_SHA: w.sha.c2 as string, ACTOR: "jgoetzmann" }));
    expect(r.output).not.toContain("promoted=true");
    expect(head(w, pr(w, 2))).toBe(w.sha.c2);
    expect(pr(w, 2).body).toContain("- Started by: a manual run by @jgoetzmann\n");
    expect(lastReply(w)).toContain("Opened #2 (by hand, @jgoetzmann)");
  });

  it("merges at once with a skip_super_reason, and records it and who ran it", () => {
    const w = started();
    const r = ok(run(w, "workflow_dispatch", { INPUT_SKIP_REASON: "a sign-in hotfix", ACTOR: "jgoetzmann" }));
    expect(r.output).toContain("promoted=true");
    expect(pr(w, 2).merged).toBe(true);
    expect(lastReply(w)).toContain("without the super run, at @jgoetzmann's word: a sign-in hotfix");
  });

  it("never moves production backwards: a commit it already has is a no-op", () => {
    const w = started();
    const r = ok(run(w, "workflow_dispatch", { INPUT_SHA: w.sha.c1 as string }));
    expect(r.text).toContain("nothing to merge");
    expect(gh(w).prs).toEqual([]);
  });

  it("fails with the reason for a commit it refuses", () => {
    const w = started();
    const red = commit(w, "c4", { "app.txt": "four\n" }, false);
    const r = run(w, "workflow_dispatch", { INPUT_SHA: red });
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("has no passing push-to-main CI run");
  });

  it("is in the workflow, with redeploy for deploying production as it is", () => {
    expect(WORKFLOW).toMatch(/^ {6}sha:\n/mu);
    expect(WORKFLOW).toMatch(/^ {6}skip_super_reason:\n/mu);
    expect(WORKFLOW).toMatch(/^ {6}redeploy:\n/mu);
    expect(WORKFLOW).toContain("          INPUT_SKIP_REASON: ${{ inputs.skip_super_reason }}\n");
    expect(WORKFLOW).toContain("          ACTOR: ${{ github.actor }}\n");
  });
});

describe("the queue in the issue", () => {
  /** The body's queue table, a row a commit of main. */
  const rows = (i: Issue): string[] => {
    const start = i.body.indexOf("| Pull request |");
    if (start === -1) return [];
    const table = i.body.slice(start).split("\n\n")[0] ?? "";
    return table.split("\n").slice(2);
  };

  it("lists what main has that production does not, by pull request, newest first, with its CI and where it stands", () => {
    const w = world();
    const four = commit(w, "c4", { "app.txt": "four\n" }, false, "Patch: four | more (#14)");
    edit(w, (s) => s.failed.push(four));
    // A pull request merged with a merge commit, its title on the body's first line.
    git(w.work, "checkout", "-q", "-b", "side");
    writeFileSync(join(w.work, "side.txt"), "x\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "side work");
    git(w.work, "checkout", "-q", "main");
    git(w.work, "merge", "-q", "--no-ff", "side", "-m", "Merge pull request #15 from acme/side", "-m", "Patch: the side quest");
    git(w.work, "push", "-q", "origin", "main");
    const merged = git(w.work, "rev-parse", "HEAD");
    w.sha.c5 = merged;
    edit(w, (s) => s.running.push(merged));
    ok(run(w, "workflow_run"));
    const body = issue(w, 1).body;
    expect(body).toContain(`Production serves \`main\` as of \`${short(w.sha.c1)}\`.`);
    expect(body).toContain("Not live yet, newest first: 4. `/promote` takes `main` up to its newest commit whose CI passed.");
    expect(body).toContain("| Pull request | CI | Production |");
    expect(rows(issue(w, 1))).toEqual([
      "| #15 Patch: the side quest | ⏳ running | waits for CI |",
      "| #14 Patch: four \\| more | ❌ failed | waits for CI |",
      `| \`${short(w.sha.c3)}\` change c3 | ✅ passed | next /promote |`,
      `| \`${short(w.sha.c2)}\` change c2 | ✅ passed | next /promote |`,
    ]);
  });

  it("marks what the open pull request carries, and says when everything is live", () => {
    const w = promoted();
    commit(w, "c4", { "app.txt": "four\n" });
    ok(run(w, "workflow_run"));
    expect(rows(issue(w, 1)).map((row) => row.split(" | ").at(-1))).toEqual(["next /promote |", "in #2 |", "in #2 |"]);
    ok(say(w, "/promote").run);
    superRun(w, w.sha.c4 as string, "success");
    ok(run(w, "workflow_run"));
    expect(rows(issue(w, 1))).toEqual([]);
    expect(issue(w, 1).body).toContain(`Production serves \`main\` as of \`${short(w.sha.c4)}\`. Everything on \`main\` is live.`);
  });
});

describe("a catalog bump", () => {
  it("waits for /promote and the super run like any other change, and is named in the pull request and on the issue", () => {
    const w = started();
    const bump = commit(w, "c4", { [PATCHES]: patches("v0.2.0a", "v0.2.0b", "v0.2.1"), "app.txt": "four\n" });
    expect(ok(run(w, "workflow_run")).output).not.toContain("promoted=true");
    expect(gh(w).prs).toEqual([]);
    ok(say(w, "/promote").run);
    expect(pr(w, 2).body).toContain("- Catalog: v0.2.1\n");
    superRun(w, bump, "success");
    ok(run(w, "workflow_run"));
    expect(tree(w, "production")).toBe(tree(w, bump));
    expect(lastReply(w)).toContain(", catalog v0.2.1;");
    expect(readFileSync(SCRIPT, "utf8")).not.toContain("render.yaml");
  });

  it("is wired to the deploy through the job's output, which `redeploy` can force", () => {
    expect(WORKFLOW).toMatch(/^ {4}outputs:\n {6}promoted: \$\{\{ steps\.promote\.outputs\.promoted \}\}$/mu);
    expect(WORKFLOW).toMatch(/^ {6}- name: promote production and keep its issue\n {8}id: promote$/mu);
    expect(WORKFLOW).toContain("needs: promote");
    expect(WORKFLOW).toContain("needs.promote.outputs.promoted == 'true'");
    expect(WORKFLOW).toMatch(/github\.event_name == 'workflow_dispatch' && inputs\.redeploy/u);
  });

  it("deploys the Worker and the server's container together, then waits for the server to report the commit", () => {
    // `--env=""` names the top level, production, which wrangler otherwise warns is left implicit.
    expect(WORKFLOW).toContain('pnpm --dir apps/edge exec wrangler deploy --config ../../wrangler.jsonc --env="" --var "DEPLOYED_COMMIT:$sha"');
    expect(WORKFLOW).not.toContain("--containers-rollout");
    expect(WORKFLOW).toMatch(/uses: \.\/\.github\/actions\/poll-live\n {8}with:\n {10}site: \$\{\{ env\.SITE \}\}\n {10}commit: \$\{\{ steps\.deploy\.outputs\.sha \}\}\n/u);
  });
});

describe("what stops it", () => {
  it("refuses to open a promotion into a production that carries something main does not", () => {
    const w = started();
    git(w.work, "fetch", "-q", "origin");
    git(w.work, "checkout", "-q", "-B", "hand", "origin/production");
    writeFileSync(join(w.work, "hotfix.txt"), "by hand\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "a hand edit on production");
    git(w.work, "push", "-q", "origin", "hand:production");
    const r = say(w, "/promote").run;
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("production has diverged");
    expect(gh(w).prs).toEqual([]);
  });

  it("retries a merge GitHub is not ready for, and fails with the pull request left open if it never is", () => {
    const w = promoted();
    superRun(w, w.sha.c3 as string, "success");
    edit(w, (s) => {
      s.failMerges = 3;
    });
    ok(run(w, "workflow_run"));
    expect(pr(w, 2).merged).toBe(true);

    const stuck = promoted();
    superRun(stuck, stuck.sha.c3 as string, "success");
    edit(stuck, (s) => {
      s.failMerges = 4;
    });
    const r = run(stuck, "workflow_run");
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("could not be merged into production");
    expect(pr(stuck, 2)).toMatchObject({ state: "open", merged: false });
  });

  it("moves production with PROMOTE_TOKEN, which may update workflows and start super, and keeps everything else on GH_TOKEN", () => {
    // GitHub refuses the Actions token a ref update that carries a change to .github/workflows, and a
    // pull request it opens starts no workflow, so super would never test it: the push, the pull
    // request, its merge and the branch's deletion go out as the bot. The issue, and every read, stay
    // on the Actions token, whose writes start no workflow.
    const w = started();
    edit(w, (s) => {
      s.tokens = [];
    });
    const tokens = { GH_TOKEN: "actions", PROMOTE_TOKEN: "bot" };
    ok(run(w, "issue_comment", tokens));
    comment(w, "/promote");
    ok(run(w, "issue_comment", tokens));
    superRun(w, w.sha.c3 as string, "success");
    ok(run(w, "workflow_run", tokens));
    expect(pr(w, 2).merged).toBe(true);
    const calls = (token: string): string[] =>
      gh(w).tokens.filter((c) => c.startsWith(`${token} `)).map((c) => c.slice(token.length + 1));
    expect(calls("bot")).toEqual(["pr create", "pr merge", "api DELETE"]);
    expect(calls("actions")).toEqual(expect.arrayContaining(["pr list", "run list", "label create", "issue comment", "issue edit"]));
    expect(calls("actions").filter((c) => ["pr create", "pr merge", "pr close", "api DELETE"].includes(c))).toEqual([]);
  });

  it("waits, with the Actions token, while main's newest commit changes a workflow and has not passed CI", () => {
    // GitHub refuses the workflow token a push whose .github/workflows differ from main's: the real
    // run of 2026-10-04 20:08 died pushing promote/<the newest green commit> for exactly that.
    const w = started();
    const red = commit(w, "c4", { ".github/workflows/new.yml": "on: push\n" }, false);
    const waited = ok(say(w, "/promote").run);
    expect(waited.text).toContain("::notice::Not promoting");
    expect(lastReply(w)).toContain("Ask again once CI on main is green.");
    expect(gh(w).prs).toEqual([]);
    expect(git(w.origin, "for-each-ref", "refs/heads/promote")).toBe("");

    const manual = run(w, "workflow_dispatch");
    expect(manual.status).not.toBe(0);
    expect(manual.text).toContain("Run this again once CI on main is green");

    edit(w, (s) => {
      s.green.push(red);
      s.newestGreen = red;
    });
    ok(say(w, "/promote").run);
    expect(head(w, pr(w, 2))).toBe(red);
  });

  it("does not wait with the bot's token, which may push workflow files", () => {
    const w = started();
    commit(w, "c4", { ".github/workflows/new.yml": "on: push\n" }, false);
    comment(w, "/promote");
    const r = ok(run(w, "issue_comment", { GH_TOKEN: "actions", PROMOTE_TOKEN: "bot" }));
    expect(r.text).not.toContain("Not promoting");
    expect(head(w, pr(w, 2))).toBe(w.sha.c3);
  });

  it("says in its error what GitHub said when it refuses a push", () => {
    const w = started();
    const hook = join(w.origin, "hooks", "pre-receive");
    writeFileSync(
      hook,
      "#!/bin/sh\nwhile read old new ref; do case $ref in refs/heads/promote/*) echo \"refusing to allow a GitHub App to create or update workflow\" >&2; exit 1;; esac; done\n",
    );
    chmodSync(hook, 0o755);
    const r = say(w, "/promote").run;
    expect(r.status).not.toBe(0);
    expect(r.text).toMatch(/::error::Could not push [0-9a-f]+:refs\/heads\/promote\/\S+: .*refusing to allow a GitHub App to create or update workflow.*BOT_GITHUB_TOKEN/u);
    expect(issue(w, 1).state).toBe("open");
  });

  it("names the command in its error when anything else fails", () => {
    const w = started();
    edit(w, (s) => {
      s.fail = ["label create"];
    });
    let r = say(w, "/promote").run;
    expect(r.status).not.toBe(0);
    expect(r.text).toMatch(/::error::scripts\/promote-production\.sh line \d+: `gh label create .*` failed \(exit 1\)/u);

    const other = started();
    edit(other, (s) => {
      s.fail = ["pr create"];
    });
    r = say(other, "/promote").run;
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("::error::`gh pr` failed (exit 1): HTTP 403: Resource not accessible by integration");
  });
});

describe("the workflow", () => {
  it("names the same label as the script, in the job's `if`, which cannot read env", () => {
    const script = readFileSync(SCRIPT, "utf8");
    expect(/: "\$\{RELEASE_LABEL:=([^}]+)\}"/u.exec(script)?.[1]).toBe(LABEL);
    expect(WORKFLOW).toContain(`contains(github.event.issue.labels.*.name, '${LABEL}')`);
  });

  it("has no schedule of its own, and wakes on green CI on main, on super runs passing on main and on super runs on promotion pull requests", () => {
    expect(WORKFLOW).not.toMatch(/^ {2}schedule:/mu);
    expect(SUPER).toMatch(/^ {2}schedule:\n {4}- cron: /mu);
    expect(WORKFLOW).toContain(
      "github.event.workflow_run.name == 'super' && github.event.workflow_run.event != 'pull_request'\n            && github.event.workflow_run.head_branch == 'main' && github.event.workflow_run.conclusion == 'success'",
    );
    expect(WORKFLOW).toMatch(/^ {2}workflow_run:\n {4}workflows: \[CI, super\]\n {4}types: \[completed\]\n {4}branches: \[main, "promote\/\*\*"\]\n/mu);
    expect(WORKFLOW).toContain("github.event.workflow_run.name == 'super' && github.event.workflow_run.event == 'pull_request'");
    expect(WORKFLOW).toContain("startsWith(github.event.workflow_run.head_branch, 'promote/')");
  });

  it("wakes for a comment only from someone with write access on the production issue, for its commands", () => {
    expect(WORKFLOW).toContain(`contains(fromJSON('["OWNER", "MEMBER", "COLLABORATOR"]'), github.event.comment.author_association)`);
    expect(WORKFLOW).toContain("github.event.comment.user.type != 'Bot'");
    expect(WORKFLOW).toContain("!github.event.issue.pull_request");
    for (const command of ["/promote", "/fast-forward", "/skip-super", "/cancel", "/hold", "/resume", "/delay"]) {
      expect(WORKFLOW).toContain(`contains(github.event.comment.body, '${command}')`);
    }
  });

  it("holds the concurrency group on the job, so a comment that wakes nothing never queues", () => {
    expect(WORKFLOW).not.toMatch(/^concurrency:/mu);
    expect(WORKFLOW).toMatch(/^ {4}concurrency:\n {6}group: promote-production\n {6}cancel-in-progress: false$/mu);
  });

  it("checks out, and moves production, with the bot's token, which has the workflow scope", () => {
    const token = "${{ secrets.BOT_GITHUB_TOKEN || github.token }}";
    expect(WORKFLOW).toContain(`          fetch-depth: 0\n          token: ${token}\n`);
    expect(WORKFLOW).toContain(`          GH_TOKEN: \${{ github.token }}\n          PROMOTE_TOKEN: ${token}\n`);
  });

  it("asks for the permissions the script uses, and no more", () => {
    expect(WORKFLOW).toMatch(/^permissions:\n {2}contents: write\n {2}actions: read\n {2}issues: write\n {2}pull-requests: write\n\n/mu);
  });
});

describe("the super run as production's gate", () => {
  it("runs on every pull request into production", () => {
    expect(SUPER).toMatch(/^ {2}pull_request:\n {4}branches: \[production\]\n/mu);
  });

  it("hands main's red run to the night bot as high priority, since production waits for a run on main that passes", () => {
    expect(SUPER).toContain('--add-assignee "$BOT_LOGIN" --add-label "bot:build" \\\n              --add-label "priority:high" \\\n');
  });

  it("hands a failed promotion or deploy to Squishy, and closes it when a deploy passes", () => {
    expect(WORKFLOW).toMatch(/^ {2}report:\n {4}name: report\n {4}needs: \[promote, deploy\]\n/mu);
    expect(WORKFLOW).toContain('PROMOTION_TITLE: "CI: production promotion failed"');
    expect(WORKFLOW).toContain("SQUISHY_LOGIN: squishy-squooby");
    expect(WORKFLOW).toContain('SQUISHY_QUEUE: "squishy:build"');
    expect(WORKFLOW).toContain('--add-assignee "$SQUISHY_LOGIN" --add-label "$SQUISHY_QUEUE"');
    expect(WORKFLOW).toContain('if [ "$DEPLOY" = success ]; then');
  });

  it("reports a promotion's red run under the title the script names, and hands it to the night bot first", () => {
    const script = readFileSync(SCRIPT, "utf8");
    const title = /: "\$\{SUPER_TITLE:=([^}]+)\}"/u.exec(script)?.[1];
    expect(title).toBe("CI: super run failed on a production candidate");
    expect(SUPER).toContain(`CANDIDATE_TITLE: "${title}"`);
    expect(SUPER).toContain('--add-label "priority:high" --add-label "difficulty:hard"');
    expect(SUPER).toContain('if [ "$EVENT" = pull_request ] && [ "$BASE" = production ]; then');
  });
});
