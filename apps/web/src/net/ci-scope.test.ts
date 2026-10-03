// scripts/ci-scope.sh: the line ci.yml's `changes` job uses to skip the heavy jobs. It may say
// `full=false` only when every changed file is one no CI job reads (the night bot's code and its
// switches, the workflows that run the bot), so each case here is a diff in a throwaway repo and the
// answer a pull request would get. Every doubt must come out `full=true`.

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

function scope(base: string | undefined, head: string | undefined): string {
  const args = [SCRIPT, ...(base === undefined ? [] : [base]), ...(head === undefined ? [] : [head])];
  const run = spawnSync("sh", args, { cwd: repo, encoding: "utf8" });
  expect(run.status, run.stderr).toBe(0);
  return run.stdout.trim();
}

const fullFor = (...files: string[]): string => {
  const { base, head } = change(...files);
  return scope(base, head);
};

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
  });

  it("runs everything the moment one changed file is read by a job", () => {
    for (const file of [
      "packages/engine/src/reduce.ts",
      "apps/web/src/main.tsx",
      "apps/server/src/index.ts",
      "e2e/cypress/e2e/01-hotseat-full-game.cy.ts",
      "SPEC.md",
      "BUILD.md",
      "REVIEW.md",
      "README.md",
      "docs/architecture.md",
      "package.json",
      "pnpm-lock.yaml",
      "vercel.json",
      "scripts/ci-scope.sh",
      ".github/workflows/ci.yml",
      ".github/actions/setup/action.yml",
      ".github/workflows/bot-selftest.yml-not",
      "apps/bot/x.ts",
      "botany/x.ts",
      ".harness-copy/x.json",
    ]) {
      expect(fullFor(file), file).toBe("full=true");
      expect(fullFor("bot/harness/state.py", file), `bot + ${file}`).toBe("full=true");
    }
  });

  it("runs everything when it cannot tell what changed", () => {
    const { base, head } = change("bot/harness/state.py");
    expect(scope(base, head)).toBe("full=false");
    expect(scope(undefined, undefined)).toBe("full=true");
    expect(scope(base, undefined)).toBe("full=true");
    expect(scope(base, base)).toBe("full=true");
    expect(scope("0".repeat(40), head)).toBe("full=true");
    expect(scope(base, "not-a-commit")).toBe("full=true");
  });

  it("reads what the branch changed, not what main changed since it was cut", () => {
    git("checkout", "-q", "-B", "work", "base");
    mkdirSync(join(repo, "bot"), { recursive: true });
    writeFileSync(join(repo, "bot/a.py"), "x\n");
    git("add", "bot/a.py");
    git("commit", "-q", "-m", "branch");
    const branch = git("rev-parse", "HEAD");
    git("checkout", "-q", "-B", "later-main", "base");
    mkdirSync(join(repo, "packages"), { recursive: true });
    writeFileSync(join(repo, "packages/new.ts"), "x\n");
    git("add", "packages/new.ts");
    git("commit", "-q", "-m", "main moved");
    const moved = git("rev-parse", "HEAD");
    expect(scope(moved, branch)).toBe("full=false");
  });
});
