# `jackioh-shared`

The logic the server and its clients share (#2956, Part B): one pure crate that every decision now made
twice, once in `crates/server` and once in `apps/web`, moves into. This file is the contract other code
relies on. The aim (the owner's, 2026-11-05) is that a Roblox version (#2898) needs as little rewriting as
possible: Roblox runs the pure crates translated from the C module (`crates/wasm/c`, #3093), so its server
scripts and client supply only I/O (timers, RemoteEvents, DataStores) and layout, and everything else
comes from this crate through that module.

It is pure (CLAUDE.md rule 4). The first thing in is the Conquest series rules (SPEC §9.5, R330 to R337),
from `crates/server` (#3098); the second is what a card face prints (#3106), from `apps/web`.

## Host contract

Every Part B issue follows it, and review holds each one to it.

1. **One copy, three callers.** The server calls the crate natively. The browser and Roblox call it
   through a body in `jackioh-wasm-core`, bound by both `crates/wasm` and `crates/wasm/c`.
   `crates/wasm/tests/export_parity.rs` fails a body bound in only one of them (`series` is the first, `face`
   the second: a binding the Luau module, which leaves with #2956 part 5, does not bind is listed in its `NORMAL_ONLY`). A body is a JSON adapter
   and nothing more: it parses its argument into this crate's types, calls this crate and writes the
   answer as JSON, so no decision is made, or restated, there.
2. **JSON at the boundary.** Every type that crosses it derives serde and, behind the `ts` feature,
   ts-rs, like the engine's wire types, so a Luau type emitter can later be added beside ts-rs. One
   call is one tagged union in and one tagged union out (`SeriesCall`, `SeriesAnswer`), so a host has two
   types to read per body, and a function that is new here is a variant, not a new export.
3. **The caller owns the state.** A long-lived thing (a match, a practice session) is a serializable
   value the caller holds and hands back on every call, like `GameState`. A Roblox server can then
   checkpoint it to MemoryStore and resume it on another server (#2898, risk 4). This crate keeps
   nothing between calls: no static, no cache.
4. **Everything impure comes in as an argument:** the time (`now_ms`, or the `now` of the first move,
   in epoch milliseconds), random seeds (drawn through the engine's `Rng`, the same 32-bit generator),
   the locale, and whatever the store read.
5. **Everything the host must do comes out as data:** rows to write, messages to send to a seat, timers
   to set. Never a trait object, a callback or a store handle across the boundary. The server's store
   code and a Roblox DataStore script then do the same job from the same output.
6. **Data is compiled in** (rule 4): quest pools, registries and translations. The module carries it, and
   no host loads files. Small tables are read with `include_str!`; the face model's (the glossary, the
   powers, the short forms, the translations and the Chinese card tables) are turned into immutable sorted
   tables by `build.rs` from `data/*.json` and `crates/cards/chinese*.json`, so a call parses no JSON and the
   crate keeps no static of its own state or cache.
7. **Its numbers** are named constants in `src/config.rs` (rule 9) and join the `constants` export of
   `jackioh-wasm-core` under their own names, so no host restates them; that module's test holds its
   list to `config.rs`.
8. **Integers:** new types use `i32` or `u32` where the value fits, because `i64` is emulated once
   translated to Luau (#2898, risk 1). An existing type's width is not changed only for this: the series
   row keeps the `i64` its stored shape has, and a time in epoch milliseconds (`now`) does not fit 32 bits
   either, so it stays `i64` (a Luau number holds it exactly). `f64` is fine.

## Purity (CLAUDE.md rule 4)

No clock, no I/O, no environment, no threads, no OS randomness, no hash-ordered collection, no mutable
static. Two things hold it, as in the other pure crates:

- `Cargo.toml` takes `serde`, `indexmap` and the engine and, behind the off-by-default `ts` feature,
  `ts-rs` (`serde_json` only as a dev-dependency). It may also take `serde_json`, the cards, the AI and
  the run, never `jackioh-luau`, and none of them takes it.
- `clippy.toml` is the other five pure crates' byte for byte (`crates/tools/tests/acceptance.rs` holds
  the six the same and runs clippy over a fixture that uses each entry once). CI's `cargo clippy
  --workspace --all-targets -- -D warnings` fails a use anywhere in the crate, tests included.

A transition never reads a clock: it takes `now`, and the host passes the time it read.

## `src/config.rs`

`MS_PER_SECOND`, `SERIES_WINS_NEEDED` (R330), `SERIES_MAX_GAMES` (R334), `SERIES_PICK_SECONDS`
(R333), and the face model's `TEMPORARY_LAST_COUNT` (R637, R2179001) and Glitch's `GLITCH_SEED`, `GLITCH_SWAP_IN`,
`GLITCH_MARK_IN` and `GLITCH_ODDS_SCALE` (issue #170). The server re-exports the series' from its own `config.rs` (so `serverConfig.ts` and its code name
them as before), and `jackioh-wasm-core`'s `constants` carries them, which the browser and the C module
(`crates/wasm/c`) both bind.

## `src/series.rs`: the Conquest series

A series is a `SeriesRow`: both sides' frozen trios, their sealed picks, the games so far, the pick
clock and the `version` the store writes by compare-and-set. The functions below are pure over it.

| Function | Answers | Rulings |
| --- | --- | --- |
| `new_series(&NewSeriesInput, now)` | the row in game 1's pick phase, its clock running from `now` | R331, R333, R263 |
| `pick_deck(&row, seat, slot, now, game_no)` | the row with that seat's pick sealed in, the game begun when it completes both; refused for a closed clock, a bad or won slot, a pick already in, or another game's pick | R331, R330, R333 |
| `begin_game(&row, now)` | both picks are in, so the game is recorded and `playing` | R331 |
| `game_ended(&row, winner, reason, now, new_match_id)` | the game in play ended: its deck locked, then the series decided, exhausted, or the next pick phase opened (a seat with one deck left is picked for) | R330, R334, R332, R263 |
| `timeout_picks(&row, now)` | the clock ran out: a seat that did not pick gets its first unwon deck, and a series where neither picked is abandoned | R333, R332 |
| `abandon_unstarted(&row, now)` | a game whose match could not be started: taken off the record, the series abandoned and unrated | R263, R262 |
| `forfeit_series(&row, seat, now)` | the seat leaves between games, the other side wins | R334 |
| `rate_series(&row, rating_move)` | the row with the series' one rating move recorded; both fields null when abandoned or unranked | R262 |
| `game_seats(&row)` | the two seats and the seed of the game in play | R335, R642, R1834002 |
| `already_picked(&row, seat, slot, game_no)` | whether that pick is already in: a retried request is a success | R331 |
| `project_series(&row, viewer, now)` | what one player may see of the series, or null for a profile that is not one of its two | R336 |
| `series_score(&row)` | series `p1`'s score once over, 1, 0.5 or 0, null when not over or abandoned | R262 |

The reads under them (`seat_of`, `won_slots`, `unwon_slots`, `first_unwon`, `both_picked`,
`first_seat_of`, `trio_deck`, `seat_index`, `other_seat`) are public for the server's bot picks.

- **The row types are what the store writes.** `series.state`, a ticket and a room are stored as the serde
  form of `SeriesRow`, `FrozenTrio` and the rest, so changing a field, a rename or a `skip_serializing_if`
  is a store change, with its migration and the store contract's tests.
- **A transition returns a new row at `version` + 1 (R263)** and leaves the one it was given alone; the
  host writes the new one only over the version it read. `rate_series` is the one exception, because it
  rides on the ending's own write.
- **A refusal is data.** A transition that does not apply returns a `SeriesRefusal` (a
  `SeriesRefusalReason` and the sentence a player reads), never a panic; the reasons after the first five
  are the host calling out of turn after a retry, which it treats as "nothing to do".
- **The projection hides the opponent (R336).** `project_series` gives a player all of their own trio and
  of the opponent's only which slots have won and whether a pick is in: never its slot before the game
  has begun, never a deck name, never a card.
- **`game_seats` panics on a row whose picked slot is no deck.** The picks are checked when they are
  made, so only a row that was not written by these transitions can do it.

### The boundary

`jackioh-wasm-core`'s `series(call)` takes one `SeriesCall` as JSON and answers one `SeriesAnswer`:

```json
{ "call": "pickDeck", "series": { "...": "the row" }, "seat": "p1", "slot": 2, "now": 1700000000000 }
{ "row": { "...": "the next row, to write" } }
{ "refused": { "reason": "pick_sealed", "message": "Your pick for this game is already in, and it is final." } }
```

The calls are `newSeries`, `pickDeck`, `beginGame`, `gameEnded`, `timeoutPicks`, `abandonUnstarted`,
`forfeitSeries`, `rateSeries`, `gameSeats`, `alreadyPicked`, `projectSeries` and `seriesScore`; the
answers are `row`, `refused`, `seats`, `picked`, `view` and `score`. A call that does not parse is
refused by the body as an error (thrown in the browser, status 1 in the C module), and a rules refusal
is never one. `apply` is the one function behind it, and `tests/boundary.rs` holds it to the functions it
names.

## `src/face/`: what a card face prints

The face model of `apps/web/src/cards/model.ts` and its neighbours (`inPlay.ts`, `cardState.ts`,
`shortText.ts`, `radiantDiff.ts`, `rules.ts`'s tokenizer, `glossary.ts`), moved here with the translations of
the face and glossary strings, so a client lays out what this returns and writes none of it. The decision
the owner took on #2956 (2026-11-05): the model itself is Rust, not only its wording. Nothing in it is a rule
(CLAUDE.md rule 7): it reads the catalog, the definitions a view carries and what `view_for` says of a card,
never state, so a hidden card cannot be named by it.

| Call | Takes | Answers |
| --- | --- | --- |
| `card` | a `FaceSource` (a `defId`, optionally its `def`, the face, and, in a game, `inPlay`), `locale`, `defs` | the `FaceModel` |
| `live` | a `LiveCard` (a `CardView`, `UnitView` or face-up backrow card as the view lists it), its `def`, `facts`, `locale` | the face in play the view says it has (R243) |
| `view` | a `PlayerView`, `locale` | every face the view shows, by instance id: one call for a whole view |
| `named` | a `PlayerView`, a `NamedCard`, `locale` | the face of a card an event or a prompt names, or `noFace` for the sentinel |
| `text` | words from the engine or a card, `locale` | the same words as spans, the glossary's terms in bold |
| `wordDiff` | two texts | what the newer adds and what the older loses, by word, in UTF-16 offsets (R277, R388) |

```json
{ "call": "card", "locale": "zh-CN", "source": { "defId": "core-020", "radiant": true } }
{ "answer": "face", "face": { "name": "…", "text": { "full": "…", "spans": [ { "kind": "mark", "children": [ … ] } ] }, … } }
```

- **Spans, not markup.** A face's text is `RichText { full, spans }`: `full` the plain words (what a client
  measures and keys on), `spans` the same words as `Span`s: `text`, `keyword` (the term and its glossary
  entry), `number` (a number the view moved off its printed value, with whether that is better or worse and a
  title that says so), `card` (a name that points at another card), `mark` (words a Radiant face adds to the
  base face's, R277) and `value` (what a card's formula comes to, R280). `card` and `mark` hold their
  children already nested, a mark that crosses a name's edge cut there. No HTML, Markdown or CSS: the web maps
  spans to elements and Roblox to RichText.
- **Final words.** Everything a face prints is in the reader's language: the name and text (from the Chinese
  card table for a Chinese card, R1301, or for any card when the reader reads Chinese, R711000), the type line,
  tag chips, keyword words, state badges and their sentences, the tuning chips and tooltips, the glossary
  entries, a Heroic Power's rolled power and a Quest's stage. The translations of those strings are
  `data/zh-CN.json` (English sentence to words; a key with `{name}` tokens also matches a finished sentence,
  as the engine's quest texts arrive); the web's UI strings stay in `apps/web/src/i18n/`.
- **What the rules box prints** is decided here too: `text` (the whole), `short` (a card's short form, issues
  #814 and #1017: `data/short-forms.json`), `summit` (the Summit's header and the next Ascent line), `questBox`
  (a Quest's one stage, R2450000) and `clamp`. Whether the text fits its box is the client's layout.
- **Locale and catalog come in as arguments.** The catalog is the engine's registered one, with the
  definitions a view carries (`PlayerView.defs`) consulted next.
- **The oracle.** `data/glossary.json`, `power-words.json` and `short-forms.json` are the tables the web
  held in `glossary.ts`, `inPlay.ts` and `shortText.ts`; the web's `rules.test.ts` still holds the glossary
  to `spec/06-keywords.md`, and `src/face/power.rs` holds the powers to the engine's own table.

## `src/deck.rs`

What a ticket, a room or a series freezes of a player's decks, and the seat a match starts from:
`DeckSlots` (R1834002), `FrozenDeck` (R642), `FrozenTrio` (R259) and `MatchSeat`. `absent_or_null`
(`src/lib.rs`) keeps a deck's absent, `null` and chosen portrait apart.

## Tests

`tests/series.rs` holds the series rules' tests, moved from `crates/server`'s API tests
unchanged, `tests/boundary.rs` the JSON boundary, and `tests/face.rs` the face model: a face as the web's
tests held it, in both locales. The `ts` feature writes the types that cross to
`apps/web/src/wire/generated/`; commit any diff.

```
cargo test -p jackioh-shared
cargo test -p jackioh-shared --features ts
cargo clippy -p jackioh-shared --all-targets -- -D warnings
```
