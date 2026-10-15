#!/bin/sh
# `sh scripts/ci-scope.sh <base> <head>`: prints one `<name>=true|false` line per job it scopes,
# which ci.yml's `changes` job appends to $GITHUB_OUTPUT. In order:
#
#   full               every job but `bot selftest`; false only for a diff of skip-list files
#   db                 true when any of the three below is (the `db` job, as one line)
#   db_sql             the sql invariants (crates/server/tests/sql/run.sh)
#   db_store           the store contract against Postgres (crates/server/tests/db/run.sh)
#   db_deploy          the deploy rehearsal (crates/server/tests/deploy/rehearse.sh)
#   clippy             `rust (fmt, clippy)`
#   rust_engine_rules  `rust (test engine rules)`, and so on: each `rust_<group>` is one entry of
#   rust_engine        ci.yml's `rust-test` matrix, named by its `group` (`tools, wasm` is
#   rust_cards_lib_core     `rust_tools_wasm`)
#   rust_cards_lib_classic
#   rust_cards_lib_classic_plus
#   rust_cards_lib_meditative
#   rust_cards
#   rust_server
#   rust_ai
#   rust_ai_slow_a
#   rust_ai_slow_b
#   rust_luau
#   rust_tools_wasm
#   web                `web (typecheck, eslint)` and `web (unit k/3)`
#   e2e                `e2e smoke`
#
# Until ci.yml is wired to them (#751), it reads only `full` and `db`; every other line is printed
# so that wiring needs no change here.
#
# `full=false` means every file the pull request changes sits on the SKIP list below, which holds
# only paths that no CI job other than `bot selftest` ever reads (that check has its own workflow and
# always runs): the night bot's code and its switches, the workflows that run the bot or watch the
# repo (the pull request title check among them), the agent instruction files (CLAUDE.md, AGENTS.md,
# GEMINI.md, CONTRIBUTING.md: code cites them in comments and nothing opens them), the repository's
# meta that only GitHub reads (the issue forms, the pull request template, CODEOWNERS), and the
# training lanes' prompts and loop (training/*.md, training/loop.sh: Devin and the training box read
# them, no job does), and the notebooks' project, analysis/ (#445: no file in crates/, apps/, e2e/,
# spec/ or a per-pull-request job reads it; super.yml's `notebooks` job runs its tests once a day).
# Nothing a test, a build or a check reads is on the list. The other docs are
# not: `spec check` reads spec/, the radiant-standard test reads docs/radiant-audit.md, web tests
# read two spec/ notes (06-keywords.md and 05-card-types.md), and the `training gate` job reads
# training/history/. A `full=false` diff prints every other line false too.
#
# Past the skip list, each changed file turns lines on, and a line no file turned on is false:
#
#  - Run everything but the db suites: the workspace's Cargo.toml, Cargo.lock, rust-toolchain.toml,
#    .cargo/, anything under .github/ that is not on the skip list, and this script.
#  - The db suites, each on what its script exercises (each script's header says): the migrations
#    all three; tests/sql/ the sql invariants; the store (src/db/), the Supabase bootstrap
#    (tests/db/bootstrap.sql), the server's manifest and Cargo.lock the store contract and the deploy
#    rehearsal; the rest of tests/db/, tests/store/ and what they compile with (tests/support/,
#    tests/server.rs) the store contract; the server's boot path (main.rs, app.rs, env.rs, the
#    catalog's seeding, and its route, which is Render's readiness probe), the Dockerfile,
#    render.yaml and tests/deploy/ the deploy rehearsal. A suite a pull request leaves runs in the
#    daily super run (docs/v0.3.0/README.md §7), and its skip passes the check.
#  - clippy: any Rust source, manifest, lockfile, clippy.toml, rustfmt.toml, the toolchain or
#    .cargo/.
#  - The Rust test groups follow the crate graph (`cargo metadata`, dev-dependencies included): a
#    group runs when the change is in a crate it tests or in any crate those build on. The engine or
#    the cards run every group (the engine's tests build on the cards and read their catalog); the AI
#    runs its three groups (ai, ai slow a and ai slow b), server and tools, wasm; the server runs
#    server; tools and wasm run tools, wasm; luau runs luau and tools, wasm (the tools build on it).
#    A crate the map does not know runs every group. A crate's Cargo.toml also runs tools, wasm,
#    whose acceptance test holds this map to `cargo metadata`, so a new path dependency is checked
#    on the pull request that adds it.
#  - The files outside a crate that its tests read run that crate's groups: apps/web/src/wire/
#    (the engine's and the server's export tests write it) engine and server; docs/radiant-audit.md
#    (radiant_standard.rs) cards; e2e/support/config.ts (the server's tests/api/e2e.rs) server;
#    crates/server/.env.example, render.yaml and apps/web/.env.production (the catalog version's
#    sites, which the tools' patches tests read) tools, wasm, the first as well as server;
#    render.yaml too (the server's tests/api/health.rs reads its healthCheckPath, #861) server; and
#    packages/, apps/server/ and ladder/ (acceptance.rs's V28) tools, wasm.
#  - acceptance.rs (tools) also scans every source under apps/, crates/ and e2e/ for a Markdown read
#    (V23) and every Rust source for a file it includes from outside its crate, so a diff there that
#    adds a line holding a reader call (`readFileSync(`, `include_str!(`, `include_bytes!(`,
#    `read_to_string(`) or `CARGO_MANIFEST_DIR` runs tools, wasm.
#  - web runs for everything but what no web test, build or typecheck reads: the server's, the
#    tools' and luau's crates (except each manifest, which every cargo command loads, and the files
#    the web's tests read there: env.rs, the Dockerfile and tests/deploy/), every crate's README, the
#    engine's, the cards', the AI's and the WASM crate's tests (except the engine's tests/fixtures/,
#    which two web tests read), docs/, training/, reviews/, analysis/ and the top-level Markdown.
#  - e2e runs for everything but what neither the client, the server, the `jackioh` binary (whose
#    `replay` specs 01 and 13 run) nor a spec reads: every crate's tests (except the engine's golden
#    traces, which tools' `replay` includes) and README, the web's unit tests, the docs, spec/,
#    training/, reviews/, analysis/, the top-level Markdown, the deploy files (render.yaml, the
#    Dockerfile, vercel.json, wrangler.jsonc), the lint and unit-test configs, and the scripts no
#    build runs.
#
# apps/web/src/net/ci-scope.test.ts holds every line both ways, and holds `web` and `e2e` to every
# repository path their sources name; crates/tools/tests/acceptance.rs holds the Rust groups to
# `cargo metadata` and to every file a crate includes from outside itself.
#
# Anything else, and any doubt, runs everything (every line true): an empty diff, an unreadable one,
# a missing argument. ci.yml never calls this for a push or a dispatch, which always run everything.
#
# Adding a path to the skip list means checking, first, that nothing in crates/, apps/, e2e/, spec/
# or the workflow's other jobs reads it, and narrowing a line means the same for that line's job.

base=$1
head=$2

# The loops below split `git diff`'s output, $LINES and $RUST on whitespace; no name may expand as
# a glob.
set -f

LINES="full db db_sql db_store db_deploy clippy rust_engine_rules rust_engine rust_cards_lib_core rust_cards_lib_classic rust_cards_lib_classic_plus rust_cards_lib_meditative rust_cards rust_server rust_ai rust_ai_slow_a rust_ai_slow_b rust_luau rust_tools_wasm web e2e"
RUST="rust_engine_rules rust_engine rust_cards_lib_core rust_cards_lib_classic rust_cards_lib_classic_plus rust_cards_lib_meditative rust_cards rust_server rust_ai rust_ai_slow_a rust_ai_slow_b rust_luau rust_tools_wasm"

# Every line, set to $1.
every() {
  for name in $LINES; do
    echo "$name=$1"
  done
}

if [ -z "$base" ] || [ -z "$head" ]; then
  every true
  exit 0
fi

# --no-renames: a move lists both paths, so a file moved onto the skip list still counts where it was.
files=$(git diff --no-renames --name-only "$base...$head" 2>/dev/null) || files=
if [ -z "$files" ]; then
  echo "ci-scope: no readable diff for $base...$head, running everything" >&2
  every true
  exit 0
fi

# The skip list: true for a path no job but `bot selftest` reads.
skipped() {
  case "$1" in
    bot/* | .harness/* | .squishy/*) ;;
    CLAUDE.md | AGENTS.md | GEMINI.md | CONTRIBUTING.md) ;;
    .github/ISSUE_TEMPLATE/* | .github/pull_request_template.md | .github/CODEOWNERS) ;;
    training/*.md | training/loop.sh) ;;
    analysis/*) ;;
    .github/workflows/bot-commands.yml | .github/workflows/bot-night.yml | .github/workflows/bot-selftest.yml) ;;
    .github/workflows/squishy-run.yml | .github/workflows/squishy-commands.yml) ;;
    .github/workflows/triage.yml | .github/workflows/deploy-watch.yml | .github/workflows/ci-duration.yml) ;;
    .github/workflows/pr-title.yml) ;;
    *) return 1 ;;
  esac
}

full=false
for file in $files; do
  if ! skipped "$file"; then
    echo "ci-scope: $file is not on the skip list, running everything" >&2
    full=true
    break
  fi
done

if [ "$full" = false ]; then
  every false
  exit 0
fi

# The lines the diff turns on, each named once.
turned=
on() {
  for name in "$@"; do
    case " $turned " in
      *" $name "*) ;;
      *) turned="$turned $name" ;;
    esac
  done
}

for file in $files; do
  skipped "$file" && continue

  case "$file" in
    Cargo.toml | Cargo.lock | rust-toolchain.toml | .cargo/* | .github/* | scripts/ci-scope.sh)
      echo "ci-scope: $file runs every job" >&2
      on clippy $RUST web e2e
      ;;
  esac

  case "$file" in
    crates/server/migrations/*) on db_sql db_store db_deploy ;;
    crates/server/tests/sql/*) on db_sql ;;
    crates/server/tests/db/bootstrap.sql | crates/server/src/db/* | crates/server/Cargo.toml | Cargo.lock) on db_store db_deploy ;;
    crates/server/tests/db/* | crates/server/tests/store/* | crates/server/tests/support/* | crates/server/tests/server.rs) on db_store ;;
    crates/server/src/env.rs | crates/server/src/main.rs | crates/server/src/app.rs | crates/server/src/cli/seed_catalog.rs | \
      crates/server/src/api/catalog.rs | crates/server/Dockerfile | render.yaml | crates/server/tests/deploy/*) on db_deploy ;;
  esac

  case "$file" in
    *.rs | Cargo.toml | */Cargo.toml | Cargo.lock | clippy.toml | */clippy.toml | rustfmt.toml | rust-toolchain.toml | .cargo/*) on clippy ;;
  esac

  # The crate map, which acceptance.rs holds to `cargo metadata`.
  case "$file" in
    crates/server/.env.example) on rust_server rust_tools_wasm ;;
    crates/engine/* | crates/cards/*) on $RUST ;;
    crates/ai/*) on rust_ai rust_ai_slow_a rust_ai_slow_b rust_server rust_tools_wasm ;;
    crates/server/*) on rust_server ;;
    crates/luau/*) on rust_luau rust_tools_wasm ;;
    crates/tools/* | crates/wasm/*) on rust_tools_wasm ;;
    crates/*)
      echo "ci-scope: $file is in no crate the map knows, running every Rust group" >&2
      on $RUST
      ;;
    apps/web/src/wire/*) on rust_engine rust_server ;;
    docs/radiant-audit.md) on rust_cards ;;
    e2e/support/config.ts) on rust_server ;;
    render.yaml) on rust_server rust_tools_wasm ;;
    apps/web/.env.production | packages/* | apps/server/* | ladder/*) on rust_tools_wasm ;;
  esac

  case "$file" in
    crates/*/Cargo.toml) on rust_tools_wasm ;;
  esac

  case "$file" in
    crates/server/Cargo.toml | crates/tools/Cargo.toml | crates/luau/Cargo.toml) on web ;;
    crates/server/src/env.rs | crates/server/Dockerfile | crates/server/tests/deploy/* | crates/engine/tests/fixtures/*) on web ;;
    crates/server/* | crates/tools/* | crates/luau/* | crates/*/README.md) ;;
    crates/engine/tests/* | crates/cards/tests/* | crates/ai/tests/* | crates/wasm/tests/*) ;;
    docs/* | training/* | reviews/* | analysis/*) ;;
    SPEC.md | BUILD.md | REVIEW.md | README.md | ARCHITECTURE-CCG.md | JackiOh_*.md) ;;
    *) on web ;;
  esac

  case "$file" in
    crates/engine/tests/golden/*) on e2e ;;
    crates/*/tests/* | crates/*/README.md | apps/web/*.test.ts | apps/web/*.test.tsx) ;;
    docs/* | spec/* | training/* | reviews/* | analysis/*) ;;
    SPEC.md | BUILD.md | REVIEW.md | README.md | ARCHITECTURE-CCG.md | JackiOh_*.md) ;;
    render.yaml | crates/server/Dockerfile | vercel.json | wrangler.jsonc | eslint.config.js | vitest.config.ts) ;;
    scripts/browse.sh | scripts/promote-production.sh | scripts/vercel-ignore.sh | scripts/vercel-install.sh | scripts/worktree.sh) ;;
    *) on e2e ;;
  esac
done

# acceptance.rs's scans, which read every source under apps/, crates/ and e2e/.
if git diff --no-renames -U0 "$base...$head" -- apps crates e2e 2>/dev/null |
  grep -qE '^\+.*(readFileSync\(|include_str!\(|include_bytes!\(|read_to_string\(|CARGO_MANIFEST_DIR)'; then
  on rust_tools_wasm
fi

echo "full=true"
case " $turned " in
  *" db_sql "* | *" db_store "* | *" db_deploy "*) echo "db=true" ;;
  *) echo "db=false" ;;
esac
for name in $LINES; do
  case "$name" in
    full | db) continue ;;
  esac
  case " $turned " in
    *" $name "*) echo "$name=true" ;;
    *) echo "$name=false" ;;
  esac
done
