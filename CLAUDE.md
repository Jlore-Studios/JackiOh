# JackiOh — project instructions for Claude Code

JackiOh is a 1v1 card game: Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes with a hidden
trap backrow. This file binds every agent, not only Claude.

## Where things are

- `SPEC.md` points to `spec/`, the only source of rules, cards and engine design: one note per section
  (`spec/NN-<slug>.md`), one per ruling (`spec/rulings/R<nnnn>.md`) and their index (`spec/INDEX.md`), mapped
  by `spec/README.md`. Section references (§) point here. If a doc disagrees with the spec, the spec wins and
  the doc is the bug.
- `BUILD.md` is the work order (milestones, acceptance criteria, the per-card must-pass table); `REVIEW.md` is
  the audit (Part A: the spec against the source notes; Part B: the code against the spec and BUILD). Audit
  reports go in `reviews/`.
- Each crate's and app's `README.md` (`crates/*`, `apps/web`, `e2e`) is its contract: read it before you
  change that code. Also: `docs/ADDING_CARDS.md`, `docs/issues-and-patches.md` (titles, labels, patch and
  ruling numbers), `docs/architecture.md` and `docs/deploy-cloudflare.md` (deployment: the web on Cloudflare
  with Vercel as staging, the server on Render, Postgres and Auth on Supabase), `bot/README.md`,
  `training/README.md`, `docs/polish/` (design notes), `docs/v0.3.0/` (the Rust rewrite, #306; `SURFACE.md`
  is every boundary between the crates and the client) and `docs/v0.4.0/` (the Luau card scripts, #442, and
  their return to Rust in v0.5.0, #902: `README.md` the run's record, `PORTING.md` and `api-gaps.md` kept as
  the record and for the custom-card host).
- The designer's source notes (`JackiOh_*.md`, `ARCHITECTURE-CCG.md`, with the design briefs
  `docs/classic-sets.md` and `docs/meditative-set.md`) are what REVIEW Part A checks the spec against; read
  them only when the task is about them.

## Rules of engagement

1. Before a task, read `spec/README.md` and the spec sections and rulings the task touches; re-read them
   before each change. Never implement a rule from memory of Hearthstone or Yu-Gi-Oh when the spec states it.
2. Work BUILD.md in order. A task is done when its acceptance items are green tests. Do not open the next
   milestone until the current gate passes.
3. Rulings live in SPEC §11, one note per ruling, `spec/rulings/R<nnnn>.md` (zero-padded to four digits at
   least: R58 is `R0058.md`), its front matter holding `id`, `title`, `cards` and `proven_in`. A decision the
   spec does not make follows Hearthstone semantics and gets a note in the same PR, numbered with the lowest
   free in its issue's block (issue N owns R(N×1000) to R(N×1000+999); `docs/issues-and-patches.md`, Ruling
   numbers). Name the proving test after it: in Rust a `fn` or `mod` whose name has the `_`-delimited token
   `r58` (`fn r58_…`, `fn r90_r81_…`), in TypeScript an `it(`, `test(` or `describe(` whose title has `R58` as
   a word. List its file in `proven_in` and run `cargo jackioh spec index`; `cargo jackioh spec check` fails
   on any mismatch between the notes, their tests, the index and the R-ids cited in `crates/`, `apps/web/src/`
   or `e2e/`, comments included.
4. `crates/engine`, `crates/cards`, `crates/ai` and `crates/luau` are pure: no clock, I/O, environment,
   threads, async, OS randomness, hash-ordered collection or mutable static. Every random draw goes through
   the `Rng` rebuilt from the state's seed and cursor; every player choice is a `PendingChoice` in state.
   Each crate's `Cargo.toml` takes only `serde`, `serde_json`, `indexmap`, the others of the four (no game
   crate takes `jackioh-luau`: it is the custom-card host, #902 V4), `mlua` (`jackioh-luau`'s) and, behind a
   `ts` feature (the engine's, and `jackioh-luau`'s own, off by default), `ts-rs`; its `clippy.toml` bans
   what would break purity, tests included. The only statics are the catalog and the script registry,
   `OnceLock`s set once by `jackioh_cards::register_all()` (the older shipped catalogs a replay folds under
   among them, each built once on first use), `jackioh-luau`'s thread-local VM, which holds compiled
   chunks and scripts and no game state (#442 L8), and two engine thread-locals, each set for one call and
   put back after it: the shipped catalog a replay is folding under (R767000), and the custom-script host
   `custom::with_custom_scripts` lends (#915, R915002). A Luau script is kept pure by its host: the sandbox
   (L5), the integer boundary every value crosses (L6) and the lint (L6, L7), all in
   `crates/luau/README.md`. Data reaches a pure crate only at compile time (`include_str!`,
   `crates/cards/build.rs`); tooling that needs the disk or a clock goes in `crates/tools`.
5. Card scripts return `Vec<Effect>` built from the engine's effects module (`jackioh_engine::effects`, in
   reach through `jackioh_engine::prelude`). Never mutate state in a card file: a hook reads its
   `EffectContext` and the engine's read helpers and returns effects. (A custom card's Luau script does the
   same through the card kit, `docs/custom-cards.md`, which `crates/luau` turns into the same effects.)
6. Every card has one script file, `crates/cards/src/scripts/<set>/<file>.rs` (its `pub const ID`, its
   header comment and `pub fn script()`), with its tests in the same file (`#[cfg(test)] mod tests`)
   covering base and Radiant behaviour per the BUILD M4-T4 table (M9's for Classic and Classic+).
7. The client sends intent and renders `viewFor` (the engine's `view_for`); it never enforces rules and never
   sees hidden information.
8. Before claiming a milestone is done, run the paste-in prompt at the end of REVIEW.md as a separate session
   and attach the report.
9. Every number is a named constant: rules numbers in `crates/engine/src/config.rs` (BUILD §2, the "decide"
   rulings included), server numbers in `crates/server/src/config.rs`, the AI's in `crates/ai/src/`
   (`config.rs` and its modules' constants). Those the client reads are generated into
   `apps/web/src/wire/engineConfig.ts` and `serverConfig.ts` by each crate's `tests/export_config.rs`, and CI
   fails when a run changes them. Nothing else states a number.
10. Before any card work (adding, changing or removing a card), read `docs/ADDING_CARDS.md`.
11. An agent that opens an issue gives it one `difficulty:*` label by `docs/issues-and-patches.md` (How hard
    it is): **easy** only under `bot/harness/easy.py`'s rule (small, one crate or app, no spec, BUILD,
    shared-surface, engine, AI, catalog, card or migration work, a unit test checks it, no decision left,
    nothing blocking it); **hard** for engine rules, the AI, design across crates, a migration, or about 40
    files or more; **medium** for the rest. Torn between two, take the harder. The bot never changes your
    label, so a wrong easy reaches a weak model: when you cannot judge it, leave it off and the planner rates
    it.
12. No agent merges a pull request, or reports one green or done, while a required check on its head is red
    or has not run, and none calls a failure pre-existing or environmental without a link to the same check
    failing on `main`. A report names every red check by its name. #825 was merged with its golden replay
    red and closed as "all gates green" (`reviews/2026-10-14-issue-814-first-pass.md`).
13. Never delete or disable anything called from outside this codebase without asking a person first: a
    webhook, a scheduled job (a workflow's `schedule:`), an auth callback, an HTTP route that a deploy probes
    or a person or another service calls (`/api/catalog` is Render's health check), or a `cargo jackioh`,
    server or harness subcommand that a workflow, a deploy or a person runs. That nothing in the tree calls
    it is no proof it is dead (#900).

Code comments cite these rules by number ("CLAUDE.md rule 7"), so add new rules at the end and never renumber.

## Commands

Rust 1.97.0 for `crates/` (`rust-toolchain.toml`, with the `wasm32-unknown-unknown` target) and a C++
compiler, since `mlua` builds Luau from source for `crates/luau`; Node ≥ 22.13 and pnpm 11 for `apps/web` and
`e2e` only.
`cargo jackioh <command>` is `cargo run --release -p jackioh-tools --` (`crates/tools/README.md` lists every
command).

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings      # includes rule 4's purity lint
cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts   # every crate's tests, the golden replay included; rewrites apps/web/src/wire/ (commit any diff)
cargo jackioh catalog check        # catalog.json's shape and census: 370 cards and 80 tokens across Core, Classic, Classic+ and Meditative
cargo jackioh patches check        # every catalog change claimed by exactly one pending fragment
cargo jackioh spec check           # rule 3: ruling notes vs proving tests vs R-ids cited in code
cargo jackioh fuzz --seeds 200     # CI's fuzz gate: random-policy games with replay hashing; --handicap for the handicapped wave, --duos for co-op games (§13)
cargo jackioh golden check         # the golden traces from the TypeScript engine, replayed (cargo test replays them too)
cargo jackioh gate                 # the AI's quality gates at smoke size; --full at full size (the daily run's); --duos for the co-op AI team's (§13.7)
cargo jackioh sweep                # the shadow-ban sweep (R186); copy its SHADOW_BAN rows into crates/ai/src/shadow_ban.rs by hand
cargo jackioh stats                # an AI development run for the card statistics (R378); --out writes it
cargo jackioh catalog-version      # the catalog version this checkout compiles in
cargo jackioh patches <version> <date> "<title>"   # a branch's catalog change as a pending fragment (R388, R646); `patches ship` promotes it after merge
cargo test -p jackioh-luau --features ts --test export_api   # regenerates crates/cards/luau/jackioh.luau, the host's typed `J` (commit any diff)
luau-analyze --mode=strict crates/cards/luau/jackioh.luau  # Luau 0.740's strict checker over the typed module, where it is installed
cargo jackioh luau surface         # the engine API each card script calls (hooks, verbs, readers, pools); --json, --card <id>
cargo jackioh luau diff --parent-bin <path> --cards <ids>   # 200 games with the cards forced, replayed by a parent build: any divergence fails (docs/v0.4.0/PORTING.md)
cargo jackioh luau bench --parent-bin <path>   # the performance gate against a parent build (#442 L13); --record <path> writes the numbers
sh scripts/build-wasm.sh           # the engine as WebAssembly into apps/web/src/wasm/pkg (the web's dev, build, build:e2e and test run it first)
pnpm install
pnpm --dir apps/web dev            # http://localhost:5173: hotseat at /dev/hotseat, the AI at /practice
pnpm --dir apps/web build          # the production bundle
pnpm --dir apps/web typecheck
pnpm --dir apps/web test           # the web's unit tests (vitest, jsdom)
pnpm exec eslint apps/web e2e
sh crates/server/tests/sql/run.sh          # schema, RLS and trigger invariants (Docker: a throwaway postgres:16)
sh crates/server/tests/db/run.sh           # the server's store against postgres:16 (Docker; KEEP_DB=1 keeps it)
sh crates/server/tests/deploy/rehearse.sh  # Render's deploy rehearsed: the image, a non-superuser migration, a production boot (Docker)
pnpm --filter @jackioh/web gen:voice   # re-render changed voice lines (macOS `say`); `--check` runs anywhere; from Linux, `[gen:voice]` in a commit message renders them on GitHub's runners (R1448)
pnpm --filter @jackioh/web gen:music   # re-render changed music scores (FluidSynth, ffmpeg; R631); `--check` runs anywhere
cargo jackioh fuzz --from <seed> --seeds 1   # replay one failing fuzz seed (--handicap for the handicapped wave)
uv sync --frozen --project analysis              # the balance notebooks' pinned Python 3.13 and dependencies (analysis/README.md)
uv run --project analysis pytest analysis/tests  # the notebooks' loader and every notebook, on the fixtures
```

`crates/server/README.md` covers the server against a real Supabase project (its `.env`, `migrate`,
`seed-catalog`, `mint-code`, `release`) and its other subcommands.

### Running a subset

```
cargo test -p jackioh-engine --features testkit --test rules       # the engine's rule tests
cargo test -p jackioh-cards c002_bigot                              # one card's tests
cargo test -p jackioh-engine --features testkit --test rules r58   # by ruling: every test whose name holds r58
cargo test -p jackioh-server --test server actor::                  # one area of the server suite
pnpm --dir apps/web exec vitest run src/game/animations.test.ts     # one web test file
```

### E2E

```
# Cypress; the root `pnpm install` covers e2e/
E2E=1 pnpm --dir apps/web dev                  # http://localhost:5173, serves /dev/hotseat
E2E=1 cargo run --release -p jackioh-server    # :8787 + /ws/match, /ws/duos, /ws/watch; needed for networked specs 05, 06, 09, 10, 18, 19, 20, 26, 27, 35, 37, 38, 39, 40, 41, 42, 44
cargo build --release -p jackioh-tools         # target/release/jackioh, whose `replay` specs 01, 13 and 40 fold the browser's log with
cd e2e && pnpm exec cypress run --spec cypress/e2e/01-hotseat-full-game.cy.ts
pnpm --dir e2e test:component                  # component/pixel specs, no server needed
pnpm --dir apps/web build:e2e && pnpm --dir apps/web exec vite preview --port 5173 --strictPort   # a built client, as CI serves it
```

The server reseeds its fixture accounts and invite codes at boot and on `POST /api/e2e/reseed` (R144), which
spec 10 calls first, so spec 10 re-runs with no restart. `vite dev` reloads the page whenever `apps/web/src`
changes, restarting a hotseat game at turn 0, so serve a built client while you edit; a plain `build` strips
`window.__jackioh` and every spec fails. `e2e/README.md` has the rest.

### CI

Branch protection on `main` requires five checks: `rust`, `web`, `e2e smoke` and `db` (`ci.yml`, each a
summary over jobs of under five minutes) and `bot selftest` (`bot-selftest.yml`). `rust` also fails when
`apps/web/src/wire/` or `crates/cards/luau/jackioh.luau` differs from what the Rust generates, and runs
`luau-analyze --mode=strict` over the Luau there (#442 L10). A pull request that changes only paths on
`scripts/ci-scope.sh`'s skip list (the bots, the agent instruction files, the issue forms, …) skips every job
but `bot selftest`; adding a path to that list takes proof that nothing in `crates/`, `apps/`, `e2e/` or
`spec/` reads it. Everything slow or statistical (fuzz at 10,000 seeds, `gate --full`, `golden check`,
coverage against `.github/coverage-floor.txt`, every e2e spec, the db suites, the notebooks' tests) runs daily in `super.yml`. A job
over seven minutes opens an issue (`ci-duration.yml`): split it until it finishes under five. `hygiene.yml`
runs weekly: two issues for Squishy, one pull request each that a person reviews and merges
(`person merges`), for dead files and archives and for the agent instructions; the first lists stale
branches as checkboxes, deleted once a person ticks them and comments `/delete-branches` (#839, #888). Each
workflow's header says what it does.

## Architecture

A Cargo workspace (`crates/`) holds everything but the browser UI; `apps/web` and `e2e/` are TypeScript. The
README of each is its contract.

- `crates/engine`: the rules, the wire types every layer shares (`src/wire/`, exported to
  `apps/web/src/wire/generated/`) and the deck validator. The entry points are `reduce(state, action)`,
  `legal_actions` and `view_for(state, player)`. `reduce` clones state, applies the action and returns new
  state plus events; an illegal action is an error that leaves state unchanged. Each rule has one owning
  module (`reduce.rs`'s header maps them), and `legal_actions` and the reducer's refusals call the same
  function. `triggers::settle` runs after every action. A prompt ends the action (`state.pending`; the
  mulligan, both seats at once, is the one exception, so code that plays both seats asks `seat_to_act`). Any
  sequence that can pause parks its remainder on `state.work` as plain-data `Resume` records, never closures,
  so a paused state survives JSON and replays exactly: read `work.rs`'s header before touching anything that
  can open a prompt. `replay.rs` folds a log and hashes a state. The engine depends on neither `crates/cards`
  nor `crates/luau`; its tests use fixture scripts and the `testkit` feature (`scenario()` and its verbs).
- `crates/cards`: `catalog.json` and one Rust script per card. `build.rs` generates the registry and fails on
  a missing, extra or doubled file. A script writes through the effects module and reads state only through
  the prelude's read helpers, never `state.players[…]`; a missing verb or board fact goes into the engine
  (board facts in `src/query.rs`) with its test. `patches/` is the card patch history.
- `crates/ai`: the practice opponent, pure and seeded like the engine. Hidden cards never change a decision
  (`redact`, `determinize`); its difficulty tiers are engine handicaps, not AI behaviour.
- `crates/luau`: the Luau host (#442), kept for custom cards (#902, #915) through `mlua`: the sandbox, the
  integer boundary, the lint, one VM per thread, the `J` bindings (`src/api/`, `reads.rs`, `hooks.rs`),
  `load_card`, which turns a compiled script into the engine's `CardScripts`, and the card kit custom cards
  are written in. No game crate links it but the browser's custom-card module; Luau is compiled from source
  (`luau0-src`), so building it needs a C++ compiler.
- `crates/wasm`: the engine, cards, AI and validator for the browser, JSON in and out, built for
  `wasm32-unknown-unknown` with wasm-bindgen by `scripts/build-wasm.sh`.
- `crates/server`: one binary, the HTTP API (`src/api`) and one in-memory match actor per match (`src/actor`)
  pushing `view_for` over a WebSocket. The store is the `Db` and `Tx` enums over Postgres or an in-memory
  fake; `cargo test` runs the fake, so RLS and triggers are checked only by the Docker suites, and a store
  change goes in `tests/store/contract.rs`. It reaches the engine only through `src/actor/engine.rs`.
- `crates/tools`: the `jackioh` binary behind `cargo jackioh`.
- `apps/web`: the Vite/React client. Components take a `PlayerView` and nothing else. The WASM module is
  called only from `src/wasm/index.ts`, and a game's state is reached only through `src/game/engine.ts`
  (hotseat) and `src/practice/core.ts` (the practice worker, the only caller of the AI). Legality comes from
  `legalActions`, never a client-side check. `Game.tsx` is the one board; each of its layers (animations,
  effects, audio, cards, drag, settings) has one owning module.

## The night bot

`@jgoetzmann-bot` (the night bot, `.harness/`) and `@squishy-squooby` (Squishy, `.squishy/`) build issues on
the owner's subscriptions; `bot/README.md` covers both. The night bot takes an issue only when it is labelled
`bot:build`, assigned to it, or named in a `/harness build` or `@jgoetzmann-bot …` comment; `human` keeps
every bot off an issue, `priority:*` sets the order and `difficulty:*` the weakest model that may build it.
Neither bot takes an issue that is the other's.

When you are a bot's builder or reviewer, this file binds you like anyone else. You must not change
`.github/`, `.harness/`, `.squishy/`, `bot/` or `.claude/`, except that Squishy may change `.claude/skills/`
(the full lists are `forbidden_paths` in each bot's `config.json`): the harness puts such a change back, and
the deliver job refuses to push one. When the task is nothing but such a change, or needs a secret, a
dashboard or an account, stop with `"human_required": true` (your prompt shows the line) and say what a person
must do.

## Parallel work

`scripts/worktree.sh <name> [base-ref]` creates a git worktree at `../jackioh-wt/<name>` on branch `wt/<name>`
with the web's dependencies linked; cargo builds each worktree into its own `target/`, so each tests only its
own changes (the first build there is a full one). It suits isolated work like card scripts. Don't use it for
shared surfaces (one issue's block of ruling numbers in `spec/rulings/`, `crates/engine/src/wire/events.rs`,
`script.rs`, `state.rs`, `effects/mod.rs`): parallel edits to those conflict on merge.
