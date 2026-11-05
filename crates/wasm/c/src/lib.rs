//! `jackioh-wasm-c`: the engine's C boundary (#3093, SURFACE §10.5): every binding the browser module
//! binds (`crates/wasm`, SURFACE §10.1) as an `extern "C"` function over JSON strings, built as a plain
//! `wasm32-unknown-unknown` module with no host imports: no wasm-bindgen glue, no JavaScript and no
//! system interface, so any host that runs WebAssembly (a Roblox port's translation among them, #2898)
//! instantiates it with an empty import object. It is a third wrapper over `jackioh-wasm-core`'s bodies
//! (#1505), beside `crates/wasm` (wasm-bindgen) and `crates/wasm/luau` (the same C names through
//! emscripten, with the Luau VM, which part 5 of #2956 deletes); the bodies are not copied.
//! `crates/wasm/tests/export_parity.rs` holds this module to every binding the browser module binds.
//! `README.md` is the contract, written for a host that is not JavaScript.
//!
//! The calling convention. WebAssembly passes numbers only, so a string is a pointer and a length into
//! the module's memory, and a result is read back through the module as well:
//!
//! - a host writes each argument's UTF-8 bytes into a buffer from `jackioh_alloc(len)` and frees it with
//!   `jackioh_free(ptr, len)` after the call (a string is not NUL-terminated; an empty one is `len` 0);
//! - every binding returns a status, `STATUS_OK` or `STATUS_FAILED`, and leaves its answer in the
//!   module's result buffer: the JSON text on success, the failure's message on failure. The host reads
//!   `jackioh_result_len()` bytes at `jackioh_result_ptr()` and copies them out before the next call,
//!   which replaces them. The module owns that buffer; the host never frees it;
//! - `reduce` refuses an illegal action in the `ReduceResult`'s `error`, never with the failed status
//!   (SURFACE §6.1). A panic is an engine invariant broken (SURFACE §4.4.9): a trap, after which the
//!   instance is dead, but the panic's text is still in the result buffer for the host to read.
//!
//! The pure crates read no clock (CLAUDE.md rule 4, SURFACE §3) and this module imports none, so
//! `ai_decide` and `lethal_hint` hand their bodies no clock: the search ends at its `budget` alone, and
//! the decision depends only on `(state, seed, cursor, budget)`.

use std::cell::RefCell;

use jackioh_engine::prelude::IndexMap;
use jackioh_engine::{CreateGameArgs, SetupKind};
use jackioh_wasm_core as wasm_core;

// The memory interface: buffers in, one result buffer out

/// The status of a binding that answered: the result buffer holds its JSON text.
pub const STATUS_OK: i32 = 0;
/// The status of a binding that failed: the result buffer holds its message, as text.
pub const STATUS_FAILED: i32 = 1;

thread_local! {
    /// The last binding's answer, or its failure's message, or a panic's text; replaced by every call.
    /// WebAssembly runs one thread, and `thread_local!` is how a module keeps data between calls without
    /// a `static mut`.
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A buffer of `len` bytes (at least one, so its address is never 0) for the host to write an argument
/// into. Free it with `jackioh_free(ptr, len)`, giving the same `len`.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len.max(1)].into_boxed_slice()).cast::<u8>()
}

/// Free a buffer `jackioh_alloc` returned. A null `ptr` is ignored.
///
/// # Safety
///
/// `ptr` must be null or a pointer `jackioh_alloc(len)` returned, with this same `len`, not freed
/// before and not read after. The result buffer is not one of these: the module owns it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jackioh_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len.max(1))));
        }
    }
}

/// Where the last binding's answer starts. Valid until the next call into the module; do not read it
/// when `jackioh_result_len()` is 0.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_result_ptr() -> *const u8 {
    RESULT.with(|result| result.borrow().as_ptr())
}

/// How many bytes the last binding's answer holds: UTF-8 text, not NUL-terminated.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_result_len() -> usize {
    RESULT.with(|result| result.borrow().len())
}

/// Stow a binding's answer in the result buffer and return its status: `STATUS_OK` with the text,
/// `STATUS_FAILED` with the message.
fn answer(result: Result<String, String>) -> i32 {
    let (status, text) = match result {
        Ok(text) => (STATUS_OK, text),
        Err(message) => (STATUS_FAILED, message),
    };
    RESULT.with(|result| *result.borrow_mut() = text.into_bytes());
    status
}

/// One string argument: `len` bytes at `ptr`. A null `ptr` with a length and non-UTF-8 bytes are refused
/// with the binding's own message; no length is the empty string, whatever `ptr` is.
fn arg<'a>(what: &str, ptr: *const u8, len: usize) -> Result<&'a str, String> {
    if len == 0 {
        return Ok("");
    }
    if ptr.is_null() {
        return Err(format!("{what}: the argument is missing"));
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) })
        .map_err(|error| format!("{what}: the argument is not UTF-8: {error}"))
}

/// Run a binding's body and answer it.
fn boundary(body: impl FnOnce() -> Result<String, String>) -> i32 {
    answer(body())
}

/// The deadline `ai_decide` and `lethal_hint` hand their bodies: none, so the body reads no clock.
const NO_DEADLINE_MS: f64 = 0.0;

/// The clock the bodies take, which no body reads under `NO_DEADLINE_MS`: this module has none.
fn no_clock() -> f64 {
    NO_DEADLINE_MS
}

/// The setup checks, run before the engine is called: `create_game` panics on a bad setup (SURFACE §6.1),
/// and a panic in WebAssembly is a trap. R180's handicaps come first, so a bad deck size is named as the
/// handicap's fault, then each deck against its seat's size (R184); the functions are the engine's own
/// (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs, kind: SetupKind) -> Result<(), String> {
    // R915009: this module holds no Luau, so it plays no game with custom cards.
    if args.custom_cards.as_ref().is_some_and(|cards| !cards.is_empty()) {
        return Err("a game with custom cards is not played by this module (R915009)".to_owned());
    }
    wasm_core::check_decks(args, &IndexMap::new(), kind)
}

// Boot and the catalog

/// Once, before any other binding: a panic's text is kept in the result buffer for the host to read
/// after the trap (WebAssembly only: a native test keeps its own harness's hook), and the catalog and
/// the card scripts are registered with the engine (`jackioh_cards::register_all`, idempotent).
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    #[cfg(target_arch = "wasm32")]
    {
        static HOOK: std::sync::Once = std::sync::Once::new();
        HOOK.call_once(|| {
            std::panic::set_hook(Box::new(|info| {
                // `try_with` and `try_borrow_mut`: a panic while the buffer is borrowed, or while the
                // thread's data is torn down, leaves it as it was rather than panicking again.
                let _ = RESULT.try_with(|result| {
                    if let Ok(mut result) = result.try_borrow_mut() {
                        *result = info.to_string().into_bytes();
                    }
                });
            }));
        });
    }
    jackioh_cards::register_all();
}

/// The registered catalog, `CardDefs`.
#[unsafe(no_mangle)]
pub extern "C" fn catalog() -> i32 {
    answer(wasm_core::catalog())
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
#[unsafe(no_mangle)]
pub extern "C" fn catalog_version() -> i32 {
    answer(Ok(wasm_core::catalog_version()))
}

/// R767000: the shipped versions this module carries for replays, oldest first: in the browser, those
/// from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM` on.
#[unsafe(no_mangle)]
pub extern "C" fn carried_versions() -> i32 {
    answer(wasm_core::carried_versions())
}

// The dungeon run (docs/dungeon-run.md DR14 part C)

// A run-rule refusal is a returned value (`{ "error": … }`, or `run_check_setup`'s nullable reason),
// never the failed status: only JSON that does not parse, or an argument that is not text, is one.

/// `RunContent`: `run_catalog_content` over the compiled-in catalog and the Sandbox's custom cards.
#[unsafe(no_mangle)]
pub extern "C" fn run_content(custom_cards_json_ptr: *const u8, custom_cards_json_len: usize) -> i32 {
    boundary(|| {
        wasm_core::run_content(arg(
            "runContent: the custom cards",
            custom_cards_json_ptr,
            custom_cards_json_len,
        )?)
    })
}

/// `null` when the setup may start a run, else the reason.
#[unsafe(no_mangle)]
pub extern "C" fn run_check_setup(
    setup_json_ptr: *const u8,
    setup_json_len: usize,
    unlocks_json_ptr: *const u8,
    unlocks_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
) -> i32 {
    boundary(|| {
        let setup = arg("runCheckSetup: the setup", setup_json_ptr, setup_json_len)?;
        let unlocks = arg("runCheckSetup: the unlocks", unlocks_json_ptr, unlocks_json_len)?;
        let content = arg("runCheckSetup: the content", content_json_ptr, content_json_len)?;
        wasm_core::run_check_setup(setup, unlocks, content)
    })
}

/// `{ state }` for a run started, `{ error }` for a setup the run refuses.
#[unsafe(no_mangle)]
pub extern "C" fn run_create(
    setup_json_ptr: *const u8,
    setup_json_len: usize,
    unlocks_json_ptr: *const u8,
    unlocks_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
) -> i32 {
    boundary(|| {
        let setup = arg("runCreate: the setup", setup_json_ptr, setup_json_len)?;
        let unlocks = arg("runCreate: the unlocks", unlocks_json_ptr, unlocks_json_len)?;
        let content = arg("runCreate: the content", content_json_ptr, content_json_len)?;
        wasm_core::run_create(setup, unlocks, content)
    })
}

/// `{ state }` for the run after one action, `{ error }` for an action the run refuses.
#[unsafe(no_mangle)]
pub extern "C" fn run_reduce(
    state_json_ptr: *const u8,
    state_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
    action_json_ptr: *const u8,
    action_json_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("runReduce: the state", state_json_ptr, state_json_len)?;
        let content = arg("runReduce: the content", content_json_ptr, content_json_len)?;
        let action = arg("runReduce: the action", action_json_ptr, action_json_len)?;
        wasm_core::run_reduce(state, content, action)
    })
}

/// `{ state }` for a run file folded, `{ error }` naming the first refused step.
#[unsafe(no_mangle)]
pub extern "C" fn run_fold(
    file_json_ptr: *const u8,
    file_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
) -> i32 {
    boundary(|| {
        let file = arg("runFold: the file", file_json_ptr, file_json_len)?;
        let content = arg("runFold: the content", content_json_ptr, content_json_len)?;
        wasm_core::run_fold(file, content)
    })
}

/// The fight being played, or `null` outside one.
#[unsafe(no_mangle)]
pub extern "C" fn run_fight_setup(
    state_json_ptr: *const u8,
    state_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("runFightSetup: the state", state_json_ptr, state_json_len)?;
        let content = arg("runFightSetup: the content", content_json_ptr, content_json_len)?;
        wasm_core::run_fight_setup(state, content)
    })
}

/// `{ action }` for the `FightOver` a finished fight sends, `{ error }` for a game that reports none.
#[unsafe(no_mangle)]
pub extern "C" fn run_fight_report(
    game_json_ptr: *const u8,
    game_json_len: usize,
    human_json_ptr: *const u8,
    human_json_len: usize,
) -> i32 {
    boundary(|| {
        let game = arg("runFightReport: the game", game_json_ptr, game_json_len)?;
        let human = arg("runFightReport: the human", human_json_ptr, human_json_len)?;
        wasm_core::run_fight_report(game, human)
    })
}

/// The run's score so far.
#[unsafe(no_mangle)]
pub extern "C" fn run_score(
    state_json_ptr: *const u8,
    state_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("runScore: the state", state_json_ptr, state_json_len)?;
        let content = arg("runScore: the content", content_json_ptr, content_json_len)?;
        wasm_core::run_score(state, content)
    })
}

/// The Embers a score pays.
#[unsafe(no_mangle)]
pub extern "C" fn run_embers(score_json_ptr: *const u8, score_json_len: usize) -> i32 {
    boundary(|| wasm_core::run_embers(arg("runEmbers: the score", score_json_ptr, score_json_len)?))
}

/// The run's max HP, before no fight and after every artifact and blessing.
#[unsafe(no_mangle)]
pub extern "C" fn run_max_hp(
    state_json_ptr: *const u8,
    state_json_len: usize,
    content_json_ptr: *const u8,
    content_json_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("runMaxHp: the state", state_json_ptr, state_json_len)?;
        let content = arg("runMaxHp: the content", content_json_ptr, content_json_len)?;
        wasm_core::run_max_hp(state, content)
    })
}

/// The Embers that raise the starting deck from `level` to the next, or `null` at the ladder's top.
#[unsafe(no_mangle)]
pub extern "C" fn run_level_cost(level_json_ptr: *const u8, level_json_len: usize) -> i32 {
    boundary(|| wasm_core::run_level_cost(arg("runLevelCost: the level", level_json_ptr, level_json_len)?))
}

/// The run's numbers the screens show, so the client states none (CLAUDE.md rule 9).
#[unsafe(no_mangle)]
pub extern "C" fn run_constants() -> i32 {
    boundary(wasm_core::run_constants)
}

/// A seed of `RUN_SEED_LENGTH` capital letters drawn from any text.
#[unsafe(no_mangle)]
pub extern "C" fn run_seed(source_json_ptr: *const u8, source_json_len: usize) -> i32 {
    boundary(|| wasm_core::run_seed(arg("runSeed: the source", source_json_ptr, source_json_len)?))
}

/// The daily run's seed: the date's and the day's run number.
#[unsafe(no_mangle)]
pub extern "C" fn run_daily_seed(
    date_json_ptr: *const u8,
    date_json_len: usize,
    n_json_ptr: *const u8,
    n_json_len: usize,
) -> i32 {
    boundary(|| {
        let date = arg("runDailySeed: the date", date_json_ptr, date_json_len)?;
        let n = arg("runDailySeed: the run number", n_json_ptr, n_json_len)?;
        wasm_core::run_daily_seed(date, n)
    })
}

// The engine (SURFACE §6.1)

/// `GameState` from JSON arguments.
#[unsafe(no_mangle)]
pub extern "C" fn create_game(args_json_ptr: *const u8, args_json_len: usize) -> i32 {
    boundary(|| wasm_core::create_game(arg("createGame", args_json_ptr, args_json_len)?, check_setup))
}

/// `ReduceResult`: `{ state, events, error? }`.
#[unsafe(no_mangle)]
pub extern "C" fn begin_game(state_json_ptr: *const u8, state_json_len: usize) -> i32 {
    boundary(|| wasm_core::begin_game(arg("beginGame", state_json_ptr, state_json_len)?))
}

/// `ReduceResult`. An illegal action is refused in `error`, never the failed status (SURFACE §6.1).
#[unsafe(no_mangle)]
pub extern "C" fn reduce(
    state_json_ptr: *const u8,
    state_json_len: usize,
    action_json_ptr: *const u8,
    action_json_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("reduce: the state", state_json_ptr, state_json_len)?;
        let action = arg("reduce: the action", action_json_ptr, action_json_len)?;
        wasm_core::reduce(state, action)
    })
}

/// `ActionBody[]`: everything `player` may legally do now.
#[unsafe(no_mangle)]
pub extern "C" fn legal_actions(
    state_json_ptr: *const u8,
    state_json_len: usize,
    player_id_ptr: *const u8,
    player_id_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("legalActions: the state", state_json_ptr, state_json_len)?;
        let player = arg("legalActions: the player", player_id_ptr, player_id_len)?;
        wasm_core::legal_actions(state, player)
    })
}

/// `PlayerView`: the only window the client has onto the game (SPEC §10.8).
#[unsafe(no_mangle)]
pub extern "C" fn view_for(
    state_json_ptr: *const u8,
    state_json_len: usize,
    player_id_ptr: *const u8,
    player_id_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("viewFor: the state", state_json_ptr, state_json_len)?;
        let player = arg("viewFor: the player", player_id_ptr, player_id_len)?;
        wasm_core::view_for(state, player)
    })
}

/// `"p1"`, `"p2"`, or `""` when no seat owes an action.
#[unsafe(no_mangle)]
pub extern "C" fn seat_to_act(state_json_ptr: *const u8, state_json_len: usize) -> i32 {
    boundary(|| wasm_core::seat_to_act(arg("seatToAct", state_json_ptr, state_json_len)?))
}

/// SURFACE §5.2's hash, eight lower-case hex digits.
#[unsafe(no_mangle)]
pub extern "C" fn hash_state(state_json_ptr: *const u8, state_json_len: usize) -> i32 {
    boundary(|| wasm_core::hash_state(arg("hashState", state_json_ptr, state_json_len)?))
}

/// `{ state, errors: [{ nonce, error }] }`. Setup is checked first; refused actions are collected.
#[unsafe(no_mangle)]
pub extern "C" fn fold(args_json_ptr: *const u8, args_json_len: usize) -> i32 {
    boundary(|| {
        // The fold's input is `createGame`'s plus `log`, so the same JSON reads as both.
        wasm_core::fold(arg("fold", args_json_ptr, args_json_len)?, check_setup)
    })
}

/// R768: `ReplayOpen`. The setup is checked as `fold` checks it, under the catalog the game was played
/// on (R767000): this build's own or one it carries. A game of any other version is refused as
/// `earlier_patch` before its decks are read.
#[unsafe(no_mangle)]
pub extern "C" fn replay_open(
    args_json_ptr: *const u8,
    args_json_len: usize,
    record_json_ptr: *const u8,
    record_json_len: usize,
) -> i32 {
    boundary(|| {
        let record = arg("replayOpen: the record", record_json_ptr, record_json_len)?;
        let args = arg("replayOpen", args_json_ptr, args_json_len)?;
        wasm_core::replay_open(args, record, check_setup)
    })
}

/// R768: `ReplayPage`, `{ from, steps: [{ step, turn, view }], reduces }`.
#[unsafe(no_mangle)]
pub extern "C" fn replay_page(
    checkpoints_json_ptr: *const u8,
    checkpoints_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
    from: u32,
    count: u32,
) -> i32 {
    boundary(|| {
        let checkpoints = arg(
            "replayPage: the checkpoints",
            checkpoints_json_ptr,
            checkpoints_json_len,
        )?;
        let seat = arg("replayPage: the seat", seat_ptr, seat_len)?;
        wasm_core::replay_page(checkpoints, seat, from, count)
    })
}

/// `LastBoardEntry[]`: the board `seat` takes away (R417, R508).
#[unsafe(no_mangle)]
pub extern "C" fn last_board_for(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("lastBoardFor: the state", state_json_ptr, state_json_len)?;
        let seat = arg("lastBoardFor: the seat", seat_ptr, seat_len)?;
        wasm_core::last_board_for(state, seat)
    })
}

/// `"p1"` or `"p2"`: the seat the player who began in `home` plays now (R677).
#[unsafe(no_mangle)]
pub extern "C" fn seat_played_by(
    state_json_ptr: *const u8,
    state_json_len: usize,
    home_ptr: *const u8,
    home_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("seatPlayedBy: the state", state_json_ptr, state_json_len)?;
        let home = arg("seatPlayedBy: the home seat", home_ptr, home_len)?;
        wasm_core::seat_played_by(state, home)
    })
}

/// `CardInstance` JSON, or `"null"` when no card has that id.
#[unsafe(no_mangle)]
pub extern "C" fn find_instance(
    state_json_ptr: *const u8,
    state_json_len: usize,
    instance_id_ptr: *const u8,
    instance_id_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("findInstance: the state", state_json_ptr, state_json_len)?;
        let instance = arg("findInstance: the instance", instance_id_ptr, instance_id_len)?;
        wasm_core::find_instance(state, instance)
    })
}

/// R3100000: `CombatForecast`, `{ attacks: AttackForecast[], lethal }`: what each attack `seat` may declare
/// would do, as the board shows it, for the tutorial's coach (`engine::combat_forecast`).
#[unsafe(no_mangle)]
pub extern "C" fn combat_forecast(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("combatForecast: the state", state_json_ptr, state_json_len)?;
        let seat = arg("combatForecast: the seat", seat_ptr, seat_len)?;
        wasm_core::combat_forecast(state, seat)
    })
}

/// `EmoteId[]`: emote hand `seat` is dealt for `seed` (R1341), matching the match actor (R1342).
#[unsafe(no_mangle)]
pub extern "C" fn deal_emote_hand(
    seed_ptr: *const u8,
    seed_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
) -> i32 {
    boundary(|| {
        let seed = arg("dealEmoteHand: the seed", seed_ptr, seed_len)?;
        let seat = arg("dealEmoteHand: the seat", seat_ptr, seat_len)?;
        wasm_core::deal_emote_hand(seed, seat)
    })
}

/// R1420's preview: from this call on, for the module's life (WebAssembly runs one thread), `sets`
/// (`SetName[]`) count as sets that ship. Only a module built with the `preview` feature can preview
/// (`JACKIOH_WASM_C=1` builds without it); any other refuses a set (#552's play-through). The result is
/// empty on success.
#[unsafe(no_mangle)]
pub extern "C" fn preview_sets(sets_json_ptr: *const u8, sets_json_len: usize) -> i32 {
    boundary(|| {
        wasm_core::preview_sets(arg("previewSets: the sets", sets_json_ptr, sets_json_len)?)
            .map(|()| String::new())
    })
}

// Duos co-op (#936 SPEC S4, S8): the wrappers over two duels. The bodies live in core behind its `duos`
// feature, which this crate's `duos` feature reaches; without it each answers the reason, as failed.

/// `DuosState` from options JSON.
#[unsafe(no_mangle)]
pub extern "C" fn duos_create(options_json_ptr: *const u8, options_json_len: usize) -> i32 {
    boundary(|| wasm_core::duos_create(arg("duosCreate", options_json_ptr, options_json_len)?))
}

/// `{ state, events, error? }`: both duels set up, all four mulligans open (B1, B2).
#[unsafe(no_mangle)]
pub extern "C" fn duos_begin(state_json_ptr: *const u8, state_json_len: usize) -> i32 {
    boundary(|| wasm_core::duos_begin(arg("duosBegin", state_json_ptr, state_json_len)?))
}

/// `{ state, events, error? }` for one `DuosAction`; an illegal one is refused in `error`, never failed.
#[unsafe(no_mangle)]
pub extern "C" fn duos_reduce(
    state_json_ptr: *const u8,
    state_json_len: usize,
    action_json_ptr: *const u8,
    action_json_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("duosReduce: the state", state_json_ptr, state_json_len)?;
        let action = arg("duosReduce: the action", action_json_ptr, action_json_len)?;
        wasm_core::duos_reduce(state, action)
    })
}

/// `DuosActionBody[]`: everything `seat` may do now.
#[unsafe(no_mangle)]
pub extern "C" fn duos_legal_actions(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("duosLegalActions: the state", state_json_ptr, state_json_len)?;
        let seat = arg("duosLegalActions: the seat", seat_ptr, seat_len)?;
        wasm_core::duos_legal_actions(state, seat)
    })
}

/// `DuosView`: the only window the client has onto a Duos game (B23).
#[unsafe(no_mangle)]
pub extern "C" fn duos_view_for(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("duosViewFor: the state", state_json_ptr, state_json_len)?;
        let seat = arg("duosViewFor: the seat", seat_ptr, seat_len)?;
        wasm_core::duos_view_for(state, seat)
    })
}

// The AI (SURFACE §9)

/// Whether `seat` owes an action the AI should take now (R187, R188): the result is the JSON text `true`
/// or `false`.
#[unsafe(no_mangle)]
pub extern "C" fn ai_to_act(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
) -> i32 {
    boundary(|| {
        let state = arg("aiToAct: the state", state_json_ptr, state_json_len)?;
        let seat = arg("aiToAct: the seat", seat_ptr, seat_len)?;
        wasm_core::ai_to_act(state, seat).map(|owes| owes.to_string())
    })
}

/// `{ decision: Decision | null, rngCursor }`. The AI draws only from its own stream, rebuilt from
/// `(rngSeed, rngCursor)`; the cursor it stopped at comes back so the caller resumes it there. This
/// module has no clock to read, so `deadline_ms` is accepted for the browser's signature and has no
/// effect: the search ends at `options.budget` (default `AI_BUDGET`) alone, and the decision depends only
/// on `(state, seed, cursor, budget)`.
#[unsafe(no_mangle)]
pub extern "C" fn ai_decide(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
    options_json_ptr: *const u8,
    options_json_len: usize,
    _deadline_ms: f64,
) -> i32 {
    boundary(|| {
        let state = arg("decide: the state", state_json_ptr, state_json_len)?;
        let seat = arg("decide: the seat", seat_ptr, seat_len)?;
        let options = arg("decide: the options", options_json_ptr, options_json_len)?;
        wasm_core::ai_decide(state, seat, options, NO_DEADLINE_MS, no_clock)
    })
}

/// R2102000: whether `seat` has a sure win this turn by what it can see, the practice AI's lethal hint:
/// the result is the JSON text `true` or `false`. As `ai_decide`'s, `deadline_ms` is accepted and has no
/// effect: this module has no clock, so the search runs to its end.
#[unsafe(no_mangle)]
pub extern "C" fn lethal_hint(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
    _deadline_ms: f64,
) -> i32 {
    boundary(|| {
        let state = arg("lethalHint: the state", state_json_ptr, state_json_len)?;
        let seat = arg("lethalHint: the seat", seat_ptr, seat_len)?;
        wasm_core::lethal_hint(state, seat, NO_DEADLINE_MS, no_clock).map(|sure| sure.to_string())
    })
}

/// `{ deck, rngCursor }`. Every key but `rngSeed`, `rngCursor` and `size` is `AiDeckOptions`' own, read
/// by its own serde, so an absent `banned` (the shadow ban), an absent `theme` (roll one) and a `null`
/// one (none) keep their meanings, and so does an absent `leanSet` (no lean, R1370).
#[unsafe(no_mangle)]
pub extern "C" fn build_ai_deck(options_json_ptr: *const u8, options_json_len: usize) -> i32 {
    boundary(|| wasm_core::build_ai_deck(arg("buildAiDeck", options_json_ptr, options_json_len)?))
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy, whose default skip set is
/// `jackioh_ai::random_action`'s, drawing from `(rng_seed, rng_cursor)`.
#[unsafe(no_mangle)]
pub extern "C" fn choose_action(
    state_json_ptr: *const u8,
    state_json_len: usize,
    seat_ptr: *const u8,
    seat_len: usize,
    rng_seed_ptr: *const u8,
    rng_seed_len: usize,
    rng_cursor: f64,
) -> i32 {
    boundary(|| {
        let state = arg("chooseAction: the state", state_json_ptr, state_json_len)?;
        let seat = arg("chooseAction: the seat", seat_ptr, seat_len)?;
        let seed = arg("chooseAction: the seed", rng_seed_ptr, rng_seed_len)?;
        wasm_core::choose_action(state, seat, seed, rng_cursor)
    })
}

/// Every rules, AI and run number a client reads, as one JSON object: `AI_BUDGET`, `AI_GATE_BUDGET`,
/// `SHADOW_BAN_IDS`, `FIXTURE_BANNED_IDS`, `AI_DECK`, `RUN` and each constant of `engineConfig.ts` by its
/// name (`DECK_SIZE`, …). A host reads its numbers from here and restates none (CLAUDE.md rule 9).
#[unsafe(no_mangle)]
pub extern "C" fn constants() -> i32 {
    answer(wasm_core::constants())
}

/// R1011000: the learned evaluation's parity fixture (`crates/ai/tests/fixtures/learned-parity.json`)
/// as this build computes it, so a host can hold its own port to the fixture's numbers bit for bit.
#[unsafe(no_mangle)]
pub extern "C" fn learned_parity(fixture_json_ptr: *const u8, fixture_json_len: usize) -> i32 {
    boundary(|| {
        wasm_core::learned_parity(arg(
            "learnedParity: the fixture",
            fixture_json_ptr,
            fixture_json_len,
        )?)
    })
}

// The engine's tables the web's tests read (`subsystems.*`)

/// ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost, so the
/// block editor shows what the reducer will say and the client decides nothing (CLAUDE.md rule 7).
/// `prompt_json` is the open prompt's `{ craftMode, craftOffer }` (R1563007).
#[unsafe(no_mangle)]
pub extern "C" fn craft_preview(
    recipe_json_ptr: *const u8,
    recipe_json_len: usize,
    cost: f64,
    prompt_json_ptr: *const u8,
    prompt_json_len: usize,
) -> i32 {
    boundary(|| {
        let recipe = arg("craftPreview: the recipe", recipe_json_ptr, recipe_json_len)?;
        let prompt = arg("craftPreview: the prompt", prompt_json_ptr, prompt_json_len)?;
        wasm_core::craft_preview(recipe, cost, prompt)
    })
}

/// `{ chaosEffects: [{label}], chaosPlusEffects: [{label}], chaosMedEffects: [{label}], heroPowerNames,
/// heroPowers: [{name, x, title, radiantTitle, label, radiantLabel}] }`.
#[unsafe(no_mangle)]
pub extern "C" fn engine_tables() -> i32 {
    boundary(wasm_core::engine_tables)
}

// The validator (SPEC §9.4: one module, client and server)

/// One validator call by its TypeScript name; the input and the output are that function's argument
/// and result, as JSON (SURFACE §10.1). `checkDeckDraft` takes core's `DeckDraftRequest`.
#[unsafe(no_mangle)]
pub extern "C" fn validator(
    call_ptr: *const u8,
    call_len: usize,
    input_json_ptr: *const u8,
    input_json_len: usize,
) -> i32 {
    boundary(|| {
        let call = arg("validator: the call", call_ptr, call_len)?;
        let input = arg("validator: the input", input_json_ptr, input_json_len)?;
        wasm_core::validator(call, input)
    })
}

// The Conquest series rules (SPEC §9.5, #3098: `jackioh-shared`, which the server calls natively)

/// One call of the Conquest series rules: a `SeriesCall` as JSON in, a `SeriesAnswer` as JSON out. The
/// host holds the series row and hands it back on every call, and passes the time in
/// (`crates/shared/README.md`, the host contract). A rules refusal is the answer's `refused`, never a
/// failed status: only a call that does not parse is one.
#[unsafe(no_mangle)]
pub extern "C" fn series(call_json_ptr: *const u8, call_json_len: usize) -> i32 {
    boundary(|| wasm_core::series(arg("series: the call", call_json_ptr, call_json_len)?))
}

// The match host's machine (SPEC §9.5, #3101: `jackioh-shared`, which the server calls natively)

/// One call of the match host's machine: a `HostCall` as JSON in, a `HostAnswer` as JSON out. The host
/// holds the `DuelHost` and hands it back on every call, and passes the time in
/// (`crates/shared/README.md`, the host contract). A rules refusal is an answer's data, never a failed
/// status: only a call that does not parse is one.
#[unsafe(no_mangle)]
pub extern "C" fn host(call_json_ptr: *const u8, call_json_len: usize) -> i32 {
    boundary(|| wasm_core::host(arg("host: the call", call_json_ptr, call_json_len)?))
}

// The ladder bots (SPEC §9.12, #3103: `jackioh-shared`, which the server calls natively)

/// One call of the decisions around a ladder bot: a `BotCall` as JSON in, a `BotAnswer` as JSON out. The
/// host reads its store into plain records and writes what comes out (`crates/shared/README.md`, the host
/// contract).
#[unsafe(no_mangle)]
pub extern "C" fn bots(call_json_ptr: *const u8, call_json_len: usize) -> i32 {
    boundary(|| wasm_core::bots(arg("bots: the call", call_json_ptr, call_json_len)?))
}

/// The move a ladder bot owes: a `DecideCall` (the `DuelHost` and the `DecideRequest` the machine made) as
/// JSON in, the `BotChoice` to feed it back as `decided` out. As `ai_decide`'s, `deadline_ms` is accepted
/// and has no effect: this module has no clock, so the search ends at the strength's budget.
#[unsafe(no_mangle)]
pub extern "C" fn bot_decide(call_json_ptr: *const u8, call_json_len: usize, _deadline_ms: f64) -> i32 {
    boundary(|| {
        wasm_core::bot_decide(
            arg("botDecide: the call", call_json_ptr, call_json_len)?,
            NO_DEADLINE_MS,
            no_clock,
        )
    })
}

#[cfg(test)]
mod tests;
