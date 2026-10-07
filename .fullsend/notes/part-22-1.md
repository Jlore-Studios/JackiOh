# Slice: part 22, chunk 1 of 3 (the jackioh CLI: replay, gate, sweep, stats, trace)
BUILDS-RUN: 0

## FILES
- `crates/tools/src/gate.rs` (new): `packages/ai/test/gate-random.test.ts`, `gate-greedy.test.ts`,
  `gate-hard-easy.test.ts`, `gate-perf.test.ts`, their shard helper `test/_shard.ts` (`gateShard`,
  `gamesToPlay`, `ShardFile`, `writeShard`) and `packages/ai/scripts/gate-merge.ts`, whole. `Args` (`--full`,
  `--shard k/K`, `--out dir`, subcommand `merge [dir]`) + `run`. Every TS `it` is a check function run by the CLI
  and by a `#[test]` (smoke: 4 games per matchup, held to `gateNeeded(matchup, 4)` and B31), plus unit tests of the
  shard switch, the shard split and `merge`.
- `crates/tools/src/stats.rs` (new): `packages/ai/scripts/stats.ts` whole (`newestPatch`, `count`, the run), with
  `dev-run.test.ts`'s `parseDevRunArgs` case ported against the clap `Args`.
- `crates/tools/src/sweep.rs` (new): `packages/ai/scripts/sweep.ts` whole (`nameOf`, `meanDelta`, `row`,
  `TABLE_HEAD`, `timed`, `pass1`, `pass2`, `report`, `readLines`, `isPass2`, `main` → `run`), plus `to_fixed` (JS
  `toFixed`) and `utc_date_today` (TS's `Intl.DateTimeFormat`).
- `crates/tools/src/trace.rs` (new): `packages/ai/scripts/trace.ts` whole (`nameOf`, `board`, `describe`, `main` →
  `run`).
- `crates/tools/src/replay.rs` (new): SURFACE §12's `replay` (stdin `{seed, decks, log, handicaps?, dealt?,
  lastBoards?, glitchBoards?}` → `{"hash", "errors"}`), replacing `e2e/support/tasks/replay-runner.ts`'s TS fold;
  tests fold the hotseat fixture to `a798906b`.

Not written by this chunk: `catalog.rs`, `patches.rs` (chunk 2's whole files, `Args` and `run` included) and
`fuzz.rs` (chunk 3's). Row 1 of the table ("each `pub struct Args` + `pub fn run`") is met for those by their
owners; `replay.rs` is named only in row 1, so it is this chunk's.

## SURFACE
- §12: commands and flags as the table writes them, and every TS knob kept as a flag: `gate` also reads
  `JACKIOH_AI_GATE=full`, `JACKIOH_AI_GATE_SHARD`, `JACKIOH_AI_GATE_OUT` when the flag is absent (CI's env stays
  valid); `stats` adds `--from` and `--series` (TS had them); `trace` adds an optional third positional
  `gate|full` and `--series` (or `SERIES`); `sweep` takes `--json`, `--pass2 <files,>`, `--report <files…>` and
  ids.
- §9: `decide`, `play_match`, `build_ai_deck`, `greedy_action`, `SHADOW_BAN`/`SHADOW_WATCH` as fixed there.
- §5.2/§6.1: `fold`, `hash_state` from `jackioh_engine::replay`.
- Output to stdout; progress (stats, sweep) to stderr as TS wrote it; a failed check returns `Err` with every
  problem joined (exit 1).

## DEPENDS-ON (names this chunk calls that other parts write)
`jackioh_ai` (part 17):
- `Matchup`: `Copy`, `Serialize`/`Deserialize` as its TS literals ("ai-vs-random" …). This chunk never names a
  variant: `gate::parse_matchup`/`matchup_name` go through serde.
- `AI_GATE: AiGate { seed_series: &str, smoke_seeds, full_seeds, brief_rate, measured_rate, false_alarm: f64,
  max_decision_ms, perf_smoke_games, perf_full_games, perf_repeats, calibration_games, calibration_ref_ms }`, with
  `full_seeds`, `brief_rate`, `measured_rate` indexable by `Matchup` (`AI_GATE.full_seeds[matchup]`, as part 1's
  `AiDifficulty` is by `Difficulty`). Integer widths do not matter: every read is cast with `as`.
- `AI_TUNING_SERIES: &str`; `AI_BUDGET`, `AI_GATE_BUDGET: SearchBudget` (`Copy`, `PartialEq`, `Debug`,
  `Serialize`, field `nodes`).
- `gate_needed(Matchup, i32)`, `binomial_tail(i32, f64, i32) -> f64`, `game_config(Matchup, i32, SearchBudget,
  &str) -> MatchConfig`, `run_gate_games(Matchup, &[i32], SearchBudget) -> GateReport` (`.games: Vec<GateGame>`),
  `gate_shard_games(i32, i32, i32) -> Vec<_>`; `pub struct GateGame { seed: String, subject_seat: PlayerId,
  record: MatchRecord, won: bool, replay_hash: String, replay_errors }` (must be `pub`, and `Send`).
- `MatchConfig { seed, decks: (Vec<String>, Vec<String>), handicaps: Option<PerPlayerOpt<Handicap>>,
  controllers: PerPlayer<SeatController>, max_actions: Option<_> }`; `SeatController::{Ai { budget:
  Option<SearchBudget> }, Greedy, Random}` (`PartialEq`, `Debug`); `MatchRecord { result: Option<GameResult>,
  hash, turns, rejected: Vec<_: Debug>, thrown: Vec<_: Debug>, fallbacks, … }`.
- `MatchHooks<'a> { after_action: Option<Box<dyn FnMut(&GameState, &GameState, PlayerId, &ActionBody) + 'a>>,
  time_decision: … }` with `Default` (closures borrow local state, so the box needs a lifetime, not `'static`).
- `AiOptions { rng, budget, should_stop }` (SURFACE); `Decision { reason: DecisionReason (Serialize), stats:
  SearchStats { nodes } }`; `candidate_actions(&GameState, PlayerId) -> Vec<_>`; `AiDeckOptions { mana_cap:
  Option<i32>, .. }` with `Default`.
- `AI_EVAL`, `GREEDY_EVAL: EvalWeights` (`PartialEq`); `evaluate(&GameState, PlayerId, NextSwing, &EvalWeights)
  -> f64` with `NextSwing::Enemy` (TS's two defaulted arguments passed explicitly).
- `AI_DEV_RUN { games, series: &str }`, `DevRunOptions { series: String, patch: String, budget:
  Option<SearchBudget> }`, `dev_game_record(i32, &DevRunOptions) -> Option<GameRecord>`.
- Sweep: `AI_SWEEP { tiers (iterable of Difficulty), seeds_per_card, seeds_per_card_at_risk, at_risk_boost, … }`;
  `SweepOptions<'a> { seeds: Option<i32>, now: Option<&'a dyn Fn() -> f64>, tier: Option<Difficulty> }` (my name
  for TS's anonymous `{ seeds?, now?, tier? }`), by value; `sweep_card(&str, SweepOptions) -> SweepResult`;
  `sweep_at_risk(&str, &[String], &[String], SweepOptions) -> SweepPass2`; `at_risk_ids(&[SweepResult],
  &[(&str, &str)], &[(&str, &str)]) -> Vec<String>`; `pass2_keep_out(&[SweepResult], &[(&str, &str)]) ->
  Vec<String>`; `pass2_stats(&[SweepPass2], &str, Difficulty) -> SweepStats`; `sweep_verdict(&[SweepResult],
  &[SweepPass2]) -> SweepVerdict`. Types: `SweepResult { #[serde(flatten)] stats: SweepStats, tier: Difficulty,
  flags: Vec<SweepFlag>, unswept: bool }` (TS's `SweepStats & {…}`; `Clone`, serde camelCase); `SweepStats {
  def_id, games, drawn_games, affordable_turns, plays, errors, timeouts, eval_delta_sum: f64, eval_delta_count }`
  (serde camelCase); `SweepPass2 { forced, tier, games, cards: Vec<SweepStats>, suspects: Vec<SweepSuspect> }`
  (serde); `SweepSuspect { def_id, seed, forced, errors, timeouts }`; `SweepVerdict { def_id, flags, unswept,
  reason: Option<String>, watch: Option<String> }`; `SweepFlag: Serialize` as its literal.

`jackioh_engine`:
- `replay::{FoldArgs, fold, hash_state}` (part 5): `FoldArgs: Deserialize` with camelCase keys (TS
  `ReplayInput`), unknown keys ignored; `FoldResult { state, errors }` with `errors: Serialize` as
  `[{nonce, error}]`.
- `catalog::{query, CatalogQueryArgs}` (part 2): `query(&CatalogQueryArgs) -> Vec<CardDef>`, `CatalogQueryArgs:
  Default` (TS `query()`).
- `zones::active_units_of(&GameState, PlayerId)` (either `Vec<CardInstance>` or `Vec<&CardInstance>` works),
  `layers::unit_view(&GameState, &CardInstance) -> UnitView { attack, health, keywords: Vec<Keyword>, position:
  Position }`.
- `testkit::scenario(Value)` with `.state() -> &GameState` (part 5), for the perf gate's wide boards.

Data: `crates/engine/tests/golden/01-hotseat-full-game.json` (part 23's copy, SURFACE §13) for replay's tests.

## GAPS
- Not portable: gate-greedy's "turn AI_EVAL upside down and ask greedy again" (TS assigned into the config object
  at run time; Rust's `AI_EVAL` is a constant). `check_greedy_frozen` keeps the rest: `GREEDY_EVAL != AI_EVAL`,
  greedy acts in its main phase more than 3 times, and its decisions are a function of state and rng (asked twice,
  the same). That greedy reads `GREEDY_EVAL` alone is now a fact of `baselines.rs`'s source.
- Not portable: trace's `OVERRIDE_<CONFIG>` and `BAN` knobs (run-time assignment into constants; SURFACE §3 bans
  mutable statics). A tuning run edits `crates/ai/src/config.rs` or `shadow_ban.rs` and rebuilds.
- Replaced, not ported: `stats.ts`'s `parseDevRunArgs` (clap's `Args` reads the same flags; `count`'s refusal text
  is kept as the value parser's message; an unknown option is clap's exit 2), each script's `main()` (→ `run`).
- `replay` prints exactly SURFACE's `{"hash", "errors"}`: no `ok` and no `browserHash`, which the TS runner added
  for `e2e/support/tasks/replay.ts`. Part 21, which rewrites that task over `cargo jackioh replay`, hashes the
  browser's state itself (WASM `hash_state`) or SURFACE §12 grows the key.
- Every name in DEPENDS-ON whose shape SURFACE does not fix is a guess by §4.2/§4.3; part 31 aligns them with part
  17's actual code (the likeliest misses: `SweepResult`'s `stats` field vs flat fields, `SweepOptions`'s name,
  `MatchHooks`'s field types, `evaluate`'s arity, `AI_GATE`'s per-matchup fields).

## Decisions
- TS default arguments are passed explicitly (`game_config(m, n, AI_GATE_BUDGET, AI_GATE.seed_series)`,
  `at_risk_ids(first, SHADOW_BAN, SHADOW_WATCH)`, `evaluate(s, seat, NextSwing::Enemy, &AI_EVAL)`); no Rust
  default parameters exist.
- `gate` plays each matchup's games in parallel, one `run_gate_games(m, &[n], budget)` per rayon task, collected
  in order, and counts wins, turn-cap draws and rate itself (TS's `GateReport` numbers; a gate game is a pure
  function of `(matchup, n)`, so nothing changes but the wall clock). The perf gate is sequential.
- The CLI runs every check the TS gate files ran, in their order (seating, `binomialTail`, `gateNeeded`'s rule,
  the series and shadow-ban deal, greedy's frozen weights, then the run, its report and B31), collecting every
  problem instead of stopping at the first; then the perf gate. Exit 1 lists them all.
- `gate merge` keeps TS's shard file format and names (`<matchup>-<k>-of-<K>.json`, pretty JSON + newline) and
  reads `*.json` recursively, sorted; a shard with the wrong `total` stops the merge as TS threw.
- The perf `#[test]` judges the node budget and the wide boards' width in every build, and the clock only when
  `!cfg!(debug_assertions)` (an unoptimised test binary slows search and yardstick by different factors). The CLI
  (`cargo jackioh` is `--release`) always judges the clock. The yardstick's warm-up run is kept.
- `stats`: the newest patch is read at run time from `crates/cards/patches/patches.json` through
  `CARGO_MANIFEST_DIR` (TS read `../../cards/patches/patches.json` beside the script); a test pins it to
  `jackioh_cards::catalog_version()`. `--out` resolves against the cwd (TS's `INIT_CWD` was pnpm's) and is written
  a line per game, flushed.
- `sweep`: the ready-made entries print as Rust tuples, `    ("core-011", "<reason>"),`, for `shadow_ban.rs`'s
  `&[(&str, &str)]` (SURFACE §9), under "For crates/ai/src/shadow_ban.rs" and a header line naming `cargo
  jackioh sweep`; the date is today's UTC by civil-from-days (no `time` crate in the tools). Numbers print with
  `to_fixed`, which reproduces JS `toFixed` (an exact tie rounds away from zero; Rust's `{:.N}` rounds it to even).
- Shared helpers live in my own files only: `gate::{literal, matchup_name, parse_matchup, subject_seat_of}` and
  `sweep::to_fixed` are `pub(crate)` and used by `trace.rs`.
- Test names keep TS's B-numbers and ruling tokens (`r180_b30_…`, `r388_…`).
