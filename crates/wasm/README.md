# `jackioh-wasm`

The engine, the cards, the AI and the validator compiled to WebAssembly for the web client: hotseat,
practice, the tutorial and the deck builder run them in the browser. JSON strings in, JSON strings
out; nothing else crosses. The state is the `GameState` JSON object, which the client keeps behind its
opaque `EngineState` brand (CLAUDE.md rule 7) and hands back on every call.

## The bindings (`src/main.rs`)

The boundary is C over NUL-terminated UTF-8 strings: `mlua` (the Luau host) builds Luau for
WebAssembly only through `wasm32-unknown-emscripten`, which `wasm-bindgen` cannot target, so the
module links through `emcc` and every binding is a `#[no_mangle] extern "C"` function. A string
argument goes in, an owned string comes back; the caller frees both with `jackioh_free`. A binding
that would have thrown answers NULL and stows its message for `jackioh_last_error`. The function
names and the JSON that crosses are the ones SURFACE §10.1 gave the `wasm-bindgen` module, so
nothing above `apps/web/src/wasm/index.ts` changes.

This crate is a binary: `rustc` only emits emscripten's JavaScript glue (the `MODULARIZE` factory)
when the `emcc` link produces a program, and a `cdylib` link produces a bare `.wasm` with no glue.
`fn main` does nothing; the client calls `init()` after instantiating.

| Binding | Answers |
| --- | --- |
| `init()` | installs the panic hook and calls `jackioh_cards::register_all()` |
| `catalog()`, `catalog_version()` | the compiled-in catalog and its version |
| `carried_versions()` | R767000: the older shipped versions it carries for replays, oldest first (from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM`) |
| `create_game(args)`, `begin_game(state)`, `reduce(state, action)` | a `GameState`, then `ReduceResult`s (`{ state, events, error? }`) |
| `legal_actions(state, player)`, `view_for(state, player)`, `seat_to_act(state)` | as the engine's |
| `hash_state(state)`, `fold(args)` | the state hash, and a log folded (`{ state, errors }`) |
| `replay_open(args, record)`, `replay_page(checkpoints, seat, from, count)` | R768's `ReplayOpen`, then `ReplayPage` |
| `last_board_for`, `seat_played_by`, `find_instance` | as the engine's |
| `deal_emote_hand(seed, seat)` | `EmoteId[]`: the seat's hand of eight emotes for that seed, as the server deals it (R1341), for hotseat and practice |
| `ai_to_act(state, seat)` | 1, 0, or -1 with the message stowed (R187, R188) |
| `ai_decide(state, seat, options, deadline_ms)` | the AI; `options` is `{ rngSeed, rngCursor, budget? }`, the answer `{ decision, rngCursor }` |
| `build_ai_deck(options)` | `{ deck, rngCursor }`; `options` is `{ rngSeed, rngCursor, size }` and `AiDeckOptions`' own keys, `leanSet` among them (R1370) |
| `choose_action(state, seat, rng_seed, rng_cursor)` | spec §10.7's random policy, `{ action, rngCursor }` |
| `validator(call, input)` | one of the validator's functions (`validateDeck`, `validateTrio`, `checkDeckDraft`, `checkImportRoom`, …) by name |
| `constants()`, `engine_tables()` | the AI budgets, shadow-ban ids and deck builder's numbers (`AI_DECK`); the three Call to Chaos tables (Core's, Classic+'s and Meditative's) and the Heroic Power table the client prints |
| `preview_sets(sets)` | R1400, R1420: treat `SetName[]` as shipped for the module's life (the dev hotseat's E2E injection); 0 on success, 1 with the message stowed; a module built without the `preview` feature refuses any set |
| `duos_create(options)`, `duos_begin(state)`, `duos_reduce(state, action)`, `duos_legal_actions(state, seat)`, `duos_view_for(state, seat)` | #936: the Duos wrapper for `/dev/duos`: a `DuosState` (its four decks and handicaps checked first, as `create_game`'s setup is), then `{ state, events, error? }`, `DuosActionBody[]` and `DuosView`; the seat crosses as its string (`"a1"`). Built only with the `duos` feature (development builds: `JACKIOH_WASM_DUOS`, on by default, off for the production bundle and the unit tests' module); without it each answers NULL with the reason |
| `craft_preview(recipe, cost)` | ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe — a `CraftPreview` — so the block editor shows what the reducer will say |
| `jackioh_free(text)` | frees a returned string or a spent argument buffer (both live on the one emscripten heap) |
| `jackioh_last_error()` | the last failure's message, valid until the next failing call |

A binding returns NULL, which JavaScript reads as a thrown `Error` with the engine's message,
where a JSON argument does not parse or the setup is refused (a deck or handicap `createGame` will
not take). `reduce` never fails on an illegal action: its refusal is the result's `error`.

The pure crates never read a clock (CLAUDE.md rule 4). The AI's wall-clock cap is this crate's:
`ai_decide` gives `decide` a `should_stop` that compares `SystemTime::now` (the libc clock
emscripten provides) with `deadline_ms` (`deadline_ms <= 0`: no clock).

A panic is an engine invariant broken; the hook writes it with `emscripten_console_error` before
the abort, which the glue raises as a `WebAssembly.RuntimeError`.

## Build and the client's seams

`scripts/build-wasm.sh` clones and installs the pinned emsdk into `.cache/emsdk` when it is
missing, builds the module in release for `wasm32-unknown-emscripten` (the `emcc` link's `-s`
flags live in `.cargo/config.toml`) and copies `jackioh-wasm.js` and `jackioh_wasm.wasm` into
`apps/web/src/wasm/pkg/` (gitignored) as `jackioh_wasm.js` and `jackioh_wasm.wasm`, with a small
`jackioh_wasm.d.ts` for the factory's types. `apps/web/package.json`'s `predev`, `prebuild`,
`prebuild:e2e` and `pretest` run it. By default it builds with the `preview` feature (the engine's
testkit, for `preview_sets`, R1400); `prebuild`, the production bundle's, sets
`JACKIOH_WASM_PREVIEW=0` and builds without it. It also builds with the `duos` feature by default
(#936's Duos bindings, which only the development-only `/dev/duos` calls); `prebuild` and
`pretest` set `JACKIOH_WASM_DUOS=0`, so neither the module a player loads nor the one
`apps/web/src/wasm/size.test.ts` weighs (#442 L12) carries them.

`apps/web/src/wasm/index.ts` is the only caller: `loadWasm()` (or `loadWasmSync(bytes)` under
jsdom, which awaits the factory like the page does) and one typed function per binding, named as
the TypeScript engine named it. `apps/web/src/wire/` maps the client's `@jackioh/*` imports onto
those functions and the generated types (`apps/web/README.md`).

```
sh scripts/build-wasm.sh
cargo build -p jackioh-wasm --release --target wasm32-unknown-emscripten
```
