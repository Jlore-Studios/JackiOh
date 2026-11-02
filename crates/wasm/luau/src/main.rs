//! `jackioh-wasm-luau`: the Luau browser module (#919, SPEC §12.10, R915009): the normal module's
//! bindings (`crates/wasm`, SURFACE §10.1), each run with the card kit lent to the engine
//! (`custom::with_custom_scripts(KitHost)`), so a game without a custom card runs exactly as there. Its
//! own are `check_custom_card` and the setup check that takes a game's custom cards (R915005, R915006).
//! The web loads it lazily (`apps/web/src/wasm/luau.ts`), only for a practice game with a custom card and
//! for the creator.
//!
//! The boundary is C: `mlua` builds Luau for WebAssembly only through `wasm32-unknown-emscripten`, which
//! `wasm-bindgen` cannot target, so the module links through `emcc` and every binding is an `extern "C"`
//! function over NUL-terminated UTF-8 strings, with the names and JSON SURFACE §10.1 gives. A call that
//! fails returns NULL (or -1, 1) and stows its message for `jackioh_last_error`; `reduce`'s refusal of an
//! illegal action is the `ReduceResult`'s `error` (SURFACE §6.1). A panic is an engine invariant broken
//! (SURFACE §4.4.9), printed before the abort.
//!
//! Every body, the JSON helpers and the state cache live in `jackioh-wasm-core` (#1505). The pure crates
//! read no clock (CLAUDE.md rule 4, SURFACE §3): `ai_decide` hands the body `wall_now_ms`.

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::rc::Rc;
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

use jackioh_engine::custom::{check_custom_setup, custom_def, validate_custom_card, with_custom_scripts};
use jackioh_engine::prelude::IndexMap;
use jackioh_engine::{CardDef, CreateGameArgs, CustomCard, SetupKind};
use jackioh_luau::custom::{KitHost, check_custom_source};
use jackioh_wasm_core::{self as wasm_core, parse, to_json};
use serde_json::json;

// The C boundary: strings in, an owned string (or NULL) out

// The message of the last binding call that failed, on this thread: valid until the next failing
// call, so `index.ts` copies it at once.
thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("").expect("empty error"));
}

/// Stow `message` and answer NULL, the failure path every binding below returns through.
fn failed<T>(message: String) -> *mut T {
    LAST_ERROR.with(|last| {
        *last.borrow_mut() = CString::new(message).unwrap_or_else(|_| {
            CString::new("could not serialize the error message").expect("static message")
        });
    });
    std::ptr::null_mut()
}

/// The last failure's message, NUL-terminated UTF-8, valid until the next failing call.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_last_error() -> *const c_char {
    LAST_ERROR.with(|last| last.borrow().as_ptr())
}

/// Free a string this module returned, or an argument buffer `index.ts` is done with. Both live on the
/// one emscripten heap (Rust's allocator here is libc's `malloc`, which the glue's `stringToNewUTF8`
/// writes through), and every argument is valid UTF-8 with no interior NUL, so a buffer the glue
/// allocated reads back as exactly the `CString` this frees.
///
/// # Safety
///
/// `text` must be NULL or a pointer this module returned (or the glue's `stringToNewUTF8` wrote),
/// not freed before, and not read after.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jackioh_free(text: *mut c_char) {
    if !text.is_null() {
        unsafe {
            drop(CString::from_raw(text));
        }
    }
}

/// One string argument: NULL and non-UTF-8 are refused with the binding's own message.
fn arg<'a>(what: &str, text: *const c_char) -> Result<&'a str, String> {
    if text.is_null() {
        return Err(format!("{what}: the argument is missing"));
    }
    unsafe { CStr::from_ptr(text) }
        .to_str()
        .map_err(|error| format!("{what}: the argument is not UTF-8: {error}"))
}

/// Run a binding's body with the card kit lent to the engine (R915002): the owned string on success,
/// NULL (with the message stowed) on failure.
fn boundary(body: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    out(with_custom_scripts(Rc::new(KitHost), body))
}

/// Answer a binding: the owned string on success, NULL (with the message stowed) on failure.
fn out(result: Result<String, String>) -> *mut c_char {
    match result {
        Ok(text) => CString::new(text)
            .map(|owned| owned.into_raw())
            .unwrap_or_else(|_| failed("could not serialize the result: an interior NUL".to_owned())),
        Err(message) => failed(message),
    }
}

/// The setup checks, run before the engine is called: `create_game` panics on a bad setup (SURFACE §6.1),
/// and a panic in WebAssembly is an abort the page cannot catch. R180's handicaps come first, so a bad
/// deck size is named as the handicap's fault, then each deck against its seat's size (R184); the
/// functions are the engine's own (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs, kind: SetupKind) -> Result<(), String> {
    // R915005, R915006: the game's custom cards, each valid and given once, and every custom id a deck
    // names among them; the decks are then judged with their definitions.
    let customs: &[CustomCard] = args.custom_cards.as_deref().unwrap_or(&[]);
    check_custom_setup(customs, &args.decks)?;
    let custom_defs: IndexMap<String, CardDef> = customs
        .iter()
        .map(|card| (card.id.clone(), custom_def(card)))
        .collect();
    wasm_core::check_decks(args, &custom_defs, kind)
}

// The panic hook writes through emscripten, straight to the JavaScript console. Off the browser
// (a host `cargo test` or `clippy` run) there is no such symbol, so the hook writes to stderr there.
#[cfg(target_os = "emscripten")]
unsafe extern "C" {
    /// Write a NUL-terminated UTF-8 string to the JavaScript console as an error.
    ///
    /// # Safety
    ///
    /// `text` must point at a valid NUL-terminated string for the duration of the call.
    fn emscripten_console_error(text: *const c_char);
}

#[cfg(target_os = "emscripten")]
fn console_error(message: &CStr) {
    unsafe {
        emscripten_console_error(message.as_ptr());
    }
}

#[cfg(not(target_os = "emscripten"))]
fn console_error(message: &CStr) {
    eprintln!("jackioh-wasm-luau panic: {}", message.to_string_lossy());
}

/// Once, before any other binding: panics print on the console, and the catalog and the card scripts
/// are registered with the engine (`jackioh_cards::register_all`, idempotent).
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let message = CString::new(info.to_string()).unwrap_or_else(|_| {
                CString::new("a panic with a non-UTF-8 message").expect("static message")
            });
            console_error(&message);
        }));
    });
    jackioh_cards::register_all();
}

/// The registered catalog, `CardDefs`.
#[unsafe(no_mangle)]
pub extern "C" fn catalog() -> *mut c_char {
    out(wasm_core::catalog())
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
#[unsafe(no_mangle)]
pub extern "C" fn catalog_version() -> *mut c_char {
    out(Ok(wasm_core::catalog_version()))
}

/// R767000: the shipped versions this module carries for replays, oldest first: in the browser, those
/// from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM` on.
#[unsafe(no_mangle)]
pub extern "C" fn carried_versions() -> *mut c_char {
    out(wasm_core::carried_versions())
}

// The engine (SURFACE §6.1)

/// `GameState` from JSON arguments.
#[unsafe(no_mangle)]
pub extern "C" fn create_game(args_json: *const c_char) -> *mut c_char {
    boundary(|| wasm_core::create_game(arg("createGame", args_json)?, check_setup))
}

/// `ReduceResult`: `{ state, events, error? }`.
#[unsafe(no_mangle)]
pub extern "C" fn begin_game(state_json: *const c_char) -> *mut c_char {
    boundary(|| wasm_core::begin_game(arg("beginGame", state_json)?))
}

/// `ReduceResult`. An illegal action is refused in `error`, never NULL (SURFACE §6.1).
#[unsafe(no_mangle)]
pub extern "C" fn reduce(state_json: *const c_char, action_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = arg("reduce: the state", state_json)?;
        let action = arg("reduce: the action", action_json)?;
        wasm_core::reduce(state, action)
    })
}

/// `ActionBody[]`: everything `player` may legally do now.
#[unsafe(no_mangle)]
pub extern "C" fn legal_actions(state_json: *const c_char, player_id: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = arg("legalActions: the state", state_json)?;
        let player = arg("legalActions: the player", player_id)?;
        wasm_core::legal_actions(state, player)
    })
}

/// `PlayerView`: the only window the client has onto the game (SPEC §10.8).
#[unsafe(no_mangle)]
pub extern "C" fn view_for(state_json: *const c_char, player_id: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = arg("viewFor: the state", state_json)?;
        let player = arg("viewFor: the player", player_id)?;
        wasm_core::view_for(state, player)
    })
}

/// `"p1"`, `"p2"`, or `""` when no seat owes an action.
#[unsafe(no_mangle)]
pub extern "C" fn seat_to_act(state_json: *const c_char) -> *mut c_char {
    boundary(|| wasm_core::seat_to_act(arg("seatToAct", state_json)?))
}

/// SURFACE §5.2's hash, eight lower-case hex digits.
#[unsafe(no_mangle)]
pub extern "C" fn hash_state(state_json: *const c_char) -> *mut c_char {
    boundary(|| wasm_core::hash_state(arg("hashState", state_json)?))
}

/// `{ state, errors: [{ nonce, error }] }`. Setup is checked first; refused actions are collected.
#[unsafe(no_mangle)]
pub extern "C" fn fold(args_json: *const c_char) -> *mut c_char {
    boundary(|| {
        // The fold's input is `createGame`'s plus `log`, so the same JSON reads as both.
        wasm_core::fold(arg("fold", args_json)?, check_setup)
    })
}

/// R768: `ReplayOpen`. The setup is checked as `fold` checks it, under the catalog the game was played
/// on (R767000): this build's own or one it carries. A game of any other version is refused as
/// `earlier_patch` before its decks are read.
#[unsafe(no_mangle)]
pub extern "C" fn replay_open(args_json: *const c_char, record_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let record = arg("replayOpen: the record", record_json)?;
        let args = arg("replayOpen", args_json)?;
        wasm_core::replay_open(args, record, check_setup)
    })
}

/// R768: `ReplayPage`, `{ from, steps: [{ step, turn, view }], reduces }`.
#[unsafe(no_mangle)]
pub extern "C" fn replay_page(
    checkpoints_json: *const c_char,
    seat: *const c_char,
    from: u32,
    count: u32,
) -> *mut c_char {
    boundary(|| {
        let checkpoints = arg("replayPage: the checkpoints", checkpoints_json)?;
        let seat = arg("replayPage: the seat", seat)?;
        wasm_core::replay_page(checkpoints, seat, from, count)
    })
}

/// `LastBoardEntry[]`: the board `seat` takes away (R417, R508).
#[unsafe(no_mangle)]
pub extern "C" fn last_board_for(state_json: *const c_char, seat: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = arg("lastBoardFor: the state", state_json)?;
        let seat = arg("lastBoardFor: the seat", seat)?;
        wasm_core::last_board_for(state, seat)
    })
}

/// `"p1"` or `"p2"`: the seat the player who began in `home` plays now (R677).
#[unsafe(no_mangle)]
pub extern "C" fn seat_played_by(state_json: *const c_char, home: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = arg("seatPlayedBy: the state", state_json)?;
        let home = arg("seatPlayedBy: the home seat", home)?;
        wasm_core::seat_played_by(state, home)
    })
}

/// `CardInstance` JSON, or `"null"` when no card has that id.
#[unsafe(no_mangle)]
pub extern "C" fn find_instance(state_json: *const c_char, instance_id: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = arg("findInstance: the state", state_json)?;
        let instance = arg("findInstance: the instance", instance_id)?;
        wasm_core::find_instance(state, instance)
    })
}

/// `EmoteId[]`: emote hand `seat` is dealt for `seed` (R1341), matching the match actor (R1342).
#[unsafe(no_mangle)]
pub extern "C" fn deal_emote_hand(seed: *const c_char, seat: *const c_char) -> *mut c_char {
    boundary(|| {
        let seed = arg("dealEmoteHand: the seed", seed)?;
        let seat = arg("dealEmoteHand: the seat", seat)?;
        wasm_core::deal_emote_hand(seed, seat)
    })
}

/// R1420's preview, in the browser: from this call on, for the module's life (WebAssembly runs one
/// thread), `sets` (`SetName[]`) count as sets that ship. Only a module built with the `preview` feature
/// can preview (`scripts/build-wasm.sh`'s development, unit-test and e2e builds); any other refuses a set
/// (#552's play-through). Answers 0 on success, 1 with the message stowed for `jackioh_last_error`.
#[unsafe(no_mangle)]
pub extern "C" fn preview_sets(sets_json: *const c_char) -> i32 {
    match arg("previewSets: the sets", sets_json).and_then(wasm_core::preview_sets) {
        Ok(()) => 0,
        Err(message) => {
            failed::<()>(message);
            1
        }
    }
}

// The AI (SURFACE §9)

/// Whether `seat` owes an action the AI should take now (R187, R188): 1 for yes, 0 for no, and -1 with
/// the message stowed for `jackioh_last_error` on failure, which `index.ts` throws.
#[unsafe(no_mangle)]
pub extern "C" fn ai_to_act(state_json: *const c_char, seat: *const c_char) -> i32 {
    match (|| {
        let state = arg("aiToAct: the state", state_json)?;
        let seat = arg("aiToAct: the seat", seat)?;
        wasm_core::ai_to_act(state, seat)
    })() {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(message) => {
            failed::<()>(message);
            -1
        }
    }
}

/// The wall clock in milliseconds, as `Date.now()` reads it: libc clock emscripten provides.
fn wall_now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|age| age.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// `{ decision: Decision | null, rngCursor }`. The AI draws only from its own stream, rebuilt from
/// `(rngSeed, rngCursor)`; the cursor it stopped at comes back so the caller resumes it there. Once the
/// wall clock passes `deadline_ms` the search answers with its best so far; `deadline_ms <= 0` means no
/// clock, so the decision depends only on `(state, seed, cursor, budget)`.
#[unsafe(no_mangle)]
pub extern "C" fn ai_decide(
    state_json: *const c_char,
    seat: *const c_char,
    options_json: *const c_char,
    deadline_ms: f64,
) -> *mut c_char {
    boundary(|| {
        let state = arg("decide: the state", state_json)?;
        let seat = arg("decide: the seat", seat)?;
        let options = arg("decide: the options", options_json)?;
        wasm_core::ai_decide(state, seat, options, deadline_ms, wall_now_ms)
    })
}

/// R2102000: whether `seat` has a sure win this turn by what it can see, the practice AI's lethal hint:
/// 1, 0, or -1 with the message stowed, as `ai_to_act` answers. `deadline_ms` is a clock as
/// `ai_decide`'s is. Its solver plays lines through the engine, so the card kit is lent to it (R915002),
/// as `boundary` lends it.
#[unsafe(no_mangle)]
pub extern "C" fn lethal_hint(state_json: *const c_char, seat: *const c_char, deadline_ms: f64) -> i32 {
    match with_custom_scripts(Rc::new(KitHost), || {
        let state = arg("lethalHint: the state", state_json)?;
        let seat = arg("lethalHint: the seat", seat)?;
        wasm_core::lethal_hint(state, seat, deadline_ms, wall_now_ms)
    }) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(message) => {
            failed::<()>(message);
            -1
        }
    }
}

/// `{ deck, rngCursor }`. Every key but `rngSeed`, `rngCursor` and `size` is `AiDeckOptions`' own, read
/// by its own serde, so an absent `banned` (the shadow ban), an absent `theme` (roll one) and a `null`
/// one (none) keep their meanings, and so does an absent `leanSet` (no lean, R1370).
#[unsafe(no_mangle)]
pub extern "C" fn build_ai_deck(options_json: *const c_char) -> *mut c_char {
    boundary(|| wasm_core::build_ai_deck(arg("buildAiDeck", options_json)?))
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy, whose default skip set is
/// `jackioh_ai::random_action`'s, drawing from `(rng_seed, rng_cursor)`.
#[unsafe(no_mangle)]
pub extern "C" fn choose_action(
    state_json: *const c_char,
    seat: *const c_char,
    rng_seed: *const c_char,
    rng_cursor: f64,
) -> *mut c_char {
    boundary(|| {
        let state = arg("chooseAction: the state", state_json)?;
        let seat = arg("chooseAction: the seat", seat)?;
        let seed = arg("chooseAction: the seed", rng_seed)?;
        wasm_core::choose_action(state, seat, seed, rng_cursor)
    })
}

/// `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS, FIXTURE_BANNED_IDS, AI_DECK }` for practice, the tutorial
/// harness, the "More cards from the newest set" toggle (`AI_DECK.leanMinShare`, R1370) and their tests.
/// `FIXTURE_BANNED_IDS` is the frozen fixture pool (issue #1113), `SHADOW_BAN_IDS` as it stood when the
/// fixtures were dealt.
#[unsafe(no_mangle)]
pub extern "C" fn constants() -> *mut c_char {
    out(wasm_core::constants())
}

// The engine's tables the web's tests read (`subsystems.*`)

/// ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost, so the
/// block editor shows what the reducer will say and the client decides nothing (CLAUDE.md rule 7).
/// `prompt_json` is the open prompt's `{ craftMode, craftOffer }` (R1563007).
#[unsafe(no_mangle)]
pub extern "C" fn craft_preview(
    recipe_json: *const c_char,
    cost: f64,
    prompt_json: *const c_char,
) -> *mut c_char {
    boundary(|| {
        let recipe = arg("craftPreview: the recipe", recipe_json)?;
        let prompt = arg("craftPreview: the prompt", prompt_json)?;
        wasm_core::craft_preview(recipe, cost, prompt)
    })
}

/// `{ chaosEffects: [{label}], chaosPlusEffects: [{label}], chaosMedEffects: [{label}], heroPowerNames,
/// heroPowers: [{name, x, title, radiantTitle, label, radiantLabel}] }`.
#[unsafe(no_mangle)]
pub extern "C" fn engine_tables() -> *mut c_char {
    boundary(wasm_core::engine_tables)
}

// The validator (SPEC §9.4: one module, client and server)

/// One validator call by its TypeScript name; the input and the output are that function's argument
/// and result, as JSON (SURFACE §10.1).
#[unsafe(no_mangle)]
pub extern "C" fn validator(call: *const c_char, input_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let call = arg("validator: the call", call)?;
        let input = arg("validator: the input", input_json)?;
        wasm_core::validator(call, input)
    })
}

/// The program's entry: it does nothing; the client instantiates the module, then calls `init()`. A
/// `cdylib` link through `emcc` produces a bare `.wasm` with no JavaScript glue; only a program link
/// emits the `MODULARIZE` factory the client loads, so the bindings live in a binary.
fn main() {}

// Custom cards (#913, #919, SPEC §12)

/// The creator's check of one custom card, `{ frame, script, def }`: the frame's issues by the engine's
/// validator (`CustomIssue[]`, R915001), the script's problems, each on its line when it has one
/// (`{ line?, message }[]`, #913), and the definition the frame builds (R915000), which the creator's
/// preview draws. Both lists empty for a card that may be saved and played. No game is started.
#[unsafe(no_mangle)]
pub extern "C" fn check_custom_card(card_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let card: CustomCard = parse("checkCustomCard", arg("checkCustomCard", card_json)?)?;
        to_json(&json!({
            "frame": validate_custom_card(&card),
            "script": check_custom_source(&card),
            "def": custom_def(&card),
        }))
    })
}
