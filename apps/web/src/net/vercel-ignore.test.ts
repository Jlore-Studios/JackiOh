// scripts/vercel-ignore.sh, run the way vercel.json runs it (its `ignoreCommand`), in a throwaway git
// repo: Vercel deploys on the last push and only when the push can change what the site serves.
// Exit 1 builds, exit 0 skips, and every doubt must build: a skipped deploy that should have run
// leaves production stale, while an extra build costs one deployment.

import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const { ignoreCommand } = JSON.parse(readFileSync(join(ROOT, "vercel.json"), "utf8")) as { ignoreCommand: string };

let repo = "";
let base = "";

function git(...args: string[]): string {
  return execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", ...args], {
    cwd: repo,
    encoding: "utf8",
  }).trim();
}

/** A commit on main, on top of the base, that adds each file (and more commits for more arrays). */
function commits(...pushes: string[][]): void {
  git("checkout", "-q", "-B", "main", base);
  for (const files of pushes) {
    for (const file of files) {
      mkdirSync(dirname(join(repo, file)), { recursive: true });
      writeFileSync(join(repo, file), `${file}\n`);
    }
    git("add", "-A");
    git("commit", "-q", "-m", "change");
  }
}

type Env = { ref?: string; message?: string; env?: string; previous?: string | null };

/** Whether Vercel would build, running ignoreCommand in `cwd` with the variables Vercel sets. */
function builds({ ref = "main", message = "A change", env = "preview", previous = base }: Env = {}, cwd = repo): boolean {
  const vars: Record<string, string> = {
    PATH: process.env.PATH ?? "",
    VERCEL_GIT_COMMIT_REF: ref,
    VERCEL_GIT_COMMIT_MESSAGE: message,
    VERCEL_ENV: env,
  };
  if (previous !== null) vars.VERCEL_GIT_PREVIOUS_SHA = previous;
  const run = spawnSync("sh", ["-c", ignoreCommand], { cwd, env: vars, encoding: "utf8" });
  expect(run.error, "sh ran").toBeUndefined();
  expect([0, 1], `exit status ${String(run.status)}: ${run.stderr}`).toContain(run.status);
  return run.status === 1;
}

beforeAll(() => {
  repo = mkdtempSync(join(tmpdir(), "vercel-ignore-"));
  git("init", "-q", "-b", "main");
  mkdirSync(join(repo, "scripts"));
  copyFileSync(join(ROOT, "scripts/vercel-ignore.sh"), join(repo, "scripts/vercel-ignore.sh"));
  writeFileSync(join(repo, "README.md"), "start\n");
  git("add", "-A");
  git("commit", "-q", "-m", "built");
  base = git("rev-parse", "HEAD");
});

afterAll(() => {
  rmSync(repo, { recursive: true, force: true });
});

describe("scripts/vercel-ignore.sh", () => {
  it("builds main when a file the bundle reads changed", () => {
    for (const file of [
      "packages/engine/src/reduce.ts",
      "packages/cards/catalog.json",
      "apps/web/src/main.tsx",
      "apps/web/public/robots.txt",
      "package.json",
      "pnpm-lock.yaml",
      "vercel.json",
      "assets/music/a.mid",
      "tsconfig.base.json",
      "apps/web/vite.config.ts",
      "apps/web/index.html",
      "packages/ai/src/decide.ts",
      "packages/shared/src/index.ts",
      "packages/cards/patches/patches.json",
      // The card scripts are bundled: a package's scripts/ is tooling, but src/scripts/ is not.
      "packages/cards/src/scripts/001-bigot.ts",
      // Named like test support, but not a *.test.* file or a test/ directory, so not proved unread.
      "apps/web/src/game/deckbuilder/testkit.ts",
      "apps/web/src/patches/fixtures.ts",
    ]) {
      commits([file]);
      expect(builds(), file).toBe(true);
    }
  });

  it("skips main when only the night bot, architecture, docs, the server, CI, tests or tooling changed", () => {
    for (const files of [
      ["bot/harness/state.py", ".harness/config.json"],
      [".squishy/config.json", ".github/workflows/squishy-run.yml"],
      [".github/workflows/ci.yml", ".github/actions/setup/action.yml"],
      ["docs/architecture.md", "SPEC.md", "BUILD.md", "CLAUDE.md", "REVIEW.md", "README.md"],
      ["apps/server/src/index.ts", "apps/server/package.json", "render.yaml"],
      ["e2e/cypress/e2e/01-hotseat-full-game.cy.ts", "reviews/2026-10-03.md"],
      ["scripts/ci-scope.sh", "JackiOh_Core_Cards.md", "ARCHITECTURE-CCG.md"],
      // Tests and tooling inside the client and the packages are never imported by the bundle.
      ["apps/web/src/game/Board.test.tsx", "apps/web/src/net/x.test.ts", "apps/web/src/test/setup.ts"],
      ["packages/cards/test/002-bigot.test.ts", "packages/engine/test/reduce.test.ts", "packages/ai/test/_support.ts"],
      ["packages/validator/test/x.test.ts", "packages/shared/test/y.test.ts", "packages/engine/src/z.test.ts"],
      ["packages/cards/scripts/patch.ts", "packages/ai/scripts/sweep.ts", "apps/web/scripts/gen-voice.mjs"],
      ["packages/cards/README.md", "packages/shared/README.md", "apps/web/README.md"],
    ]) {
      commits(files);
      expect(builds(), files.join(", ")).toBe(false);
    }
  });

  it("builds when one file the bundle reads sits among files that do not", () => {
    commits(["bot/harness/state.py", "docs/architecture.md", "packages/ai/src/decide.ts"]);
    expect(builds()).toBe(true);
    commits(["docs/x.md", "apps/web/src/x.ts"]);
    expect(builds()).toBe(true);
    // A path that only looks like a skipped one.
    commits(["apps/serverless/x.ts"]);
    expect(builds()).toBe(true);
    commits(["botany/x.ts"]);
    expect(builds()).toBe(true);
    commits(["docs-copy/x.md"]);
    expect(builds()).toBe(true);
    // A test or tooling file beside a source file the bundle reads is still a build.
    commits(["apps/web/src/game/Board.test.tsx", "apps/web/src/game/Board.tsx"]);
    expect(builds()).toBe(true);
    commits(["packages/cards/test/002-bigot.test.ts", "packages/cards/src/scripts/002-bigot.ts"]);
    expect(builds()).toBe(true);
    commits(["packages/cards/scripts/patch.ts", "packages/cards/catalog.json"]);
    expect(builds()).toBe(true);
    // Paths that only look like tests or tooling: a test/ or scripts/ below src, another suffix.
    commits(["packages/cards/src/test/x.ts"]);
    expect(builds()).toBe(true);
    commits(["packages/cards/src/scripts/scripts/x.ts"]);
    expect(builds()).toBe(true);
    commits(["apps/web/src/game/Board.tsx.test"]);
    expect(builds()).toBe(true);
    commits(["apps/web/src/net/test.ts"]);
    expect(builds()).toBe(true);
  });

  it("never skips past a commit that was not built: it diffs against the last one that was", () => {
    // A changed the bundle and its build never ran (the daily cap); B only touched the bot.
    commits(["packages/engine/src/a.ts"], ["bot/harness/b.py"]);
    expect(builds()).toBe(true);
    // Once A is the last build, B alone is skipped.
    const built = git("rev-parse", "HEAD~1");
    expect(builds({ previous: built })).toBe(false);
  });

  it("builds on any doubt about what was last built", () => {
    commits(["bot/harness/state.py"]);
    expect(builds({ previous: null })).toBe(true);
    expect(builds({ previous: "" })).toBe(true);
    expect(builds({ previous: "0".repeat(40) })).toBe(true);
    expect(builds({ previous: "not-a-commit" })).toBe(true);
    // Nothing changed since the last build (a redeploy of the same commit).
    expect(builds({ previous: git("rev-parse", "HEAD") })).toBe(true);
    // The last build is not an ancestor of this commit (a preview from another branch). Every file
    // between the two is a skippable one, so only the ancestry check can make this a build.
    git("checkout", "-q", "-B", "elsewhere", base);
    mkdirSync(join(repo, "docs"), { recursive: true });
    writeFileSync(join(repo, "docs/elsewhere.md"), "x\n");
    git("add", "-A");
    git("commit", "-q", "-m", "elsewhere");
    const elsewhere = git("rev-parse", "HEAD");
    commits(["bot/harness/state.py"]);
    expect(builds({ previous: elsewhere })).toBe(true);
  });

  it("builds a commit whose message says [vercel], skipped path or not, on any branch", () => {
    commits(["docs/architecture.md"]);
    expect(builds({ message: "Final polish [vercel]" })).toBe(true);
    expect(builds({ message: "Final polish [Vercel]\n\nbody" })).toBe(true);
    expect(builds({ ref: "feat/x", message: "Ship it [vercel]" })).toBe(true);
    // The flag is the bracketed word: a message that only names Vercel does not turn builds on.
    expect(builds({ message: "Why vercel skips builds" })).toBe(false);
    expect(builds({ ref: "feat/x", message: "Why vercel skips builds" })).toBe(false);
  });

  it("skips every branch but main (and production) without the flag, whatever it changed", () => {
    commits(["packages/engine/src/reduce.ts"]);
    expect(builds({ ref: "feat/live-cards" })).toBe(false);
    expect(builds({ ref: "fix/anim-double", env: "preview" })).toBe(false);
    expect(builds({ ref: "" })).toBe(false);
    // A production deploy of another ref is judged by its files, like main.
    expect(builds({ ref: "release", env: "production" })).toBe(true);
    commits(["docs/architecture.md"]);
    expect(builds({ ref: "release", env: "production" })).toBe(false);
  });

  it("finds the script from a subdirectory, and builds when it cannot find it", () => {
    commits(["docs/architecture.md"]);
    mkdirSync(join(repo, "apps/web"), { recursive: true });
    expect(builds({}, join(repo, "apps/web"))).toBe(false);
    commits(["packages/engine/src/reduce.ts"]);
    expect(builds({}, join(repo, "apps/web"))).toBe(true);
    // Outside any repository the script is not there, and a missing script is a build.
    const elsewhere = mkdtempSync(join(tmpdir(), "no-repo-"));
    try {
      expect(builds({ ref: "feat/x" }, elsewhere)).toBe(true);
    } finally {
      rmSync(elsewhere, { recursive: true, force: true });
    }
  });
});
