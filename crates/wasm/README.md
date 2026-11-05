# `jackioh-wasm`

The engine, the cards, the AI, the validator and the dungeon run compiled to WebAssembly for the
web client: hotseat, practice, the tutorial, the deck builder and the dungeon run screen run them in
the browser. JSON strings in, JSON strings out; nothing else crosses. The state is the `GameState`
JSON object, which the client keeps behind its opaque `EngineState` brand (CLAUDE.md rule 7) and
hands back on every call; a run's `RunState` crosses the same way, as plain JSON the client hands back.

## The bindings (`src/lib.rs`)

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
| `combat_forecast(state, seat)` | R3100000: `{ attacks, lethal }`, what each attack `seat` may declare would do (the damage after Armor and Divine Shield, the strike back, whether it kills, whether the attacker lives, whether the shield breaks) and whether the attackers that reach the enemy hero finish it, as the board shows it (`jackioh_engine::combat_forecast`); the practice worker asks it of a tutorial lesson's game, and the coach names its attacks by it. The browser and C modules bind it |
| `deal_emote_hand(seed, seat)` | `EmoteId[]`: the seat's hand of eight emotes for that seed, as the server deals it (R1341), for hotseat and practice |
| `ai_to_act(state, seat)`, `ai_decide(state, seat, options, deadline_ms)` | the AI; `options` is `{ rngSeed, rngCursor, budget? }`, the answer `{ decision, rngCursor }` |
| `lethal_hint(state, seat, deadline_ms)` | R2102000: whether `seat` has a sure win this turn by what it can see (`jackioh_ai::lethal_hint`), for the practice AI's lethal hint; a search still running at `deadline_ms` (as `ai_decide`'s) answers no |
| `build_ai_deck(options)` | `{ deck, rngCursor }`; `options` is `{ rngSeed, rngCursor, size }` and `AiDeckOptions`' own keys, `leanSet` among them (R1370) |
| `choose_action(state, seat, rng_seed, rng_cursor)` | spec §10.7's random policy, `{ action, rngCursor }` |
| `validator(call, input)` | one of the validator's functions (`validateDeck`, `validateTrio`, `checkDeckDraft`, `checkImportRoom`, …) by name |
| `series(call)` | #3098: one call of `jackioh-shared`'s Conquest series rules (SPEC §9.5, R330 to R337): a `SeriesCall` (`{ "call": "pickDeck", "series": …, "seat": "p1", "slot": 2, "now": … }`), answered as a `SeriesAnswer` (`{ "row": … }` to write back, `{ "refused": … }`, or a read). The host holds the series row and passes the time in (`crates/shared/README.md`, the host contract); a rules refusal is the returned `refused`, never thrown. The C module binds it as `series(call_ptr, call_len)` |
| `host(call)` | #3101: one call of `jackioh-shared`'s match host machine (the clocks, the disconnect grace, the rematch offers and the aim relay as one pure state machine): a `HostCall` (`{ "call": "stepDuel", "host": …, "event": …, "now": … }`), answered as a `HostAnswer` (`{ "answer": "stepped", "host": …, "outputs": [ … ] }` or a read). The host holds the `DuelHost` and passes the time in (`crates/shared/README.md`, the host contract); the outputs are timers to set, messages to send, rows to append and moves to decide, carried out in order. |
| `constants()`, `engine_tables()` | every rules, AI and run number a client reads, as one flat object (#3093): the AI budgets, shadow-ban ids and deck builder's numbers (`AI_BUDGET`, `AI_GATE_BUDGET`, `SHADOW_BAN_IDS`, `FIXTURE_BANNED_IDS`, `AI_DECK`), `RUN` (`run_constants`'s object) and each constant of `apps/web/src/wire/engineConfig.ts` under its own name (`DECK_SIZE`, …), so a host restates none (CLAUDE.md rule 9); the three Call to Chaos tables (Core's, Classic+'s and Meditative's) and the Heroic Power table the web's tests check the card text against (`wire/engine.ts`'s `subsystems`) |
| `preview_sets(sets)` | R1400, R1420: treat `SetName[]` as shipped for the module's life (the dev hotseat's E2E injection); a module built without the `preview` feature refuses any set |
| `duos_create(options)`, `duos_begin(state)`, `duos_reduce(state, action)`, `duos_legal_actions(state, seat)`, `duos_view_for(state, seat)` | #936: the Duos wrapper for `/dev/duos`: a `DuosState` (its four decks and handicaps checked first, as `create_game`'s setup is), then `{ state, events, error? }`, `DuosActionBody[]` and `DuosView`; the seat crosses as its string (`"a1"`). Built only with the `duos` feature (the dev server's build only: `JACKIOH_WASM_DUOS=1`, off by default); without it each throws the reason |
| `craft_preview(recipe, cost, prompt)` | ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe — a `CraftPreview` — so the block editor shows what the reducer will say; `prompt` is the open prompt's `{ craftMode, craftOffer }`, its frame and offer (R1563007) |
| `run_content()` | `RunContent`: `run_catalog_content` over the compiled-in catalog |
| `run_check_setup(setup, unlocks, content)`, `run_create(setup, unlocks, content)` | whether the setup may start a run (`null`, or the reason), then the run's `{ state }` |
| `run_reduce(state, content, action)`, `run_fold(file, content)` | the run after one action, or a run file folded from its start: `{ state }` |
| `run_fight_setup(state, content)` | the fight being played (`RunFightSetup`), or `null` outside one |
| `run_fight_report(game, human)` | the `FightOver` a finished fight sends: `{ action }` |
| `run_score(state, content)`, `run_embers(score)`, `run_max_hp(state, content)` | the run's score, the Embers a score pays, the run's max HP |
| `run_level_cost(level)`, `run_constants()` | the Embers one level costs (or `null` at the top), and the run's numbers the screens show |
| `run_seed(source)`, `run_daily_seed(date, n)` | a seed of capital letters, or the daily run's seed |

A binding returns `Err(JsError)`, which JavaScript sees as a thrown `Error` with the engine's message,
where a JSON argument does not parse or the setup is refused (a deck or handicap `create_game` will
not take). `reduce` never throws on an illegal action: its refusal is the result's `error`. The run
bindings work the same way: a run-rule refusal is a returned `{ "error": "…" }` value, never thrown —
except `run_check_setup`, whose refusal is the returned reason string (`string | null`). A fight the
game has not finished reports `{ "error": "…" }` from `run_fight_report`, as an illegal action reports
`error` from `reduce`.

The pure crates never read a clock (CLAUDE.md rule 4). The AI's wall-clock cap is this crate's:
`ai_decide` gives `decide` a `should_stop` that compares `js_sys::Date::now()` with `deadline_ms`
(`deadline_ms <= 0`: no clock).

## Build and the client's seams

`scripts/build-wasm.sh` builds the module in release for `wasm32-unknown-unknown` and runs
`wasm-bindgen --target web` into `apps/web/src/wasm/pkg/` (gitignored). It adds the target and
downloads the pinned `wasm-bindgen-cli` (the same version as the crate, `=0.2.129`) only when they
are missing. `apps/web/package.json`'s `predev`, `prebuild`, `prebuild:staging`, `prebuild:e2e` and
`pretest` run it. By default it builds with the `preview` feature (the engine's testkit, for
`preview_sets`, R1400); `prebuild` and `prebuild:staging`, the production and staging bundles', set
`JACKIOH_WASM_PREVIEW=0` and build without it. The `duos`
feature (#936's Duos bindings, which only the development-only `/dev/duos` calls) is off by default
and on only for `predev` (`JACKIOH_WASM_DUOS=1`), so no other module, the one a player loads
included, carries them.

It keeps nothing between builds itself: both deploys build on GitHub's runners, whose setup action
restores `target/`, so cargo rebuilds only what changed.

`apps/web/src/wasm/index.ts` is the only caller: `loadWasm()` (or `loadWasmSync(bytes)` under jsdom)
and one typed function per binding, named as the TypeScript engine named it. `apps/web/src/wire/`
maps the client's `@jackioh/*` imports onto those functions and the generated types
(`apps/web/README.md`).

```
sh scripts/build-wasm.sh
cargo build -p jackioh-wasm --release --target wasm32-unknown-unknown
```

## The shared bodies (`core/`, #1505)

The two wrappers wrap one set of bodies: `crates/wasm/core` (`jackioh-wasm-core`) holds the
JSON helpers, the state cache, the setup check and every binding's work as `Result<String, String>`
(or `Result<bool | (), String>`) functions. `src/lib.rs` keeps only the wasm-bindgen wrappers, the
panic hook and the clock; `c/src/lib.rs` keeps only the C wrappers on a plain
`wasm32-unknown-unknown` link, the memory interface, the panic hook and no clock. The exports stay
identical, and behavior does not change.

A binding cannot be added to one side and forgotten in the other: `tests/export_parity.rs` reads each
module's export list and fails when the C module lacks a binding the browser module binds, or exports
one of its own that `C_ONLY` (its memory interface, with the reason) does not name. There is no list of
bindings to edit: a binding added for the web is bound in `src/lib.rs`, exported in `c/src/lib.rs` and
tested in `c/src/tests.rs`, and the C module, the one a Roblox port translates (#2898), has it by
itself. The test also holds the C module's sources to importing nothing from the host.

```
cargo test -p jackioh-wasm --test export_parity
```

## The C module (`c/`, #3093)

`crates/wasm/c` (`jackioh-wasm-c`) is the engine's C boundary on its own: every binding the browser
module binds, as an `extern "C"` function over JSON strings, built as a plain `wasm32-unknown-unknown`
`cdylib` with no emscripten, no wasm-bindgen glue, no JavaScript and no system interface,
so its import section is empty. A host that is not JavaScript (a Roblox port's translation of it,
#2898) runs it as it is, with the numbers it needs read from `constants()` and none restated. Its
memory interface (`jackioh_alloc`, `jackioh_free`, `jackioh_result_ptr`, `jackioh_result_len`), its
calling convention, its errors and its determinism are written in `c/README.md`, the contract. It reads no clock:
`ai_decide` and `lethal_hint` take `deadline_ms` for the browser's signature and ignore it, so the
search ends at its `budget`.

```
cargo test -p jackioh-wasm-c                                  # each export against its core body
cargo build -p jackioh-wasm-c --release --target wasm32-unknown-unknown
JACKIOH_WASM_C=1 JACKIOH_WASM_NORMAL=0 sh scripts/build-wasm.sh   # the same, and checks its imports with node; CI's `web (wasm c module)` runs it and records the size
```
