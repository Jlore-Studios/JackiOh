# Slice: part 5, chunk 4 of 4 (turn, game over, instance view, wire aim and stats, the invariant monitor, the code-input fixture)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run. One `node
--experimental-strip-types` run read `packages/shared/test/fixtures/code-input-cases.ts` to write
its rows as JSON (a conversion, not a build or a test; the script checked its own escape round trip).

## FILES
- `crates/engine/src/game_over.rs` ← `packages/engine/src/gameOver.ts`
- `crates/engine/src/instance_view.rs` ← `packages/engine/src/instanceView.ts`
- `crates/engine/src/turn.rs` ← `packages/engine/src/turn.ts`
- `crates/engine/src/wire/aim.rs` ← `packages/shared/src/aim.ts` (plus two small unit tests of its own)
- `crates/engine/src/wire/stats.rs` ← `packages/shared/src/stats.ts`, with `packages/shared/test/stats.test.ts` as `mod tests` (every `it`, one `mod` per `describe`)
- `crates/engine/src/testkit/invariants.rs` ← `packages/cards/test/_invariants.ts` (669 lines, not the brief's 258: I1–I6, the whole file)
- `crates/engine/tests/fixtures/code-input-cases.json` ← `packages/shared/test/fixtures/code-input-cases.ts`

## SURFACE
Matched §4.1 paths, §4.2 names, §4.3 types. Shapes decided here (other parts call these):
- `game_over::end_game(sink: &mut EngineSink, winner: impl Into<Winner>, reason: GameOverReason)`: a `PlayerId` or a `Winner` both pass.
- `instance_view::instance_data_view(&GameState, &CardInstance) -> InstanceData` (a struct of the five `CardView` keys, each `Option`, serialised as `CardView` writes them) with `InstanceData::apply_to(self, &mut CardView)` for TS's spread; `hand_keywords_view(&GameState, &CardInstance) -> Option<Vec<Keyword>>`.
- `turn`: `start_turn(&mut EngineSink, PlayerId)`, `end_turn(&mut EngineSink)`, `concede(&mut EngineSink, PlayerId)`, `offer_draw`, `answer_draw(&mut EngineSink, PlayerId, bool)`, `can_offer_draw(&GameState, PlayerId) -> bool`, `has_standing_draw_offer(&GameState, PlayerId) -> bool`, `standing_draw_offer(&GameState) -> Option<PlayerId>`, `clear_return_flags(&mut GameState)`, `trigger_order(&EngineSink, HookName, Option<PlayerId>) -> Vec<CardInstance>`, `START_OF_TURN_WORK`, `END_OF_TURN_WORK` (`&str`), and the two work handlers `run_owed_start_of_turn(&mut EngineSink, &WorkItem)` / `run_owed_end_of_turn(&mut EngineSink, &WorkItem)` that `work.rs`'s dispatcher must call for `"@startOfTurn"` / `"@endOfTurn"` (SURFACE §6.6; TS registered them).
- `wire::aim`: `AimEnd` (tagged on `at`: `Hero { player }`, `Zone { player, row, lane: i32 }`, `Hand { player, index: usize }`), `Aim { source, target: Option<AimEnd> }` (null, not skipped), `parse_aim_end(&Value) -> Option<AimEnd>`, `parse_aim(&Value) -> Option<Option<Aim>>` (`None` = TS undefined, `Some(None)` = TS null), `aim_key(Option<&Aim>) -> String`.
- `wire::stats`: `GameSource`, `GameMode`, `Pilot`, `SourceFilter`, `PilotFilter`, `Breakdown` (string unions), their `GAME_SOURCES`, `GAME_MODES`, `PILOTS`, `SOURCE_FILTERS`, `PILOT_FILTERS`, `BREAKDOWNS` slices, `GAME_OVER_REASONS`, `SeatSummary`, `GameSummary { first, winner: Winner, reason, turns: i32, seats: PerPlayer<SeatSummary> }`, `GameRecord`, `DEV_RECORD_ID_PREFIX`, `CardStatsFilter`, `DEFAULT_CARD_STATS_FILTER` (a `const`), `Tally` (`i32`s), `CardStats` (fields per breakdown, `Index<Breakdown>`, `tally_mut`), `CardStatsReport`, `BREAKDOWN_TITLES` (a unit struct with `Index<Breakdown, Output = str>`), `sources_of`, `record_matches`, `seats_counted`, `card_stats(&[GameRecord], &CardStatsFilter)` (no default argument: pass `&DEFAULT_CARD_STATS_FILTER`), `win_rate -> Option<f64>`, `played_delta -> Option<f64>`, `describe_filter`, `format_tally`, `format_delta(Option<f64>)`, `format_card_stats(&CardStatsReport, impl Fn(&str) -> Option<String>)`, `parse_game_record(&Value) -> Result<GameRecord, EngineError>`, `parse_game_record_lines(&str) -> Result<Vec<GameRecord>, EngineError>` (TS threw; the message is TS's).
- `testkit::invariants`: `I6_GATE_STRIDE: usize`, `hidden_information_violations(&GameState, PlayerId, &PlayerView, &[ActionBody]) -> Vec<String>`, `InvariantMonitor` (= SURFACE §8's `Monitor`, a type alias) with `new`/`create_invariant_monitor(&GameState)`, `before(&mut self, &GameState, PlayerId, &ActionBody)`, `after(&mut self, &[GameEvent], &GameState)`, `hidden(&self, &GameState)`, each `-> Vec<String>`.
- The fixture keeps TS's row keys verbatim (`name`, `input`, `canonical`, `formatted`, `problem`, `character` only where TS has it, `foundInText`), all for `INVITE_CODE_FORMAT`, with every character outside printable ASCII written as a `\uXXXX` escape so the invisible ones read in the file.

## DEPENDS-ON
Names called at their TS module's Rust path (fullsend builder rule 6), signatures guessed from the TS:
- `crate::faces::{card_type_of(&GameState, &CardInstance) -> CardType, running_face(&GameState, &CardInstance) -> CardFace (or &)}` (part 2)
- `crate::catalog::def_of(&GameState, &str) -> &CardDef` (`.type_`, `.token` read) (part 2)
- `crate::brittle_count::active_brittle_count(&CardInstance) -> Option<i32>`
- `crate::params::params_view(&GameState, &CardInstance) -> Option<IndexMap<String, i32>>`
- `crate::tuning::is_tuned(&CardInstance) -> bool`
- `crate::layers::{card_keywords(&GameState, &CardInstance) -> Vec<Keyword>, unit_view(&GameState, &CardInstance) -> UnitView}` (`.keywords`)
- `crate::scripts::script_of(&GameState, &str) -> CardScripts` (or `&`/`Arc`; `.base`/`.radiant` read) as SURFACE §6.6 words it
- `crate::resolve::HookName` (an enum with `StartOfTurn`, `StartOfOpponentTurn`, `EndOfTurn` and `as_str()`)
- `crate::triggers::{settle(&mut EngineSink, SettleOptions), SettleOptions: Default, dispatch_pending(&mut EngineSink), queue_hooks_in_trigger_order(&mut EngineSink, HookName, Option<PlayerId>)}`
- `crate::state_check::state_check(&mut EngineSink)`
- `crate::prompts::{run_resume(&mut EngineSink, &Resume, ResumeOptions), ResumeOptions { controller: Option<PlayerId>, .. }: Default}`
- `crate::effects::delay::run_engine_delayed(&mut EngineSink, &DelayedEffect) -> bool`
- `crate::modifiers::expire_modifiers(&mut EngineSink, PlayerId)`
- `crate::mana::{refresh_mana(&mut PlayerState), mana_event(PlayerId, &PlayerState) -> GameEvent, NEXT_REFRESH_MODIFIER_ID: &str}`
- `crate::brittle::brittle_tick`, `crate::animated::{animate_at_turn_start, return_at_cleanup}`, `crate::temporary::discard_temporary_cards` (each `(&mut EngineSink, PlayerId)`)
- `crate::draw::draw(&mut EngineSink, PlayerId, i32)` (result ignored)
- `crate::traps::{run_trap_window(&mut EngineSink, &GameEvent), end_handed_over_turn(&mut EngineSink)}`
- `crate::query::unspent_mana_of(&GameState, PlayerId) -> i32`
- `crate::subsystems::board_history::record_board_snapshot(&mut GameState)`
- `crate::zones::active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>` (or owned)
- `crate::announce::announce_of(&GameState, &str) -> Option<&AnnounceRecord>`
- `crate::combat::attack_targets(&GameState, &CardInstance) -> Vec<AttackTarget>` with `crate::damage::DamageTarget::{Hero { player }, Unit { instance }}`
- `crate::setup::returned_awaiting_shuffle(&GameState) -> Vec<String>`
- `crate::subsystems::copied_text::copied_text_of(&GameState, &CardInstance) -> Option<PlayRecord>` (`.def_id`)
- `crate::view_for::view_for`, `crate::reduce::legal_actions` (SURFACE §6.1)

## GAPS
- None of my functions left out; no stub in my files.
- `work.rs` (part 3) must dispatch `turn::START_OF_TURN_WORK` → `turn::run_owed_start_of_turn(sink, &item)` and `turn::END_OF_TURN_WORK` → `turn::run_owed_end_of_turn(sink, &item)`; if it hands the item by value, add the `&`.
- The brief described the fixture as "`{input, format, expected}` objects"; the TS rows have no `format` or `expected`, so the JSON keeps TS's own keys (above). Whoever ports `codes.test.ts` (part 5's `wire/codes.rs` chunk) and the web's `CodeField.test.tsx` read `row.canonical`, `row.formatted`, `row.problem`, `row.character`, `row.foundInText`, `row.name` as the TS did.
- SURFACE §5.1 makes every wire type derive `ts_rs::TS`; `Aim`, `AimEnd`, the stats types now also generate files under `apps/web/src/wire/generated/`, while part 21 also copies `aim.ts` and `stats.ts` (which define the same names) into `apps/web/src/wire/`. Part 21 must keep one of the two from `index.ts`'s `export *` (TS2308 otherwise).
- `wire/stats.rs` imports `crate::state::EngineError` (the one error type): a wire module depending on `state`.
- `HookName` (part 3, `resolve.rs`): `trigger_order` and the three hook queues pass enum variants; if part 3 kept `&str`, switch them to the literals.

## Decisions
- Private copies (fullsend builder rule 5) in `turn.rs`: `work.paused` (`is_paused`), `work.owe` of one `Resume` = `work.pushWork` (`w<nextSeq>` id, seq, owner = active, insert at `min(workCursor, len)`, cursor = at + 1), `modifiers.dueDelayed`, `modifiers.dueStartOfTurnEffects`, `modifiers.dropDelayed`; in `invariants.rs`: `setup.mulliganPromptFor`; in `instance_view.rs`: `enchantmentsOf` and `copyTuning` (a clone).
- `turn.rs`'s `dueBefore`: a `u32` creation mark; TS's `Infinity` (no mark) is `u32::MAX`, stored in `Resume.data` as JSON `null` (what `JSON.stringify(Infinity)` writes, so the hash agrees) and read back as every entry due; any stored number is read as `ceil(n)`, which is exact for whole-number seqs.
- `turn.rs` keeps `run_owed_*` public (work.rs calls them) and documents the dropped `registerWorkHandler`.
- `trigger_order`'s backrow scan reads `players[p].backrow` by lane directly (what `slotsOf`/`cardAt` did); units come from `zones::active_units_of`. A Vanilla card has no hooks (TS's `EMPTY_SCRIPT`).
- `stats.rs`: `DECIMALS`, `PERCENT`, `NO_FIGURE` stay private named constants in the module (presentation, as TS kept them; not rules numbers for `config.rs`). `toFixed` is ported by hand (`to_fixed`): JS rounds a tie away from zero on the exact value and writes `-0.0` for a small negative, where Rust's `{:.1}` rounds ties to even. String widths count UTF-16 units, as JS `length`/`padEnd` do ("Δ", "—").
- `stats.rs`'s readers keep TS's check order (struct literal fields evaluate in source order) so the first bad field named is TS's; `Number.isInteger` accepts any whole JSON number (`3.0`).
- `stats.rs` adds `game_over_reasons_are_exhaustive` for TS's `GameOverReasonsAreExhaustive` type, and `to_fixed_rounds_as_js_does`.
- `invariants.rs`: the JSON walked is serde's (BTreeMap key order), so the path a message names for a needle found twice may differ from TS's; which needles are found does not. `hidden` catches a panic in `view_for`/`legal_actions` with `std::panic::catch_unwind` (TS's try/catch) and reports it; the panic hook still prints it. A mulligan return is read off the owed item's JSON (id, defId, owner) rather than deserialised as a `CardInstance`.
- `aim.rs`: a lane is `i32` (as everywhere on the board), a hand index `usize` (an index, §4.3); both accept any whole JSON number at least their floor, as `Number.isInteger` did.
