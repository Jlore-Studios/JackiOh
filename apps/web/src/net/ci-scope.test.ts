// scripts/ci-scope.sh: the lines ci.yml's `changes` job uses to skip jobs. It may say `full=false`
// only when every changed file is one no CI job reads (the night bot's code and its switches, the
// workflows that run the bot, the training lanes' prompts and loop), and any other line false only
// when nothing that line's job reads changed (`db` and its three suites, clippy, each Rust test
// group, the web's checks and unit tests, e2e smoke), so each case here is a diff in a throwaway repo
// and the answer a pull request would get: a path alone that leaves a line off, and the same path
// beside an engine source, which turns it on. Every doubt must come out with every line true. The
// scans at the end hold `web` and `e2e` to every repository path the web's and the client's sources
// name; crates/tools/tests/acceptance.rs holds the Rust groups to `cargo metadata`.

import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, posix } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = join(ROOT, "scripts/ci-scope.sh");

/** The script's lines, in the order it prints them. */
const NAMES = [
  "full",
  "db",
  "db_sql",
  "db_store",
  "db_deploy",
  "clippy",
  "rust_engine_rules",
  "rust_engine",
  "rust_cards_lib_core",
  "rust_cards_lib_classic",
  "rust_cards_lib_classic_plus",
  "rust_cards_lib_meditative",
  "rust_cards",
  "rust_server",
  "rust_ai",
  "rust_ai_slow_a",
  "rust_ai_slow_b",
  "rust_tools_wasm",
  "web",
  "e2e",
] as const;
type Name = (typeof NAMES)[number];
type Answer = Record<Name, boolean>;

/** ci.yml's `rust-test` groups. */
const RUST = [
  "rust_engine_rules",
  "rust_engine",
  "rust_cards_lib_core",
  "rust_cards_lib_classic",
  "rust_cards_lib_classic_plus",
  "rust_cards_lib_meditative",
  "rust_cards",
  "rust_server",
  "rust_ai",
  "rust_ai_slow_a",
  "rust_ai_slow_b",
  "rust_tools_wasm",
] as const satisfies readonly Name[];

/** A change every line but the db suites runs for. */
const ENGINE = "crates/engine/src/reduce.rs";

let repo = "";
/** The first commit's sha. */
let baseSha = "";

const GIT_CONFIG = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"];

function git(...args: string[]): string {
  return execFileSync("git", [...GIT_CONFIG, ...args], { cwd: repo, encoding: "utf8" }).trim();
}

/** A commit on top of the first one that writes each file with its content; returns the first commit's sha and the new one's. */
function changeFiles(contents: Record<string, string>): { base: string; head: string } {
  // The files go in a directory of their own, staged over the first commit's tree in an index of
  // their own, so the commit takes one process and no checkout: a case makes dozens of these, and
  // a process is what costs a case its time.
  const files = mkdtempSync(join(tmpdir(), "ci-scope-change-"));
  for (const [file, content] of Object.entries(contents)) {
    mkdirSync(dirname(join(files, file)), { recursive: true });
    writeFileSync(join(files, file), content);
  }
  const head = execFileSync(
    "sh",
    [
      "-c",
      'dir=$1; shift; git read-tree base && (cd "$dir" && git add --ignore-removal .) && git "$@" commit-tree "$(git write-tree)" -p base -m change',
      "sh",
      files,
    ].concat(GIT_CONFIG),
    {
      cwd: repo,
      encoding: "utf8",
      env: { ...process.env, GIT_DIR: join(repo, ".git"), GIT_INDEX_FILE: join(repo, ".git", "change-index") },
    },
  ).trim();
  rmSync(files, { recursive: true, force: true });
  return { base: baseSha, head };
}

/** A commit on top of the first one that adds `files`, each holding its own path. */
const change = (...files: string[]): { base: string; head: string } =>
  changeFiles(Object.fromEntries(files.map((file) => [file, `${file}\n`])));

/** The script's output lines, each `name=true|false`, the names in `NAMES`' order. */
function lines(base: string | undefined, head: string | undefined): Answer {
  const args = [SCRIPT, ...(base === undefined ? [] : [base]), ...(head === undefined ? [] : [head])];
  const run = spawnSync("sh", args, { cwd: repo, encoding: "utf8" });
  expect(run.status, run.stderr).toBe(0);
  const out = run.stdout.trim().split("\n");
  for (const line of out) expect(line, run.stdout).toMatch(/^[a-z0-9_]+=(true|false)$/);
  expect(out.map((line) => line.split("=")[0]), run.stdout).toEqual([...NAMES]);
  return Object.fromEntries(out.map((line) => [line.split("=")[0], line.endsWith("=true")])) as Answer;
}

const every = (value: boolean): Answer => Object.fromEntries(NAMES.map((name) => [name, value])) as Answer;

/** What a pull request that adds `files` gets. */
function answerFor(...files: string[]): Answer {
  const { base, head } = change(...files);
  return lines(base, head);
}

/** The lines a pull request that adds `files` turns on. */
const onFor = (...files: string[]): Name[] => {
  const answer = answerFor(...files);
  return NAMES.filter((name) => answer[name]);
};

const fullFor = (...files: string[]): boolean => answerFor(...files).full;
const dbFor = (...files: string[]): boolean => answerFor(...files).db;
const fullAndDb = (answer: Answer): { full: boolean; db: boolean } => ({ full: answer.full, db: answer.db });

beforeAll(() => {
  repo = mkdtempSync(join(tmpdir(), "ci-scope-"));
  git("init", "-q", "-b", "main");
  writeFileSync(join(repo, "README.md"), "start\n");
  git("add", "-A");
  git("commit", "-q", "-m", "start");
  git("tag", "base");
  baseSha = git("rev-parse", "base");
});

afterAll(() => {
  rmSync(repo, { recursive: true, force: true });
});

describe("scripts/ci-scope.sh", () => {
  it("skips the heavy jobs for a diff of the night bot's own files and nothing else", () => {
    expect(onFor("bot/harness/state.py")).toEqual([]);
    expect(onFor("bot/machine/README.md", ".harness/config.json")).toEqual([]);
    expect(onFor(".github/workflows/bot-night.yml", ".github/workflows/bot-commands.yml", "bot/tests/test_cross.py")).toEqual([]);
    expect(onFor(".github/workflows/triage.yml", ".github/workflows/deploy-watch.yml", ".github/workflows/ci-duration.yml")).toEqual([]);
    expect(onFor("CLAUDE.md", "AGENTS.md", "GEMINI.md", "bot/README.md")).toEqual([]);
    // The repository's meta, which only GitHub reads, and the pull request title check (#187).
    expect(
      onFor(
        "CONTRIBUTING.md",
        ".github/ISSUE_TEMPLATE/patch.yml",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
        ".github/CODEOWNERS",
        ".github/workflows/pr-title.yml",
      ),
    ).toEqual([]);
    // Squishy (#60): its switches and its two workflows, which only the bot's harness reads.
    expect(onFor(".squishy/config.json", ".github/workflows/squishy-run.yml", ".github/workflows/squishy-commands.yml")).toEqual([]);
    // The training lanes' standing prompts and loop: Devin and the training box read them, no job does.
    expect(onFor("training/README.md", "training/improve.md", "training/unban.md", "training/loop.sh")).toEqual([]);
    // The notebooks' project (#445): only super.yml's daily `notebooks` job reads it.
    expect(onFor("analysis/notebooks/01-card-win-rates.ipynb", "analysis/jackioh_records/load.py", "analysis/uv.lock")).toEqual([]);
  });

  // One case per file: each makes four processes, so a case never nears vitest's time limit.
  it.each([
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
    ".github/ISSUE_TEMPLATE-copy/x.yml",
    ".github/CODEOWNERS.bak",
    "docs/CONTRIBUTING.md",
    "analysis-copy/x.py",
    "crates/tools/analysis/x.rs",
  ])("runs everything the moment %s changes, alone or beside the bot's code", (file) => {
    expect(fullFor(file), file).toBe(true);
    expect(fullFor("bot/harness/state.py", file), `bot + ${file}`).toBe(true);
  });

  it("runs everything when it cannot tell what changed", () => {
    const { base, head } = change("bot/harness/state.py");
    expect(lines(base, head)).toEqual(every(false));
    expect(lines(undefined, undefined)).toEqual(every(true));
    expect(lines(base, undefined)).toEqual(every(true));
    expect(lines(base, base)).toEqual(every(true));
    expect(lines("0".repeat(40), head)).toEqual(every(true));
    expect(lines(base, "not-a-commit")).toEqual(every(true));
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
      expect(fullAndDb(answerFor(file)), file).toEqual({ full: true, db: true });
      expect(dbFor(ENGINE, file), `engine + ${file}`).toBe(true);
      expect(dbFor("bot/harness/state.py", file), `bot + ${file}`).toBe(true);
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
      expect(fullAndDb(answerFor(file)), file).toEqual({ full: true, db: false });
    }
    // A pull request that skips everything skips the db job too.
    expect(fullAndDb(answerFor("bot/harness/state.py", ".harness/config.json"))).toEqual({ full: false, db: false });
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
    expect(fullAndDb(lines(before, git("rev-parse", "HEAD")))).toEqual({ full: true, db: true });
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
    expect(lines(moved, branch).full).toBe(false);
    expect(lines(moved, branch).db).toBe(false);
  });

  it("runs every job but the db suites for the workspace's manifests, toolchain, cargo settings, workflows and this script", () => {
    for (const file of [
      "Cargo.toml",
      "rust-toolchain.toml",
      ".cargo/config.toml",
      ".github/workflows/ci.yml",
      ".github/actions/setup/action.yml",
      "scripts/ci-scope.sh",
    ]) {
      expect(onFor(file), file).toEqual(["full", "clippy", ...RUST, "web", "e2e"]);
    }
    // The lockfile also picks the store's sqlx, so it runs two of the db suites besides.
    expect(onFor("Cargo.lock")).toEqual(["full", "db", "db_store", "db_deploy", "clippy", ...RUST, "web", "e2e"]);
  });

  // [db_sql, db_store, db_deploy], one path for each pattern the script's db block names.
  const DB: [string, [boolean, boolean, boolean]][] = [
    ["crates/server/migrations/0034_new.sql", [true, true, true]],
    ["crates/server/tests/sql/01_schema_invariants.sql", [true, false, false]],
    ["crates/server/tests/sql/run.sh", [true, false, false]],
    ["crates/server/tests/db/bootstrap.sql", [false, true, true]],
    ["crates/server/src/db/pg.rs", [false, true, true]],
    ["crates/server/src/db/migrate.rs", [false, true, true]],
    ["crates/server/Cargo.toml", [false, true, true]],
    ["Cargo.lock", [false, true, true]],
    ["crates/server/tests/db/run.sh", [false, true, false]],
    ["crates/server/tests/db/grants.sql", [false, true, false]],
    ["crates/server/tests/store/contract.rs", [false, true, false]],
    ["crates/server/tests/support/deps.rs", [false, true, false]],
    ["crates/server/tests/server.rs", [false, true, false]],
    ["crates/server/src/env.rs", [false, false, true]],
    ["crates/server/src/main.rs", [false, false, true]],
    ["crates/server/src/app.rs", [false, false, true]],
    ["crates/server/src/cli/seed_catalog.rs", [false, false, true]],
    ["crates/server/src/api/catalog.rs", [false, false, true]],
    ["crates/server/Dockerfile", [false, false, true]],
    ["render.yaml", [false, false, true]],
    ["crates/server/tests/deploy/rehearse.sh", [false, false, true]],
  ];
  for (const [file, [sql, store, deploy]] of DB) {
    it(`runs each db suite only on what that suite tests: ${file}`, () => {
      for (const files of [[file], [ENGINE, file], ["bot/harness/state.py", file]]) {
        const { db, db_sql, db_store, db_deploy } = answerFor(...files);
        expect({ db, db_sql, db_store, db_deploy }, files.join(" + ")).toEqual({ db: true, db_sql: sql, db_store: store, db_deploy: deploy });
      }
    });
  }

  // Each line beside the Rust groups and the db suites: a path that turns it on alone, and a path
  // that leaves it off alone and is run beside an engine source.
  const SCOPED: Record<"clippy" | "web" | "e2e", { on: string[]; off: string[] }> = {
    clippy: {
      on: [
        "crates/server/src/main.rs",
        "crates/cards/build.rs",
        "crates/engine/Cargo.toml",
        "crates/engine/clippy.toml",
        "rustfmt.toml",
        ".cargo/config.toml",
      ],
      off: ["apps/web/src/main.tsx", "crates/cards/catalog.json", "crates/server/migrations/0034_new.sql", "docs/architecture.md"],
    },
    web: {
      on: [
        "apps/web/src/main.tsx",
        "e2e/support/commands.ts",
        "crates/cards/catalog.json",
        "crates/ai/src/decide.rs",
        "crates/wasm/src/lib.rs",
        "crates/server/Cargo.toml",
        "crates/tools/Cargo.toml",
        "crates/luau/Cargo.toml",
        "crates/luau/src/lib.rs",
        "crates/server/src/env.rs",
        "crates/server/Dockerfile",
        "crates/server/tests/deploy/rehearse.sh",
        "crates/engine/tests/fixtures/code-input-cases.json",
        "spec/05-card-types.md",
        "package.json",
        "render.yaml",
        "wrangler.jsonc",
      ],
      off: [
        "crates/server/src/api/queue.rs",
        "crates/tools/src/fuzz.rs",
        "crates/luau/tests/api.rs",
        "crates/engine/tests/rules/after_attack.rs",
        "crates/cards/tests/cross/catalog.rs",
        "crates/ai/tests/ai/shadow_ban.rs",
        "crates/wasm/tests/x.rs",
        "crates/server/README.md",
        "crates/engine/README.md",
        "docs/architecture.md",
        "training/history/improve.jsonl",
        "reviews/x.md",
        "analysis/README.md",
        "SPEC.md",
        "BUILD.md",
        "REVIEW.md",
        "README.md",
        "ARCHITECTURE-CCG.md",
        "JackiOh_Core_Cards.md",
      ],
    },
    e2e: {
      on: [
        "apps/web/src/main.tsx",
        "crates/server/src/main.rs",
        "crates/server/migrations/0034_new.sql",
        "crates/tools/src/replay.rs",
        "crates/cards/catalog.json",
        "crates/engine/tests/golden/01-hotseat-full-game.json",
        "package.json",
        "scripts/build-wasm.sh",
      ],
      off: [
        "crates/server/tests/api/queue.rs",
        "crates/engine/tests/rules/after_attack.rs",
        "crates/cards/README.md",
        "apps/web/src/net/ci-scope.test.ts",
        "apps/web/src/game/Board.test.tsx",
        "docs/architecture.md",
        "spec/rulings/R0195.md",
        "training/history/improve.jsonl",
        "reviews/x.md",
        "analysis/README.md",
        "SPEC.md",
        "JackiOh_Core_Cards.md",
        "render.yaml",
        "crates/server/Dockerfile",
        "vercel.json",
        "wrangler.jsonc",
        "eslint.config.js",
        "vitest.config.ts",
        "scripts/browse.sh",
        "scripts/promote-production.sh",
        "scripts/vercel-ignore.sh",
        "scripts/vercel-install.sh",
        "scripts/worktree.sh",
      ],
    },
  };
  for (const [line, { on, off }] of Object.entries(SCOPED) as [keyof typeof SCOPED, (typeof SCOPED)[keyof typeof SCOPED]][]) {
    it(`runs ${line} only when what its job reads changed`, () => {
      for (const file of on) expect(answerFor(file)[line], file).toBe(true);
      for (const file of off) {
        expect(answerFor(file)[line], file).toBe(false);
        expect(answerFor(ENGINE, file)[line], `engine + ${file}`).toBe(true);
      }
    });
  }

  // The Rust test groups each path runs, one path for each pattern of the script's crate map.
  const GROUPS: [string, (typeof RUST)[number][]][] = [
    ["crates/server/.env.example", ["rust_server", "rust_tools_wasm"]],
    ["crates/engine/src/reduce.rs", [...RUST]],
    ["crates/cards/catalog.json", [...RUST]],
    ["crates/ai/src/decide.rs", ["rust_server", "rust_ai", "rust_ai_slow_a", "rust_ai_slow_b", "rust_tools_wasm"]],
    ["crates/server/src/api/queue.rs", ["rust_server"]],
    ["crates/tools/src/fuzz.rs", ["rust_tools_wasm"]],
    ["crates/wasm/src/lib.rs", ["rust_tools_wasm"]],
    ["crates/luau/src/lib.rs", [...RUST]],
    ["crates/scratch/src/lib.rs", [...RUST]],
    ["apps/web/src/wire/generated/Action.ts", ["rust_engine", "rust_server"]],
    ["docs/radiant-audit.md", ["rust_cards"]],
    ["e2e/support/config.ts", ["rust_server"]],
    ["render.yaml", ["rust_server", "rust_tools_wasm"]],
    ["apps/web/.env.production", ["rust_tools_wasm"]],
    ["packages/engine/src/x.ts", ["rust_tools_wasm"]],
    ["apps/server/src/x.ts", ["rust_tools_wasm"]],
    ["ladder/x.ts", ["rust_tools_wasm"]],
    // A crate's manifest also runs the group whose acceptance test holds the map to `cargo metadata`.
    ["crates/server/Cargo.toml", ["rust_server", "rust_tools_wasm"]],
    ["crates/ai/Cargo.toml", ["rust_server", "rust_ai", "rust_ai_slow_a", "rust_ai_slow_b", "rust_tools_wasm"]],
  ];
  it("runs each Rust test group for the crates it builds on and the files its tests read", () => {
    const groups = (...files: string[]): Name[] => onFor(...files).filter((name) => (RUST as readonly Name[]).includes(name));
    for (const [file, expected] of GROUPS) expect(groups(file), file).toEqual(expected);
    for (const file of [
      "apps/web/src/main.tsx",
      "e2e/cypress/e2e/01-hotseat-full-game.cy.ts",
      "docs/architecture.md",
      "spec/rulings/R0195.md",
      "SPEC.md",
      "vercel.json",
    ]) {
      expect(groups(file), file).toEqual([]);
      expect(groups(ENGINE, file), `engine + ${file}`).toEqual([...RUST]);
    }
  });

  it("runs the tools group when a diff adds a line acceptance.rs's scans read", () => {
    const tools = (contents: Record<string, string>): boolean => {
      const { base, head } = changeFiles(contents);
      return lines(base, head).rust_tools_wasm;
    };
    expect(tools({ "crates/server/src/x.rs": 'const X: &str = include_str!("x.json");\n' })).toBe(true);
    expect(tools({ "crates/server/src/x.rs": 'const X: &[u8] = include_bytes!("x.bin");\n' })).toBe(true);
    expect(tools({ "crates/server/src/x.rs": "let s = std::fs::read_to_string(path);\n" })).toBe(true);
    expect(tools({ "crates/server/src/x.rs": 'const X: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/x.json");\n' })).toBe(true);
    expect(tools({ "apps/web/src/x.test.ts": 'const t = readFileSync(join(ROOT, "x.json"), "utf8");\n' })).toBe(true);
    expect(tools({ "apps/web/src/x.test.ts": "export const x = 1;\n" })).toBe(false);
    expect(tools({ "crates/server/src/x.rs": "pub const X: u32 = 1;\n" })).toBe(false);
    expect(tools({ "docs/x.json": "readFileSync(\n" })).toBe(false);
  });
});

// ------------------------------------------------------------------------------------------------
// The paths the web's and the client's sources name
// ------------------------------------------------------------------------------------------------

const TOP = new Set(readdirSync(ROOT));
/** First segments a named path never counts under: the web and e2e run for everything there already. */
const NOT_NAMED = new Set([".git", "node_modules", "apps", "e2e", "target"]);
const SOURCE = /\.(ts|tsx|mjs|js)$/;
const SKIP_DIRS = new Set(["node_modules", "dist", "pkg", ".vite"]);
const TOKENS = /\/\/[^\n]*|\/\*[\s\S]*?\*\/|"((?:[^"\\\n]|\\.)*)"|'((?:[^'\\\n]|\\.)*)'|`((?:[^`\\]|\\.)*)`/g;

/** Every source file under `root` (a repository-relative file or directory), repository-relative. */
function sourcesUnder(root: string): string[] {
  const stat = statSync(join(ROOT, root), { throwIfNoEntry: false });
  if (stat === undefined) return [];
  if (!stat.isDirectory()) return SOURCE.test(root) ? [root] : [];
  return readdirSync(join(ROOT, root), { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory()) return SKIP_DIRS.has(entry.name) ? [] : sourcesUnder(posix.join(root, entry.name));
    return SOURCE.test(entry.name) ? [posix.join(root, entry.name)] : [];
  });
}

/** The repository path a string literal names, read the ways a source may mean it. */
function readings(file: string, literal: string): string[] {
  if (!literal.startsWith(".")) return [literal];
  const out: string[] = [];
  // Against the file's own directory, as `join(HERE, "../…")` and `import` mean it.
  const resolved = posix.normalize(posix.join(posix.dirname(file), literal));
  if (!resolved.startsWith("../")) out.push(resolved);
  // Against a directory the file does not sit in: Cypress's `cy.readFile("../crates/…")` is
  // relative to e2e/, wherever the spec is.
  out.push(literal.replace(/^(\.\.?\/)+/, ""));
  return out;
}

/** The path a reading names, if it is one: under a top-level entry outside apps/ and e2e/, `*` as `x`, a directory as a file in it. */
function named(reading: string, literal: string): string | undefined {
  const path = posix.normalize(reading.replaceAll("*", "x")).replace(/\/+$/, "");
  const first = path.split("/")[0] ?? "";
  if (!TOP.has(first) || NOT_NAMED.has(first) || /\s/.test(path)) return undefined;
  const stat = statSync(join(ROOT, path), { throwIfNoEntry: false });
  if (!literal.includes("/") && !(stat?.isFile() ?? false)) return undefined;
  return stat?.isDirectory() ? `${path}/x` : path;
}

/** Each repository path the sources under `roots` name, with the files that name it. */
function namedPaths(roots: string[], keep: (file: string) => boolean): Map<string, string[]> {
  const found = new Map<string, string[]>();
  for (const file of roots.flatMap(sourcesUnder).filter(keep)) {
    for (const match of readFileSync(join(ROOT, file), "utf8").matchAll(TOKENS)) {
      const literal = match[1] ?? match[2] ?? match[3];
      if (literal === undefined || literal.includes("${")) continue;
      for (const reading of readings(file, literal)) {
        const path = named(reading, literal);
        if (path === undefined) continue;
        const files = found.get(path) ?? [];
        if (!files.includes(file)) files.push(file);
        found.set(path, files);
      }
    }
  }
  return found;
}

/** What the web's typecheck, lint and unit tests read. ci-scope.test.ts and vercel-ignore.test.ts name paths only as data for their throwaway repos. */
const WEB = namedPaths(
  ["apps/web/src", "apps/web/scripts", "apps/web/vite.config.ts", "apps/web/vitest.config.ts", "apps/web/static-pages.ts"],
  (file) => file !== "apps/web/src/net/ci-scope.test.ts" && file !== "apps/web/src/net/vercel-ignore.test.ts",
);
/** What e2e smoke's client build and specs read: e2e/, and the client without its unit tests. */
const E2E = namedPaths(
  ["e2e", "apps/web/vite.config.ts", "apps/web/static-pages.ts", "apps/web/src"],
  (file) => !(file.startsWith("apps/web/src/") && (/\.test\.tsx?$/.test(file) || file.startsWith("apps/web/src/test/"))),
);

describe("scripts/ci-scope.sh against the paths the sources name", () => {
  it("finds the paths it must, so a scan that broke cannot pass empty", () => {
    expect([...WEB.keys()]).toEqual(expect.arrayContaining(["crates/server/src/env.rs", "crates/engine/tests/fixtures/code-input-cases.json"]));
    expect([...E2E.keys()]).toEqual(expect.arrayContaining(["crates/cards/catalog.json"]));
  });

  // A path on the skip list (`bot/…`) leaves every line off, and its own case above proves it.
  for (const [line, paths] of [
    ["web", WEB],
    ["e2e", E2E],
  ] as const) {
    for (const [path, files] of paths) {
      it(`runs ${line} for ${path}, which ${files.join(", ")} names`, () => {
        const answer = answerFor(path);
        expect(!answer.full || answer[line]).toBe(true);
      });
    }
  }
});
