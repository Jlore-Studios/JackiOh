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
  `training/README.md`, `docs/polish/` (design notes) and `docs/v0.3.0/` (the Rust rewrite, #306; `SURFACE.md`
  is every boundary between the crates and the client).
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
4. `crates/engine`, `crates/cards` and `crates/ai` are pure: no clock, I/O, environment, threads, async, OS
   randomness, hash-ordered collection or mutable static (`crates/luau` too, with `mlua`, by its README).
   Every random draw goes through the `Rng` rebuilt from the state's seed and cursor; every player choice is a
   `PendingChoice` in state. Each crate's `Cargo.toml` takes only `serde`, `serde_json`, `indexmap`, the
   others of the three and (behind the engine's `ts` feature) `ts-rs`, and its `clippy.toml` bans what would
   break purity, tests included. The only statics are the catalog and the script registry, `OnceLock`s set
   once by `jackioh_cards::register_all()`. Data reaches a pure crate only at compile time (`include_str!`,
   `crates/cards/build.rs`); tooling that needs the disk or a clock goes in `crates/tools`.
5. Card scripts return `Vec<Effect>` built from the engine's effects module (`jackioh_engine::effects`, in
   reach through `jackioh_engine::prelude`). Never mutate state in a card file: a hook reads its
   `EffectContext` and the engine's read helpers and returns effects.
6. Every card has one script file, `crates/cards/src/scripts/<set>/<file>.rs` (its `pub const ID` and
   `pub fn script()`), with its tests in the same file (`#[cfg(test)] mod tests`) covering base and Radiant
   behaviour per the BUILD M4-T4 table (M9's for Classic and Classic+).
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

Code comments cite these rules by number ("CLAUDE.md rule 7"), so add new rules at the end and never renumber.

## Commands

Rust 1.97.0 for `crates/` (`rust-toolchain.toml`); Node ≥ 22.13 and pnpm 11 for `apps/web` and `e2e` only.
`cargo jackioh <command>` is `cargo run --release -p jackioh-tools --` (`crates/tools/README.md` lists every
command).

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings      # includes rule 4's purity lint
cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts   # every crate's tests, the golden replay included; rewrites apps/web/src/wire/ (commit any diff)
cargo jackioh catalog check        # catalog.json's shape and census: 370 cards and 80 tokens across Core, Classic, Classic+ and Meditative
cargo jackioh patches check        # every catalog change claimed by exactly one pending fragment
cargo jackioh spec check           # rule 3: ruling notes vs proving tests vs R-ids cited in code
cargo jackioh fuzz --seeds 200     # CI's fuzz gate: random-policy games with replay hashing; --handicap for the handicapped wave
cargo jackioh golden check         # the golden traces from the TypeScript engine, replayed (cargo test replays them too)
cargo jackioh gate                 # the AI's quality gates at smoke size; --full at full size (the daily run's)
cargo jackioh sweep                # the shadow-ban sweep (R186); copy its SHADOW_BAN rows into crates/ai/src/shadow_ban.rs by hand
cargo jackioh stats                # an AI development run for the card statistics (R378); --out writes it
cargo jackioh catalog-version      # the catalog version this checkout compiles in
cargo jackioh patches <version> <date> "<title>"   # a branch's catalog change as a pending fragment (R388, R646); `patches ship` promotes it after merge
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
E2E=1 cargo run --release -p jackioh-server    # :8787 + /ws/match; needed for networked specs 05, 06, 09, 10, 18, 19, 20, 26, 27, 35, 37, 38
cargo build --release -p jackioh-tools         # target/release/jackioh, whose `replay` specs 01 and 13 fold the browser's log with
cd e2e && pnpm exec cypress run --spec cypress/e2e/01-hotseat-full-game.cy.ts
pnpm --dir e2e test:component                  # component/pixel specs, no server needed
pnpm --dir apps/web build:e2e && pnpm --dir apps/web exec vite preview --port 5173 --strictPort   # a built client, as CI serves it
```

The server reseeds its fixture accounts and invite codes at boot (R144) and spec 10 uses them up, so restart
it before re-running spec 10. `vite dev` reloads the page whenever `apps/web/src` changes, restarting a
hotseat game at turn 0, so serve a built client while you edit; a plain `build` strips `window.__jackioh` and
every spec fails. `e2e/README.md` has the rest.

### CI

Branch protection on `main` requires five checks: `rust`, `web`, `e2e smoke` and `db` (`ci.yml`, each a
summary over jobs of under five minutes) and `bot selftest` (`bot-selftest.yml`). `rust` also fails when
`apps/web/src/wire/` differs from what the Rust generates. A pull request that changes only paths on
`scripts/ci-scope.sh`'s skip list (the bots, the agent instruction files, the issue forms, …) skips every job
but `bot selftest`; adding a path to that list takes proof that nothing in `crates/`, `apps/`, `e2e/` or
`spec/` reads it. Everything slow or statistical (fuzz at 10,000 seeds, `gate --full`, `golden check`,
coverage against `.github/coverage-floor.txt`, every e2e spec, the db suites) runs daily in `super.yml`. A job
over seven minutes opens an issue (`ci-duration.yml`): split it until it finishes under five. Each workflow's
header says what it does.

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
  can open a prompt. `replay.rs` folds a log and hashes a state. The engine does not depend on `crates/cards`;
  its tests use fixture scripts and the `testkit` feature (`scenario()` and its verbs).
- `crates/cards`: `catalog.json` and one script per card. `build.rs` generates the registry and fails on a
  missing, extra or doubled file. A script writes through the effects module and reads state only through the
  prelude's read helpers, never `state.players[…]`; a missing verb or board fact goes into the engine (board
  facts in `src/query.rs`) with its test. `patches/` is the card patch history.
- `crates/ai`: the practice opponent, pure and seeded like the engine. Hidden cards never change a decision
  (`redact`, `determinize`); its difficulty tiers are engine handicaps, not AI behaviour.
- `crates/luau`: the host that will run card scripts written in Luau (#442); nothing depends on it yet.
- `crates/wasm`: the engine, cards, AI and validator for the browser, JSON in and out.
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
`.github/`, `.harness/`, `.squishy/`, `bot/` or `.claude/` (the full list is `forbidden_paths` in
`.harness/config.json`): the harness puts such a change back, and the deliver job refuses to push one. When
the task is nothing but such a change, or needs a secret, a dashboard or an account, stop with
`"human_required": true` (your prompt shows the line) and say what a person must do.

## Parallel work

`scripts/worktree.sh <name> [base-ref]` creates a git worktree at `../jackioh-wt/<name>` on branch `wt/<name>`
with the web's dependencies linked; cargo builds each worktree into its own `target/`, so each tests only its
own changes (the first build there is a full one). It suits isolated work like card scripts. Don't use it for
shared surfaces (one issue's block of ruling numbers in `spec/rulings/`, `crates/engine/src/wire/events.rs`,
`script.rs`, `state.rs`, `effects/mod.rs`): parallel edits to those conflict on merge.
