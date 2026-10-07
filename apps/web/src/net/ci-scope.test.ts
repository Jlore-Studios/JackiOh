// scripts/ci-scope.sh: the lines ci.yml's `changes` job uses to skip jobs. It may say `full=false`
// only when every changed file is one no CI job reads (the night bot's code and its switches, the
// workflows that run the bot, the training lanes' prompts and loop), and `db=false` only when nothing
// the `db` job tests against Postgres changed, so each case here is a diff in a throwaway repo and the
// answer a pull request would get. Every doubt must come out `full=true` and `db=true`.

import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(HERE, "../../../../scripts/ci-scope.sh");

let repo = "";

function git(...args: string[]): string {
  return execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", ...args], {
    cwd: repo,
    encoding: "utf8",
  }).trim();
}

/** A commit on top of the first one that adds `files`; returns the first commit's sha and the new one's. */
function change(...files: string[]): { base: string; head: string } {
  git("checkout", "-q", "-B", "work", "base");
  for (const file of files) {
    mkdirSync(dirname(join(repo, file)), { recursive: true });
    writeFileSync(join(repo, file), `${file}\n`);
  }
  git("add", "-A");
  git("commit", "-q", "-m", "change");
  return { base: git("rev-parse", "base"), head: git("rev-parse", "HEAD") };
}

/** The script's two output lines, `full=…` and `db=…`. */
function lines(base: string | undefined, head: string | undefined): { full: string; db: string } {
  const args = [SCRIPT, ...(base === undefined ? [] : [base]), ...(head === undefined ? [] : [head])];
  const run = spawnSync("sh", args, { cwd: repo, encoding: "utf8" });
  expect(run.status, run.stderr).toBe(0);
  const out = run.stdout.trim().split("\n");
  expect(out, run.stdout).toHaveLength(2);
  expect(out[0]).toMatch(/^full=(true|false)$/);
  expect(out[1]).toMatch(/^db=(true|false)$/);
  return { full: out[0] ?? "", db: out[1] ?? "" };
}

const scope = (base: string | undefined, head: string | undefined): string => lines(base, head).full;

/** What a pull request that adds `files` gets. */
const answerFor = (...files: string[]): { full: string; db: string } => {
  const { base, head } = change(...files);
  return lines(base, head);
};

const fullFor = (...files: string[]): string => answerFor(...files).full;
const dbFor = (...files: string[]): string => answerFor(...files).db;

beforeAll(() => {
  repo = mkdtempSync(join(tmpdir(), "ci-scope-"));
  git("init", "-q", "-b", "main");
  writeFileSync(join(repo, "README.md"), "start\n");
  git("add", "-A");
  git("commit", "-q", "-m", "start");
  git("tag", "base");
});

afterAll(() => {
  rmSync(repo, { recursive: true, force: true });
});

describe("scripts/ci-scope.sh", () => {
  it("skips the heavy jobs for a diff of the night bot's own files and nothing else", () => {
    expect(fullFor("bot/harness/state.py")).toBe("full=false");
    expect(fullFor("bot/machine/README.md", ".harness/config.json")).toBe("full=false");
    expect(fullFor(".github/workflows/bot-night.yml", ".github/workflows/bot-commands.yml", "bot/tests/test_cross.py")).toBe("full=false");
    expect(fullFor(".github/workflows/triage.yml", ".github/workflows/deploy-watch.yml", ".github/workflows/ci-duration.yml")).toBe("full=false");
    expect(fullFor("CLAUDE.md", "AGENTS.md", "GEMINI.md", "bot/README.md")).toBe("full=false");
    // Squishy (#60): its switches and its two workflows, which only the bot's harness reads.
    expect(fullFor(".squishy/config.json", ".github/workflows/squishy-run.yml", ".github/workflows/squishy-commands.yml")).toBe("full=false");
    // The training lanes' standing prompts and loop: Devin and the training box read them, no job does.
    expect(fullFor("training/README.md", "training/improve.md", "training/unban.md", "training/loop.sh")).toBe("full=false");
  });

  it("runs everything the moment one changed file is read by a job", () => {
    for (const file of [
      "crates/engine/src/reduce.rs",
      "crates/cards/catalog.json",
      "crates/ai/src/decide.rs",
      "crates/ai/generation.json",
      "crates/server/src/main.rs",
      "crates/tools/src/fuzz.rs",
      "Cargo.toml",
      "Cargo.lock",
      "rust-toolchain.toml",
      "scripts/build-wasm.sh",
      "apps/web/src/main.tsx",
      "e2e/cypress/e2e/01-hotseat-full-game.cy.ts",
      // `spec check` reads the notes; the training gate reads the lanes' history.
      "spec/rulings/R0195.md",
      "spec/README.md",
      "training/history/improve.jsonl",
      "training/loop.sh.bak",
      "training/lanes/x.ts",
      "SPEC.md",
      "BUILD.md",
      "REVIEW.md",
      "README.md",
      "docs/architecture.md",
      "docs/radiant-audit.md",
      "package.json",
      "pnpm-lock.yaml",
      "vercel.json",
      "scripts/ci-scope.sh",
      ".github/workflows/ci.yml",
      ".github/workflows/super.yml",
      ".github/coverage-floor.txt",
      // promote-production.test.ts runs the script and reads the workflow.
      ".github/workflows/promote-production.yml",
      "scripts/promote-production.sh",
      ".github/actions/setup/action.yml",
      ".github/workflows/bot-selftest.yml-not",
      "apps/bot/x.ts",
      "botany/x.ts",
      ".harness-copy/x.json",
      ".squishy-copy/x.json",
      ".github/workflows/squishy-run.yml-not",
    ]) {
      expect(fullFor(file), file).toBe("full=true");
      expect(fullFor("bot/harness/state.py", file), `bot + ${file}`).toBe("full=true");
    }
  });

  it("runs everything when it cannot tell what changed", () => {
    const { base, head } = change("bot/harness/state.py");
    expect(lines(base, head)).toEqual({ full: "full=false", db: "db=false" });
    const everything = { full: "full=true", db: "db=true" };
    expect(lines(undefined, undefined)).toEqual(everything);
    expect(lines(base, undefined)).toEqual(everything);
    expect(lines(base, base)).toEqual(everything);
    expect(lines("0".repeat(40), head)).toEqual(everything);
    expect(lines(base, "not-a-commit")).toEqual(everything);
  });

  it("runs the db job only when what it tests against Postgres changed", () => {
    for (const file of [
      "crates/server/migrations/0027_new.sql",
      "crates/server/src/db/pg.rs",
      "crates/server/src/db/migrate.rs",
      "crates/server/Dockerfile",
      "render.yaml",
      "crates/server/tests/sql/01_schema_invariants.sql",
      "crates/server/tests/db/run.sh",
      "crates/server/tests/deploy/rehearse.sh",
      "crates/server/tests/store/contract.rs",
      "crates/server/Cargo.toml",
      "Cargo.lock",
    ]) {
      expect(answerFor(file), file).toEqual({ full: "full=true", db: "db=true" });
      expect(dbFor("crates/engine/src/reduce.rs", file), `engine + ${file}`).toBe("db=true");
      expect(dbFor("bot/harness/state.py", file), `bot + ${file}`).toBe("db=true");
    }
    for (const file of [
      "crates/engine/src/reduce.rs",
      "crates/server/src/api/queue.rs",
      "crates/server/src/actor/match_actor.rs",
      "crates/server/tests/api/queue.rs",
      "crates/server/src/dbx.rs",
      "crates/server/Dockerfile.dev",
      "crates/server-db/x.rs",
      "Cargo.toml",
      "apps/web/src/main.tsx",
      "docs/architecture.md",
    ]) {
      expect(answerFor(file), file).toEqual({ full: "full=true", db: "db=false" });
    }
    // A pull request that skips everything skips the db job too.
    expect(answerFor("bot/harness/state.py", ".harness/config.json")).toEqual({ full: "full=false", db: "db=false" });
  });

  it("counts a moved file's old path as well as its new one", () => {
    git("checkout", "-q", "-B", "work", "base");
    mkdirSync(join(repo, "crates/server/migrations"), { recursive: true });
    writeFileSync(join(repo, "crates/server/migrations/0001_init.sql"), "create table t (id int);\n".repeat(20));
    git("add", "-A");
    git("commit", "-q", "-m", "migration");
    const before = git("rev-parse", "HEAD");
    mkdirSync(join(repo, "bot"), { recursive: true });
    git("mv", "crates/server/migrations/0001_init.sql", "bot/0001_init.sql");
    git("commit", "-q", "-m", "moved into the bot's directory");
    expect(lines(before, git("rev-parse", "HEAD"))).toEqual({ full: "full=true", db: "db=true" });
  });

  it("reads what the branch changed, not what main changed since it was cut", () => {
    git("checkout", "-q", "-B", "work", "base");
    mkdirSync(join(repo, "bot"), { recursive: true });
    writeFileSync(join(repo, "bot/a.py"), "x\n");
    git("add", "bot/a.py");
    git("commit", "-q", "-m", "branch");
    const branch = git("rev-parse", "HEAD");
    git("checkout", "-q", "-B", "later-main", "base");
    mkdirSync(join(repo, "crates/server/migrations"), { recursive: true });
    writeFileSync(join(repo, "crates/server/migrations/0099_new.sql"), "x\n");
    git("add", "crates/server/migrations/0099_new.sql");
    git("commit", "-q", "-m", "main moved");
    const moved = git("rev-parse", "HEAD");
    expect(scope(moved, branch)).toBe("full=false");
    expect(lines(moved, branch).db).toBe("db=false");
  });
});
