# `jackioh-engine`

The rules (`spec/`), the wire types every layer shares and the deck validator, in one pure, seeded
crate. Everything else (the cards, the AI, the WASM bindings, the server, the tools) calls it; it
calls nothing of theirs. This file is the contract other crates rely on. When it and the spec
disagree, the spec wins and this file is the bug.

## Entry points

`src/lib.rs` re-exports every module whole, so `jackioh_engine::<name>` reaches any engine item.
What the other crates call:

| Function | Answers |
| --- | --- |
| `create_game(&CreateGameOptions) -> GameState` | a game at setup: seed, decks, and the optional catalog, handicaps (R180), last boards (R417), Glitch boards (R678) and dealt seats (R433) |
| `begin_game(&GameState) -> ReduceResult` | setup run (§2.1): the opening draws and the mulligan prompts |
| `reduce(&GameState, &Action) -> ReduceResult` | the next state and its events, or `error: Some(reason)` with the input state unchanged and no events |
| `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>` | every action that player may send now |
| `view_for(&GameState, PlayerId) -> PlayerView` | the only thing a client may see (§10.8); `view_for_with_clock` adds the server's clock (R79) |
| `seat_to_act(&GameState)` | the seat the game waits on first, for code that plays both seats (R265) |
| `hash_state`, `fold`, `canonical`, `fnv1a32_utf16` (`replay.rs`) | the state hash, and an action log folded back into a state |
| `summarize_game` (`game_summary.rs`) | a finished game's record for the card statistics (R376) |
| `last_board_for`, `seat_played_by`, `seats_swapped`, `mulligan_owed` | the facts the server and practice read at the edges of a game (R417, R677, R265) |
| `registered_catalog()` | the catalog `jackioh_cards::register_all()` installed |
| `validator::*` | deck and trio checks (§9.4: D1–D4, T1–T3, L1–L6, R250–R253, R340), shared by the server and, through WASM, the client |

`reduce` never panics on an illegal action. It takes no `rng`: the match rng is rebuilt from
`(state.seed, state.rng_cursor)` and the cursor written back, so `(seed, decks, log)` folds to the
same state anywhere.

## Where each rule lives

`reduce.rs`'s header maps every action to the module that owns it (`play_steps`, `combat`,
`subsystems::activate`, `prompts`, `setup`, `turn`). `legal_actions` and the reducer's refusals call
the same function for each rule, so a greyed-out button and a refusal cannot disagree (§9.3).

After every action the resolution loop runs (`triggers::settle`, §10.3): it dispatches the events,
drains `state.work`, runs the state check (`state_check.rs`) and pops the trigger queue until nothing
is left or a prompt stops it. A prompt ends the action (`state.pending`); the one exception is the
mulligan, both seats' prompts open at once in `state.mulligan` (R265–R268). Any sequence that can
pause parks what it still owes on `state.work` as plain-data `Resume` records, never closures, in the
order R113 sets (`state.work_cursor`). Read `work.rs`'s header before touching anything that can
open a prompt.

## Purity (CLAUDE.md rule 4)

No clock, no I/O, no environment, no threads, no OS randomness, no hash-ordered collection, no
mutable static. Two things hold it:

- `Cargo.toml` depends on `serde`, `serde_json`, `indexmap` and, behind the off-by-default `ts`
  feature, `ts-rs`, and nothing else. A crate it cannot name it cannot call.
- `clippy.toml` (the same file sits in `crates/cards` and `crates/ai`) bans `HashMap`, `HashSet`,
  `Instant`, `SystemTime`, `File`, `Thread`, `Mutex`, `RwLock` and `RefCell`, and the methods
  `Instant::now`, `SystemTime::now`, `env::var`, `env::vars`, `fs::read`, `fs::read_to_string`,
  `fs::write`, `thread::spawn` and `process::exit`. CI runs `cargo clippy --workspace --all-targets
  -- -D warnings`, so a use anywhere in the crate, tests included, fails the build.

Every random draw goes through `Rng` (`rng.rs`, seeded, its cursor in the state), in the same order
on every run. Every player choice is a `PendingChoice` in the state. The only statics are the
catalog and the script registry, each a `OnceLock` set once by `jackioh_cards::register_all()`;
under the `testkit` feature a thread-local override lets a test install fixture cards, the one
allowed `RefCell`.

## Effects, reads and scripts

- `effects` is the whole vocabulary a card writes with (§10.9): one module per verb family, every
  verb a function returning an `Effect`. A hook is `Arc<dyn Fn(&mut EffectContext) -> Vec<Effect>>`,
  built with `hook(|ctx| …)`, and never writes the state itself (CLAUDE.md rule 5).
- `query.rs` (with `zones`, `layers`, `mana`, `numbers`, `params`) is the read half: the board facts
  a card reads instead of `GameState` fields. `crates/cards/README.md` lists them.
- `prelude` is what a card file names: `use jackioh_engine::prelude::*;`.
- `subsystems/` holds the card-specific systems (fuse, rotation, scorer, the random policy
  `ai_policy`, heroic power, combo index, Call to Chaos, quests, …), each tested through fixture
  scripts in `tests/rules/fixtures/`.
- `scripts::script_of(state, def_id)` finds a card's script; a fused card's is composed from its
  ingredients on lookup (`subsystems::fuse::compose_fused_scripts`).

The engine never depends on `jackioh-cards` (only its tests do). An engine test that needs a card's
behaviour uses a test-only script in `tests/rules/fixtures/`, and the real card's test covers the
same case again.

## Constants (CLAUDE.md rule 9)

Every rules number is a named constant in `src/config.rs` (BUILD §2), the "decide" rulings (R1, R4,
R5, R14, R26, R39) and R2's turn cap included, with the training lanes' gate (`TRAINING_GAMES`,
`TRAINING_IMPROVE`, `TRAINING_UNBAN`). The ones the web reads are written to
`apps/web/src/wire/engineConfig.ts` by `tests/export_config.rs`.

## Generated TypeScript

With `--features ts`, every wire type derives `ts_rs::TS` and its `export_bindings_*` test writes it
to `apps/web/src/wire/generated/`. Those files and `engineConfig.ts` are committed, and CI's
`rust (test)` job fails when a run changes them (V20). Regenerate both with the CI command:

```
cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts
```

## Features

| Feature | Default | What |
| --- | --- | --- |
| `testkit` | off | `jackioh_engine::testkit`: `scenario()` and the harness verbs, the I1–I4 invariant monitor `jackioh fuzz` runs, the glow helpers, the registry override and the test seams (`seams.rs`). Every test crate enables it as a dev-dependency |
| `ts` | off | the ts-rs derives above |

## Tests

| Binary | Path | What |
| --- | --- | --- |
| `rules` | `tests/rules.rs`, `tests/rules/<x>.rs` | one module per rule area; needs `--features testkit` |
| `golden` | `tests/golden.rs`, `tests/golden/` | the 240 golden traces recorded from the TypeScript engine before it was deleted, replayed hash for hash, and the hotseat fixture's fold to `a798906b` |
| `export_config` | `tests/export_config.rs` | writes `engineConfig.ts` |

A test that proves a ruling is named after it: `fn r58_…` (CLAUDE.md rule 3).

```
cargo test -p jackioh-engine --features testkit                   # every engine test
cargo test -p jackioh-engine --features testkit --test rules      # the rules binary only
cargo test -p jackioh-engine --features testkit --test rules r58  # the tests whose name holds r58
cargo clippy -p jackioh-engine --all-targets -- -D warnings       # the purity lint
```
