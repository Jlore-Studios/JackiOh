# JackiOh — project instructions for Claude Code

JackiOh is a 1v1 card game: Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes with a hidden
trap backrow. This file binds every agent, not only Claude.

## Where things are

- `SPEC.md` points to `spec/`, the only source of rules, cards and engine design: one note per section
  (`spec/NN-<slug>.md`), one per ruling (`spec/rulings/R<nnnn>.md`) and their index (`spec/INDEX.md`), mapped
  by `spec/README.md`. Section references (§) point here. If a doc disagrees with the spec, the spec wins and
  the doc is the bug.
- `BUILD.md` is the work order (milestones, acceptance criteria, the per-card must-pass table); `REVIEW.md` is
  the audit, its reports in `reviews/`.
- Each crate's and app's `README.md` (`crates/*`, `apps/web`, `e2e`) is its contract: read it before you
  change that code. Also: `docs/ADDING_CARDS.md`, `docs/issues-and-patches.md` (titles, labels, patch and
  ruling numbers), `docs/architecture.md`, `docs/deploy-cloudflare.md` (what runs where), `bot/README.md`,
  `training/README.md`, `docs/v0.3.0/SURFACE.md` (every boundary between the crates and the client) and
  `docs/v0.4.0/` (the Luau card scripts, #442, and their return to Rust, #902).
- The designer's source notes (`JackiOh_*.md`, `ARCHITECTURE-CCG.md` and the design briefs
  `docs/classic-sets.md`, `docs/meditative-set.md` and `docs/meditative-plus-set.md`) are what REVIEW Part A
  checks the spec against; read them only when the task is about them.

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

`rust-toolchain.toml` pins Rust and its two WebAssembly targets for `crates/`, and `crates/luau` needs a C++
compiler (`mlua` builds Luau from source); Node ≥ 22.13 and pnpm 11 for `apps/web` and `e2e` only.
`cargo jackioh <command>` is `cargo run --release -p jackioh-tools --`.

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings      # includes rule 4's purity lint
cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts   # every crate's tests, the golden replay included; rewrites apps/web/src/wire/ (commit any diff)
cargo jackioh catalog check        # catalog.json's shape and its census of cards and tokens by set
cargo jackioh patches check        # every catalog change claimed by exactly one pending fragment
cargo jackioh spec check           # rule 3: ruling notes vs proving tests vs R-ids cited in code
cargo jackioh fuzz --seeds 200     # CI's fuzz gate: random-policy games with replay hashing; --handicap for the handicapped wave, --duos for co-op games (§13)
cargo jackioh golden check         # the golden traces from the TypeScript engine, replayed (cargo test replays them too)
cargo jackioh gate                 # the AI's quality gates at smoke size; --full at full size (the daily run's); --duos for the co-op AI team's (§13.7)
cargo jackioh patches "<title>" [--plays <ids>] [--rule]   # a branch's card change as a pending fragment, no version (R388, R646, R1366000); `patches ship` names the version by tier after merge
cargo jackioh classify --base origin/main [--parent-bin <path>] [--claims]   # what the change does: its kind, tier and the cards that play differently (R1365000); --claims holds the fragments to it, as CI's `rust (classify)` does (R1370000)
cargo test -p jackioh-luau --features ts --test export_api   # regenerates crates/cards/luau/jackioh.luau, the host's typed `J` (commit any diff)
sh scripts/build-wasm.sh           # the engine as WebAssembly into apps/web/src/wasm/pkg (the web's dev, build, build:e2e and test run it first)
pnpm --dir apps/web dev            # http://localhost:5173: hotseat at /dev/hotseat, the AI at /practice
pnpm --dir apps/web build          # the production bundle
pnpm --dir apps/web typecheck
pnpm --dir apps/web test           # the web's unit tests (vitest, jsdom)
pnpm exec eslint apps/web e2e
cd bot && python3 -m unittest discover -s tests -t .   # the bot selftest's suite, the skills' frontmatter included; inside a bot's run, unset HARNESS_HOME and HARNESS_DRY_RUN first
sh crates/server/tests/sql/run.sh          # schema, RLS and trigger invariants (Docker: a throwaway postgres:16)
sh crates/server/tests/db/run.sh           # the server's store against postgres:16 (Docker; KEEP_DB=1 keeps it)
cargo jackioh fuzz --from <seed> --seeds 1   # replay one failing fuzz seed (--handicap for the handicapped wave)
```

The rest are in `crates/tools/README.md` (every `cargo jackioh` command), `crates/luau/README.md`
(`luau-analyze`), `docs/ADDING_CARDS.md` (`gen:voice`, `gen:music`), `analysis/README.md` (the notebooks) and
`crates/server/README.md` (the deploy rehearsal, a real Supabase project).

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
E2E=1 cargo run --release -p jackioh-server    # :8787 + /ws/match, /ws/duos, /ws/watch; the networked specs need it (e2e/README.md lists them)
cargo build --release -p jackioh-tools         # target/release/jackioh: the specs that check the browser's log run its `replay`
cd e2e && pnpm exec cypress run --spec cypress/e2e/01-hotseat-full-game.cy.ts
pnpm --dir e2e test:component                  # component/pixel specs, no server needed
pnpm --dir apps/web build:e2e && pnpm --dir apps/web exec vite preview --port 5173 --strictPort   # a built client, as CI serves it
```

`vite dev` reloads the page whenever `apps/web/src` changes, restarting a hotseat game at turn 0, so serve a
built client while you edit; a plain `build` strips `window.__jackioh` and every spec fails. `e2e/README.md`
has the rest, the server's fixture reseed among it.

### CI

Branch protection on `main` requires five checks, all jobs of `ci.yml`: `rust`, `web`, `e2e smoke`, `db` and
`bot selftest`. On a pull request `rust` includes `rust (classify)`, which labels it with
its kind and tier and fails when the pending fragments do not match the cards that play differently
(R1370000): run `cargo jackioh classify --base origin/main --claims` before pushing. `rust` also fails when
`apps/web/src/wire/` or `crates/cards/luau/jackioh.luau` differs from what the Rust generates, and runs
`luau-analyze --mode=strict` over the Luau there (#442 L10). A pull request that changes only paths on
`scripts/ci-scope.sh`'s skip list (the bots, the agent instruction files, the issue forms, …) skips every job
but `bot selftest`; adding a path to that list takes proof that nothing in `crates/`, `apps/`, `e2e/` or
`spec/` reads it. `bot selftest` itself runs only when a pull request touches what the bot's tests read
(`ci.yml`'s `changes` job, `bot`; #1455). Everything slow or statistical (fuzz at 10,000 seeds, `gate --full`, `golden check`,
coverage against `.github/coverage-floor.txt`, every e2e spec, the db suites, the notebooks' tests) runs daily
in `super.yml`. Each workflow's header says what it does, `ci.yml`'s the five-minute budget of every job.

## Architecture

- `crates/engine`: the rules, the wire types every layer shares and the deck validator, entered through
  `reduce(state, action)`, `legal_actions` and `view_for(state, player)`. Read `work.rs`'s header before
  touching anything that can open a prompt. It depends on neither `crates/cards` nor `crates/luau`; its tests
  use fixture scripts and the `testkit` feature (`scenario()` and its verbs).
- `crates/cards`: `catalog.json`, one Rust script per card and `patches/`, the card patch history. A script
  reads state only through the prelude's read helpers, never `state.players[…]`; a missing verb or board fact
  goes into the engine (board facts in `src/query.rs`) with its test.
- `crates/ai`: the practice opponent, pure and seeded like the engine; hidden cards never change a decision.
- `crates/server`: one binary, the HTTP API and one in-memory match actor per match pushing `view_for` over a
  WebSocket. `cargo test` runs the store's in-memory fake, so RLS and triggers are checked only by the Docker
  suites, and a store change goes in `tests/store/contract.rs`.
- `apps/web`: the Vite/React client. Components take a `PlayerView` and nothing else; legality comes from
  `legalActions`, never a client-side check. `Game.tsx` is the one board; each of its layers (animations,
  effects, audio, cards, drag, settings) has one owning module.

## The night bot

`@jgoetzmann-bot` (the night bot, `.harness/`) and `@squishy-squooby` (Squishy, `.squishy/`) build issues on
the owner's subscriptions; `bot/README.md` says which issues each takes. `human` keeps every bot off an issue,
and `bot:build` queues one for the night bot, so never add it while retitling or relabelling.

A bot's builder or reviewer must not change `.github/`, `.harness/`, `.squishy/`, `bot/` or `.claude/`, except
that Squishy may change `.claude/skills/` (the full lists are `forbidden_paths` in each bot's `config.json`):
the harness puts such a change back, and the deliver job refuses to push one. When the task is nothing but
such a change, or needs a secret, a dashboard or an account, stop with `"human_required": true` (your prompt
shows the line) and say what a person must do.

## Parallel work

`scripts/worktree.sh <name> [base-ref]` creates a git worktree at `../jackioh-wt/<name>` on branch `wt/<name>`
with the web's dependencies linked; cargo builds each worktree into its own `target/`, so each tests only its
own changes (the first build there is a full one). It suits isolated work like card scripts. Don't use it for
shared surfaces (one issue's block of ruling numbers in `spec/rulings/`, `crates/engine/src/wire/events.rs`,
`script.rs`, `state.rs`, `effects/mod.rs`): parallel edits to those conflict on merge.
