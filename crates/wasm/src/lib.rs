//! `jackioh-wasm`: the bindings the web client loads (docs/v0.3.0/SURFACE.md §10.1). JSON strings in
//! and out, nothing else crosses: the state is the `GameState` JSON object, which the client keeps
//! behind its opaque `EngineState` brand (CLAUDE.md rule 7) and hands back on every call.
//!
//! `apps/web/src/wasm/index.ts` is the only caller: the hotseat port (`game/engine.ts`), the practice
//! worker (`practice/core.ts`) and the `@jackioh/*` aliases (`apps/web/src/wire/`) reach the engine, the
//! AI and the validator through it.
//!
//! Errors: a JSON argument that does not parse, or a setup the engine refuses, returns `Err(JsError)`,
//! an `Error` thrown in JavaScript (SURFACE §10.1). `reduce` never throws on an illegal action: its
//! refusal is the `error` of the `ReduceResult` (SURFACE §6.1). A panic is an engine invariant broken
//! (SURFACE §4.4.9); `console_error_panic_hook` prints it on the console before the trap.
//!
//! Every body, the JSON helpers and the state cache live in `jackioh-wasm-core` (#1505), which the Luau
//! module wraps too; this module keeps the wasm-bindgen wrappers, the panic hook, the clock and the
//! custom-card refusal (R915009). The pure crates never read a clock (CLAUDE.md rule 4, SURFACE §3):
//! `ai_decide` hands the body `js_sys::Date::now` against the deadline the page computed.

use jackioh_engine::prelude::IndexMap;
use jackioh_engine::{CreateGameArgs, SetupKind};
use jackioh_wasm_core as wasm_core;
use wasm_bindgen::prelude::*;

/// One body's answer as JavaScript sees it: the value, or an `Error` thrown with the body's message.
fn js<T>(answer: Result<T, String>) -> Result<T, JsError> {
    answer.map_err(|message| JsError::new(&message))
}

/// The setup checks, run before the engine is called: `create_game` panics on a bad setup (SURFACE §6.1),
/// and a panic in WebAssembly is a trap the page cannot catch. R180's handicaps come first, so a bad deck
/// size is named as the handicap's fault, then each deck against its seat's size (R184); the functions
/// are the engine's own (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs, kind: SetupKind) -> Result<(), String> {
    // R915009: this module holds no Luau, so a game with custom cards runs in the Luau module.
    if args.custom_cards.as_ref().is_some_and(|cards| !cards.is_empty()) {
        return Err("a game with custom cards plays in the Luau module, not this one (R915009)".to_owned());
    }
    wasm_core::check_decks(args, &IndexMap::new(), kind)
}

// Boot and the catalog

/// Once, before any other binding: panics print on the console, and the catalog and the card scripts
/// are registered with the engine (`jackioh_cards::register_all`, idempotent).
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
    jackioh_cards::register_all();
}

/// The registered catalog, `CardDefs`.
#[wasm_bindgen]
pub fn catalog() -> Result<String, JsError> {
    js(wasm_core::catalog())
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
#[wasm_bindgen]
pub fn catalog_version() -> String {
    wasm_core::catalog_version()
}

/// R767000: the shipped versions this module carries for replays, oldest first: in the browser, those
/// from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM` on.
#[wasm_bindgen]
pub fn carried_versions() -> Result<String, JsError> {
    js(wasm_core::carried_versions())
}

// The engine (SURFACE §6.1)

/// `GameState` from JSON arguments.
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

/// SURFACE §5.2's hash, eight lower-case hex digits.
#[wasm_bindgen]
pub fn hash_state(state_json: &str) -> Result<String, JsError> {
    js(wasm_core::hash_state(state_json))
}

/// `{ state, errors: [{ nonce, error }] }`. Setup is checked first; refused actions are collected.
#[wasm_bindgen]
pub fn fold(args_json: &str) -> Result<String, JsError> {
    js(wasm_core::fold(args_json, check_setup))
}

/// R768: `ReplayOpen`. The setup is checked as `fold` checks it, under the catalog the game was played
/// on (R767000): this build's own or one it carries. A game of any other version is refused as
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

/// `CardInstance` JSON, or `"null"` when no card has that id.
#[wasm_bindgen]
pub fn find_instance(state_json: &str, instance_id: &str) -> Result<String, JsError> {
    js(wasm_core::find_instance(state_json, instance_id))
}

/// `EmoteId[]`: emote hand `seat` is dealt for `seed` (R1341), matching the match actor (R1342).
#[wasm_bindgen]
pub fn deal_emote_hand(seed: &str, seat: &str) -> Result<String, JsError> {
    js(wasm_core::deal_emote_hand(seed, seat))
}

/// R1420's preview, in the browser: from this call on, for the module's life (WebAssembly runs one
/// thread), `sets` (`SetName[]`) count as sets that ship. Only a module built with the `preview` feature
/// can preview (`scripts/build-wasm.sh` builds it for development, the unit tests and the e2e client;
/// apps/web's `prebuild` builds the production bundle without it); any other refuses a set. The hotseat
/// route calls it for its E2E injection's `preview`, in development builds only (#552's play-through).
#[wasm_bindgen]
pub fn preview_sets(sets_json: &str) -> Result<(), JsError> {
    js(wasm_core::preview_sets(sets_json))
}

// Duos co-op (#936 SPEC S4, S8): the wrappers over two duels, for `/dev/duos`'s four-seat hotseat.
// The bodies live in core behind its `duos` feature, which this crate's `duos` feature reaches.

/// `DuosState` from options JSON.
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

// The AI (SURFACE §9)

/// Whether `seat` owes an action the AI should take now (R187, R188).
#[wasm_bindgen]
pub fn ai_to_act(state_json: &str, seat: &str) -> Result<bool, JsError> {
    js(wasm_core::ai_to_act(state_json, seat))
}

/// `{ decision: Decision | null, rngCursor }`. The AI draws only from its own stream, rebuilt from
/// `(rngSeed, rngCursor)`; the cursor it stopped at comes back so the caller resumes it there. Once the
/// clock passes `deadline_ms` (a `Date.now()` time) the search answers with its best so far;
/// `deadline_ms <= 0` means no clock, so the decision depends only on `(state, seed, cursor, budget)`.
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

/// R2102000: whether `seat` has a sure win this turn by what it can see, the practice AI's lethal hint.
/// A search still running once the clock passes `deadline_ms` (as `ai_decide`'s) lights nothing;
/// `deadline_ms <= 0` means no clock.
#[wasm_bindgen]
pub fn lethal_hint(state_json: &str, seat: &str, deadline_ms: f64) -> Result<bool, JsError> {
    js(wasm_core::lethal_hint(
        state_json,
        seat,
        deadline_ms,
        js_sys::Date::now,
    ))
}

/// `{ deck, rngCursor }`. Every key but `rngSeed`, `rngCursor` and `size` is `AiDeckOptions`' own, read
/// by its own serde, so an absent `banned` (the shadow ban), an absent `theme` (roll one) and a `null`
/// one (none) keep their meanings, and so does an absent `leanSet` (no lean, R1370).
#[wasm_bindgen]
pub fn build_ai_deck(options_json: &str) -> Result<String, JsError> {
    js(wasm_core::build_ai_deck(options_json))
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy drawing from `(rng_seed, rng_cursor)`.
#[wasm_bindgen]
pub fn choose_action(
    state_json: &str,
    seat: &str,
    rng_seed: &str,
    rng_cursor: f64,
) -> Result<String, JsError> {
    js(wasm_core::choose_action(state_json, seat, rng_seed, rng_cursor))
}

/// `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS, FIXTURE_BANNED_IDS, AI_DECK }` for practice, the tutorial
/// harness, the "More cards from the newest set" toggle (`AI_DECK.leanMinShare`, R1370) and their tests.
/// `FIXTURE_BANNED_IDS` is the frozen fixture pool (issue #1113), `SHADOW_BAN_IDS` as it stood when the
/// fixtures were dealt.
#[wasm_bindgen]
pub fn constants() -> Result<String, JsError> {
    js(wasm_core::constants())
}

/// R1011000: the learned evaluation's parity fixture (`crates/ai/tests/fixtures/learned-parity.json`)
/// as this build computes it. The web's tests hold the WASM build to the fixture's numbers bit for bit,
/// as `cargo test -p jackioh-ai` holds the native one.
#[wasm_bindgen]
pub fn learned_parity(fixture_json: &str) -> Result<String, JsError> {
    js(wasm_core::learned_parity(fixture_json))
}

// The engine's tables the web's tests read (`subsystems.*`)

/// ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost, so the
/// block editor shows what the reducer will say and the client decides nothing (CLAUDE.md rule 7).
/// `prompt_json` is the open prompt's `{ craftMode, craftOffer }` (R1563007).
#[wasm_bindgen]
pub fn craft_preview(recipe_json: &str, cost: f64, prompt_json: &str) -> Result<String, JsError> {
    js(wasm_core::craft_preview(recipe_json, cost, prompt_json))
}

/// `{ chaosEffects: [{label}], chaosPlusEffects: [{label}], chaosMedEffects: [{label}], heroPowerNames,
/// heroPowers: [{name, x, title, radiantTitle, label, radiantLabel}] }`.
#[wasm_bindgen]
pub fn engine_tables() -> Result<String, JsError> {
    js(wasm_core::engine_tables())
}

// The validator (SPEC §9.4: one module, client and server)

/// One validator call by its TypeScript name; the input and the output are that function's argument
/// and result, as JSON (SURFACE §10.1). `checkDeckDraft` takes core's `DeckDraftRequest`.
#[wasm_bindgen]
pub fn validator(call: &str, input_json: &str) -> Result<String, JsError> {
    js(wasm_core::validator(call, input_json))
}
