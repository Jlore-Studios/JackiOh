// scripts/vercel-ignore.sh, run the way vercel.json runs it (its `ignoreCommand`), in a throwaway git
// repo: Vercel deploys on the last push, only when the push can change what the site serves, and on
// main only once MIN_GAP_SECONDS have passed since the last build.
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

/** How long main waits after the last build (rule 4), read from the script so it is stated once. */
const MIN_GAP_SECONDS = Number(/^MIN_GAP_SECONDS=(\d+)$/m.exec(readFileSync(join(ROOT, "scripts/vercel-ignore.sh"), "utf8"))?.[1]);

/** A day, in seconds: the default distance between two commits, past rule 4's wait. */
const DAY = 86_400;

let repo = "";
let base = "";
/** The committer date of every git call, in seconds since the epoch; `commit` moves it on. */
let now = Date.UTC(2026, 0, 1) / 1000;

/** Where the throwaway repo lives: the checkout's own ignored `.cache/`, not the machine's shared temp
 * directory, and with git's background upkeep off, so nothing but this file touches it while it runs
 * (#1016: on main at 67811e0 a `git add` failed mid-test because, from git 2.54, the first case's
 * commits start a detached repack whose prune-packed removed the objects/xx directory the add had
 * just made). */
const SCRATCH = join(ROOT, ".cache");

function git(...args: string[]): string {
  const quiet = ["-c", "maintenance.auto=false", "-c", "gc.auto=0"];
  return execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", ...quiet, ...args], {
    cwd: repo,
    encoding: "utf8",
    env: { ...process.env, GIT_COMMITTER_DATE: `@${String(now)} +0000` },
  }).trim();
}

/** Commits what is staged, `after` seconds after the commit before it (a day unless a test says). */
function commit(message: string, after = DAY): string {
  now += after;
  git("commit", "-q", "-m", message);
  return git("rev-parse", "HEAD");
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
    commit("change");
  }
}

/** `remote` stands in for the repository on GitHub, where rule 2a reads the countdown issue's tags. */
type Env = { ref?: string; message?: string; env?: string; previous?: string | null; remote?: string };

/** Whether Vercel would build, running ignoreCommand in `cwd` with the variables Vercel sets. */
function builds({ ref = "main", message = "A change", env = "preview", previous = base, remote }: Env = {}, cwd = repo): boolean {
  const vars: Record<string, string> = {
    PATH: process.env.PATH ?? "",
    VERCEL_GIT_COMMIT_REF: ref,
    VERCEL_GIT_COMMIT_MESSAGE: message,
    VERCEL_ENV: env,
  };
  if (previous !== null) vars.VERCEL_GIT_PREVIOUS_SHA = previous;
  if (remote !== undefined) vars.VERCEL_STAGING_REMOTE = remote;
  const run = spawnSync("sh", ["-c", ignoreCommand], { cwd, env: vars, encoding: "utf8" });
  expect(run.error, "sh ran").toBeUndefined();
  expect([0, 1], `exit status ${String(run.status)}: ${run.stderr}`).toContain(run.status);
  return run.status === 1;
}

beforeAll(() => {
  mkdirSync(SCRATCH, { recursive: true });
  repo = mkdtempSync(join(SCRATCH, "vercel-ignore-"));
  git("init", "-q", "-b", "main");
  mkdirSync(join(repo, "scripts"));
  copyFileSync(join(ROOT, "scripts/vercel-ignore.sh"), join(repo, "scripts/vercel-ignore.sh"));
  writeFileSync(join(repo, "README.md"), "start\n");
  git("add", "-A");
  base = commit("built");
});

afterAll(() => {
  rmSync(repo, { recursive: true, force: true });
});

describe("scripts/vercel-ignore.sh", () => {
  it("builds main when a file the bundle reads changed", () => {
    for (const file of [
      // The four crates scripts/build-wasm.sh compiles into the WASM module, and the card data.
      "crates/engine/src/reduce.rs",
      "crates/engine/src/wire/view.rs",
      "crates/engine/Cargo.toml",
      "crates/cards/catalog.json",
      "crates/cards/flavour.json",
      "crates/cards/chinese.json",
      "crates/cards/chinese-terms.json",
      "crates/cards/build.rs",
      "crates/cards/patches/patches.json",
      "crates/ai/src/decide.rs",
      "crates/ai/generation.json",
      "crates/wasm/src/lib.rs",
      // The card scripts are compiled in: a scripts/ directory at the root is tooling, src/scripts/ is not.
      "crates/cards/src/scripts/core/c001_big_d_fender.rs",
      // The testkit is a feature of the engine's own src/, not a tests/ directory.
      "crates/engine/src/testkit/scenario.rs",
      // What cargo reads to build any member: the workspace, the lockfile, the toolchain, every manifest.
      "Cargo.toml",
      "Cargo.lock",
      "rust-toolchain.toml",
      ".cargo/config.toml",
      "crates/server/Cargo.toml",
      "crates/tools/Cargo.toml",
      "scripts/build-wasm.sh",
      "scripts/vercel-install.sh",
      "apps/web/src/main.tsx",
      "apps/web/src/wasm/index.ts",
      "apps/web/src/wire/generated/PlayerView.ts",
      "apps/web/public/robots.txt",
      "package.json",
      "pnpm-lock.yaml",
      "pnpm-workspace.yaml",
      "vercel.json",
      "assets/music/a.mid",
      "tsconfig.base.json",
      "apps/web/vite.config.ts",
      "apps/web/index.html",
      // Named like test support, but not a *.test.* file or a test/ directory, so not proved unread.
      "apps/web/src/game/deckbuilder/testkit.ts",
      "apps/web/src/patches/fixtures.ts",
    ]) {
      commits([file]);
      expect(builds(), file).toBe(true);
    }
  });

  it("skips main when only the night bot, docs, the spec, the training lanes, the server, the CLI, CI, tests or tooling changed", () => {
    for (const files of [
      ["bot/harness/state.py", ".harness/config.json"],
      [".squishy/config.json", ".github/workflows/squishy-run.yml"],
      [".github/workflows/ci.yml", ".github/actions/setup/action.yml"],
      ["docs/architecture.md", "SPEC.md", "BUILD.md", "CLAUDE.md", "REVIEW.md", "README.md", "CONTRIBUTING.md"],
      // The server and the CLI are never compiled into the WASM module; only their manifests are read.
      ["crates/server/src/main.rs", "crates/server/migrations/0001_init.sql", "crates/server/Dockerfile", "render.yaml"],
      ["crates/server/tests/server.rs", "crates/server/.env.example", "crates/tools/src/fuzz.rs"],
      ["e2e/cypress/e2e/01-hotseat-full-game.cy.ts", "reviews/2026-10-03.md"],
      ["scripts/ci-scope.sh", "scripts/golden/record.ts", "JackiOh_Core_Cards.md", "ARCHITECTURE-CCG.md"],
      ["spec/rulings/R0195.md", "spec/INDEX.md", "training/README.md", "training/history/improve.jsonl"],
      // Tests and tooling inside the client and the crates are never imported by the bundle.
      ["apps/web/src/game/Board.test.tsx", "apps/web/src/net/x.test.ts", "apps/web/src/test/setup.ts"],
      ["crates/engine/tests/rules/combat.rs", "crates/engine/tests/golden/games.jsonl", "crates/engine/tests/fixtures/code-input-cases.json"],
      ["crates/cards/tests/cross/catalog.rs", "crates/ai/tests/ai.rs", "crates/wasm/tests/x.rs"],
      ["apps/web/scripts/gen-voice.mjs", "apps/web/README.md"],
    ]) {
      commits(files);
      expect(builds(), files.join(", ")).toBe(false);
    }
  });

  it("builds when one file the bundle reads sits among files that do not", () => {
    commits(["bot/harness/state.py", "docs/architecture.md", "crates/ai/src/decide.rs"]);
    expect(builds()).toBe(true);
    commits(["docs/x.md", "apps/web/src/x.ts"]);
    expect(builds()).toBe(true);
    // A path that only looks like a skipped one.
    commits(["apps/serverless/x.ts"]);
    expect(builds()).toBe(true);
    commits(["crates/serverless/src/x.rs"]);
    expect(builds()).toBe(true);
    commits(["botany/x.ts"]);
    expect(builds()).toBe(true);
    commits(["docs-copy/x.md"]);
    expect(builds()).toBe(true);
    commits(["training-data/x.json"]);
    expect(builds()).toBe(true);
    // A test or tooling file beside a source file the bundle reads is still a build.
    commits(["apps/web/src/game/Board.test.tsx", "apps/web/src/game/Board.tsx"]);
    expect(builds()).toBe(true);
    commits(["crates/cards/tests/cross/catalog.rs", "crates/cards/src/scripts/core/c002_bigot.rs"]);
    expect(builds()).toBe(true);
    commits(["crates/server/src/main.rs", "crates/engine/src/reduce.rs"]);
    expect(builds()).toBe(true);
    commits(["crates/tools/src/patches.rs", "crates/cards/catalog.json"]);
    expect(builds()).toBe(true);
    // The script that builds the module sits in scripts/, which is otherwise tooling.
    commits(["scripts/ci-scope.sh", "scripts/build-wasm.sh"]);
    expect(builds()).toBe(true);
    // A manifest of a member the module does not use is still read by cargo.
    commits(["crates/server/src/main.rs", "crates/server/Cargo.toml"]);
    expect(builds()).toBe(true);
    // Paths that only look like tests or tooling: a tests/ below src, another suffix.
    commits(["crates/cards/src/tests/x.rs"]);
    expect(builds()).toBe(true);
    commits(["crates/engine/tests.rs"]);
    expect(builds()).toBe(true);
    commits(["apps/web/src/game/Board.tsx.test"]);
    expect(builds()).toBe(true);
    commits(["apps/web/src/net/test.ts"]);
    expect(builds()).toBe(true);
  });

  it("builds when a file the bundle reads is moved onto the skip list", () => {
    commits(["apps/web/src/game/Board.tsx"]);
    const built = git("rev-parse", "HEAD");
    writeFileSync(join(repo, "apps/web/src/game/Board.tsx"), "export const board = 1;\n".repeat(20));
    git("add", "-A");
    const withBoard = commit("board");
    mkdirSync(join(repo, "docs"), { recursive: true });
    git("mv", "apps/web/src/game/Board.tsx", "docs/Board.tsx");
    commit("moved into docs");
    expect(builds({ previous: withBoard })).toBe(true);
    expect(builds({ previous: built })).toBe(true);
  });

  it("never skips past a commit that was not built: it diffs against the last one that was", () => {
    // A changed the bundle and its build never ran (the daily cap); B only touched the bot.
    commits(["crates/engine/src/a.rs"], ["bot/harness/b.py"]);
    expect(builds()).toBe(true);
    // Once A is the last build, B alone is skipped.
    const built = git("rev-parse", "HEAD~1");
    expect(builds({ previous: built })).toBe(false);
  });

  it("waits MIN_GAP_SECONDS after the last build on main, then builds what the wait skipped", () => {
    expect(MIN_GAP_SECONDS, "the script states MIN_GAP_SECONDS").toBeGreaterThan(0);
    commits(["crates/engine/src/a.rs"]);
    const built = git("rev-parse", "HEAD");
    // A bundle change committed in the wait's last second is skipped, unless the flag asks for it.
    writeFileSync(join(repo, "crates/engine/src/a.rs"), "changed\n");
    git("add", "-A");
    commit("soon after", MIN_GAP_SECONDS - 1);
    expect(builds({ previous: built })).toBe(false);
    expect(builds({ previous: built, message: "Fix staging [vercel]" })).toBe(true);
    // Once the wait is over, a push that only touched the docs builds, carrying the skipped change.
    mkdirSync(join(repo, "docs"), { recursive: true });
    writeFileSync(join(repo, "docs/after.md"), "x\n");
    git("add", "-A");
    commit("after the wait", 1);
    expect(builds({ previous: built })).toBe(true);
  });

  it("builds a commit dated before the last build, rather than wait on a clock it cannot trust", () => {
    commits(["crates/engine/src/a.rs"]);
    const built = git("rev-parse", "HEAD");
    writeFileSync(join(repo, "crates/engine/src/a.rs"), "changed\n");
    git("add", "-A");
    commit("dated earlier", -60);
    expect(builds({ previous: built })).toBe(true);
  });

  it("builds on any doubt about what was last built", () => {
    commits(["bot/harness/state.py"]);
    expect(builds({ previous: null })).toBe(true);
    expect(builds({ previous: "" })).toBe(true);
    expect(builds({ previous: "0".repeat(40) })).toBe(true);
    expect(builds({ previous: "not-a-commit" })).toBe(true);
    // Nothing changed since the last build (a redeploy of the same commit, which rule 4's wait does
    // not skip either).
    expect(builds({ previous: git("rev-parse", "HEAD") })).toBe(true);
    // The last build is not an ancestor of this commit (a preview from another branch). Every file
    // between the two is a skippable one, so only the ancestry check can make this a build.
    git("checkout", "-q", "-B", "elsewhere", base);
    mkdirSync(join(repo, "docs"), { recursive: true });
    writeFileSync(join(repo, "docs/elsewhere.md"), "x\n");
    git("add", "-A");
    const elsewhere = commit("elsewhere");
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
    commits(["crates/engine/src/reduce.rs"]);
    expect(builds({ ref: "feat/live-cards" })).toBe(false);
    expect(builds({ ref: "fix/anim-double", env: "preview" })).toBe(false);
    expect(builds({ ref: "" })).toBe(false);
    // A production deploy of another ref is judged by its files, like main.
    expect(builds({ ref: "release", env: "production" })).toBe(true);
    commits(["docs/architecture.md"]);
    expect(builds({ ref: "release", env: "production" })).toBe(false);
  });

  describe("rule 2a, the countdown issue's Vercel commands, as tags on GitHub", () => {
    /** A bare repository standing in for GitHub, with the tags it is given pointing at commits of `repo`. */
    function github(tags: Record<string, string>): string {
      const remote = mkdtempSync(join(SCRATCH, "vercel-remote-"));
      execFileSync("git", ["init", "-q", "--bare", remote]);
      for (const [tag, sha] of Object.entries(tags)) git("push", "-q", remote, `${sha}:refs/tags/${tag}`);
      return remote;
    }
    const remotes: string[] = [];
    afterAll(() => {
      for (const remote of remotes) rmSync(remote, { recursive: true, force: true });
    });
    const remote = (tags: Record<string, string>): string => {
      const made = github(tags);
      remotes.push(made);
      return made;
    };

    it("builds nothing on main while vercel-hold is there, the [vercel] flag aside, and as before once it is gone", () => {
      commits(["crates/engine/src/reduce.rs"]);
      const head = git("rev-parse", "HEAD");
      const held = remote({ "vercel-hold": head });
      expect(builds({ remote: held })).toBe(false);
      expect(builds({ remote: held, env: "production" })).toBe(false);
      expect(builds({ remote: held, previous: null })).toBe(false);
      expect(builds({ remote: held, message: "Hotfix [vercel]" })).toBe(true);
      // Another branch is skipped by rule 2 first, held or not.
      expect(builds({ remote: held, ref: "feat/x" })).toBe(false);
      expect(builds({ remote: remote({}) })).toBe(true);
    });

    it("builds past rule 4's wait while vercel-fast-forward names a commit this one has and the last build lacks", () => {
      commits(["crates/engine/src/a.rs"]);
      const built = git("rev-parse", "HEAD");
      writeFileSync(join(repo, "crates/engine/src/a.rs"), "changed\n");
      git("add", "-A");
      const asked = commit("soon after", MIN_GAP_SECONDS - 60);
      // Without the tag, the wait skips it.
      expect(builds({ previous: built })).toBe(false);
      expect(builds({ previous: built, remote: remote({}) })).toBe(false);
      // The tag on it, or on a commit before a newer one, builds it now.
      expect(builds({ previous: built, remote: remote({ "vercel-fast-forward": asked }) })).toBe(true);
      writeFileSync(join(repo, "crates/engine/src/a.rs"), "changed again\n");
      git("add", "-A");
      commit("a minute later", 60);
      expect(builds({ previous: built, remote: remote({ "vercel-fast-forward": asked }) })).toBe(true);
      // Once a build has the commit, the tag is spent: the next push waits again.
      expect(builds({ previous: asked, remote: remote({ "vercel-fast-forward": asked }) })).toBe(false);
      // A hold wins over a fast-forward (which lifts the hold, removing its tag, when it is asked for).
      expect(builds({ previous: built, remote: remote({ "vercel-fast-forward": asked, "vercel-hold": asked }) })).toBe(false);
    });

    it("leaves the rules as they were when it cannot read the tags", () => {
      commits(["crates/engine/src/reduce.rs"]);
      expect(builds({ remote: join(SCRATCH, "no-such-remote") })).toBe(true);
      commits(["docs/architecture.md"]);
      expect(builds({ remote: join(SCRATCH, "no-such-remote") })).toBe(false);
    });
  });

  it("finds the script from a subdirectory, and builds when it cannot find it", () => {
    commits(["docs/architecture.md"]);
    mkdirSync(join(repo, "apps/web"), { recursive: true });
    expect(builds({}, join(repo, "apps/web"))).toBe(false);
    commits(["crates/engine/src/reduce.rs"]);
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
