//! `jackioh-wasm`: the bindings the web client loads (docs/v0.3.0/SURFACE.md §10.1). JSON strings in
//! and out, nothing else crosses: the state is the `GameState` JSON object, which the client keeps
//! behind its opaque `EngineState` brand (CLAUDE.md rule 7) and hands back on every call.
//!
//! `apps/web/src/wasm/index.ts` is the only caller. It parses and stringifies, names each binding as
//! the TypeScript engine named it (`createGame`, `reduce`, `decide`, …), and is what the hotseat
//! port (`game/engine.ts`), the practice worker (`practice/core.ts`) and the `@jackioh/*` aliases
//! (`apps/web/src/wire/`) reach the engine, the AI and the validator through.
//!
//! Errors. A binding whose JSON argument does not parse, or whose setup TypeScript refused with a
//! `throw` (a deck or a handicap `createGame` would not take, an unknown validator call), returns
//! `Err(JsError)`: JavaScript sees the return type SURFACE §10.1 gives and an `Error` thrown with
//! TypeScript's own message, where TypeScript threw. `reduce` never throws on an illegal action: its
//! refusal is the `error` of the `ReduceResult` (SURFACE §6.1). A panic is an engine invariant broken
//! (SURFACE §4.4.9); `console_error_panic_hook` prints it on the console before the trap.
//!
//! Every body, the JSON helpers and the state cache live in `jackioh-wasm-core` (`crates/wasm/core`,
//! #1505), which the Luau module wraps too; this module keeps only the wasm-bindgen wrappers, the
//! panic hook, the clock and the custom-card refusal (R915009).
//!
//! The clock. The pure crates never read one (CLAUDE.md rule 4, SURFACE §3). The AI's wall-clock cap
//! is this crate's: `ai_decide` hands the body `js_sys::Date::now` against the deadline the page
//! computed, as TypeScript's `shouldStop` read `performance.now()`.

use jackioh_engine::CreateGameArgs;
use jackioh_engine::prelude::IndexMap;
use jackioh_wasm_core as wasm_core;
use wasm_bindgen::prelude::*;

/// One body's answer as JavaScript sees it: the value, or an `Error` thrown with the body's message.
fn js<T>(answer: Result<T, String>) -> Result<T, JsError> {
    answer.map_err(|message| JsError::new(&message))
}

/// The checks TypeScript's `createGame` threw on, in its order (R180's handicaps first, so a bad
/// deck size is named as the handicap's fault, then each deck against its seat's size, R184), run
/// before the engine is called: Rust's `create_game` panics where TypeScript threw (SURFACE §6.1),
/// and a panic in WebAssembly is a trap the page cannot catch as the error TypeScript gave it. The
/// functions are the engine's own (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs) -> Result<(), String> {
    // R915009: this module holds no Luau, so a game with custom cards runs in the Luau module.
    if args.custom_cards.as_ref().is_some_and(|cards| !cards.is_empty()) {
        return Err("a game with custom cards plays in the Luau module, not this one (R915009)".to_owned());
    }
    wasm_core::check_decks(args, &IndexMap::new())
}

// ---------------------------------------------------------------------------------------------
// Boot and the catalog
// ---------------------------------------------------------------------------------------------

/// Once, before any other binding: panics print on the console, and the catalog and the card
/// scripts are registered with the engine (`jackioh_cards::register_all`, idempotent). This is what
/// TypeScript's `registerAll()` calls did; the web has nothing left to register.
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
    jackioh_cards::register_all();
}

/// The registered catalog, `CardDefs` (TS `registeredCatalog()`).
#[wasm_bindgen]
pub fn catalog() -> Result<String, JsError> {
    js(wasm_core::catalog())
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
#[wasm_bindgen]
pub fn catalog_version() -> String {
    wasm_core::catalog_version()
}

/// R767000: the shipped versions this module carries for replays, oldest first: in the browser,
/// those from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM` on.
#[wasm_bindgen]
pub fn carried_versions() -> Result<String, JsError> {
    js(wasm_core::carried_versions())
}

// ---------------------------------------------------------------------------------------------
// The engine (SURFACE §6.1)
// ---------------------------------------------------------------------------------------------

/// `GameState` (TS `createGame({ seed, decks, catalog?, handicaps?, lastBoards?, glitchBoards?, dealt? })`).
#[wasm_bindgen]
pub fn create_game(args_json: &str) -> Result<String, JsError> {
    js(wasm_core::create_game(args_json, check_setup))
}

/// `ReduceResult`: `{ state, events, error? }`.
#[wasm_bindgen]
pub fn begin_game(state_json: &str) -> Result<String, JsError> {
    js(wasm_core::begin_game(state_json))
}

/// `ReduceResult`. An illegal action is refused in `error`, never thrown (SURFACE §6.1).
#[wasm_bindgen]
pub fn reduce(state_json: &str, action_json: &str) -> Result<String, JsError> {
    js(wasm_core::reduce(state_json, action_json))
}

/// `ActionBody[]`: everything `player` may legally do now.
#[wasm_bindgen]
pub fn legal_actions(state_json: &str, player_id: &str) -> Result<String, JsError> {
    js(wasm_core::legal_actions(state_json, player_id))
}

/// `PlayerView`: the only window the client has onto the game (SPEC §10.8).
#[wasm_bindgen]
pub fn view_for(state_json: &str, player_id: &str) -> Result<String, JsError> {
    js(wasm_core::view_for(state_json, player_id))
}

/// `"p1"`, `"p2"`, or `""` when no seat owes an action.
#[wasm_bindgen]
pub fn seat_to_act(state_json: &str) -> Result<String, JsError> {
    js(wasm_core::seat_to_act(state_json))
}

/// SURFACE §5.2's hash, eight lower-case hex digits: TypeScript's `hashState`, bit for bit.
#[wasm_bindgen]
pub fn hash_state(state_json: &str) -> Result<String, JsError> {
    js(wasm_core::hash_state(state_json))
}

/// `{ state, errors: [{ nonce, error }] }` (TS `fold`). The setup is checked first, as TypeScript's
/// `fold` threw on it through `createGame`; a refused action in the log is collected, not thrown.
#[wasm_bindgen]
pub fn fold(args_json: &str) -> Result<String, JsError> {
    js(wasm_core::fold(args_json, check_setup))
}

/// R768: `ReplayOpen`. The setup is checked as `fold` checks it, under the catalog the game was
/// played on (R767000): this build's own or one it carries. A game of any other version is refused as
/// `earlier_patch` before its decks are read.
#[wasm_bindgen]
pub fn replay_open(args_json: &str, record_json: &str) -> Result<String, JsError> {
    js(wasm_core::replay_open(args_json, record_json, check_setup))
}

/// R768: `ReplayPage`, `{ from, steps: [{ step, turn, view }], reduces }`.
#[wasm_bindgen]
pub fn replay_page(checkpoints_json: &str, seat: &str, from: u32, count: u32) -> Result<String, JsError> {
    js(wasm_core::replay_page(checkpoints_json, seat, from, count))
}

/// `LastBoardEntry[]`: the board `seat` takes away (R417, R508).
#[wasm_bindgen]
pub fn last_board_for(state_json: &str, seat: &str) -> Result<String, JsError> {
    js(wasm_core::last_board_for(state_json, seat))
}

/// `"p1"` or `"p2"`: the seat the player who began in `home` plays now (R677).
#[wasm_bindgen]
pub fn seat_played_by(state_json: &str, home: &str) -> Result<String, JsError> {
    js(wasm_core::seat_played_by(state_json, home))
}

/// `CardInstance` JSON, or `"null"` when no card has that id (`state.rs`'s `find_instance`).
#[wasm_bindgen]
pub fn find_instance(state_json: &str, instance_id: &str) -> Result<String, JsError> {
    js(wasm_core::find_instance(state_json, instance_id))
}

/// `EmoteId[]`: the emote hand `seat` is dealt in the game seeded `seed` (R1341), so hotseat and
/// practice deal both seats exactly as the server's match actor does (R1342).
#[wasm_bindgen]
pub fn deal_emote_hand(seed: &str, seat: &str) -> Result<String, JsError> {
    js(wasm_core::deal_emote_hand(seed, seat))
}

/// R1420's preview, in the browser: from this call on the module treats `sets` (`SetName[]`) as sets
/// that ship, as the testkit's `preview_sets` does on a test's thread, so a deck may hold their cards
/// and a pool that names no set reaches them. WebAssembly runs one thread, so the preview lasts the
/// module's life. Only a module built with the `preview` feature can preview (the engine's testkit:
/// `scripts/build-wasm.sh` builds it for development, the unit tests and the e2e client, and apps/web's
/// `prebuild` builds the production bundle without it); any other refuses a set. The hotseat route
/// calls it for its E2E injection's `preview`, in development builds only (#552's play-through).
#[wasm_bindgen]
pub fn preview_sets(sets_json: &str) -> Result<(), JsError> {
    js(wasm_core::preview_sets(sets_json))
}

// ---------------------------------------------------------------------------------------------
// Duos co-op (#936 SPEC S4, S8): the wrappers over two duels, for `/dev/duos`'s four-seat hotseat.
// The bodies live in core behind its `duos` feature, which this crate's `duos` feature reaches.
// ---------------------------------------------------------------------------------------------

/// `DuosState` (TS `duosCreate({ seed, decks: { a1, a2, b1, b2 }, catalog?, teams?, handicaps?, dealt? })`).
#[wasm_bindgen]
pub fn duos_create(options_json: &str) -> Result<String, JsError> {
    js(wasm_core::duos_create(options_json))
}

/// `{ state, events, error? }`: both duels set up, all four mulligans open (B1, B2).
#[wasm_bindgen]
pub fn duos_begin(state_json: &str) -> Result<String, JsError> {
    js(wasm_core::duos_begin(state_json))
}

/// `{ state, events, error? }` for one `DuosAction`; an illegal one is refused in `error`, never thrown.
#[wasm_bindgen]
pub fn duos_reduce(state_json: &str, action_json: &str) -> Result<String, JsError> {
    js(wasm_core::duos_reduce(state_json, action_json))
}

/// `DuosActionBody[]`: everything `seat` may do now.
#[wasm_bindgen]
pub fn duos_legal_actions(state_json: &str, seat: &str) -> Result<String, JsError> {
    js(wasm_core::duos_legal_actions(state_json, seat))
}

/// `DuosView`: the only window the client has onto a Duos game (B23).
#[wasm_bindgen]
pub fn duos_view_for(state_json: &str, seat: &str) -> Result<String, JsError> {
    js(wasm_core::duos_view_for(state_json, seat))
}

// ---------------------------------------------------------------------------------------------
// The AI (SURFACE §9)
// ---------------------------------------------------------------------------------------------

/// Whether `seat` owes an action the AI should take now (R187, R188).
#[wasm_bindgen]
pub fn ai_to_act(state_json: &str, seat: &str) -> Result<bool, JsError> {
    js(wasm_core::ai_to_act(state_json, seat))
}

/// `{ decision: Decision | null, rngCursor }`. The AI draws only from its own stream, rebuilt here
/// from `(rngSeed, rngCursor)`; the cursor it stopped at comes back so the caller resumes it there.
/// `deadline_ms` is a `Date.now()` time: the search stops and answers with its best so far once the
/// clock passes it (TS `shouldStop`). `deadline_ms <= 0` means no clock, so the decision depends
/// only on `(state, seed, cursor, budget)`.
#[wasm_bindgen]
pub fn ai_decide(
    state_json: &str,
    seat: &str,
    options_json: &str,
    deadline_ms: f64,
) -> Result<String, JsError> {
    js(wasm_core::ai_decide(
        state_json,
        seat,
        options_json,
        deadline_ms,
        js_sys::Date::now,
    ))
}

/// `{ deck, rngCursor }` (TS `buildAiDeck(createRng(rngSeed, rngCursor), size, options)`). Every key
/// but `rngSeed`, `rngCursor` and `size` is `AiDeckOptions`' own, read by its own serde, so an absent
/// `banned` (the shadow ban), an absent `theme` (roll one) and a `null` one (none) keep their meanings,
/// and so does an absent `leanSet` (no lean, R1370).
#[wasm_bindgen]
pub fn build_ai_deck(options_json: &str) -> Result<String, JsError> {
    js(wasm_core::build_ai_deck(options_json))
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy (TS
/// `subsystems.chooseAction(state, seat, rng)`, whose default skip set is `jackioh_ai::random_action`'s),
/// drawing from `(rng_seed, rng_cursor)`.
#[wasm_bindgen]
pub fn choose_action(
    state_json: &str,
    seat: &str,
    rng_seed: &str,
    rng_cursor: f64,
) -> Result<String, JsError> {
    js(wasm_core::choose_action(state_json, seat, rng_seed, rng_cursor))
}

/// `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS, FIXTURE_BANNED_IDS, AI_DECK }` for practice,
/// the tutorial harness, the "More cards from the newest set" toggle (`AI_DECK.leanMinShare`,
/// R1370) and their tests. `SHADOW_BAN_IDS` is TypeScript's `Object.keys(SHADOW_BAN).sort()`
/// (`jackioh_ai::SHADOW_BAN_IDS`); `FIXTURE_BANNED_IDS` is the frozen fixture pool (issue #1113),
/// the same ids in the same order.
#[wasm_bindgen]
pub fn constants() -> Result<String, JsError> {
    js(wasm_core::constants())
}

/// R1011000: the learned evaluation's parity fixture (`crates/ai/tests/fixtures/learned-parity.json`)
/// as this build computes it: its integer network's correction for each case, in order. The web's
/// tests hold the WASM build to the fixture's numbers bit for bit, as `cargo test -p jackioh-ai` holds
/// the native one.
#[wasm_bindgen]
pub fn learned_parity(fixture_json: &str) -> Result<String, JsError> {
    js(wasm_core::learned_parity(fixture_json))
}

// ---------------------------------------------------------------------------------------------
// The engine's tables the web's tests read (`subsystems.*`)
// ---------------------------------------------------------------------------------------------

/// ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost —
/// a `CraftPreview` — so the block editor shows what the reducer will say and the client decides
/// nothing (CLAUDE.md rule 7).
#[wasm_bindgen]
pub fn craft_preview(recipe_json: &str, cost: f64) -> Result<String, JsError> {
    js(wasm_core::craft_preview(recipe_json, cost))
}

/// `{ chaosEffects: [{label}], chaosPlusEffects: [{label}], chaosMedEffects: [{label}], heroPowerNames,
/// heroPowers: [{name, x, title, radiantTitle, label, radiantLabel}] }`. `heroPowerNames` is
/// TypeScript's `HERO_POWERS.map((power) => power.name)`.
#[wasm_bindgen]
pub fn engine_tables() -> Result<String, JsError> {
    js(wasm_core::engine_tables())
}

// ---------------------------------------------------------------------------------------------
// The validator (SPEC §9.4: one module, client and server)
// ---------------------------------------------------------------------------------------------

/// One validator call by its TypeScript name; the input and the output are that function's
/// argument and result, as JSON (SURFACE §10.1). `checkDeckDraft` takes `DeckDraftRequest` above.
#[wasm_bindgen]
pub fn validator(call: &str, input_json: &str) -> Result<String, JsError> {
    js(wasm_core::validator(call, input_json))
}
