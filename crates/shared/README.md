# `jackioh-shared`

The logic the server and its clients share (#2956, Part B): one pure crate that every decision now made
twice, once in `crates/server` and once in `apps/web`, moves into. This file is the contract other code
relies on. The aim (the owner's, 2026-11-05) is that a Roblox version (#2898) needs as little rewriting as
possible: Roblox runs the pure crates translated from the C module (`crates/wasm/c`, #3093), so its server
scripts and client supply only I/O (timers, RemoteEvents, DataStores) and layout, and everything else
comes from this crate through that module.

It is pure (CLAUDE.md rule 4). The first thing in is the Conquest series rules (SPEC §9.5, R330 to R337),
from `crates/server` (#3098); the second is the match host's decisions (#3101), which the server's 1v1
and Duos actors and the later practice and ladder-bot hosts (#3099, #3103) run on; the third, the loadout visibility rule
(SPEC §10.8, R1834003, #3102); the fourth is the ranked ladder (#3105): Glicko-2, the visible ladder, the
seasons and one rated game decided as data; the fifth, the ladder bots (#3103): a bot is a seat kind of that
machine, and the queue's and the pool's decisions around it are here too.

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

- `Cargo.toml` takes `serde`, `indexmap`, the engine and the AI (the host machine asks it who owes a
  move), `serde_json` (a saved deck's cosmetics, which only the page reads), the cards (the practice
  session names a kept game's catalog as they do, and the machine's tests play on the real catalog), the
  run (a dungeon run's fight report, R2706000) and, behind the off-by-default `ts` feature, `ts-rs`. It
  never takes `jackioh-luau`, and none of them takes it.
- `clippy.toml` is the other five pure crates' byte for byte (`crates/tools/tests/acceptance.rs` holds
  the six the same and runs clippy over a fixture that uses each entry once). CI's `cargo clippy
  --workspace --all-targets -- -D warnings` fails a use anywhere in the crate, tests included.

A transition never reads a clock: it takes `now`, and the host passes the time it read.

## `src/config.rs`

`MS_PER_SECOND`, `SERIES_WINS_NEEDED` (R330), `SERIES_MAX_GAMES` (R334) and `SERIES_PICK_SECONDS`
(R333), and the match host's: `TURN_CLOCK_SECONDS`, `PROMPT_CLOCK_SECONDS` (R79), `MULLIGAN_CLOCK_SECONDS`
(R268), `DISCONNECT_GRACE_SECONDS`, `MATCH_CEILING_MINUTES`, `MATCH_SERVER_ACTION_RETRY_MS` (R776000),
`BOT_WRITE_RETRY_MS` (R1452), `AIM_RELAY_INTERVAL_MS` (R738), `REMATCH_OFFER_TTL_MS` (R672, R2685000) and
the rematch stakes `STAKE_NORMAL` and `STAKE_DOUBLE` (R672), and the ranked ladder's (#3105): the hidden
rating's (`RATING_START`, `RATING_DEVIATION_START`, `RATING_VOLATILITY_START`, `GLICKO_TAU`, `GLICKO_SCALE`,
`GLICKO_CONVERGENCE`, `GLICKO_MAX_ITERATIONS`; R603), the visible ladder's (`RANK_TIER_PERCENTS` and the
`RANK_*` pips, divisions and placements, `JLORIOUS_SIZE`; R605 to R608), the seasons' (`SEASON_RESET_STRENGTH`,
`SEASON_RESET_DEVIATION_BOOST`; R609), `BOT_PROMOTION_DEVIATION_BOOST` (R1560000) and the leaderboard key's
(`SORT_KEY_GRAPE_BASE`, `SORT_KEY_JLORIOUS_BASE`, `SORT_KEY_RATING_BITS`; `RANK_STREAK_FLAME_LENGTH` is only the
client's and stays in the server), and the ladder bots': their strengths and seed
ratings (`BOT_STRENGTHS`, `SKILL_ELO_TABLE`, R747000, R748000), think times and habits (`THINK_TIME_FITS`,
`BOT_CONCEDE_FIT`, `BOT_REMATCH_FIT` and the other fits of R639000, R1442), emote personas (`AI_PERSONAS`,
`AI_EMOTE`, R645), pool, schedule and grind (`BOT_POOL_*`, `BOT_GRIND_*`, R1454, R1555000), fallback and
pick numbers (`BOT_FALLBACK_*`, `BOT_PICK_*`, R1453, R641000) and username shapes (`BOT_NAME_*`, R1454).
The practice session's are `PRACTICE_AI_CLOCK_MS` (R2575000), `PRACTICE_LETHAL_HINT_CLOCK_MS` (R2102000) and
`PRACTICE_REPLAYS_KEPT` (R768). The server re-exports the others from its own `config.rs` (so `serverConfig.ts` and its code name them as
before), and `jackioh-wasm-core`'s `constants` carries them, which the browser and the C module
(`crates/wasm/c`) both bind. `bot-calibrate --write` and `timing-fit` rewrite and print items of this file.

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
| `decided` | a bot's move (a `BotChoice`), as `decide` asked for it |
| `emotesMuted` | the two accounts block each other: no persona answers a person's emote from now on (R1351000) |
| `resultRecorded`, `rematchOffered`, `rematchSent` | the result `record` asked for is written; a person offered a rematch (a bot may meet it); a bot's offer has been made |
| `stop` | the host is going away |

| Output | What the host does |
| --- | --- |
| `send` | a message for the account that began in `home`: a seat's `view` (only that seat's `view_for` with its clock, and its own legal actions, CLAUDE.md rule 7), `clock`, `promptForYou`, `promptForOpponent`, `aim`, `choiceHover` |
| `timer` | `set` (key, id, at) or `cancel` a timer; a firing comes back as `timerFired` with the same key and id, and a stale one does nothing |
| `persistClocks`, `append` | write the clocks (§9.5), or the row of an action (row before state, §9.3) and feed `logged` |
| `answer` | what an action's sender is told (`ack`, `illegal`, `matchOver`, `panicked`, `notRecorded`) |
| `decide` | decide a move for the AI or ladder bot at a seat: one separate, deadline-bounded step (`DuelHost::job` makes what to run, `bots::decide` is what it runs), answered by `decided` |
| `botEmote` | a bot's persona sends an emote: the host sends it as a person's (hand, rate gate, relay, telemetry) and feeds `emote` if it let it through (R645, R1127) |
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
- **A ladder bot** (`bot.rs`, `crates/shared/src/bots/`, #3103): a `BotSeat` in the setup (its strength, speed,
  persona and whether it is hidden). The machine asks the host for its move (`decide`), and the host answers
  with the AI's find (`decided`); the machine does the rest as timers and actions: the think time (R1452,
  R639000), the aim shown a beat before a targeted move and taken down after (R738), the persona's emotes
  and its replies (R645), the concession, draw offer and draw answer (R639000, R2660000, R2660001), the
  turn clock it lets run out now and then, what falls back when its move is refused (R188, R1452) and its
  stay for a rematch (R639000). **Hidden** (`BotSeat.hidden`, R3103000) is the one switch for all that exists
  only so that a bot passes for a person: the server turns it on; a host whose players see a bot labelled
  (a Roblox server) leaves it off, and the bot then plays as the practice AI does, at once and
  without chatter, concessions or rematches.

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

The calls are `newDuel`, `stepDuel`, `duelView`, `duelPresence`, `newDuos`, `stepDuos`, `duosSeatView`,
`checkOffer`, `rematchWindow`, `offer`, `seen`, `release`, `sweep` and the `again*` ones; `boundary::apply` is the one function behind it, and `tests/boundary.rs`
holds it to the functions it names. Not in the Luau module (`export_parity.rs`'s `NORMAL_ONLY`). The practice session's calls ride the same
export (below).

The AI's search is a body of its own, `bot_decide(call, deadline_ms)`: a `DecideCall` (the `DuelHost` and the
`DecideRequest` a `decide` output made) in, the `BotChoice` to feed back as `decided` out. A host with a clock
(the browser) passes the `deadline_ms` the request's `deadlineMs` makes of its clock, and the search ends
there with its best line; the C module has no clock, so the search ends at the strength's budget.

### Duos co-op (`src/host/duos.rs`, `src/host/again.rs`)

The co-op host (`crates/server/src/duos/actor.rs`, fullsend SPEC S4 to S6, B35 to B40) is the twin of the
1v1 one under the same shapes: `step_duos(host, event, now) -> (host, outputs)` over a serializable
`DuosHost` holding the `DuosState` of two duels and four seats (a1 and a2 are people, b1 and b2 the AI's).
`DuosHostEvent` (`arm`, `connected` / `attached` / `hello` / `left` / `disconnected`, `action`, `logged`,
`timerFired`, `decided`, `rowChecked`, `stop`) and `DuosHostOutput` (`send`, `timer`, `append`, `answer`,
`greeting`, `refused`, `panicked`, `pushed`, `decide`, `record`, `ceilingDraw`, `checkRow`, `letGo`, `note`)
read like the 1v1 machine's, and its timers are the `duos*` `TimerKey`s. What it decides:

- **The clocks** (S6, B2, B38): the one mulligan clock over the four mulligans (§13.2, R268), the round
  clock of a round with a person in it, a prompt clock for a prompt a person holds outside its own round,
  each person's disconnect grace (R744) and the hard ceiling (R1047000) with its retry (R776000). An expiry
  hands seats to the AI and logs nothing itself: a lapsed mulligan keeps the hand, a lapsed round answers
  the prompts and ends the turn of each round seat, a lapsed prompt answers that prompt alone.
- **Who plays a seat** (B36 to B38): b1 and b2 always; a person's seat past its grace (until the person is
  back, no loss by disconnect) or in the mode an expiry forced. The pause before every AI action
  (`DUOS_AI_STEP_DELAY_MS`), whether the move needs the AI's search at all (a mulligan kept whole or a
  turn simply ended does not), the move it falls back on and R188's forbidden ones, a refused move
  deciding again, an unwritten one retried after `BOT_WRITE_RETRY_MS` (or the match let go if its row is
  gone: `checkRow`, `rowChecked`).
- **The flow of a change** (B35, B40): the order of every view and clock, the result recorded once with every
  clock gone, a server action's panic ending the match as a ceiling draw (R1253000).

The host keeps its sockets, its store, its abuse guards, the pings (B39), its watchers (R2881000) and the
search itself (`decide`, answered by `decided`: one separate, deadline-bounded step).

`AgainBook` is the co-op Play again's rendezvous state (R2092000, R1398000): an offer counts only inside the
finished match's window, the offer that completes the pair makes the game, one that lands mid-create waits
on it (the claim is a number, and the host keeps the wake-up under it), and a failed create gives the claim
back. The boundary's `HostCall` carries the Duos host (`newDuos`, `stepDuos`, `duosSeatView`) and the book
(`againWindow`, `againClaim`, `againSeen`, `againRelease`, `againMade`).

### The practice session (#3099)

`host/practice/` is practice as a mode of the host machine: a game against the AI seat, with practice's own
setup and rules (SPEC §9.9, §9.10), in place of the web's practice core and the server's copy of it. The
web's practice worker (`apps/web/src/practice/driver.ts`) runs it through `crates/wasm`, the server checks a
claim by stepping it natively over the claimed game (`check_claim`), and a Roblox server runs it through
`crates/wasm/c`: one body for all three. It rides `host(call)` as `{ "call": "practice", "op": … }` (a
`PracticeCall`, answered as `{ "answer": "practice", "reply": … }`), so no export is added.

```
PracticeHost::start(config, catalog, kept, now) -> (host, outputs)       PracticeHost::resume(config, save, build, kept, now)
step(host, event, now) -> (host, outputs)
```

| Event | What it is |
| --- | --- |
| `act` | the human's action, minted `h<n>` and sent from the seat the human plays now (R677) |
| `aiTurn` | the host's pacing says the AI's turn: the machine answers `decide` if the AI owes a move, else just the snapshot |
| `decided` | the AI's move as the host decided it, with the cursor of the AI's stream (`"{seed}:ai"`); a move R188 forbids, or the engine refuses, falls back on `endTurn` first |
| `fold` | an action of a game already played, under its own nonce and seat: a save resumed (R668), a claim checked (R2560000) |
| `replays`, `claim`, `runReport`, `debug` | kept games are listed and opened only once the game is over (R768); the finished game's claim (R1303000); a run fight's report (DR14); the raw game, dev builds only |

| Output | What the host does |
| --- | --- |
| `began` | the game is on: the seat the AI plays |
| `snapshot` | what the page is told (CLAUDE.md rule 7): `view_for` and the legal actions of the human's seat, whether the AI owes a move, the engine's last refusal, the last board of a finished game (R508), the kept game's number (R768000), a lesson's combat forecast (R3100000) |
| `lethal` | R2102000: ask the lethal check for that seat under `PRACTICE_LETHAL_HINT_CLOCK_MS`; when it says yes, the snapshot says `lethal`. A host without the check leaves it out |
| `decide` | the AI owes a move: search for it under `PRACTICE_AI_CLOCK_MS` (the node budget stays the real bound, R2560000) on `ai.seat`, from `rng_seed` at `rng_cursor`, and feed `decided`: one separate, deadline-bounded step |
| `save` | R668: write the save of a free game in progress, or clear it (a lesson, a finished game); a run fight leaves it as it was |
| `keep` | R768: append a finished free game to the kept ones, keep the newest `keep` of them (`PRACTICE_REPLAYS_KEPT`) |
| `finished`, `voided` | the game's end with what a store must record: the human's result, turns, the human's actions (for the economy's XP, R1302004), the AI's moves, the board the human takes (R508), the final hash; or R679's void, which records nothing |
| `claim`, `runReport`, `debug`, `failed` | the answers to the events of those names, and the reason an event was refused (nothing changed) |

What the machine decides: how the game is dealt (`deal.rs`: the human's deck, a list or the random deck dealt
on `"{seed}:human-deck"` leaning as asked and counted dealt, R433, R1373, R924000; the AI's on
`"{seed}:ai-deck"`; the tier's handicap on the AI's seat alone, R180; a lesson's decks under
`TUTORIAL_HUMAN` and `AI_TUTORIAL`, R290, R1705005; a run fight's setup, R2706000; the human's last board,
R508; a sandbox, R923000), the nonces (`h<n>`, `a<n>`, each counting only accepted actions, so a refused
action spends nothing and the log folds exactly: R187, R433, R508), the seat each action is sent from, what
the AI may never take (R188, R923000), the snapshot, the save and its resume, the kept game and its number,
the game's end and the claim. What stays the host's: the store the saves and kept games go to, the AI's
search and the lethal check under their clocks (the pure crates read none), the pacing of the AI's turn, the
public card data, and the web's own content a config names, which the host puts in the config before the
machine reads it: a preset's cards (`PracticeDeckChoice::Preset { cards }`) and a lesson's decks
(`PracticeConfig::lesson_deal`).

`replays.rs` holds the kept games, which are stateless calls over what the host read from its store
(`list_replays`, `page_replay`): which fold on this build (R767000), the `ReplayOpen` a first page answers with
(the host keeps it for the next page), and the seat the human played at each step (R677). `claim.rs` is the
server's check: `check_claim(config, catalog, claimed, log)` starts the machine on the setup its seed was
issued for, feeds each logged action as `fold`, and answers `ClaimRefusal` (`DoesNotFold`, `NotFinished`,
`Voided`, `ResultMismatch`) or what the game recorded; it re-runs no AI move (R2560000). The server maps its
stored `PracticeSetup` to a `PracticeConfig` (`economy::practice::config_of`) and keeps its refusals
as its store counts them.

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

## `src/ranked/`: the ranked ladder

The ranked ladder's rules (SPEC §9.12, R603 to R612), from `crates/server/src/ranked/` and the deciding half of
`crates/server/src/api/ranked.rs` (#3105). The server keeps the store, the leaderboards' queries and the timers; it
reads the standings, calls these natively and writes what they return. A Roblox host keeps the same records in an
OrderedDataStore and a MemoryStore and calls the same code through the `ranked` body.

| Module | Holds | Rulings |
| --- | --- | --- |
| `glicko2` | the hidden rating: `glicko2_period`, `rate_game`, `rate_game_scored`, `widen_for_promotions`, Glickman's steps in his order | R603, R1843004, R1560000 |
| `ladder` | the visible ladder: `place_of`, `percentile_of`, `target_ladder`, `pip_delta`, `apply_ranked_game`, `jlorious_order`, `visible_rank`, `peak_badge` and the shapes `SeasonRank`, `Standing`, `VisibleRank`, `PeakBadge`, `Ladder` | R605 to R608, R2778000 |
| `season` | `season_id_of`, `plan_season` (which season a build plays in, from the seasons that exist and the time), `soft_reset` | R609 |
| `game` | `decide(&RateGameInput) -> RatedDecision`: one rated game, as data | R603 to R611, R640000, R672, R1560000, R1843004, R3074001 |
| `board` | `SeasonStanding`, `RatingRecord`, `sort_key`: the leaderboard's integer | R608, R612, R1436, R3074001 |

- **`decide`** takes `ratings, result, season, now_ms` as one `RateGameInput`: the ladder's standings (every
  season row with its rating), the two sides (their rating going in, a bot's last-rated AI generation), the winner and the
  reason (R1843004), the stake (R672), the season, the ladder, `strict` (the server's `DEV_STRICT`, R3074001) and `now_ms`.
  It returns a `RatedDecision`: each side's rating and visible rank before and after, the season rows to write, the
  bystanders' Jlorious peaks, the people who may be owed the Jlorious card back, and the rating records. It reads no
  store, clock or environment, and its floating point is the server's own to the bit (same functions, same order of
  operations): ratings are stored.
- **The season's dates come in.** `plan_season(seasons, patch_version, now_ms)` names the season and when it starts;
  the first season of all resets nothing, every later one soft-resets each ladder's rated players (`soft_reset`) in
  the transaction that creates it. Seasons are the only thing that ages a rating: there is no separate decay, only
  the deviation widening R609 adds at a reset and R1560000 adds for an AI promotion.
- **The sort key** (`board.rs`) is one integer on each rating record that a store sorts as it is. It is `0` for a
  player not on the board (unplaced, or a hand-set rating on a strict server), `SORT_KEY_GRAPE_BASE` plus the ladder
  position for a Grape player (it rises with the position, so across tiers), and `SORT_KEY_JLORIOUS_BASE` plus the
  rating in fixed point (`SORT_KEY_RATING_BITS` fractional bits) for a Jlorious player: so descending order is the
  public leaderboard's, Jlorious by rating and then Mythic Grape down to Rotten by position, equal keys by profile id
  (R1436). It stays under `2^51`, so a Luau number holds it exactly. The two orders part only between two Jlorious
  ratings closer than `2^-36` (about `1.5e-11` of a point), which the key ties and the board tells apart.
  `tests/board.rs` holds the key's order to the leaderboard's on ladders of hundreds, and the server's
  `tests/api/ranked.rs` holds it to its real `GET /api/leaderboard`. Whether a record is Jlorious depends on the others'
  ratings, so a game that brings a player into Jlorious or pushes one out also returns that player's record, with the
  key that moved: `RatedDecision::records` is what to write to the board, and any record it leaves out keeps its key.
- **The server's types.** `Season`, `SeasonStanding`, `RatedReason`, `RankedSideInput`, `PeakWrite` and the ladder's
  shapes are stored or sent (`seasons`, `season_ranks`, `rated_games`, the `/api/ranked` and `/api/leaderboard` bodies),
  so changing a field, a rename or a `skip_serializing_if` is a store or wire change. `crates/server/src/lib.rs`
  re-exports the module as `ranked`, so `crate::ranked::ladder::…` names these still.

### The boundary

`jackioh-wasm-core`'s `ranked(call)` takes one `RankedCall` as JSON and answers one `RankedAnswer`:

```json
{ "call": "rateGame", "game": { "standings": [], "sides": [ … ], "winnerSide": 0, "reason": "hero-death", "season": { … }, "ladder": "constructed", "nowMs": 1700000000000 } }
{ "call": "planSeason", "patchVersion": "v0.3.1", "seasons": [ … ], "nowMs": 1700000000000 }
{ "game": { "sides": [ … ], "ranks": [ … ], "peaks": [], "jlorious": [], "records": [ … ] } }
{ "season": { "plan": "new", "season": { … }, "first": false } }
```

The calls are `rateGame`, `planSeason`, `softReset`, `visibleRank`, `peakBadge` and `jloriousOrder`; the answers are `game`,
`season`, `reset`, `rank`, `badge` and `jlorious`. A call that does not parse is refused by the body as an error.
`apply` is the one function behind it, and `tests/ranked_game.rs` holds it to the functions it names.

## `src/bots/`: the ladder bots

What a ladder bot decides, over plain records the host read from its store (#3103). At its seat the bot is
the machine's (above); these are the modules the machine and the queue are made of.

| Module | Holds | Rulings |
| --- | --- | --- |
| `decide` | the move the AI finds at the bot's strength (`decide_styled` after a skill roll), the concession and draw habits of a hidden bot, the aim of a targeted move | R1452, R639000, R747000, R2214000, R2660000, R2660001, R738 |
| `think` | the think time a person-paced bot waits (R1442's fits and hand-set priors), and the cap on a decision | R1452, R639000 |
| `signals` | a bot's habits at the end of a game: concede, offer and answer draws, idle a turn, linger for a rematch | R639000, R2660000, R2660001 |
| `persona` | its emotes: rolls, caps and the rate limit, as data the machine keeps (`EmotePersona` takes its draws as an argument) | R645, R1344 |
| `aim` | the arrow a person's client would show | R738 |
| `pick` | its sealed picks in a Conquest series | R641000 |
| `fallback` | the empty-queue pairing: a ticket's fallback time, which bot it pairs with, two bots' kickoff | R1453, R641000, R1555000 |
| `pool` | the pool's size, who retires, returns and grinds, a new bot, its decks and trio | R1454, R750000, R1555000 |
| `names` | a new bot's username shapes and word lists; what a name may be is the host's (`draw_username`'s `accepts`) | R1454 |

- **Records, not rows.** `BotRecord` (a bot), `TicketRecord` and `RatedGameRecord` are what the pairing and the pool
  read; the server implements them for its store rows, and `BotFacts`, `TicketFacts` and `GameFacts` are
  the same as plain JSON for a host with no row type of its own (a Roblox script, the boundary).
- **Hidden is the one switch** (`BotSeat.hidden`). The fallback time (R1453: a ticket waits 15 to 40 s so that a
  pairing does not come the moment it is queued) is part of it: `fallback_due_keyed(key, ticket, now, hidden)`
  has every ticket due at once when the bots are not hidden. The key is the host's secret.
- **Every draw is a stream of the match's or the pool's seed**, apart from the match rng, so none reaches
  a state, the log or a replay hash; the think time, the idle roll and the rematch plan are drawn by the
  machine from the seed it was made with.
- **Cosmetics are still the server's** (`crates/server/src/bots/cosmetics.rs`, R1834005): they read the
  cosmetics registry and the economy's ledger types, which move into this crate with the economy (#3104).

### The boundary

`jackioh-wasm-core`'s `bots(call)` takes one `BotCall` and answers one `BotAnswer`. The calls are `poolTarget`,
`planPool`, `planGrind`, `newBot`, `botTargetStrength`, `drawDecks`, `drawTrio`, `fallbackSeconds`, `fallbackDue`,
`hourOf`, `chooseBot`, `chooseBotPair`, `botMatches`, `botMatchGapPassed`, `pickDueAt`, `pickIsDue`, `pickSlot`
and `botPick`; `bots::boundary::apply` is the one function behind it, and `tests/bots_boundary.rs` holds it to
the functions it names. The deals read the catalog the module registered, and a new bot's username is the
host's to give.

## `src/deck.rs`

What a ticket, a room or a series freezes of a player's decks, and the seat a match starts from:
`DeckSlots` (R1834002), `FrozenDeck` (R642), `FrozenTrio` (R259) and `MatchSeat`. `absent_or_null`
(`src/lib.rs`) keeps a deck's absent, `null` and chosen portrait apart.

## Tests

`tests/series.rs` holds the series rules' tests, moved from `crates/server`'s API tests
unchanged, and `tests/boundary.rs` the JSON boundary; `tests/loadout.rs` holds the loadout rule's, moved from
`crates/server/src/loadout.rs`, its JSON calls among them, over a game dealt from the shipped cards (the
`jackioh-cards` and `jackioh-ai` dev-dependencies). The ladder bots': `tests/bots_decide.rs` (the decision and
the think time), `tests/bots_signals.rs`, `tests/bots_persona.rs`, `tests/bots_pick.rs` and
`tests/bots_boundary.rs`, moved from the server's bot tests; the bot seat of the machine is in `tests/duel.rs`. The host machine's: `tests/clock.rs` (the clock's
tests, moved from the server's actor tests on a simulated host), `tests/duel.rs` (the machine played on
the real catalog), `tests/rematch.rs` (the book), `tests/duos.rs` and `tests/again.rs` (the co-op machine and its Play again
book) and `tests/host_boundary.rs`; the practice session's, `tests/practice.rs` (played on the real catalog with a `Session`
standing in for a host). The ranked ladder's: `tests/glicko2.rs`, `tests/ladder.rs` and `tests/season.rs` (moved from the server's API tests
with the maths), `tests/board.rs` (the sort key) and `tests/ranked_game.rs` (`decide` and its JSON calls). The `ts` feature writes the types that cross to
`apps/web/src/wire/generated/`; commit any diff.

```
cargo test -p jackioh-shared
cargo test -p jackioh-shared --features ts
cargo clippy -p jackioh-shared --all-targets -- -D warnings
```
