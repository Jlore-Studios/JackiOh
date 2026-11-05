# `jackioh-wasm-c`

The engine, the cards, the AI, the validator and the dungeon run as a plain `wasm32-unknown-unknown`
module that **imports nothing**: no wasm-bindgen, no JavaScript glue, no WASI, no emscripten and no
`emcc`. Every binding of the browser module (`crates/wasm`, `docs/v0.3.0/SURFACE.md` §10.1) is an
`extern "C"` function over JSON text, with the same name and the same JSON; a small memory interface
moves the text across. It is the module a host that is not a browser runs, and the one the Roblox port
translates (#2898, #3093): everything the browser and the server share reaches that host through it,
and nothing is restated by hand.

This file is the module's contract. It is written for a host that is not JavaScript: an embedder
(wasmtime, wasmer, wazero), or a translation of the module into another language.

## Build

```
cargo build -p jackioh-wasm-c --release --target wasm32-unknown-unknown
# target/wasm32-unknown-unknown/release/jackioh_wasm_c.wasm

JACKIOH_WASM_C=1 JACKIOH_WASM_LUAU=0 JACKIOH_WASM_NORMAL=0 sh scripts/build-wasm.sh   # builds it, and fails on an import
```

`rustc` links it with its own `rust-lld`, so the C++ toolchain the Luau module needs is not involved.
`scripts/build-wasm.sh` reads the built module's import section with `node` and fails if it holds any
import. The `preview` and `duos` features (R1420's `preview_sets`, #936's Duos bindings) are off by
default, as in the production browser module; build with `--features preview,duos` to turn them on.

## Instantiate

Instantiate the module with an **empty import object**. It exports its linear memory (`memory`), the
four memory-interface functions below, and one function per binding. Call `init()` once, before any
other binding: it registers the catalog and the card scripts (idempotent) and, on WebAssembly, installs
the panic hook that keeps a panic's message readable (Errors and traps).

## The memory interface

The boundary has no strings. The host and the module share the module's linear memory, and four
functions move text through it. On wasm32 a pointer and a length are `i32`; the module is single-threaded.

| Export | Signature (wasm32) | Does |
| --- | --- | --- |
| `jackioh_alloc(len)` | `(i32) -> i32` | A buffer of `len` bytes (one byte, for `len` 0) for the host to write an argument into. The address is never 0. Memory the instance cannot grow to hold it traps. |
| `jackioh_free(ptr, len)` | `(i32, i32) -> ()` | Gives a `jackioh_alloc` buffer back. `len` **must be the one it was allocated with**; a wrong one is undefined behavior. `ptr` 0 is ignored. |
| `jackioh_result_ptr()` | `() -> i32` | Where the last binding's text starts. |
| `jackioh_result_len()` | `() -> i32` | How many bytes of UTF-8 it holds. With 0 there is nothing to read, and the pointer means nothing. |

## Calling a binding

1. For each text argument, `jackioh_alloc(len)` and write its UTF-8 bytes into the buffer. Strings are
   **not NUL-terminated**, and address 0 is never a buffer. An empty argument may be passed as
   `(any pointer, 0)`; it reads as the empty text, and the binding's own parse refuses it.
2. Call the binding with `(ptr, len)` for each text argument, in the order the browser binding takes them.
   A number argument is passed as itself: `f64` for `deadline_ms`, `cost` and `rng_cursor`, `i32`
   (read as `u32`) for `replay_page`'s `from` and `count`.
3. The binding returns an `i32` status: `0` (`STATUS_OK`) or `1` (`STATUS_FAILED`).
4. Read `jackioh_result_len()` bytes at `jackioh_result_ptr()`: the answer if the status is 0, the
   failure's message if it is 1. **Copy them out.**
5. `jackioh_free` each argument buffer with the `len` it was allocated with.

```
state = alloc(len(state_json)); write(state, state_json)
status = seat_to_act(state, len(state_json))
text = read(result_ptr(), result_len())     -- "p1", "p2" or ""; copy it now
free(state, len(state_json))
```

### Ownership and lifetimes

- The host owns every buffer it allocated: it frees each argument after the call, and the module never
  keeps a pointer into one.
- The **module owns the result**. It is valid until the next call into any binding (`init` and the
  four memory functions leave it alone), which replaces it. The host copies it out and **never frees
  it**.
- A binding that answers nothing (`preview_sets`) leaves an empty result, not the one before.

### What the answers are

| Status | The result is |
| --- | --- |
| `0` | The binding's JSON, exactly as the browser binding returns it (`crates/wasm/README.md` has each shape). Where the browser binding returns a boolean (`ai_to_act`, `lethal_hint`) it is the JSON text `true` or `false`; where it returns nothing (`preview_sets`) it is empty. |
| `1` | A message in plain text: a JSON argument that does not parse, a setup the engine refuses (a deck or handicap `create_game` will not take, a game with custom cards, R915009), an argument that is missing (a null pointer with a length) or not UTF-8, or the Duos or preview feature the build left out. |

A rules refusal is **not** a failure. `reduce` answers an illegal action with status 0 and the
`ReduceResult`'s `error` (SURFACE §6.1); the run's bindings answer `{ "error": … }`, and
`run_check_setup` answers its reason string or `null`, with status 0 (`crates/wasm/README.md`).

## Errors and traps

A panic is an engine invariant broken (SURFACE §4.4.9), and in WebAssembly it **traps** (`unreachable`):
the call that panicked never returns a status. The instance is dead after it and must be replaced by a
new instantiation; it is not safe to call again. Its message is still readable: the panic hook wrote it to
the result buffer before the trap, so `jackioh_result_len()` bytes at `jackioh_result_ptr()` are the text
(call those two on the trapped instance, nothing else). `init()` must have run for that, and a host
that has not called it gets the trap alone.

A call that returns normally can be trusted to have left a coherent state; the module's only state
between calls is the catalog, the result buffer and a cache of the few game states it parsed last (by
their exact JSON text, so a cache never changes an answer).

## The bindings

Text arguments are `(ptr, len)` pairs, in this order; numbers are scalars at the position shown. Each
binding returns the status. The JSON of each is the browser binding's of the same name: see
`crates/wasm/README.md` and SURFACE §10.1.

| Area | Bindings |
| --- | --- |
| Boot | `init()` |
| Catalog | `catalog()`, `catalog_version()`, `carried_versions()` |
| Constants | `constants()`: every number a client reads, one JSON object (below) |
| The engine | `create_game(args)`, `begin_game(state)`, `reduce(state, action)`, `legal_actions(state, player)`, `view_for(state, player)`, `seat_to_act(state)`, `hash_state(state)`, `fold(args)`, `last_board_for(state, seat)`, `seat_played_by(state, home)`, `find_instance(state, instance)`, `deal_emote_hand(seed, seat)`, `preview_sets(sets)` |
| Replays | `replay_open(args, record)`, `replay_page(checkpoints, seat, from: u32, count: u32)` |
| The AI | `ai_to_act(state, seat)`, `ai_decide(state, seat, options, deadline_ms: f64)`, `lethal_hint(state, seat, deadline_ms: f64)`, `build_ai_deck(options)`, `choose_action(state, seat, rng_seed, rng_cursor: f64)` |
| The tables and the validator | `engine_tables()`, `validator(call, input)`, `craft_preview(recipe, cost: f64, prompt)`, `learned_parity(fixture)` |
| The dungeon run | `run_content(custom_cards)`, `run_check_setup(setup, unlocks, content)`, `run_create(setup, unlocks, content)`, `run_reduce(state, content, action)`, `run_fold(file, content)`, `run_fight_setup(state, content)`, `run_fight_report(game, human)`, `run_score(state, content)`, `run_embers(score)`, `run_max_hp(state, content)`, `run_level_cost(level)`, `run_constants()`, `run_seed(source)`, `run_daily_seed(date, n)` |
| Duos co-op | `duos_create(options)`, `duos_begin(state)`, `duos_reduce(state, action)`, `duos_legal_actions(state, seat)`, `duos_view_for(state, seat)` (built only with the `duos` feature; without it each answers status 1 and the reason) |

`crates/wasm/tests/export_parity.rs` holds this list to the browser module's: every binding the browser
module binds is exported here, with these four memory functions the only others, so **a binding added
for the web reaches this module by itself** or that test fails. The Luau module's custom-card check
(`check_custom_card`) and its error channel (`jackioh_last_error`) are not here, and leave with the
Luau module (part 5 of #2956).

### The state

The state is the `GameState` JSON object. It is **opaque to a client**: a host keeps it as text, hands it
back on every call, and reads the game only through `view_for`, which is its window onto the game and
hides what the seat may not see (CLAUDE.md rule 7). A host may keep that text anywhere (a datastore, a
checkpoint) and rebuild the game from it on a new instance.

### Constants

`constants()` carries every number a client reads, as one JSON object, so a host states none of them
(CLAUDE.md rule 9):

- the AI's: `AI_BUDGET`, `AI_GATE_BUDGET`, `SHADOW_BAN_IDS`, `FIXTURE_BANNED_IDS`, `AI_DECK`;
- the dungeon run's: `RUN`, the object `run_constants()` also answers;
- the rules': the constants of `apps/web/src/wire/engineConfig.ts` under their own names
  (`constants.DECK_SIZE`, `constants.HERO_HEALTH`, `constants.AI_DIFFICULTY`, …), as the web imports them.

The server's numbers are not in it. A later shared crate adds its own here (#3098), so a number a host
needs is read from this object and not copied.

## The clock

The module imports nothing, so it has **no clock**, and a deadline that depends on one could never
come. `ai_decide` and `lethal_hint` take `deadline_ms` so their signatures are the browser module's, and
**ignore it**: a search is bounded by its node budget (`options.budget`, `AI_BUDGET` when absent), and
its answer depends only on `(state, seed, cursor, budget)`. Bound the AI with `options.budget`, and
run it where a slow turn may take its time (the host decides when to ask).

## Determinism and threading

- The pure crates read no clock, environment or OS randomness (CLAUDE.md rule 4): every random draw goes
  through an `Rng` rebuilt from the state's seed and cursor, so the same bytes in give the same bytes out,
  on any host.
- The module is **single-threaded and not reentrant**: one call at a time per instance, and no call may
  begin from inside another. An instance holds its catalog and its caches, so a host that serves many
  games either runs one instance with the states it keeps as text, or one instance per worker.
- Memory only grows. A long-lived instance that has parsed a large state keeps its high-water mark; a
  host that cares replaces the instance.

## Tests

`cargo test -p jackioh-wasm-c` calls each export natively, with JSON the way a host does, and holds its
status and result to what `jackioh-wasm-core`'s body answers for the same JSON; `every_export_has_a_test`
fails on an export no test calls. `cargo test -p jackioh-wasm --test export_parity` holds the export list.
