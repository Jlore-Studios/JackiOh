// scripts/promote-production.sh, the logic of .github/workflows/promote-production.yml: the merge of
// main into production through a pull request, and the countdown issue that can be held or delayed.
// Each case runs the real script in a fresh clone of a throwaway "origin", with a stand-in for `gh`
// (below) that keeps issues, pull requests, comments and reactions in a JSON file and does the merge
// for real with git, so what production ends up holding is what GitHub would leave it.

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
const LABEL = "production merge";

// --- the stand-in for `gh` -------------------------------------------------------------------

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
  "--match-head-commit", "-X", "-f"]);
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
function out(data) {
  if (f("--jq") === undefined) return void process.stdout.write(JSON.stringify(data) + "\n");
  const r = cp.spawnSync("jq", ["-r", f("--jq")], { input: JSON.stringify(data), encoding: "utf8" });
  if (r.status !== 0) { process.stderr.write(r.stderr); process.exit(1); }
  process.stdout.write(r.stdout);
}
const bot = (n, body) => st.issues.find((i) => i.number === n).comments.push({ id: st.nextComment++, login: "github-actions[bot]", type: "Bot", assoc: "NONE", at: iso(), body });
const issue = (n) => st.issues.find((i) => i.number === Number(n));
st.log.push(argv.join(" "));
const [a, b] = pos;
if (a === "run" && b === "list") {
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
  st.issues.push({ number, title: f("--title"), body: fs.readFileSync(f("--body-file"), "utf8"), labels: flag["--label"], state: "open", reason: "", comments: [] });
  process.stdout.write("https://github.com/acme/game/issues/" + number + "\n");
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
  out(st.prs.filter((p) => p.state === "open" && p.base === f("--base") && p.head === f("--head")).map((p) => ({ number: p.number })));
} else if (a === "pr" && b === "create") {
  const number = st.next++;
  st.prs.push({ number, head: f("--head"), base: f("--base"), title: f("--title"), body: fs.readFileSync(f("--body-file"), "utf8"), labels: flag["--label"], state: "open", merged: false });
  process.stdout.write("https://github.com/acme/game/pull/" + number + "\n");
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
`;

interface Comment { id: number; login: string; type: string; assoc: string; at: string; body: string }
interface Issue { number: number; title: string; body: string; labels: string[]; state: "open" | "closed"; reason: string; comments: Comment[] }
interface Pr { number: number; head: string; base: string; title: string; body: string; labels: string[]; state: string; merged: boolean }
interface Gh {
  next: number;
  nextComment: number;
  issues: Issue[];
  prs: Pr[];
  reactions: { comment: number; content: string }[];
  labels: string[];
  green: string[];
  newestGreen: string;
  failMerges: number;
  log: string[];
}

// --- a world: origin, a working copy, the stand-in -------------------------------------------

interface World { root: string; origin: string; work: string; bin: string; sha: Record<string, string> }
const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

const git = (cwd: string, ...args: string[]): string =>
  execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", ...args], { cwd, encoding: "utf8" }).trim();

const at = (iso: string): number => Date.parse(iso) / 1000;
const stateFile = (w: World): string => join(w.root, "gh", "state.json");
const gh = (w: World): Gh => JSON.parse(readFileSync(stateFile(w), "utf8")) as Gh;
const edit = (w: World, change: (state: Gh) => void): void => {
  const state = gh(w);
  change(state);
  writeFileSync(stateFile(w), JSON.stringify(state));
};

/** A commit on main (pushed, and recorded green); `files` are written whole. */
function commit(w: World, name: string, files: Record<string, string>, green = true): string {
  if (Object.keys(w.sha).length > 0) git(w.work, "checkout", "-q", "main");
  for (const [file, text] of Object.entries(files)) {
    mkdirSync(dirname(join(w.work, file)), { recursive: true });
    writeFileSync(join(w.work, file), text);
  }
  git(w.work, "add", "-A");
  git(w.work, "commit", "-q", "-m", `change ${name}`);
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

const renderYaml = (catalog: string): string =>
  `services:\n  - type: web\n    envVars:\n      - key: NODE_ENV\n        value: production\n      - key: CATALOG_VERSION\n        value: ${catalog}\n`;

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
  const empty: Gh = { next: 1, nextComment: 1000, issues: [], prs: [], reactions: [], labels: [], green: [], newestGreen: "", failMerges: 0, log: [] };
  writeFileSync(stateFile(w), JSON.stringify(empty));
  commit(w, "c1", { "render.yaml": renderYaml("v0.2.1"), "app.txt": "one\n" });
  git(w.work, "push", "-q", "origin", `${w.sha.c1}:refs/heads/production`);
  commit(w, "c2", { "app.txt": "two\n" });
  commit(w, "c3", { "app.txt": "three\n" });
  return w;
}

interface Run { status: number | null; text: string }
let runs = 0;
function run(w: World, event: string, now: string, extra: Record<string, string> = {}): Run {
  const runner = join(w.root, `runner-${++runs}`);
  git(w.root, "clone", "-q", w.origin, runner);
  const result = spawnSync("bash", [SCRIPT], {
    cwd: runner,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${w.bin}:${process.env.PATH ?? ""}`,
      GITHUB_REPOSITORY: "acme/game",
      GITHUB_STEP_SUMMARY: "/dev/null",
      GITHUB_RUN_ID: "",
      GH_TOKEN: "x",
      EVENT: event,
      NOW: String(at(now)),
      MERGE_RETRY_SECONDS: "0",
      FAKE_GH_DIR: join(w.root, "gh"),
      FAKE_GH_ORIGIN: w.origin,
      RUN_SHA: "",
      INPUT_SHA: "",
      ...extra,
    },
  });
  return { status: result.status, text: `${result.stdout}${result.stderr}` };
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
const openIssues = (w: World): Issue[] => gh(w).issues.filter((i) => i.state === "open");
const marker = (i: Issue): string => /promote-production (due=\d+ held=[01] last=\d+)/u.exec(i.body)?.[1] ?? "";
const production = (w: World): string => git(w.origin, "rev-parse", "refs/heads/production");
const tree = (w: World, rev: string): string => git(w.origin, "rev-parse", `${rev}^{tree}`);
const comment = (w: World, number: number, body: string, when: string, who: { login?: string; assoc?: string; type?: string } = {}): number => {
  let id = 0;
  edit(w, (s) => {
    id = s.nextComment++;
    s.issues.find((i) => i.number === number)?.comments.push({ id, login: who.login ?? "maintainer", type: who.type ?? "User", assoc: who.assoc ?? "MEMBER", at: when, body });
  });
  return id;
};
const reactions = (w: World, id: number): string[] => gh(w).reactions.filter((r) => r.comment === id).map((r) => r.content);

/** The first check, which opens issue #1 for Oct 5 15:00 UTC; the rest of the cases start from there. */
function counting(): World {
  const w = world();
  ok(run(w, "schedule", "2026-10-04T16:00:00Z"));
  expect(openIssues(w).map((i) => i.number)).toEqual([1]);
  return w;
}

const DUE = at("2026-10-05T15:00:00Z");

describe("the countdown issue", () => {
  it("opens one, labelled, saying how many hours are left, and merges nothing yet", () => {
    const w = world();
    const before = production(w);
    ok(run(w, "schedule", "2026-10-04T16:00:00Z"));
    const [only, ...rest] = openIssues(w);
    expect(rest).toEqual([]);
    expect(only?.title).toBe("Merging to production in 23 hours (2026-10-05 15:00 UTC)");
    expect(only?.labels).toEqual([LABEL]);
    expect(marker(only as Issue)).toBe(`due=${DUE} held=0 last=0`);
    expect(gh(w).labels).toEqual([LABEL]);
    expect(gh(w).prs).toEqual([]);
    expect(production(w)).toBe(before);
  });

  it("does nothing between its checks until the time comes", () => {
    const w = counting();
    ok(run(w, "schedule", "2026-10-05T14:07:00Z"));
    expect(gh(w).prs).toEqual([]);
    expect(gh(w).issues).toHaveLength(1);
  });

  it("at the time merges through a pull request, closes the issue with it and opens the next", () => {
    const w = counting();
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    const { prs } = gh(w);
    expect(prs).toHaveLength(1);
    const pr = prs[0] as Pr;
    expect(pr).toMatchObject({ number: 2, base: "production", state: "closed", merged: true, labels: [LABEL] });
    expect(pr.title).toBe(`Promote main to production: 2 commit(s) up to ${w.sha.c3?.slice(0, 7)}`);
    expect(pr.body).toContain("Countdown issue: #1");
    expect(pr.body).toContain("Catalog: v0.2.1");
    expect(pr.body).toMatch(/~~~\n[0-9a-f]{7} change c3\n[0-9a-f]{7} change c2\n~~~/u);

    // Production holds main's files, through a merge commit, and the promote branch is gone.
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
    expect(git(w.origin, "rev-list", "--parents", "-n1", "production").split(" ")).toHaveLength(3);
    expect(git(w.origin, "for-each-ref", "refs/heads/promote")).toBe("");

    const old = issue(w, 1);
    expect(old).toMatchObject({ state: "closed", reason: "completed" });
    expect(old.comments.at(-1)?.body).toContain("Merged to production in #2 (the scheduled merge): 2 commits");
    const [next, ...rest] = openIssues(w);
    expect(rest).toEqual([]);
    expect(next).toMatchObject({ number: 3, title: "Merging to production in 24 hours (2026-10-06 15:00 UTC)" });
    expect(marker(next as Issue)).toBe(`due=${at("2026-10-06T15:00:00Z")} held=0 last=0`);
  });

  it("keeps merging day after day, each a merge commit of production's own history", () => {
    const w = counting();
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    commit(w, "c4", { "app.txt": "four\n" });
    commit(w, "c5", { "app.txt": "five\n" });
    ok(run(w, "schedule", "2026-10-06T15:07:00Z"));
    expect(gh(w).prs.map((p) => [p.number, p.merged])).toEqual([[2, true], [4, true]]);
    expect(tree(w, "production")).toBe(tree(w, w.sha.c5 as string));
    expect(openIssues(w).map((i) => i.number)).toEqual([5]);
  });

  it("closes it as nothing to merge when production already has everything", () => {
    const w = counting();
    git(w.work, "push", "-q", "origin", `${w.sha.c3}:refs/heads/production`);
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    expect(gh(w).prs).toEqual([]);
    expect(issue(w, 1)).toMatchObject({ state: "closed", reason: "completed" });
    expect(issue(w, 1).comments.at(-1)?.body).toContain("Nothing to merge");
    expect(openIssues(w).map((i) => i.number)).toEqual([2]);
  });

  it("starts the next one at the next release hour that is at least 12 hours away", () => {
    const w = counting();
    ok(run(w, "workflow_dispatch", "2026-10-06T02:00:00Z"));
    expect(marker(openIssues(w)[0] as Issue)).toContain(`due=${at("2026-10-06T15:00:00Z")}`);
    commit(w, "c4", { "app.txt": "four\n" });
    ok(run(w, "workflow_dispatch", "2026-10-06T14:00:00Z"));
    expect(marker(openIssues(w)[0] as Issue)).toContain(`due=${at("2026-10-07T15:00:00Z")}`);
  });

  it("closes any older duplicate as not planned and keeps the newest", () => {
    const w = counting();
    ok(run(w, "schedule", "2026-10-04T16:30:00Z"));
    edit(w, (s) => s.issues.push({ ...(s.issues[0] as Issue), number: s.next++, comments: [] }));
    ok(run(w, "schedule", "2026-10-04T17:00:00Z"));
    expect(issue(w, 1)).toMatchObject({ state: "closed", reason: "not planned" });
    expect(openIssues(w).map((i) => i.number)).toEqual([2]);
  });

  it("schedules a fresh time when someone deleted its state line", () => {
    const w = counting();
    edit(w, (s) => {
      (s.issues[0] as Issue).body = "no state here";
    });
    ok(run(w, "schedule", "2026-10-04T17:00:00Z"));
    expect(marker(issue(w, 1))).toBe(`due=${DUE} held=0 last=0`);
  });
});

describe("holding and delaying from the comments", () => {
  it("holds on /hold from a collaborator, with a thumbs up, and merges nothing while held", () => {
    const w = counting();
    const id = comment(w, 1, "/hold waiting on the almanac art", "2026-10-05T10:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T10:01:00Z"));
    expect(reactions(w, id)).toEqual(["+1"]);
    expect(issue(w, 1).title).toBe("Merging to production: on hold (was due 2026-10-05 15:00 UTC)");
    expect(marker(issue(w, 1))).toBe(`due=${DUE} held=1 last=${id}`);
    expect(issue(w, 1).body).toContain("**On hold.**");
    ok(run(w, "schedule", "2026-10-05T16:07:00Z"));
    ok(run(w, "schedule", "2026-10-06T16:07:00Z"));
    expect(gh(w).prs).toEqual([]);
    expect(openIssues(w).map((i) => i.number)).toEqual([1]);
  });

  it("merges the moment a comment says /resume once the time has passed", () => {
    const w = counting();
    comment(w, 1, "/hold", "2026-10-05T10:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T10:01:00Z"));
    const id = comment(w, 1, "Done, thanks.\n/resume", "2026-10-05T17:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T17:00:30Z"));
    expect(reactions(w, id)).toEqual(["+1"]);
    expect(gh(w).prs).toHaveLength(1);
    expect(issue(w, 1)).toMatchObject({ state: "closed", reason: "completed" });
    expect(openIssues(w)[0]?.title).toBe("Merging to production in 22 hours (2026-10-06 15:00 UTC)");
  });

  it("goes back to counting down after /resume when the time has not come", () => {
    const w = counting();
    comment(w, 1, "/hold", "2026-10-05T08:00:00Z");
    comment(w, 1, "/RESUME", "2026-10-05T09:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T09:01:00Z"));
    expect(issue(w, 1).title).toBe("Merging to production in 6 hours (2026-10-05 15:00 UTC)");
    expect(marker(issue(w, 1))).toContain("held=0");
    expect(gh(w).prs).toEqual([]);
  });

  it("delays by hours or days, and merges once the new time has come", () => {
    const w = counting();
    const id = comment(w, 1, "/delay 3h", "2026-10-05T10:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T10:01:00Z"));
    expect(reactions(w, id)).toEqual(["+1"]);
    expect(issue(w, 1).title).toBe("Merging to production in 8 hours (2026-10-05 18:00 UTC)");
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    expect(gh(w).prs).toEqual([]);
    ok(run(w, "schedule", "2026-10-05T18:07:00Z"));
    expect(gh(w).prs).toHaveLength(1);

    const other = counting();
    comment(other, 1, "/delay 2 days", "2026-10-05T10:00:00Z");
    ok(run(other, "issue_comment", "2026-10-05T10:01:00Z"));
    expect(marker(issue(other, 1))).toContain(`due=${DUE + 48 * 3600}`);
  });

  it("counts a delay from the comment when the time has already passed", () => {
    const w = counting();
    comment(w, 1, "/hold", "2026-10-05T12:00:00Z");
    comment(w, 1, "/delay 2h", "2026-10-06T09:00:00Z");
    comment(w, 1, "/resume", "2026-10-06T09:30:00Z");
    ok(run(w, "issue_comment", "2026-10-06T09:31:00Z"));
    expect(marker(issue(w, 1))).toContain(`due=${at("2026-10-06T11:00:00Z")} held=0`);
    expect(gh(w).prs).toEqual([]);
    ok(run(w, "schedule", "2026-10-06T11:07:00Z"));
    expect(gh(w).prs).toHaveLength(1);
  });

  it("refuses commands from anyone without write access, and bots", () => {
    const w = counting();
    const stranger = comment(w, 1, "/hold", "2026-10-05T10:00:00Z", { login: "stranger", assoc: "NONE" });
    const contributor = comment(w, 1, "/delay 3h", "2026-10-05T10:01:00Z", { login: "once", assoc: "CONTRIBUTOR" });
    const robot = comment(w, 1, "/hold", "2026-10-05T10:02:00Z", { login: "some-app[bot]", type: "Bot", assoc: "MEMBER" });
    ok(run(w, "issue_comment", "2026-10-05T10:03:00Z"));
    expect([stranger, contributor, robot].map((id) => reactions(w, id))).toEqual([["confused"], ["confused"], ["confused"]]);
    expect(marker(issue(w, 1))).toContain(`due=${DUE} held=0`);
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    expect(gh(w).prs).toHaveLength(1);
  });

  it("refuses a delay it cannot read or that is too long, and says to /hold instead", () => {
    const w = counting();
    const ids = ["/delay soon", "/delay", "/delay 0h", "/delay 9999h", "/delay 30d", "/delay -3h"].map((body, i) =>
      comment(w, 1, body, `2026-10-05T10:0${i}:00Z`),
    );
    const out = ok(run(w, "issue_comment", "2026-10-05T10:09:00Z"));
    expect(ids.map((id) => reactions(w, id))).toEqual(ids.map(() => ["confused"]));
    expect(out.text).toContain("use /hold for longer");
    expect(marker(issue(w, 1))).toContain(`due=${DUE} held=0`);
  });

  it("hears a command only at the start of a line, whatever its case, and ignores plain talk", () => {
    const w = counting();
    const quoted = comment(w, 1, "> /hold\nnot this one", "2026-10-05T10:00:00Z");
    const talk = comment(w, 1, "we could /hold it, or not", "2026-10-05T10:01:00Z");
    const real = comment(w, 1, "Sorry, one thing first.\n  /Hold  the api change", "2026-10-05T10:02:00Z");
    ok(run(w, "issue_comment", "2026-10-05T10:03:00Z"));
    expect([quoted, talk].map((id) => reactions(w, id))).toEqual([[], []]);
    expect(reactions(w, real)).toEqual(["+1"]);
    expect(marker(issue(w, 1))).toBe(`due=${DUE} held=1 last=${real}`);
  });

  it("reads each comment once", () => {
    const w = counting();
    comment(w, 1, "/delay 1h", "2026-10-05T10:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T10:01:00Z"));
    ok(run(w, "schedule", "2026-10-05T11:07:00Z"));
    ok(run(w, "schedule", "2026-10-05T12:07:00Z"));
    expect(marker(issue(w, 1))).toContain(`due=${DUE + 3600}`);
    expect(gh(w).reactions).toHaveLength(1);
  });
});

describe("the catalog fast path", () => {
  it("merges a catalog bump at once, past a hold, and leaves the countdown standing", () => {
    const w = counting();
    comment(w, 1, "/hold", "2026-10-05T10:00:00Z");
    ok(run(w, "issue_comment", "2026-10-05T10:01:00Z"));
    const bump = commit(w, "c4", { "render.yaml": renderYaml("v0.2.2"), "app.txt": "four\n" });
    ok(run(w, "workflow_run", "2026-10-05T11:00:00Z", { RUN_SHA: bump }));
    expect(gh(w).prs).toHaveLength(1);
    expect(gh(w).prs[0]?.title).toContain("3 commit(s)");
    expect(tree(w, "production")).toBe(tree(w, bump));
    expect(issue(w, 1).state).toBe("open");
    expect(marker(issue(w, 1))).toContain("held=1");
    expect(issue(w, 1).comments.at(-1)?.body).toContain("Catalog v0.2.2 could not wait");
    expect(gh(w).prs[0]?.body).toContain("a catalog bump to v0.2.2, which is never held");
  });

  it("leaves a commit that does not change the catalog for the scheduled merge", () => {
    const w = counting();
    const before = production(w);
    const plain = commit(w, "c4", { "app.txt": "four\n" });
    expect(ok(run(w, "workflow_run", "2026-10-05T11:00:00Z", { RUN_SHA: plain })).text).toContain("No catalog change");
    expect(production(w)).toBe(before);
    expect(gh(w).prs).toEqual([]);
  });

  it("is also caught by the hourly check, should the event have been lost", () => {
    const w = counting();
    commit(w, "c4", { "render.yaml": renderYaml("v0.2.2") });
    ok(run(w, "schedule", "2026-10-04T17:07:00Z"));
    expect(gh(w).prs).toHaveLength(1);
    expect(issue(w, 1).state).toBe("open");
  });
});

describe("a green push to main, for a cron GitHub never starts", () => {
  it("does the whole check: opens the countdown issue when there is none, and merges nothing yet", () => {
    const w = world();
    const plain = commit(w, "c4", { "app.txt": "four\n" });
    ok(run(w, "workflow_run", "2026-10-04T16:00:00Z", { RUN_SHA: plain }));
    expect(openIssues(w).map((i) => i.title)).toEqual(["Merging to production in 23 hours (2026-10-05 15:00 UTC)"]);
    expect(gh(w).prs).toEqual([]);
  });

  it("waits for the issue's time, then the first one after it merges, with no scheduled run at all", () => {
    const w = counting();
    const early = commit(w, "c4", { "app.txt": "four\n" });
    ok(run(w, "workflow_run", "2026-10-05T14:59:00Z", { RUN_SHA: early }));
    expect(gh(w).prs).toEqual([]);
    const late = commit(w, "c5", { "app.txt": "five\n" });
    ok(run(w, "workflow_run", "2026-10-05T15:40:00Z", { RUN_SHA: late }));
    expect(gh(w).prs.map((p) => [p.number, p.merged])).toEqual([[2, true]]);
    expect(tree(w, "production")).toBe(tree(w, late));
    expect(issue(w, 1)).toMatchObject({ state: "closed", reason: "completed" });
    expect(openIssues(w).map((i) => i.number)).toEqual([3]);
  });

  it("is held by a hold, like any other check", () => {
    const w = counting();
    comment(w, 1, "/hold", "2026-10-05T10:00:00Z");
    const late = commit(w, "c4", { "app.txt": "four\n" });
    ok(run(w, "workflow_run", "2026-10-05T16:00:00Z", { RUN_SHA: late }));
    expect(gh(w).prs).toEqual([]);
    expect(marker(issue(w, 1))).toContain("held=1");
  });
});

describe("running it by hand", () => {
  it("merges the newest green commit now, hold or no hold, and starts the countdown over", () => {
    const w = counting();
    comment(w, 1, "/hold", "2026-10-04T17:00:00Z");
    ok(run(w, "issue_comment", "2026-10-04T17:01:00Z"));
    ok(run(w, "workflow_dispatch", "2026-10-04T18:00:00Z"));
    expect(tree(w, "production")).toBe(tree(w, w.sha.c3 as string));
    expect(issue(w, 1)).toMatchObject({ state: "closed", reason: "completed" });
    expect(openIssues(w)[0]?.title).toBe("Merging to production in 21 hours (2026-10-05 15:00 UTC)");
  });

  it("takes the commit it is given, which must be a green one on main", () => {
    const w = counting();
    ok(run(w, "workflow_dispatch", "2026-10-04T18:00:00Z", { INPUT_SHA: w.sha.c2 as string }));
    expect(tree(w, "production")).toBe(tree(w, w.sha.c2 as string));

    const red = commit(w, "c4", { "app.txt": "four\n" }, false);
    const refused = run(w, "workflow_dispatch", "2026-10-04T19:00:00Z", { INPUT_SHA: red });
    expect(refused.status).not.toBe(0);
    expect(refused.text).toContain("has no passing push-to-main CI run");
    git(w.work, "checkout", "-q", "-b", "side", w.sha.c1 as string);
    writeFileSync(join(w.work, "side.txt"), "x\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "side");
    const side = git(w.work, "rev-parse", "HEAD");
    git(w.work, "push", "-q", "origin", "side");
    edit(w, (s) => s.green.push(side));
    const offMain = run(w, "workflow_dispatch", "2026-10-04T19:00:00Z", { INPUT_SHA: side });
    expect(offMain.status).not.toBe(0);
    expect(offMain.text).toContain("is not on main");
  });

  it("leaves the countdown alone when there was nothing to merge", () => {
    const w = counting();
    git(w.work, "push", "-q", "origin", `${w.sha.c3}:refs/heads/production`);
    ok(run(w, "workflow_dispatch", "2026-10-04T18:00:00Z"));
    expect(openIssues(w).map((i) => i.number)).toEqual([1]);
    expect(gh(w).prs).toEqual([]);
  });

  it("creates production, with no pull request, when there is none yet", () => {
    const w = world();
    git(w.origin, "update-ref", "-d", "refs/heads/production");
    ok(run(w, "workflow_dispatch", "2026-10-04T18:00:00Z"));
    expect(production(w)).toBe(w.sha.c3);
    expect(gh(w).prs).toEqual([]);
    expect(openIssues(w).map((i) => i.title)).toEqual(["Merging to production in 21 hours (2026-10-05 15:00 UTC)"]);
  });
});

describe("what stops it", () => {
  it("refuses to merge into a production that carries something main does not", () => {
    const w = counting();
    git(w.work, "fetch", "-q", "origin");
    git(w.work, "checkout", "-q", "-B", "hand", "origin/production");
    writeFileSync(join(w.work, "hotfix.txt"), "by hand\n");
    git(w.work, "add", "-A");
    git(w.work, "commit", "-q", "-m", "a hand edit on production");
    git(w.work, "push", "-q", "origin", "hand:production");
    const r = run(w, "schedule", "2026-10-05T15:07:00Z");
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("production has diverged");
    expect(gh(w).prs).toEqual([]);
    expect(issue(w, 1).state).toBe("open");
  });

  it("retries a merge GitHub is not ready for, and fails with the pull request left open if it never is", () => {
    const w = counting();
    edit(w, (s) => {
      s.failMerges = 3;
    });
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    expect(gh(w).prs[0]?.merged).toBe(true);

    const stuck = counting();
    edit(stuck, (s) => {
      s.failMerges = 4;
    });
    const r = run(stuck, "schedule", "2026-10-05T15:07:00Z");
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("could not be merged into production");
    expect(gh(stuck).prs[0]).toMatchObject({ state: "open", merged: false });
    expect(issue(stuck, 1).state).toBe("open");
  });

  it("never moves production backwards: promoting a commit it already has is a no-op", () => {
    const w = counting();
    ok(run(w, "schedule", "2026-10-05T15:07:00Z"));
    const before = production(w);
    const out = ok(run(w, "workflow_dispatch", "2026-10-05T16:00:00Z", { INPUT_SHA: w.sha.c2 as string }));
    expect(out.text).toContain("nothing to merge");
    expect(production(w)).toBe(before);
    expect(gh(w).prs).toHaveLength(1);
    expect(openIssues(w).map((i) => i.number)).toEqual([3]);
  });

  it("refuses an event it does not know", () => {
    const w = world();
    expect(run(w, "push", "2026-10-04T16:00:00Z").status).not.toBe(0);
  });
});

describe("the workflow", () => {
  it("names the same label as the script, in the job's `if`, which cannot read env", () => {
    const script = readFileSync(SCRIPT, "utf8");
    expect(/: "\$\{RELEASE_LABEL:=([^}]+)\}"/u.exec(script)?.[1]).toBe(LABEL);
    expect(WORKFLOW).toContain(`contains(github.event.issue.labels.*.name, '${LABEL}')`);
  });

  it("wakes for a comment only from someone with write access on the countdown issue", () => {
    expect(WORKFLOW).toContain(`contains(fromJSON('["OWNER", "MEMBER", "COLLABORATOR"]'), github.event.comment.author_association)`);
    expect(WORKFLOW).toContain("github.event.comment.user.type != 'Bot'");
    expect(WORKFLOW).toContain("!github.event.issue.pull_request");
  });

  it("holds the concurrency group on the job, so a comment that wakes nothing never queues", () => {
    expect(WORKFLOW).not.toMatch(/^concurrency:/mu);
    expect(WORKFLOW).toMatch(/^ {4}concurrency:\n {6}group: promote-production\n {6}cancel-in-progress: false$/mu);
  });

  it("asks for the permissions the script uses, and no more", () => {
    expect(WORKFLOW).toMatch(/^permissions:\n {2}contents: write\n {2}actions: read\n {2}issues: write\n {2}pull-requests: write\n/mu);
  });
});
