# Slice: part 29 (the training arena, the promotion gate and the lane prompts), chunk 1 of 1
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run. `sh -n` and `dash -n`
(syntax only) were run on `training/loop.sh`.

## FILES
All were empty placeholders or absent on `staging`; all are complete. No `todo!`, `unimplemented!` or `// TODO`.
- `crates/tools/src/agent.rs` (SURFACE §14.1): `Args {}`, `run`, `AgentInfo`, `decide_with`, `own_info`,
  `own_shadow_ban`, `panic_message`; 3 unit tests.
- `crates/tools/src/arena.rs` (SURFACE §14.2; port of `packages/ai/scripts/duel.ts` and of match.ts's
  `playMatch` referee loop with agents for controllers): `Args`, `run`, `AgentSpec`, `Entrant`, `entrant`,
  `query_info`, `ArenaGame`, `ArenaOutcome`, `setups`, `game_setup`, `play_games` (rayon), `play_game`,
  `BinAgent`, `replays`, `record_id`, `game_record`, `records_path`, `append_records`, `utc_date`,
  `tally`, `records_of`, `won_by`; 6 unit tests incl. "two self agents over 4 games are deterministic".
- `crates/tools/src/promote.rs` (SURFACE §14.2): `Lane` (clap ValueEnum), `Args`, `run`, `GateCounts`,
  `gate_failures`, `wins_of`, `game_seed`, `ai_tree`, `GenerationRecord`, `verify_mismatches`; 8 unit
  tests (V26: both lanes' tables with synthetic counts, strictly-fewer bans, draws as losses, seeds stable
  across a generation.json-only commit in a throwaway git repo, verify's compared fields, history).
- `crates/ai/generation.json`: generation 0, exactly the brief's object (pretty-printed).
- `training/README.md`, `training/improve.md`, `training/unban.md`, `training/loop.sh` (POSIX sh, `set -u`,
  executable), `training/history/improve.jsonl`, `training/history/unban.jsonl` (empty).

## SURFACE
- `cargo jackioh arena --a self|random|bin:<path> --b … --games N --seed <base> [--out <dir>]`
- `cargo jackioh agent` (no flags; ops `info`, `decide`, `quit`, answers exactly §14.1's keys)
- `cargo jackioh promote --lane improve|unban --parent-bin <path> [--dry-run] [--verify]`
- Exit codes per §12: 0 pass, 1 failed gate / failed check (`Err`), 2 usage (a bad `--a`/`--b` is a clap
  value-parser error, so exit 2).

## DEPENDS-ON
`jackioh_ai` (part 17), all at the crate root as `lib.rs` globs them:
- `decide(&GameState, PlayerId, &mut AiOptions) -> Option<Decision>`, `AiOptions { rng, budget,
  should_stop }`, `Decision.action: ActionBody`, `AI_GATE_BUDGET: SearchBudget` (a const or a `Copy` static).
- `redact(&GameState, PlayerId) -> GameState`; `random_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>`.
- `build_ai_deck(&mut Rng, i32, &AiDeckOptions) -> Vec<String>`; `AiDeckOptions: Default` with
  `banned: Option<Vec<String>>` and `mana_cap: Option<i32>`.
- `SHADOW_BAN: &[(&str, &str)]`.
`jackioh_engine` root:
- `create_game(&CreateGameOptions)`, `begin_game`, `reduce(&GameState, &Action) -> ReduceResult { state,
  error: Option<String>, .. }`, `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`,
  `seat_to_act(&GameState) -> Option<PlayerId>`, `hash_state`, `fold(&FoldArgs) -> FoldResult { state,
  errors }` (part 5: `FoldArgs = ReplayInput`, `Deserialize`, built from `{seed, decks, log}` JSON),
  `summarize_game(&FoldArgs) -> Option<GameSummary>` (`GameSummary: Serialize`).
- `GameRecord: Deserialize + Serialize + PartialEq + Debug` (wire/stats.rs, part 5), built by
  `serde_json::from_value` from R376's JSON so no field or variant name is assumed; `DEV_RECORD_ID_PREFIX`.
- part 1's frozen `AI_DIFFICULTY`, `TRAINING_GAMES`, `TRAINING_IMPROVE`, `TRAINING_UNBAN`, `TrainingGate`,
  `Action::new`, `ActionBody`, `ActionType`, `GameResult`, `Winner`, `GameOverReason`, `PerPlayer`, `Rng`.
`jackioh_cards::{register_all, catalog_version}` (part 1).
Tools: `git` on PATH (promote's tree, merge base, dirty check; the promote tests make a throwaway repo).
loop.sh: `git`, `gh`, `cargo`, `devin`, `timeout`, `cmp`; `jackioh-server stats-import <file>` (part 20).

## GAPS
- None of my functions is left out.
- `AiDeckOptions` (part 17) must derive `Default` with `banned: Option<Vec<String>>`, `mana_cap: Option<i32>`
  (part 22 assumed the same). If part 17 typed `banned` as `Vec<String>`, `arena::deck_for` needs `Some(…)` dropped.
- `seat_to_act` is taken as SURFACE §6.1's `Option<PlayerId>`.
- SURFACE §14.2 says the records' `pilots` "name the two agents", but R376's `Pilot` is `"human" | "ai"`
  and `stats-import` parses records as `GameRecord`. Pilots are `{"p1":"ai","p2":"ai"}`; the agents are
  named in the record id instead (Decisions). Part 31: fix the SURFACE sentence.
- SURFACE §14.1/§14.2 do not say which shadow ban the `random` seat's deck uses; see Decisions.
- Part 30's `training-gate` job: run `promote --lane <lane> --parent-bin <main's jackioh> --verify` on a
  clean checkout of the branch (it refuses a dirty `crates/ai/src`); it needs `origin/main` fetched only for
  the report's parent commit (verify does not compare `parent` or `date`).
- Part 39 (the training box): the services' environment needs `DEVIN_MODEL`, `GH_TOKEN`, `PATH` with
  cargo, `devin`, `gh`, and optionally `DATABASE_URL`; `WorkingDirectory` the lane user's checkout;
  `ExecStart=<checkout>/training/loop.sh <lane>`; `Restart=always`. The loop exits 0 (to be restarted)
  when `training/loop.sh` changed on `main`, and 2 when `DEVIN_MODEL` is unset.

## Decisions
- **Agents.** `self` = `agent::decide_with` on `redact(state, seat)`, the exact call the `agent` subcommand
  makes on what it is sent, so `self` and `bin:<this binary>` play identical games. `random` = §10.7's
  policy on the referee's true state, as gate.ts plays the baseline (legal actions are the seat's own).
  `bin:<path>` spawns one `<path> agent` child per game (stateless agent, so no pool; a crash costs one
  game, a spawn failure fails the run). A leading `~/` in `bin:~/…` is expanded from `$HOME`.
- **Streams.** Each seat's stream is `createRng("<seed>:ctl:<seat>")` (playMatch's); the referee owns it,
  sends `rngSeed`/`rngCursor`, and resumes it at the answered cursor; an agent's `"action": null` falls
  back to `random_action` on that same stream (playMatch's fallback). Decks: `"<seed>:deck:<seat>"`
  (gate.ts's), Easy's `deck_size`/`mana_cap` (= HUMAN_HANDICAP, so no handicap stored).
- **The random seat's ban list** is this build's `SHADOW_BAN` (gate.ts: both seats of a gate use one list).
  `self` uses this build's, `bin:` the one its `info` reports.
- **Budget.** Agents decide at `AI_GATE_BUDGET` (the browser's), no `should_stop`.
- **Referee = playMatch.** `ARENA_MAX_ACTIONS = 3000` (match.ts's private `AI_MATCH.maxActions`, named in
  arena.rs), nonce `m<log length>`, refused action → `rejected` + replacements (endTurn, answers/mulligan,
  others; `AI_SKIPPED_ACTIONS` copied as `SKIPPED_ACTIONS`), panics in a controller or `reduce` caught
  (`catch_unwind`) and recorded in `thrown`, ending the game without a result.
- **Records.** `id = dev:<patch>:arena:<a label>-vs-<b label>:<seed>` (labels `self`, `random`,
  `bin-gen<N>`), so a promotion's two series, which share seeds, are distinct records. `source: "dev"`,
  `mode: "random"`, `patch = catalog_version()`, pilots both `"ai"`. Only finished games are records
  (devRun.ts). Arena: `--out <dir>/<date>.jsonl`, else `$JACKIOH_TRAINING_OUT/<date>.jsonl`, else stdout;
  appended. Promote: `$JACKIOH_TRAINING_OUT` only (its stdout is the report). Date = UTC by
  civil-from-days (the tools have no `time` crate).
- **duel.ts's line** is kept per game (`n, seed, subject, won, draw, lost, aborted, reason, turns, hash,
  rejected, thrown, fallbacks, replayOk, decisions, msSum, msMax, msP95`) plus a closing tally; to stdout
  when the records went to a file, else to stderr. Dropped from duel.ts: `nodes` (an agent reports no
  search stats) and its env knobs (`SUBJECT_<CONFIG>`, `OPPONENT_<CONFIG>`, `*_BUDGET`, `OPPONENT`,
  `SWAP_DECKS`, `SERIES`): Rust's AI config is `const`, and an arena run must measure the AI as built;
  experiments are other builds (`bin:`).
- **Tree and dirtiness.** `<tree>` is literally `git rev-parse HEAD:crates/ai/src`. Since the seeds come
  from the committed AI, a real promotion and `--verify` refuse a dirty `crates/ai/src`; a dry run warns
  and measures on HEAD's seeds. The prompts' order is therefore: work commit → real promote (fresh
  seeds) → on 0 amend `generation.json` + history into that commit (tree unchanged, so verify replays the
  same seeds); on 1 `git reset --soft HEAD~1`.
- **generation** = parent's `info.generation + 1`; `parent` = `git merge-base HEAD origin/main` (else
  `main`, else null); `generation.json` pretty-printed in SURFACE's key order (a struct, not a `Value`,
  since serde_json has no `preserve_order`); history appended compact, a last line of the same
  generation and lane replaced (a re-run after a rebase stays one line).
- **--verify** compares `generation, lane, tree, vsRandom, vsParent, shadowBan, parentShadowBan`; not
  `date` or `parent` (main may move without its AI moving). A mismatch or a failed gate exits 1.
- **Gate.** `gate_failures` lists every failed rule; a draw and a game without a result are not wins;
  the unban rule is `shadow_ban < parent_shadow_ban`. The improve lane has no ban rule (its prompt tells
  Devin to leave `shadow_ban.rs` to the unban lane).
- **loop.sh** runs from a copy of itself (the reset rewrites the checkout), waits while a PR from
  `ai/<lane>` is open, treats "anything under `crates/`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml` changed on main since the parent was built" as "the parent changed" (drop, keep a
  local `ai/<lane>-stale-<t>` branch, note it in attempts.md), re-checks with `promote --verify` (its
  report is the PR body), pushes with `--force-with-lease` (the lane branch is reset every cycle), caps a
  session with `timeout` at 4 h 30 min, and imports each finished day's records once (`imported.txt`).
