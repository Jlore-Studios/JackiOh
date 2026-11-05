# `jackioh-shared`

The logic the server and its clients share (#2956, Part B): one pure crate that every decision now made
twice, once in `crates/server` and once in `apps/web`, moves into. This file is the contract other code
relies on. The aim (the owner's, 2026-11-05) is that a Roblox version (#2898) needs as little rewriting as
possible: Roblox runs the pure crates translated from the C module (`crates/wasm/c`, #3093), so its server
scripts and client supply only I/O (timers, RemoteEvents, DataStores) and layout, and everything else
comes from this crate through that module.

It is pure (CLAUDE.md rule 4). The first thing in is the Conquest series rules (SPEC §9.5, R330 to R337),
from `crates/server` (#3098); next, the loadout visibility rule (SPEC §10.8, R1834003, #3102).

## Host contract

Every Part B issue follows it, and review holds each one to it.

1. **One copy, three callers.** The server calls the crate natively. The browser and Roblox call it
   through a body in `jackioh-wasm-core`, bound by both `crates/wasm` and `crates/wasm/c`.
   `crates/wasm/tests/export_parity.rs` fails a body bound in only one of them (`series` is the first, `loadout`
   the next: a binding the Luau module, which leaves with #2956 part 5, does not bind is listed in its
   `NORMAL_ONLY`). A body is a JSON adapter
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
6. **Data is compiled in** (`include_str!`, rule 4): quest pools, registries and translations. The
   module carries it, and no host loads files.
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

`MS_PER_SECOND`, `SERIES_WINS_NEEDED` (R330), `SERIES_MAX_GAMES` (R334) and `SERIES_PICK_SECONDS`
(R333). The server re-exports them from its own `config.rs` (so `serverConfig.ts` and its code name
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

## `src/loadout.rs`: the loadout visibility rule

What a seat's cosmetic loadout lets a receiver see of a player's deck, without a decklist leaking (SPEC §10.8;
R1834003, R3013002). The server freezes a loadout when a match starts (`crates/server/src/loadout.rs`, which
keeps the registry, the freezing and the store reads, R1834002) and decides with these functions which of a
seat's cards' own picks and stamps a receiver is sent, and when.

| Function | Answers | Rulings |
| --- | --- | --- |
| `dealt_cards(&state)` | every card the decks dealt, by instance id: its seat and the card it was dealt as; a card minted after the deal (R943) is left out | R1834003 |
| `shown_defs(&view, &dealt)` | the `(seat, card)` pairs a view shows: each card in it dealt from that seat's deck and still shown as dealt, wherever it is now; a face-down card, the other hand, a library, the events and the prompts show none | R1834003 |
| `cards_due(&shown, &loadouts, &mut sent)` | the `SeatCards` entries newly due a receiver (a card's picks, and its stamp), by seat, the pairs then marked in `sent`; a pair whose card wears nothing of its own is marked and sends nothing | R1834003, R3013002 |

- **The types are what the store and the frames carry.** `Loadout`, `SeatLoadout`, `SeatCards`, `CardPicks`,
  `FullArtPick` and `CardStamp` are what `matches.loadouts` stores and the `loadouts` and `cardLoadouts` frames send,
  so changing a field, a rename or a `skip_serializing_if` is a store and wire change. `CardStamp.stamp` keeps the
  `i64` of its stored shape (README's integers). What needs the cosmetics registry (`defaults_of`, a kind's slot) stays
  in the server, as extension traits of these types.
- **The caller owns the state.** The deal is read once when a match begins; a receiver's `sent` set lives with
  the receiver, and `cards_due` marks it. Over the boundary it is an argument and part of the answer.
- **Only the deal counts.** A card counts for the deck that dealt it, not for the seat that owns it now, and only
  while it shows as what was dealt (R1834003's rejected alternative): `shown_defs` and `dealt_cards` are the two
  halves of that, so a host calls both from here and restates neither.

### The boundary

`jackioh-wasm-core`'s `loadout(call)` takes one `LoadoutCall` as JSON and answers one `LoadoutAnswer`:

```json
{ "call": "dealtCards", "state": { "...": "the game's state at its start" } }
{ "call": "shownDefs", "view": { "...": "a PlayerView" }, "dealt": { "<instance id>": ["p1", "core-001"] } }
{ "call": "cardsDue", "shown": [["p1", "core-001"]], "loadouts": [{ "...": "p1's Loadout" }, { "...": "p2's" }], "sent": [] }
{ "dealt": { "<instance id>": ["p1", "core-001"] } }
{ "shown": [["p1", "core-001"]] }
{ "due": { "cards": [{ "seat": "p1", "cards": [], "stamps": [] }], "sent": [["p1", "core-001"]] } }
```

`apply` is the one function behind it. A call that does not parse is refused by the body as an error, and a
rules answer is never one. `tests/loadout.rs` holds the calls to the functions they name.

## `src/deck.rs`

What a ticket, a room or a series freezes of a player's decks, and the seat a match starts from:
`DeckSlots` (R1834002), `FrozenDeck` (R642), `FrozenTrio` (R259) and `MatchSeat`. `absent_or_null`
(`src/lib.rs`) keeps a deck's absent, `null` and chosen portrait apart.

## Tests

`tests/series.rs` holds the series rules' tests, moved from `crates/server`'s API tests
unchanged, and `tests/boundary.rs` the JSON boundary; `tests/loadout.rs` holds the loadout rule's, moved from
`crates/server/src/loadout.rs`, its JSON calls among them, over a game dealt from the shipped cards (the
`jackioh-cards` and `jackioh-ai` dev-dependencies). The `ts` feature writes the types that cross to
`apps/web/src/wire/generated/`; commit any diff.

```
cargo test -p jackioh-shared
cargo test -p jackioh-shared --features ts
cargo clippy -p jackioh-shared --all-targets -- -D warnings
```
