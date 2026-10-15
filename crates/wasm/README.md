# `jackioh-wasm`

The engine, the cards, the AI and the validator compiled to WebAssembly for the web client: hotseat,
practice, the tutorial and the deck builder run them in the browser. JSON strings in, JSON strings
out; nothing else crosses. The state is the `GameState` JSON object, which the client keeps behind its
opaque `EngineState` brand (CLAUDE.md rule 7) and hands back on every call.

## The bindings (`src/lib.rs`)

| Binding | Answers |
| --- | --- |
| `init()` | installs the panic hook and calls `jackioh_cards::register_all()` |
| `catalog()`, `catalog_version()` | the compiled-in catalog and its version |
| `create_game(args)`, `begin_game(state)`, `reduce(state, action)` | a `GameState`, then `ReduceResult`s (`{ state, events, error? }`) |
| `legal_actions(state, player)`, `view_for(state, player)`, `seat_to_act(state)` | as the engine's |
| `hash_state(state)`, `fold(args)` | the state hash, and a log folded (`{ state, errors }`) |
| `replay_open(args, record)`, `replay_page(checkpoints, seat, from, count)` | R768's `ReplayOpen`, then `ReplayPage` |
| `last_board_for`, `seat_played_by`, `find_instance` | as the engine's |
| `deal_emote_hand(seed, seat)` | `EmoteId[]`: the seat's hand of eight emotes for that seed, as the server deals it (R1341), for hotseat and practice |
| `ai_to_act(state, seat)`, `ai_decide(state, seat, options, deadline_ms)` | the AI; `options` is `{ rngSeed, rngCursor, budget? }`, the answer `{ decision, rngCursor }` |
| `build_ai_deck(options)` | `{ deck, rngCursor }`; `options` is `{ rngSeed, rngCursor, size }` and `AiDeckOptions`' own keys, `leanSet` among them (R1370) |
| `choose_action(state, seat, rng_seed, rng_cursor)` | spec §10.7's random policy, `{ action, rngCursor }` |
| `validator(call, input)` | one of the validator's functions (`validateDeck`, `validateTrio`, `checkDeckDraft`, `checkImportRoom`, …) by name |
| `constants()`, `engine_tables()` | the AI budgets, shadow-ban ids and deck builder's numbers (`AI_DECK`); the three Call to Chaos tables (Core's, Classic+'s and Meditative's) and the Heroic Power table the client prints |
| `preview_sets(sets)` | R1400, R1420: treat `SetName[]` as shipped for the module's life (the dev hotseat's E2E injection); a module built without the `preview` feature refuses any set |
| `craft_preview(recipe, cost)` | ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe — a `CraftPreview` — so the block editor shows what the reducer will say |

A binding returns `Err(JsError)`, which JavaScript sees as a thrown `Error` with the engine's message,
where a JSON argument does not parse or the setup is refused (a deck or handicap `create_game` will
not take). `reduce` never throws on an illegal action: its refusal is the result's `error`.

The pure crates never read a clock (CLAUDE.md rule 4). The AI's wall-clock cap is this crate's:
`ai_decide` gives `decide` a `should_stop` that compares `js_sys::Date::now()` with `deadline_ms`
(`deadline_ms <= 0`: no clock).

## Build and the client's seams

`scripts/build-wasm.sh` builds the module in release for `wasm32-unknown-unknown` and runs
`wasm-bindgen --target web` into `apps/web/src/wasm/pkg/` (gitignored). It adds the target and
downloads the pinned `wasm-bindgen-cli` (the same version as the crate, `=0.2.129`) only when they
are missing. `apps/web/package.json`'s `predev`, `prebuild`, `prebuild:e2e` and `pretest` run it. By
default it builds with the `preview` feature (the engine's testkit, for `preview_sets`, R1400);
`prebuild`, the production bundle's, sets `JACKIOH_WASM_PREVIEW=0` and builds without it.

`apps/web/src/wasm/index.ts` is the only caller: `loadWasm()` (or `loadWasmSync(bytes)` under jsdom)
and one typed function per binding, named as the TypeScript engine named it. `apps/web/src/wire/`
maps the client's `@jackioh/*` imports onto those functions and the generated types
(`apps/web/README.md`).

```
sh scripts/build-wasm.sh
cargo build -p jackioh-wasm --release --target wasm32-unknown-unknown
```

## The Luau module (`luau/`, #919)

`crates/wasm/luau` (`jackioh-wasm-luau`) is the same engine, cards, AI and validator with the card
kit's host (`jackioh_luau::custom::KitHost`) lent to the engine around every call, so a game whose
state carries custom cards runs their scripts (SPEC §12, R915009). It is a second module, loaded only
for a practice game whose human deck holds a custom card and for the card creator; a normal game,
practice or online, never loads it, and the normal module refuses a game with custom cards.

- **Its target** is `wasm32-unknown-emscripten`, the one WebAssembly target `mlua` builds Luau for.
  The crate is a binary whose `main` does nothing; each binding is an `extern "C"` function over
  UTF-8 C strings (`_create_game`, `_reduce`, …), the same names and JSON as the bindings above, plus
  `check_custom_card(card)`: the engine validator's issues with the frame, the Luau module's problems
  with the script (the lint, Luau's compiler and the kit's own checks, each with its line when it has
  one) and the definition the frame builds, `{ frame, script, def }`, which the creator shows. A
  binding's error is kept for `_jackioh_last_error`, and every string it returns is freed with
  `_jackioh_free`. `.cargo/config.toml` holds the `emcc` link's `-s` flags, `EXPORTED_FUNCTIONS`
  among them.
- **Its build** is the second half of `scripts/build-wasm.sh`, unless `JACKIOH_WASM_LUAU=0`: it adds
  the target, clones and installs the pinned emsdk (`EMSDK_VERSION`) into `.cache/emsdk` when it is
  missing (CI's setup action caches that directory, keyed on the pin), builds the crate with
  `emcc`/`em++` on `PATH` (Luau itself is C++), and copies the MODULARIZE factory and its `.wasm` into
  `apps/web/src/wasm/pkg/luau/`, with a `.d.ts` it writes for the factory. Inside `pkg/`, it is
  gitignored, linted by nothing and carried by CI's `web-build` artifact with the normal module.
- **Its seam** is `apps/web/src/wasm/luau.ts`, imported only dynamically by `wasm/index.ts`:
  `useLuauModule(on)` switches the bindings every engine call goes through, which the practice worker
  does per request (`PracticeCore.needsLuau`), and `checkCustomCard(card)` loads it for the creator
  without switching. `apps/web/src/wasm/luau.test.ts` holds that no other file imports it.

```
sh scripts/build-wasm.sh                                  # both modules
JACKIOH_WASM_LUAU=0 sh scripts/build-wasm.sh              # the normal module alone
```
