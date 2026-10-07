# Slice: part 5, chunk 3 of 4 (#393): setup, game summary, codes, the validator, the glow helpers
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner. Node 22 and Python 3.13 ran
scratch scripts (outside the repo) that generated and checked the Unicode tables (below).

## FILES
- `crates/engine/src/setup.rs` ← `packages/engine/src/setup.ts`, whole.
- `crates/engine/src/game_summary.rs` ← `packages/engine/src/gameSummary.ts`, whole.
- `crates/engine/src/wire/codes.rs` ← `packages/shared/src/codes.ts`, whole, plus its NFKC and
  `packages/shared/test/codes.test.ts` as `#[cfg(test)] mod tests` (every `it`; the per-row `it`s are
  one `#[test]` each looping over the rows, the row name in every assertion message).
- `crates/engine/src/validator.rs` ← `packages/validator/src/index.ts`, whole.
- `crates/engine/tests/rules/validator.rs` ← `packages/validator/test/validator.test.ts`, every `it`.
- `crates/engine/src/testkit/glow.rs` ← `packages/cards/test/_glow.ts`, whole.

## SURFACE
Matched §4–§6 as written; where part 1's compiled freeze differs, part 1's names were used.

## DEPENDS-ON (names called, expected from other parts, with the form guessed)
- `crate::scripts::flags_of(state: &GameState, card: &CardInstance) -> StaticFlags` (part 2). TS's
  `flagsOf(card)` took no state; fused scripts are built from state (§6.6), so I passed it.
- `crate::catalog::find_def(state: Option<&GameState>, def_id: &str) -> Option<…CardDef>` (part 2;
  TS `findDef(state | null, id)`).
- `crate::zones::{move_to_zone, OffFieldZone, MoveOptions, MovePosition}` (part 2):
  `move_to_zone(state: &mut GameState, card: &CardInstance, OffFieldZone::Hand | Library, MoveOptions)`;
  `MoveOptions { position: Option<MovePosition>, keep_state: … }: Default`; `MovePosition::At(i32)`
  for TS's numeric position. setup passes a clone of the card (TS passed the live object); for the
  mulligan's shuffle-back the card is in no pile, so `move_to_zone` must insert the card it is given
  when it finds none to remove (TS pushed the object it was handed). Return value ignored.
- `crate::own_library::show_to_owner(&mut CardInstance)` (part 2, per part 1's notes).
- `crate::draw::{draw(sink, player, count: i32), add_to_hand(sink, CardInstance), cast_dealt_card(sink,
  &CardInstance), casts_on_draw(&GameState, &CardInstance) -> bool}` (draw.rs's owner).
- `crate::prompts::run_start_of_game(sink, &CardInstance, controller: PlayerId)`.
- `crate::state_check::state_check(sink)`; `crate::turn::{clear_return_flags(&mut GameState),
  start_turn(sink, PlayerId)}`.
- `crate::work::{paused(&EngineSink) -> bool, owe(sink, Vec<_>)}`: I call `owe(sink, vec![resume.into()])`,
  which compiles whether `owe` takes `Vec<Resume>` or a `Vec` of an enum with `From<Resume>` (TS's
  `...items: (Resume | WorkItem)[]`).
- `crate::reduce::{begin_game(&GameState), reduce(&GameState, &Action)} -> ReduceResult { state,
  events, error: Option<String> }` and `crate::replay::FoldArgs { seed, decks: (Vec<String>,
  Vec<String>), log: Vec<Action>, catalog, handicaps, dealt, last_boards, glitch_boards }` (part 5's
  other chunks; FoldArgs' fields typed as `CreateGameOptions`').
- `crate::wire::{GameSummary { first: PlayerId, winner: Winner, reason: GameOverReason, turns: i32,
  seats: PerPlayer<SeatSummary> }, SeatSummary { deck, opening, drawn, played, played_turns:
  Option<Vec<i32>> }}` (wire/stats.rs, part 5's other chunk).
- `crate::testkit::scenario::Scenario::view(&self, PlayerId) -> PlayerView` (scenario.rs).
- The validator test's fixture `tests/rules/fixtures/validator_loadouts.rs` (part 5's other chunk):
  consts `CATALOG_VERSION, HIT_JOB, ARCHIVIST, JELLY_BEAN, CEASELESS_VOID, NOT_IN_CATALOG` as `&str`;
  `POOL_IDS` iterable (`.iter()` of `&str` or `String`); `catalog() -> CatalogSnapshot`,
  `collection() -> Collection`, `legal_loadout() -> LoadoutInput`; every injection
  `fn(&LoadoutInput) -> LoadoutInput` (`drop_deck`, `add_deck`, `drop_card`, `add_spare_card`,
  `duplicate_in_deck`, `insert_token`, `cross_deck`, `unown_card`, `unknown_card`, `banned_card`);
  `INJECTIONS: &[Injection]` (or anything indexable) with `rule: LoadoutRule`, `label: &str`,
  `apply: fn(&LoadoutInput) -> LoadoutInput`.
- `crates/engine/tests/fixtures/code-input-cases.json` (part 5's other chunk): codes.rs's tests read
  it with `include_str!`, as an array of rows (or `{ "cases": [...] }`), each row's columns
  (`name`, `input`, `canonical`, `formatted`, `problem`, `character`, `foundInText`) at its top level
  or under `expected`, and `problem` either the kind string or `{ kind, character }`. So either the
  TS rows verbatim or the brief's `{ input, format, expected }` shape reads.

## GAPS
- Every DEPENDS-ON entry above whose shape is a guess: `flags_of`'s state argument,
  `find_def`'s `Option<&GameState>`, `MoveOptions`/`MovePosition::At`, `owe`'s list argument, the
  fixture module's by-reference injections and `POOL_IDS`' form.
- `validator::DeckDraftInput` holds TS's two predicates (`is_deckable`, `is_portrait`) as
  `&dyn Fn(&str) -> bool`, so it is not JSON: SURFACE §10.1's `validator("checkDeckDraft", json)`
  binding (part 21) has to build the predicates itself (e.g. from a list of deckable ids and
  `PORTRAIT_IDS` in the input JSON). Every other validator input/output type is serde both ways.
- The codes tests copy the server's `CODE_ALPHABET`, `CODE_INPUT_MAX_LENGTH`, `INVITE_CODE_FORMAT`
  and `ROOM_CODE_FORMAT` values (the engine cannot depend on `jackioh-server`); the server's own
  config (part 18) must build its `CodeFormat` consts with `length`, `group_size` and
  `max_input_length` as `usize` and `alphabet`/`separator` as `&'static str`.
- `CodeFormat`, `CodeInputProblem`, `CodeInputReading` and the validator's types derive `ts_rs::TS`
  under `--features ts` (SURFACE §5.1 "every type in wire/"); the web also keeps its hand copy of
  `codes.ts` (§10.4), so part 21's `apps/web/src/wire/index.ts` must not `export *` both. `DraftIssue`
  has a default type parameter (`DraftIssue<Rule = DraftRule>`), which ts-rs may not accept.

## Decisions
- **NFKC by hand.** The pure crates may name no Unicode crate, and R191 needs the Rust server to
  read a code exactly as the web's TS does. So codes.rs carries the full NFKD decompositions (5,913),
  canonical combining classes (393 runs) and primary composites (961) of Node 22's ICU (Unicode
  16.0), generated from `String.prototype.normalize` itself (CCC from Python's `unicodedata` 15.1,
  12 new Unicode 16 marks inferred from Node's reordering) and checked, as the Rust algorithm writes
  it (decompose, stable-sort non-starters, compose with blocking, Hangul by arithmetic), against
  Node's `normalize("NFKC")` on every code point and 3,000,000 random strings: no mismatch. The
  tables are `#[rustfmt::skip]` consts in a private `nfkc` module (~180 KB of the file).
- JS `\s`, `trim`, `\p{Cc}`, `\p{Cf}`, `\p{White_Space}`, `\p{Default_Ignorable_Code_Point}` are
  range tables enumerated from Node's regex engine; never Rust's `char::is_whitespace` (it differs on
  U+0085 and U+FEFF). Case mapping is Rust's `char::to_uppercase`/`to_lowercase` (full mappings, as
  JS's); a code point mapping to more than one is kept, as TS did.
- JS `.length` → UTF-16 code units (`utf16_len`) wherever TS measured a string (raw input cap,
  `format_code_characters` slices by UTF-16 units).
- `why_mulligan_refused` returns `Option<String>` (TS `string | null`); `mulligan_prompt_for` returns
  `Option<&PendingChoice>`. `run_owed_setup(sink, &WorkItem)` is pub for `work.rs`'s `"@setup"` arm.
- setup's owed records are built as JSON (`json!`) with TS's keys; serde_json sorts object keys (no
  `preserve_order`), which the hash does too.
- `delete memory[k]` is `shift_remove` (keeps the other keys' order, as JS does); JS `Map` is
  `IndexMap` with `shift_remove` for `delete`.
- `summarize_game` hands `create_game` no `dealt`, exactly as TS's `summarizeGame` did (only `fold`
  passes it); it changes no event the summary reads.
- `LoadoutResult { ok, errors }` with `errors` skipped when empty (TS's `{ ok: true }` /
  `{ ok: false, errors }`); `ImportRoom { ok, decks_short?, trios_short?, message? }` likewise.
  `LOADOUT_DECKS`/`TRIO_DECKS: usize`; `LoadoutError.deck: Option<usize>`; collection counts `i32`.
- `trio_conflicts(&[LoadoutDeck])` (TS took any `{ cards }[]`; `name` is optional, so `[{cards}]` JSON
  reads).
- The validator's property tests draw from the engine's `Rng::new("20260917", 0)` instead of
  fast-check (same seed and run count; names are 1–12 printable ASCII characters, `None` one time in
  six).
- glow.rs reads the view as JSON (`glows<T: Serialize>(Option<&T>)`), since TS asked "is the key
  there"; `viewer`/`owner` are explicit `PlayerId` arguments (TS defaulted them to `"p1"`).
