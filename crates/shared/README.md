# `jackioh-shared`

The logic the server and its clients share (#2956, Part B): one pure crate that every decision now made
twice, once in `crates/server` and once in `apps/web`, moves into. This file is the contract other code
relies on. The aim (the owner's, 2026-11-05) is that a Roblox version (#2898) needs as little rewriting as
possible: Roblox runs the pure crates translated from the C module (`crates/wasm/c`, #3093), so its server
scripts and client supply only I/O (timers, RemoteEvents, DataStores) and layout, and everything else
comes from this crate through that module.

It is pure (CLAUDE.md rule 4). The first thing in is the Conquest series rules (SPEC §9.5, R330 to R337),
from `crates/server` (#3098); the second is the match host's decisions (#3101), which the server's 1v1
actor and the later practice and ladder-bot hosts (#3099, #3103) run on; the third is the collection
economy (SPEC §14, #3104): the cosmetics registry, the card and cosmetic pack rolls, pity, the token bars, the
set-track curve and rewards, what a finished game pays and how a quest moves, each
`decide(records, inputs, seed, now_ms) -> outcome`.

## Host contract

Every Part B issue follows it, and review holds each one to it.

1. **One copy, three callers.** The server calls the crate natively. The browser and Roblox call it
   through a body in `jackioh-wasm-core`, bound by both `crates/wasm` and `crates/wasm/c`.
   `crates/wasm/tests/export_parity.rs` fails a body bound in only one of them (`series` is the first: a
   binding the Luau module, which leaves with #2956 part 5, does not bind is listed in its `NORMAL_ONLY`). A body is a JSON adapter
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

- `Cargo.toml` takes `serde`, `serde_json`, `indexmap`, the engine, the AI (the host machine asks it who
  owes a move), the cards (the cosmetics registry judges an offer against the shipped catalog) and, behind
  the off-by-default `ts` feature, `ts-rs`. It may also take the run, never `jackioh-luau`, and none of
  them takes it.
- `clippy.toml` is the other five pure crates' byte for byte (`crates/tools/tests/acceptance.rs` holds
  the six the same and runs clippy over a fixture that uses each entry once). CI's `cargo clippy
  --workspace --all-targets -- -D warnings` fails a use anywhere in the crate, tests included.

A transition never reads a clock: it takes `now`, and the host passes the time it read.

## `src/config.rs`

`MS_PER_SECOND`, `SERIES_WINS_NEEDED` (R330), `SERIES_MAX_GAMES` (R334) and `SERIES_PICK_SECONDS`
(R333), and the match host's: `TURN_CLOCK_SECONDS`, `PROMPT_CLOCK_SECONDS` (R79), `MULLIGAN_CLOCK_SECONDS`
(R268), `DISCONNECT_GRACE_SECONDS`, `MATCH_CEILING_MINUTES`, `MATCH_SERVER_ACTION_RETRY_MS` (R776000),
`BOT_WRITE_RETRY_MS` (R1452), `AIM_RELAY_INTERVAL_MS` (R738), `REMATCH_OFFER_TTL_MS` (R672, R2685000) and
the rematch stakes `STAKE_NORMAL` and `STAKE_DOUBLE` (R672), and the economy's (#3104): the drop tables
(`CARD_TIER_WEIGHTS`, `COSMETIC_TIER_WEIGHTS`, `TIER_WEIGHT_TOTAL`, `PACK_SLOTS`, the floors, the global
cosmetic roll), pity (`PITY_*_PACKS`), the token bars (`CARD_TOKEN_BAR_SIZE`, `COSMETIC_TOKEN_CYCLE` and the
points per rarity), the cosmetic pack and Chase Shard bar, the set track's `SET_TRACK_XP_PER_LEVEL`, and what a game pays and a quest does (#3104, part 2): the Jlore
and XP rates (`JLORE_*`, `XP_*`), the daily cap (`JLORE_GAMES_DAILY_CAP`, R1483000), the track levels
(`SET_TRACK_LEVELS`, `SET_TRACK_UNLOCK_LEVEL`, `SET_TRACK_FIRST_LEVEL`), the quest rhythm and pay (`QUEST_*`)
and `MS_PER_DAY`. The
server re-exports them from its own
`config.rs` (so `serverConfig.ts` and its code name them as before), and `jackioh-wasm-core`'s `constants`
carries them, which the browser and the C module (`crates/wasm/c`) both bind.

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

## `src/host/`: the match host

The decisions a match host makes, as one pure state machine (#3101). A host is whatever carries a match:
the server's actor (`crates/server/src/actor/match_actor.rs`), a practice session, a Roblox server script.
It owns sockets or RemoteEvents, timers, a store and the bots' moves; this module decides the rest.

```
step(host, event, now) -> (host, outputs)
```

`DuelHost` is the state: serializable JSON with the `GameState` inside it, so it is the checkpoint a host
that restarts resumes from. `DuelEvent` is what the host feeds it, `DuelOutput` what it carries out, in
order. Nothing in the machine reads a clock, draws from the OS or does I/O.

| Event | What it is |
| --- | --- |
| `arm` | the host is up (new, or made from a checkpoint): clocks brought up to date, a game already over ended, the opening driven |
| `connected`, `attached`, `hello` | a socket arrives (R744: the account has now been here); its greeting, the fresh full view (§9.5); a hello from a socket already attached |
| `left`, `disconnected` | a socket is gone (its aim and hover end); the grace begins and the opponent is told (§9.5) |
| `action`, `emote` | an action for a seat, by home seat or engine seat; an emote a card may hear (MD-D29, R1127) |
| `logged` | the log took (or refused) the row `append` asked for |
| `timerFired` | a timer the machine set came due, with its key and id |
| `aim`, `hover` | the account's aim (R738) or choice hover (R2739001), or its end |
| `decided` | a bot's move, as `decide` asked for it |
| `resultRecorded`, `rematchOffered`, `rematchSent` | the result `record` asked for is written; a person offered a rematch (a bot may meet it); a bot's offer has been made |
| `stop` | the host is going away |

| Output | What the host does |
| --- | --- |
| `send` | a message for the account that began in `home`: a seat's `view` (only that seat's `view_for` with its clock, and its own legal actions, CLAUDE.md rule 7), `clock`, `promptForYou`, `promptForOpponent`, `aim`, `choiceHover` |
| `timer` | `set` (key, id, at) or `cancel` a timer; a firing comes back as `timerFired` with the same key and id, and a stale one does nothing |
| `persistClocks`, `append` | write the clocks (§9.5), or the row of an action (row before state, §9.3) and feed `logged` |
| `answer` | what an action's sender is told (`ack`, `illegal`, `matchOver`, `panicked`, `notRecorded`) |
| `decide` | decide a move for the AI or ladder bot at a seat: one separate, deadline-bounded step, answered by `decided` |
| `record`, `voided`, `ceilingDraw` | the game's end with what the store must record (outcome, turns, which account each seat is credited to, the boards, the final hash); R679's void; R1253000's ceiling draw |
| `offerRematch`, `letGo` | a bot makes a rematch offer (R639000); the host forgets the match (R776000) |
| `greeting`, `applied`, `seatsSwapped`, `changed`, `pushed`, `refused`, `panicked`, `note` | markers the host may use: where its own frames go in a greeting, what telemetry reads, the abuse guard's refusal count, a panic to report, a log line |

What the machine decides:

- **The clocks** (`clock.rs`): the turn clock, a non-active holder's prompt clock, the mulligan clock both
  seats share, the disconnect grace, the ceiling and a question sequence's clock (R79, R265, R268, R147,
  R744, R1563006), and the retry of a clock whose action the log did not take (R776000). `expiry`: a
  clock that runs out is a server action under a `srv-` nonce (R270), so `(seed, log)` still folds; a
  mulligan expiry times out each seat still owing, in seat order.
- **The flow of a change**: the order every view, prompt and clock goes out in, the `prompt` frames a
  change in who owes an answer sends (§10.6, R266), a Glitch's swap and the grace that moves with it
  (R677), the end of the game (R679, R1253000) and what a ranked game with a bot does after it (R639000).
- **Who is here** (R744, R147, R1452): a bot's seat is never away; an account that has never attached is
  away from the first attach; a stored grace is kept, never extended.
- **The relays** (`relay.rs`): an aim reaches the opponent at most once per `AIM_RELAY_INTERVAL_MS`,
  coalesced, and only when every end of it is something the receiver may see (R738, R97, R177); a choice
  hover the same (R2739001).
- **A seat the host plays** (`bot.rs`): when the AI or a bot owes a move, whether it lets its clock run out
  (R639000), how long its aim shows, what falls back when its move is refused (R188, R1452).

The rematch book (`rematch.rs`) is its own state, `RematchBook`, because an offer outlives the host that
played the match: stakes are a rematch or a double-or-nothing, only a ranked match spawns the second, an
offer counts only inside the finished match's window (R2685000), two equal offers make one game, and a
failed create gives the claim back (R672). `next_seats` is the seats of the rematch (R1989000: they swap,
with each profile's deck, portrait, slots and lean; an All Random one deals fresh). The server keeps one
book per `App`; a host of one process may keep it beside its `DuelHost`.

Notes for a host:

- **One write at a time.** `append` is followed by `logged` before the next action; the machine answers
  `notRecorded` to an action fed while a row is awaited.
- **A host with a pinned catalog runs `step` inside it** (R388, R767000): the machine calls the engine as
  the engine is called.
- **Events that stand for one fact** (`connected`, `left`, `aim`, `hover`) may be fed the moment the host
  learns it; the others are the host's turns, one at a time, in order. A host with a queue (the server's
  actor) feeds the rest from it; one without sends `connected` and `attached` together.
- **`applied.before`** is the state the action was made on, for a host in the same process (telemetry); it
  is not serialized.

### The boundary

`jackioh-wasm-core`'s `host(call)` takes one `HostCall` as JSON and answers one `HostAnswer`:

```json
{ "call": "stepDuel", "host": { "...": "the DuelHost" }, "event": { "event": "timerFired", "key": { "kind": "turn" }, "id": 4 }, "now": 1700000000000 }
{ "answer": "stepped", "host": { "...": "the next DuelHost" }, "outputs": [ { "output": "send", "home": "p1", "message": { "message": "clock", "now": 1700000000000, "clocks": { "...": "" } } } ] }
```

The calls are `newDuel`, `stepDuel`, `duelView`, `duelPresence`, `checkOffer`, `rematchWindow`, `offer`,
`seen`, `release` and `sweep`; `boundary::apply` is the one function behind it, and `tests/boundary.rs`
holds it to the functions it names. Not in the Luau module (`export_parity.rs`'s `NORMAL_ONLY`).

Duos co-op (`crates/server/src/duos/actor.rs`) runs on the same event and output shapes in a second
part of #3101; until then its clocks and grace are the server's.

## `src/cosmetics.rs` and `cosmetics.json`: the registry

Every cosmetic a player can pick (SPEC §14.2, R1295000), compiled in (`include_str!("../cosmetics.json")`):
`Cosmetic`, its kinds and rarities, the registry's check (`problems`, `parse`, `registry`, `shipped`), the
offers a version or augment makes (`offered`, R2725001), ownership (`owns`, `row_owns`, `check_pick`) and the
lists a pack may drop (`pack_cosmetics`, `global_cosmetics`, `cosmetic_pack_cosmetics`). A cosmetic's offer is
judged against the shipped catalog (`shipped_defs`, `jackioh_cards::catalog_json`). The rows an account owns
(`CardCosmetic`, `OwnedCosmetics`, `FullArtPick`) are here because the registry's rules read them. The server's
`cosmetics` module re-exports all of it, and `crates/server/tests/export_cosmetics.rs` writes the registry into
`apps/web/src/wire/generated/cosmetics.ts`.

## `src/economy/`: the collection economy

What a pack opening grants, how pity and the token bars move, and what a set-track level pays (SPEC §14.3 to
§14.7), over plain records. The server's store code reads the records, calls the decision and writes the
outcome (`crates/server/src/economy/grants.rs`); a Roblox script would write the same outcome to DataStores.

| Decision | Records in | Outcome |
| --- | --- | --- |
| `open_card_pack(&PackRecords, &PackInputs, seed, now_ms)` | the packs of the kind held, the account's copies of each card, its cosmetics and its counters | the slots, the pity after, what both bars did, the `grants` in the order to write them and the `log` every row's `ref` carries (R1301003, R1339002); `EconomyRefusal` for no pack held or a pool of no card |
| `open_cosmetic_pack(&PackRecords, &CosmeticPackInputs, seed, now_ms)` | the same | the slots, the Chase Shard bar before and after (R1306002), both bars, the `grants` and the `log` (R1306003) |
| `bars::fill_bar(kind, &AccountCounters)` | a dev account's counters | the writes of a fill (R1310003), all `Cause::Dev` |
| `track_rewards::reward_for_level(level)` | a level | what reaching it pays (R1302) |
| `rewards::pay_game(&GameRecords, &GameSummary, seed)` | what the account was paid today (R1483000), its tracks and its quests | the game's Jlore held to the daily cap, its XP on every active track and the levels it reaches, the quests due as it ends, each quest's progress and what a completion pays: the `grants` in the order to write them and the `GameEconomy` a client animates (R1302006, R1304001); a bot's side pays nothing (R1454) |
| `rewards::pay_xp(&[SetTrack], xp, cause, reference)`, `starting_rewards(&[SetTrack])`, `complete_quest(&QuestBook, &[SetTrack], id, cause, reference, at)` | tracks (and quests) | the grants of an XP payout (the unlock of one older track, R1302003), of a new account's first level (R1310000), of a dev completion of a held quest |
| `quests::top_up_book(&QuestBook, seed, now)`, `replace_quest(&QuestBook, removed_today, id, seed, now)`, `give_quest(&QuestBook, seed, now)` | the account's quests, the day's replaces | the quests that fell due (R1304000), a held quest swapped for another (R2210000), a dev give; each arrival's draw is the engine's `Rng` over the host's seed and its `ref` logs `seed:<seed>:<cursor>` |

- **The inputs are what the roll draws from, not what the store holds:** the pool in catalog order
  (`roll::pack_pool` over a `CardDefs`), the cosmetics registry and the catalog version, which the log keeps so
  an opening replays however the live catalog or registry has moved (R1301003). A call across the boundary that
  leaves them out gets the compiled-in ones.
- **Every draw is the roll's, in the order its docs state (R1339002):** `Rng::new(seed, 0)`, the slot tiers first
  (pity adds none), then slot by slot the card's, the global roll's, the cosmetic tier's and the pick's. A replayed
  opening grants the same.
- **`LedgerGrant`s are the writes:** an inventory change (the pack spent first, a token paid), the cards unlocked
  (the copies each adds and the quantity then held), a cosmetic now owned, a counter's change (never a zero one),
  Jlore to the wallet, XP and levels to a set track, a quest's change (an arrival is dated when it fell due) and
  the date the next quest falls due. Each carries its `Cause` (`PackOpen`, `TokenBar`, `Dev`, `Game`, `Quest`,
  `Track`), which the store's ledger reason names, and a `reference` when its row's `ref` is not the outcome's. A
  store refuses a grant its holding cannot take and writes nothing more.
- **A payout reads the tracks it moves.** `pay_game` applies each grant to its own copy of the tracks as the store
  will, so a quest's XP lands on the tracks the game's XP already moved, and the answer's `TrackGain`s are read off
  the tracks before and after the whole payout. `quests.json` is the quest pool, compiled in like `cosmetics.json`.
- **`roll`, `cosmetic_pack`, `tiers`, `math`** are the rolls themselves (tiered rolls, the dupe chain, pity and the
  level curve), moved from `crates/server/src/economy/` with their tests; `bars` is the token bars (R1305000,
  R1953000, R2728000).

### The boundary

`jackioh-wasm-core`'s `economy(call)` takes one `EconomyCall` as JSON and answers one `EconomyAnswer`:

```json
{ "call": "openCardPack", "records": { "held": 1, "owned": {}, "cosmetics": { "cards": [], "account": [] },
  "counters": { "pity": { "epic": 0, "legendary": 0, "mythic": 0 }, "cardBar": 0, "cosmeticBar": 0,
  "cosmeticCycle": 0, "chaseShard": 0 } }, "pack": "global", "seed": "…", "now": 1700000000000 }
{ "opened": { "pack": "global", "slots": [ … ], "pity": { … }, "bars": { … }, "grants": [ … ], "log": { … }, "at": 1700000000000 } }
{ "refused": { "reason": "no_pack_held", "message": "no unopened pack of that kind" } }
```

The calls are `openCardPack`, `openCosmeticPack`, `fillBar`, `levelReward`, `payGame`, `payXp`,
`startingRewards`, `topUpQuests`, `replaceQuest`, `giveQuest` and `completeQuest`; the answers are `opened`,
`cosmeticOpened`, `filled`, `reward`, `paid`, `granted` (the writes, in order), `toppedUp` and `refused`. `boundary::apply` is the one function behind it. Not in the
Luau module (`export_parity.rs`'s `NORMAL_ONLY`).

## `src/deck.rs`

What a ticket, a room or a series freezes of a player's decks, and the seat a match starts from:
`DeckSlots` (R1834002), `FrozenDeck` (R642), `FrozenTrio` (R259) and `MatchSeat`. `absent_or_null`
(`src/lib.rs`) keeps a deck's absent, `null` and chosen portrait apart.

## Tests

`tests/series.rs` holds the series rules' tests, moved from `crates/server`'s API tests
unchanged, and `tests/boundary.rs` the JSON boundary. The economy's rolls carry their tests in their modules
(moved from `crates/server/src/economy/` and `crates/server/src/cosmetics.rs`), and `tests/economy.rs` holds the
decisions over records and their boundary. The host machine's: `tests/clock.rs` (the clock's
tests, moved from the server's actor tests on a simulated host), `tests/duel.rs` (the machine played on
the real catalog), `tests/rematch.rs` (the book) and `tests/host_boundary.rs`. The `ts` feature writes the types that cross to
`apps/web/src/wire/generated/`; commit any diff.

```
cargo test -p jackioh-shared
cargo test -p jackioh-shared --features ts
cargo clippy -p jackioh-shared --all-targets -- -D warnings
```
