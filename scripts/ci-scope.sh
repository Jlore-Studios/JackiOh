#!/bin/sh
# `sh scripts/ci-scope.sh <base> <head>`: prints one `<name>=true|false` line per job it scopes,
# which ci.yml's `changes` job appends to $GITHUB_OUTPUT. In order:
#
#   full               every job but `bot selftest` (ci.yml's `changes` job scopes it, `bot`); false
#                      only for a diff of skip-list files
#   db                 true when any of the three below is (the `db` job, as one line)
#   db_sql             the sql invariants (crates/server/tests/sql/run.sh)
#   db_store           the store contract against Postgres (crates/server/tests/db/run.sh)
#   db_deploy          the deploy rehearsal (crates/server/tests/deploy/rehearse.sh)
#   clippy             `rust (fmt, clippy)`
#   rust_engine_rules  `rust (test engine rules)`, and so on: each `rust_<group>` is one of
#   rust_engine        .github/ci/rust-test.toml's groups, by its `scope` (`tools, wasm` is
#   rust_cards_lib     `rust_tools_wasm`, which also runs `tools fuzz` and `tools duos, selfplay`;
#   rust_cards         `rust_engine` also runs `engine golden`, `rust_engine_rules` `engine
#   rust_server        rules 2` and `engine rules 3`, and `rust_server` `server api`, `server
#                      practice` and `server practice rulings`)
#   rust_ai
#   rust_ai_slow
#   rust_tools_wasm
#   web                `web (typecheck, eslint)` and `web (unit k/4)`; the WASM modules and the client
#                      build run on `web` or `e2e`
#   web_promote        `web (promote-production)`, ci.yml's run of
#                      apps/web/src/net/promote-production.test.ts, which the shards leave out (the
#                      daily super run runs it whatever changed)
#   e2e                `e2e smoke`
#   infra              `rust (checks, fuzz 200)`'s terraform steps over infra/ (#1708): fmt, init
#                      without the backend, validate, and the lock file
#
# ci.yml's `changes` job runs `main`'s copy of this script, the base's, not a pull request's own
# (#2986): an edit here scopes the pull requests after it lands, never the one that makes it, which
# runs every job (this script changed) and any line only its copy declares in `LINES=`, as `true`
# (read, never run). Keep `LINES=` one line of names.
#
# ci.yml's `changes` job exports every line and each job runs on its own (#751): a job-level `if:`
# for a job of its own, and for a matrix entry (`rust-test`'s groups, `db-suite`'s suites), whose
# job-level `if:` cannot see `matrix`, a first step that turns the entry's other steps off. Each
# summary job accepts a skip only when the skipped piece's own line says `false`.
#
# `full=false` means every file the pull request changes sits on the SKIP list below, which holds
# only paths that no CI job other than `bot selftest` ever reads (ci.yml's `changes` job scopes that
# check on its own, #1455): the night bot's code and its switches, the workflows that run the bot or watch the
# repo (the pull request title check among them), the agent instruction files (CLAUDE.md, AGENTS.md,
# GEMINI.md, CONTRIBUTING.md: code cites them in comments and nothing opens them), the repository's
# meta that only GitHub reads (the issue forms, the pull request template, CODEOWNERS), and the
# training lanes' prompts and loop (training/*.md, training/loop.sh: Muse and the training box read
# them, no job does; training/history/ is not among them), and the notebooks' project, analysis/
# (#445: no file in crates/, apps/, e2e/, spec/ or a per-pull-request job reads it; super.yml's
# `notebooks` job runs its tests once a day), but for the five record files and reports of
# analysis/fixtures/ that the tools' R1994000 test holds `stats report --games` to, and the
# mutation-testing tooling, which mutants.yml runs and checks on its own trigger
# (.cargo/mutants.toml, scripts/mutants.sh, scripts/mutants-runner.sh), and the pool probe
# (scripts/pool-probe.sh), which a person runs. Nothing in crates/, apps/, e2e/ or spec/ reads
# any of the four: only comments name them (crates/run/src/tests.rs:987,
# crates/engine/src/testkit/pool_probe.rs:7, the two Stryker configs, .github/mutants-floor.txt's
# header, mutants.yml itself), and no per-pull-request job runs them (no workflow names
# pool-probe.sh; `grep mutants .github/workflows/ci.yml` finds nothing). A later CI job that
# starts reading one of these files must take it off the list in the same pull request.
# Nothing a test, a build or a check reads is on the list, and nothing under crates/: ci.yml's
# `rust (classify)` (#1370) reads every change to the cards, the engine and the AI, and runs exactly
# when `full` does. The other docs are
# not: `spec check` reads spec/, the radiant-standard test reads docs/radiant-audit.md, the catalog
# test reads spec/07-tokens.md's table of tokens, radiant_stats.rs reads spec/08-catalog.md, web tests
# read two spec/ notes (06-keywords.md and 05-card-types.md), and the `training gate` job reads
# training/history/, and the tools' test of shadow_ban.rs reads its sweep-*.md reports. A
# `full=false` diff prints every other line false too.
#
# Past the skip list, each changed file turns lines on, and a line no file turned on is false:
#
#  - A diff in which every file is one `cargo jackioh patches ship` or `patches cut` writes runs `full`
#    and only rust_cards_lib, rust_cards, rust_server and rust_tools_wasm, plus web when
#    apps/web/.env.production is in it (#4094, an owner-decided cut of per-pull-request coverage: the
#    WASM module and the client embed the catalog, yet web, e2e, the engine's and the AI's groups and every
#    db line stay off; the push to `main` and the daily super run still run everything). Those files are
#    crates/cards/patches/'s v*.json, patches.json, shipped.json, index.json, card_metrics/, records/ and
#    pending/, crates/server/.env.example and apps/web/.env.production, and both patches.json and
#    shipped.json must be in the diff, as no other pull request edits them. Any other file in the diff
#    scopes as below.
#  - Run everything but the db suites: the workspace's Cargo.toml, Cargo.lock, rust-toolchain.toml,
#    .cargo/ but .cargo/mutants.toml, anything under .github/ that is not on the skip list, and this script. Of these, only
#    .github/ and this script run infra too: the terraform steps read no cargo file.
#  - infra: infra/, of which only infra/cloudflare.tf turns web on (apps/edge's config
#    test reads it, #4011); no e2e line turns on for it.
#  - The db suites, each on what its script exercises (each script's header says): the migrations
#    all three; tests/sql/ the sql invariants; the store (src/db/), the Supabase bootstrap
#    (tests/db/bootstrap.sql), the server's manifest and Cargo.lock the store contract and the deploy
#    rehearsal, and so do the shared crate's source and manifest (`jackioh-shared`, whose series rows
#    and frozen decks the store writes and whose build the image copies, #3098); the rest of tests/db/,
#    tests/store/ and what they compile with (tests/server_store.rs
#    and tests/support/test_app.rs, the one support module it takes) and what its seed test reads
#    (the real catalog.json and the shipped sets, catalog_types.rs) the store contract; the server's
#    boot path (main.rs, app.rs, env.rs, the catalog's seeding, and its route, which the deploys'
#    live check polls), the Dockerfile, wrangler.jsonc (the image, its build context and the
#    server's settings, which the rehearsal reads as wrangler builds and runs them),
#    crates/server/.env.example (the catalog version it boots on) and tests/deploy/ the deploy
#    rehearsal. A suite
#    a pull request leaves runs in the daily super run (docs/v0.3.0/README.md §7), and its skip
#    passes the check.
#  - clippy: any Rust source, manifest, lockfile, clippy.toml, rustfmt.toml, the toolchain or
#    .cargo/ but .cargo/mutants.toml.
#  - The Rust test groups follow the crate graph (`cargo metadata`, dev-dependencies included): a
#    group runs when the change is in a crate it tests or in any crate those build on. The engine or
#    the cards run every group (the engine's tests build on the cards and read their catalog); the AI
#    runs its two groups (ai and ai slow), server and tools, wasm; the server runs
#    server; tools and wasm run tools, wasm; the shared crate (`jackioh-shared`, which the server builds on, #3098) runs server and tools, wasm.
#    A crate the map does not know runs every group. A crate's Cargo.toml also runs tools, wasm,
#    whose acceptance test holds this map to `cargo metadata`, so a new path dependency is checked
#    on the pull request that adds it.
#  - The files outside a crate that its tests read run that crate's groups: apps/web/src/wire/
#    (the engine's and the server's export tests write it, and jackioh-wasm-core's constants test reads
#    engineConfig.ts, #3093) engine and server, and tools, wasm; docs/radiant-audit.md
#    (radiant_standard.rs), spec/07-tokens.md (the catalog test's table of tokens, R1177001) and
#    spec/08-catalog.md (radiant_stats.rs, a row's two Radiant stats cells, #3804) cards; e2e/support/config.ts (the server's tests/api/e2e.rs) server;
#    crates/server/.env.example and apps/web/.env.production (the catalog version's sites, which the
#    tools' patches tests read; the ship case above takes precedence) tools, wasm, the first as well as server; crates/server/tests/'s
#    actor/, api/, bots/, loops/ and support/ run server only because the server binary checks their
#    shared helpers itself (#2385). If a later acceptance.rs scan reads those trees, its added lines
#    must match a reader call, `CARGO_MANIFEST_DIR`, `#[ignore` or `preview_sets(` in the grep below
#    before tools, wasm may remain off. crates/server/migrations/0068_v0_5_retcon.sql and
#    0087_v0_5_13_retcon.sql (the tools' r2532000 and r3631001 retcon tests read them through
#    repo_root().join(MIGRATIONS_REL), #4157) tools, wasm as well as server; and
#    packages/, apps/server/ and ladder/ (acceptance.rs's V28) tools, wasm; training/history/sweep-*.md
#    (the tools' test that holds crates/ai/src/shadow_ban.rs to its reports of record) tools, wasm;
#    analysis/fixtures/'s dev-40.jsonl, live-40.jsonl, games.report.json, counts.jsonl and
#    counts.report.json (the tools' test that holds `stats report --games` to them, R1994000) tools,
#    wasm; apps/web/src/audio/voice-text.json (the
#    tools' export_card_text.rs writes it from crates/cards/card-text.yaml, #2498) tools, wasm; and
#    apps/web/src/game/cardModes.json (the tools' export_card_modes.rs writes it from the card
#    scripts, #2646) tools, wasm; and spec/15-dungeon-run.md and spec/README.md (acceptance.rs holds
#    the first's table of constants to crates/run/src/config.rs, and the second's map to it, B33)
#    tools, wasm. scripts/ci-shards.toml, the shard counts of ci.yml's sharded jobs (#3072), runs
#    every Rust group, and web and e2e as any file outside the lists below does, so a count it
#    changes is seen running.
#  - acceptance.rs (tools) also scans every source under apps/, crates/ and e2e/ for a Markdown read
#    (V23), every Rust source for a file it includes from outside its crate, every Rust source for
#    an `#[ignore]` that does not say it runs daily, and every Rust source for a preview of sets that
#    all ship (#2257), so a diff there that adds a line holding a reader call (`readFileSync(`,
#    `include_str!(`, `include_bytes!(`, `read_to_string(`), `CARGO_MANIFEST_DIR`, `#[ignore` or
#    `preview_sets(` runs tools, wasm.
#  - web runs for everything but what no web test, build or typecheck reads: the server's and the
#    tools' crates (except each manifest, which every cargo command loads, and the files
#    the web's tests read there: env.rs, app.rs (whose probe routes apps/edge's config test holds),
#    the Dockerfile, tests/deploy/, the practice claim fixture,
#    tests/fixtures/practice-claim.json, which the worker's claim test plays), every crate's README, the
#    engine's, the cards', the AI's, the shared crate's tests (its source and data run it: the shared crate's
#    quest pool, quests.json, has titles the quests panel's test holds to zh-CN, #3104) and the WASM crate's tests (except the engine's tests/fixtures/,
#    which two web tests read), the C module's tests (crates/wasm/c/src/tests.rs; its other sources run
#    `web`, since `web (wasm c module)` builds them, #3097), docs/, training/ (but training/file-records.sh,
#    which env-production.test.ts holds to R1933001), reviews/, analysis/, infra/ but
#    infra/cloudflare.tf (#4011) and the top-level
#    Markdown. The Worker in front of the server, apps/edge/, and wrangler.jsonc are among what it
#    runs for: the job lints, typechecks and tests the Worker, and cloudflare-config.test.ts reads
#    the file.
#  - web_promote runs for what promote-production.test.ts and promote-staging.test.ts run and read:
#    scripts/promote-production.sh and scripts/promote-staging.sh, the workflows they read (under
#    .github/, so every job runs), and the tests themselves,
#    and for what any web test runs on (the root and the web's package.json, the lockfile, the pnpm
#    workspace, .nvmrc, both vitest configs and apps/web/src/test/). It turns `web` on too, so the
#    job that runs it runs.
#  - e2e runs for everything but what neither the client, the server, the `jackioh` binary (of whose
#    commands it runs two: `replay`, whose specs 01 and 13 fold the browser's game, and `catalog-version`,
#    which the shard action runs to boot the server) nor a spec reads: every crate's tests (except the
#    engine's tests/golden/, whose hotseat fixture tools' `replay` includes) and README, the C module's
#    sources (crates/wasm/c/src/: no client, server or `jackioh` links it; its manifest, which every cargo
#    command loads, is not among them), the tools' sources but the four those commands run (#4219: main.rs,
#    which dispatches them, replay.rs, catalog.rs, whose run_version is `catalog-version`, and patches.rs,
#    whose `js` parser and repo_root it calls) and the fuzz's records (crates/tools/fuzz-corpus/), so a
#    change to the fuzz, the classifier or another command leaves e2e off. That is safe: `replay`'s shipped
#    code names no other module of the crate (acceptance.rs's scan holds it and names the file to add here
#    when that stops being true), replay.rs's own tests pin the hotseat game's hash in `rust (test tools,
#    wasm)`, which every change under crates/tools/ runs, and the push to `main` runs e2e smoke after the
#    merge (#3564). crates/tools/Cargo.toml is not among them. The web's unit tests, the docs, spec/,
#    training/, reviews/, analysis/, infra/, the top-level Markdown, the deploy files (the Dockerfile,
#    wrangler.jsonc and the Worker, apps/edge/: e2e smoke boots jackioh-server itself, behind no
#    Worker), the dev dashboard (apps/admin/: the smoke run boots jackioh-server itself and never serves
#    it; only the daily dashboard spec builds and previews it, spec 49, whose one dashboard read is
#    apps/admin/src/testids.ts, #4371), the lint and unit-test configs, and the scripts no build runs (scripts/browse.sh,
#    scripts/promote-production.sh, scripts/promote-staging.sh, scripts/server-image.sh,
#    scripts/worktree.sh).
#
# apps/web/src/net/ci-scope.test.ts holds every line both ways, and holds `web` and `e2e` to every
# repository path the web's, the Worker's and the dashboard's sources name (apps/web, apps/edge,
# apps/admin); crates/tools/tests/acceptance.rs holds the Rust groups to
# `cargo metadata` and to every file a crate includes from outside itself, and `replay`'s source
# (tools' replay.rs) to the modules the e2e case keeps on (#4219).
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

LINES="full db db_sql db_store db_deploy clippy rust_engine_rules rust_engine rust_cards_lib rust_cards rust_server rust_ai rust_ai_slow rust_tools_wasm web web_promote e2e infra"
RUST="rust_engine_rules rust_engine rust_cards_lib rust_cards rust_server rust_ai rust_ai_slow rust_tools_wasm"

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
    training/history/*) return 1 ;;
    training/*.md | training/loop.sh) ;;
    analysis/fixtures/dev-40.jsonl | analysis/fixtures/live-40.jsonl | analysis/fixtures/games.report.json) return 1 ;;
    analysis/fixtures/counts.jsonl | analysis/fixtures/counts.report.json) return 1 ;;
    analysis/*) ;;
    .github/workflows/bot-commands.yml | .github/workflows/bot-night.yml) ;;
    .github/workflows/squishy-run.yml | .github/workflows/squishy-commands.yml) ;;
    .github/workflows/triage.yml | .github/workflows/ci-duration.yml) ;;
    .github/workflows/pr-title.yml) ;;
    .cargo/mutants.toml | scripts/mutants.sh | scripts/mutants-runner.sh | scripts/pool-probe.sh) ;;
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

# Prints every line: full, and the rest by what was turned on.
emit() {
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
}

# True for a diff in which every file is one `cargo jackioh patches ship` or `patches cut` writes
# (#4094), and both history files are in it, which no other pull request edits.
ships() {
  saw_patches=false
  saw_shipped=false
  for file in $files; do
    case "$file" in
      crates/cards/patches/patches.json) saw_patches=true ;;
      crates/cards/patches/shipped.json) saw_shipped=true ;;
      crates/cards/patches/index.json | crates/cards/patches/v*.json | crates/cards/patches/card_metrics/* | \
        crates/cards/patches/records/* | crates/cards/patches/pending/* | \
        crates/server/.env.example | apps/web/.env.production) ;;
      *) return 1 ;;
    esac
  done
  [ "$saw_patches" = true ] && [ "$saw_shipped" = true ]
}

if ships; then
  echo "ci-scope: a patches ship or cut (#4094): only the cards, the server and the tools read what it writes" >&2
  on rust_cards_lib rust_cards rust_server rust_tools_wasm
  for file in $files; do
    case "$file" in
      apps/web/.env.production) on web ;;
    esac
  done
  emit
  exit 0
fi

for file in $files; do
  skipped "$file" && continue

  case "$file" in
    Cargo.toml | Cargo.lock | rust-toolchain.toml | .cargo/* | .github/* | scripts/ci-scope.sh)
      echo "ci-scope: $file runs every job" >&2
      on clippy $RUST web web_promote e2e
      ;;
  esac

  case "$file" in
    infra/* | .github/* | scripts/ci-scope.sh) on infra ;;
  esac

  case "$file" in
    crates/server/migrations/*) on db_sql db_store db_deploy ;;
    crates/server/tests/sql/*) on db_sql ;;
    crates/server/tests/db/bootstrap.sql | crates/server/src/db/* | crates/server/Cargo.toml | Cargo.lock | \
      crates/shared/src/* | crates/shared/Cargo.toml) on db_store db_deploy ;;
    crates/server/tests/db/* | crates/server/tests/store/* | crates/server/tests/support/test_app.rs | crates/server/tests/server_store.rs) on db_store ;;
    crates/cards/catalog.json | crates/engine/src/wire/catalog_types.rs) on db_store ;;
    crates/server/src/env.rs | crates/server/src/main.rs | crates/server/src/app.rs | crates/server/src/cli/seed_catalog.rs | \
      crates/server/src/api/catalog.rs | crates/server/Dockerfile | wrangler.jsonc | crates/server/.env.example | \
      crates/server/tests/deploy/*) on db_deploy ;;
  esac

  case "$file" in
    *.rs | Cargo.toml | */Cargo.toml | Cargo.lock | clippy.toml | */clippy.toml | rustfmt.toml | rust-toolchain.toml | .cargo/*) on clippy ;;
  esac

  # The crate map, which acceptance.rs holds to `cargo metadata`.
  case "$file" in
    crates/server/.env.example) on rust_server rust_tools_wasm ;;
    crates/engine/* | crates/cards/*) on $RUST ;;
    crates/ai/*) on rust_ai rust_ai_slow rust_server rust_tools_wasm ;;
    crates/server/migrations/*) on rust_server rust_tools_wasm ;;
    crates/server/*) on rust_server ;;
    crates/shared/* | crates/run/*) on rust_server rust_tools_wasm ;;
    crates/tools/* | crates/wasm/*) on rust_tools_wasm ;;
    crates/*)
      echo "ci-scope: $file is in no crate the map knows, running every Rust group" >&2
      on $RUST
      ;;
    apps/web/src/wire/*) on rust_engine rust_server rust_tools_wasm ;;
    docs/radiant-audit.md | spec/07-tokens.md | spec/08-catalog.md) on rust_cards ;;
    e2e/support/config.ts) on rust_server ;;
    e2e/cypress/e2e/01-* | e2e/cypress/e2e/13-*) on rust_tools_wasm ;;
    apps/web/.env.production | packages/* | apps/server/* | ladder/* | training/history/sweep-*.md) on rust_tools_wasm ;;
    apps/web/src/audio/voice-text.json) on rust_tools_wasm ;;
    apps/web/src/game/cardModes.json) on rust_tools_wasm ;;
    spec/15-dungeon-run.md | spec/README.md) on rust_tools_wasm ;;
    scripts/ci-shards.toml) on $RUST ;;
    analysis/fixtures/dev-40.jsonl | analysis/fixtures/live-40.jsonl | analysis/fixtures/games.report.json) on rust_tools_wasm ;;
    analysis/fixtures/counts.jsonl | analysis/fixtures/counts.report.json) on rust_tools_wasm ;;
  esac

  case "$file" in
    crates/*/Cargo.toml) on rust_tools_wasm ;;
  esac

  case "$file" in
    crates/server/Cargo.toml | crates/tools/Cargo.toml) on web ;;
    crates/server/src/env.rs | crates/server/src/app.rs | crates/server/Dockerfile | crates/server/tests/deploy/* | crates/engine/tests/fixtures/*) on web ;;
    crates/server/tests/fixtures/practice-claim.json) on web ;;
    crates/server/* | crates/tools/* | crates/*/README.md) ;;
    crates/engine/tests/* | crates/cards/tests/* | crates/ai/tests/* | crates/wasm/tests/* | crates/shared/tests/*) ;;
    crates/wasm/c/src/tests.rs) ;;
    training/file-records.sh) on web ;;
    infra/cloudflare.tf) on web ;;
    docs/* | training/* | reviews/* | analysis/* | infra/*) ;;
    SPEC.md | BUILD.md | REVIEW.md | README.md | ARCHITECTURE-CCG.md | JackiOh_*.md) ;;
    *) on web ;;
  esac

  case "$file" in
    scripts/promote-production.sh | apps/web/src/net/promote-production.test.ts) on web web_promote ;;
    scripts/promote-staging.sh | apps/web/src/net/promote-staging.test.ts) on web web_promote ;;
    package.json | apps/web/package.json | pnpm-lock.yaml | pnpm-workspace.yaml | .nvmrc) on web web_promote ;;
    vitest.config.ts | apps/web/vitest.config.ts | apps/web/src/test/*) on web web_promote ;;
  esac

  # e2e runs two commands of the `jackioh` binary (header, #4219): the four tools sources they run keep it
  # on; the rest of crates/tools/src/ and the fuzz's records do not.
  case "$file" in
    crates/engine/tests/golden/*) on e2e ;;
    # The dev dashboard's test ids: the daily dashboard spec's only dashboard read (spec 49, #4371).
    apps/admin/src/testids.ts) on e2e ;;
    crates/*/tests/* | crates/*/README.md | crates/wasm/c/src/* | apps/web/*.test.ts | apps/web/*.test.tsx) ;;
    docs/* | spec/* | training/* | reviews/* | analysis/* | infra/*) ;;
    SPEC.md | BUILD.md | REVIEW.md | README.md | ARCHITECTURE-CCG.md | JackiOh_*.md) ;;
    crates/server/Dockerfile | wrangler.jsonc | apps/edge/* | apps/admin/* | eslint.config.js | vitest.config.ts) ;;
    scripts/browse.sh | scripts/promote-production.sh | scripts/promote-staging.sh | scripts/server-image.sh | scripts/worktree.sh) ;;
    crates/tools/src/main.rs | crates/tools/src/replay.rs | crates/tools/src/catalog.rs | crates/tools/src/patches.rs) on e2e ;;
    crates/tools/src/* | crates/tools/fuzz-corpus/*) ;;
    *) on e2e ;;
  esac
done

# acceptance.rs's scans, which read every source under apps/, crates/ and e2e/.
if git diff --no-renames -U0 "$base...$head" -- apps crates e2e 2>/dev/null |
  grep -qE '^\+.*(readFileSync\(|include_str!\(|include_bytes!\(|read_to_string\(|CARGO_MANIFEST_DIR|#\[ignore|preview_sets\()'; then
  on rust_tools_wasm
fi

emit
