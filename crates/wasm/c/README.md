# `jackioh-wasm-c`

The engine's C boundary (#3093, `docs/v0.3.0/SURFACE.md` §10.5): the cards, the rules engine, the AI,
the validator and the dungeon run as one WebAssembly module whose exports are plain `extern "C"`
functions over JSON strings. It is built for `wasm32-unknown-unknown` with no emscripten, no Luau VM, no
wasm-bindgen glue, no JavaScript and no system interface, so its **import section is empty**: any host
that runs WebAssembly instantiates it with no imports. It is what a port that is not JavaScript runs
(#2898's Roblox port translates it), so everything the browser and the server share reaches such a host
through this module, with nothing restated by hand: the numbers a client reads come from `constants()`
(CLAUDE.md rule 9), the rules from the engine.

It is a third wrapper over `jackioh-wasm-core`'s bodies (#1505), beside `crates/wasm` (wasm-bindgen,
the web) and `crates/wasm/luau` (the same C names through emscripten, with the Luau VM, which part 5 of
#2956 deletes). The bodies are not copied, so an answer is the browser's answer for the same JSON, and
`crates/wasm/tests/export_parity.rs` holds this module to every binding the browser module binds: one
added for the web reaches here by itself. `crates/wasm/README.md` has each binding's JSON, which is the
same here, and `docs/v0.3.0/SURFACE.md` §10 the seams around it.

```
cargo test -p jackioh-wasm-c                                              # each export against its core body
cargo build -p jackioh-wasm-c --release --target wasm32-unknown-unknown   # target/wasm32-unknown-unknown/release/jackioh_wasm_c.wasm
JACKIOH_WASM_C=1 JACKIOH_WASM_NORMAL=0 JACKIOH_WASM_LUAU=0 sh scripts/build-wasm.sh   # the same, and checks the imports with node
```

The module is tens of megabytes: the catalog, every card script and the AI's tables are in it.

## Instantiating

Instantiate the module with an **empty import object** (an import section with anything in it fails
`scripts/build-wasm.sh`'s check and `export_parity.rs`'s test of the sources). It exports its linear
memory as `memory`, the functions below, and nothing else. Call **`init()` once, before any other
export**: it registers the catalog and the card scripts with the engine, and arranges for a panic's text
to be kept (see Errors and traps). It is idempotent.

On wasm32 a pointer or a length is an `i32` and a number argument is an `f64` (`deadline_ms`,
`rng_cursor`, `cost`) or a `u32` (`replay_page`'s `from` and `count`, an `i32` on the wire). An export
returns an `i32`.

## The memory interface

A host cannot hand the module a string; it writes bytes into the module's memory and passes where they
are. The module gives it four functions for that, and a buffer for what comes back:

| Export | Does |
| --- | --- |
| `jackioh_alloc(len) -> ptr` | A buffer of `len` bytes (at least one byte is allocated, so `ptr` is never 0) for the host to write an argument into. Its bytes start as 0. |
| `jackioh_free(ptr, len)` | Frees a buffer `jackioh_alloc(len)` returned, given the same `len`. A `ptr` of 0 is ignored. |
| `jackioh_result_ptr() -> ptr` | Where the last binding's answer starts. |
| `jackioh_result_len() -> len` | How many bytes it is. |

A wrong `len` to `jackioh_free`, a second free, and a read after a free are undefined behavior; the
module cannot catch them.

## The calling convention

Every binding takes each string argument as a **pair of numbers**, `(ptr, len)`, in the order of its
signature below, and each number argument as a number, and returns a **status**:

- `0`: it answered. The result is the binding's JSON text.
- `1`: it failed. The result is the failure's message, plain text.

Calling one, from the host's side:

1. For each string argument, `ptr = jackioh_alloc(len)`, where `len` is the byte length of its UTF-8
   encoding, and copy the bytes to `ptr`. A string is **not NUL-terminated**, and a string with no bytes
   is `(any, 0)`: no buffer is needed for it, and a `ptr` of 0 is fine.
2. Call the export with the pairs and numbers.
3. Read `jackioh_result_len()` bytes at `jackioh_result_ptr()`, as UTF-8, and **copy them out**.
4. `jackioh_free(ptr, len)` each argument buffer.

```
state = alloc_bytes(utf8(state_json))              # (ptr, len) written into the module's memory
status = seat_to_act(state.ptr, state.len)
answer = utf8_decode(memory[result_ptr() .. result_ptr() + result_len()])    # copy it now
free(state.ptr, state.len)
if status == 0: seat = answer                      # "p1", "p2" or ""
else:           raise Error(answer)
```

**Results.** The result buffer is the module's, not the host's: **never free it.** It is valid until the
next call into the module, any call, which replaces it: copy it out first. A call that answers the
empty text (`preview_sets`, or `seat_to_act` of a game no seat owes) leaves length 0; do not read at the
pointer then. The text is UTF-8, never
NUL-terminated and never invalid.

**Booleans.** `ai_to_act` and `lethal_hint` answer the JSON text `true` or `false`.

**Arguments** are read as UTF-8 and parsed as the JSON the binding names, so each is refused, with
status 1 and its own message, when it is not UTF-8 (`<binding>: the argument is not UTF-8: …`), when it
is missing (a `ptr` of 0 with a `len` above 0: `<binding>: the argument is missing`) or when its JSON
does not parse (`<binding>: <what serde says>`). The plain-text arguments are a seat or a player (`p1`,
`p2`, or `a1` … for a Duos seat), `home`, `instance_id`, `seed`, `rng_seed` and the validator's `call`,
with no quotes; every other string argument is JSON, the run bindings' small ones too (`source`, `date` and
`human` are JSON strings, `"abc"`; `n`, `level` and `score` are numbers, `1`, `250`).

## Errors and traps

A status of `1` is a call refused: bad JSON, a setup the engine will not take (a deck or handicap
`create_game` refuses), a game with custom cards (this module plays none: `R915009`), a set to preview
in a module built without the `preview` feature. The module is as it was and the next call is fine.

A game rule is **not** a status of `1`. `reduce` refuses an illegal action in its result's `error`
(`{ state, events, error }`, SURFACE §6.1) and answers status `0`, as the dungeon run's bindings refuse
a run rule with `{ "error": … }`, and `run_check_setup` with its reason.

A **panic** is an engine invariant broken (SURFACE §4.4.9). In WebAssembly it is a trap: the host sees
an `unreachable` trap and the instance is dead, so it must be **replaced by a fresh one**, with its
state lost. The panic's text (message and place) is kept before the trap, and is still readable at
`jackioh_result_ptr()` and `jackioh_result_len()` afterwards, as the result buffer holds it; log it.

## The bindings

Each takes its strings as `(ptr, len)` pairs, in this order, and returns the status. The JSON of every
argument and answer is `crates/wasm/README.md`'s, the same as the browser module's.

| Binding | Arguments | Answers |
| --- | --- | --- |
| `init()` | | nothing; once, first |
| `catalog()`, `catalog_version()`, `carried_versions()` | | the compiled-in catalog, its version, the older shipped versions it carries for replays |
| `constants()` | | every rules, AI and run number a client reads (below) |
| `create_game(args)`, `begin_game(state)`, `reduce(state, action)` | strings | a `GameState`, then `ReduceResult`s |
| `legal_actions(state, player)`, `view_for(state, player)`, `seat_to_act(state)` | strings | `ActionBody[]`, the `PlayerView`, `p1`, `p2` or the empty text |
| `hash_state(state)`, `fold(args)` | strings | the state's hash, a log folded |
| `replay_open(args, record)`, `replay_page(checkpoints, seat, from, count)` | strings, then `from: u32`, `count: u32` | a replay opened, a page of it |
| `last_board_for(state, seat)`, `seat_played_by(state, home)`, `find_instance(state, instance_id)` | strings | as the engine's |
| `deal_emote_hand(seed, seat)` | strings | `EmoteId[]` |
| `ai_to_act(state, seat)` | strings | `true` or `false` |
| `ai_decide(state, seat, options, deadline_ms)` | strings, then `deadline_ms: f64` | `{ decision, rngCursor }` |
| `lethal_hint(state, seat, deadline_ms)` | strings, then `deadline_ms: f64` | `true` or `false` |
| `build_ai_deck(options)` | string | `{ deck, rngCursor }` |
| `choose_action(state, seat, rng_seed, rng_cursor)` | strings, then `rng_cursor: f64` | `{ action, rngCursor }` |
| `validator(call, input)` | strings | one validator function's result |
| `series(call)` | string | #3098: one call of `jackioh-shared`'s Conquest series rules, a `SeriesCall` in and a `SeriesAnswer` out (`{ "row": … }` to write back, `{ "refused": … }`, or a read); the host holds the row and passes the time in (`crates/shared/README.md`, the host contract) |
| `economy(call)` | string | #3104: one call of `jackioh-shared`'s collection economy, an `EconomyCall` in and an `EconomyAnswer` out (`openCardPack` and `openCosmeticPack` take the records the store read, the seed and the time, and answer the slots, the `grants` to write in order and the `log` every row's `ref` carries; `fillBar` and `levelReward` are reads); the host reads the records from its store and writes the grants back (`crates/shared/README.md`, the host contract) |
| `host(call)` | string | #3101: one call of `jackioh-shared`'s match host machine, a `HostCall` in and a `HostAnswer` out (`stepDuel` takes the `DuelHost`, a `DuelEvent` and the time, and answers the next `DuelHost` and its `DuelOutput`s, which the host carries out in order; the rematch book's calls answer its next state); the host holds the state and passes the time in (`crates/shared/README.md`, the host contract) |
| `craft_preview(recipe, cost, prompt)` | `recipe`, then `cost: f64`, then `prompt` | the engine's verdict on a recipe |
| `engine_tables()` | | the Call to Chaos and Heroic Power tables |
| `learned_parity(fixture)` | string | the learned evaluation's answers for a fixture, to hold a port to |
| `preview_sets(sets)` | string | nothing (an empty result); a module built with `preview` previews the sets |
| `run_content(custom_cards)`, `run_constants()` | | the dungeon run's content and numbers |
| `run_check_setup(setup, unlocks, content)`, `run_create(setup, unlocks, content)` | strings | whether the setup may start a run, a run started |
| `run_reduce(state, content, action)`, `run_fold(file, content)` | strings | a run after one action, a run file folded |
| `run_fight_setup(state, content)`, `run_fight_report(game, human)` | strings | the fight being played, the `FightOver` it sends |
| `run_score(state, content)`, `run_embers(score)`, `run_max_hp(state, content)`, `run_level_cost(level)` | strings | the score, the Embers it pays, the max HP, a level's cost |
| `run_seed(source)`, `run_daily_seed(date, n)` | strings | a seed, the daily run's seed |
| `duos_create(options)`, `duos_begin(state)`, `duos_reduce(state, action)`, `duos_legal_actions(state, seat)`, `duos_view_for(state, seat)` | strings | #936's co-op wrapper; built without the `duos` feature each is status `1` with the reason |

There is no custom-card binding and no Luau (`check_custom_card` stays with the Luau module, until
part 5 of #2956 deletes both); a game whose setup carries custom cards is refused (`R915009`). The
module's own Cargo features are the browser module's: `preview` (`preview_sets` can preview a set that
does not ship yet) and `duos`, both off in the build `scripts/build-wasm.sh` makes.

**State.** `GameState` JSON is opaque to a client. Keep it as text, hand it back to every call, and ask
`view_for` for what a seat may see: it is the only window onto a game, and the client sends intent and
never enforces a rule or sees a hidden card (CLAUDE.md rule 7). The module keeps the few states it
parsed or produced last, each under its exact text, so handing back the same text is cheap, and a text
that differs by one byte is parsed again, so an answer never depends on the cache.

**Constants.** `constants()` is one flat JSON object: `AI_BUDGET`, `AI_GATE_BUDGET`, `SHADOW_BAN_IDS`,
`FIXTURE_BANNED_IDS` and `AI_DECK`; `RUN`, the dungeon run's numbers; and each constant of
`apps/web/src/wire/engineConfig.ts` under its own name (`DECK_SIZE`, `MAX_COPIES`, `DIFFICULTIES`, …),
so a host reads `constants.DECK_SIZE` as the web does; and each constant of `jackioh-shared`'s `config.rs`
(`SERIES_WINS_NEEDED`, `SERIES_MAX_GAMES`, `SERIES_PICK_SECONDS`, `MS_PER_SECOND`, #3098, and the economy's
`PACK_SLOTS`, `CARD_TIER_WEIGHTS`, `PITY_EPIC_PACKS`, `COSMETIC_TOKEN_CYCLE` and the rest of its tables, #3104). A host restates
none of them. Later shared modules add theirs here.

## Determinism, the clock and threading

- **Deterministic.** The engine and the AI draw only from their seeded streams, rebuilt from
  `(seed, cursor)`. `ai_decide` answers from `(state, seat, rngSeed, rngCursor, budget)` alone, and
  hands back the cursor it stopped at so the caller resumes it there.
- **No clock.** The module has none to read, and imports none (the pure crates never read one, CLAUDE.md
  rule 4). `ai_decide` and `lethal_hint` take `deadline_ms` so their signatures are the browser's, and
  **ignore it**: the search ends at `options.budget` (default `AI_BUDGET`), in the number of nodes it
  spends, never in time. A host that must bound the time passes a smaller `budget`.
- **One thread.** The module is single-threaded and **not reentrant**: one call at a time, from the
  thread that instantiated it, and the result buffer and the state cache are the instance's, so two
  instances do not share them.
