//! `jackioh-wasm-c` (#3093): the engine, the cards, the AI, the validator and the dungeon run as a plain
//! `wasm32-unknown-unknown` module that imports nothing from its host. It is the module a host that is
//! not a browser runs, such as the Roblox port's translation (#2898): every binding the browser module
//! has (`crates/wasm`, SURFACE §10.1) as an `extern "C"` function over JSON text, with the same names
//! and JSON, and a memory interface (`jackioh_alloc`, `jackioh_free`, `jackioh_result_ptr`,
//! `jackioh_result_len`) to move that text across. `README.md` is the contract for such a host;
//! `crates/wasm/tests/export_parity.rs` holds the exports to the browser module's, so a binding added for
//! the web reaches this module by itself.
//!
//! The boundary is C without strings: an argument is `(ptr, len)` UTF-8 bytes in the module's memory,
//! and every binding answers an `i32` status (`STATUS_OK` or `STATUS_FAILED`) with its text in a result
//! buffer the module owns. `reduce`'s refusal of an illegal action is still the `ReduceResult`'s `error`
//! (SURFACE §6.1), status `STATUS_OK`. A panic is an engine invariant broken (SURFACE §4.4.9): it traps,
//! and the message stays readable in the result buffer.
//!
//! Every body, the JSON helpers and the state cache live in `jackioh-wasm-core` (#1505). The pure crates
//! read no clock (CLAUDE.md rule 4, SURFACE §3) and this module has none to lend them: a search is
//! bounded by its budget alone (`NO_DEADLINE_MS`), and the setup check refuses a game with custom cards
//! (R915009), as the normal module does.

use std::cell::RefCell;

use jackioh_engine::prelude::IndexMap;
use jackioh_engine::{CreateGameArgs, SetupKind};
use jackioh_wasm_core as wasm_core;

// The C boundary: bytes in, a status out, the text in the result buffer

/// A binding that did its work: the result buffer holds its JSON (or `true`, `false`, or nothing).
pub const STATUS_OK: i32 = 0;
/// A binding that failed (a JSON argument that does not parse, a setup the engine refuses, an argument
/// that is missing or not UTF-8): the result buffer holds the message.
pub const STATUS_FAILED: i32 = 1;

/// The deadline every search is handed: none. This module has no clock, so a time to stop at could
/// never come; `deadline_ms` is accepted for the browser module's signature and the search is bounded by
/// its `options.budget` alone (`jackioh_ai::AI_BUDGET` when absent), so its answer depends only on
/// `(state, seed, cursor, budget)`.
const NO_DEADLINE_MS: f64 = 0.0;

thread_local! {
    /// The last binding's text: valid until the next binding call. WebAssembly runs one thread;
    /// `thread_local!` is how a module keeps data between calls without a `static mut`.
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Keep a binding's answer in the result buffer and say whether it worked.
fn answer(result: Result<String, String>) -> i32 {
    let (status, text) = match result {
        Ok(text) => (STATUS_OK, text),
        Err(message) => (STATUS_FAILED, message),
    };
    RESULT.with(|result| *result.borrow_mut() = text.into_bytes());
    status
}

/// One text argument, `len` bytes at `ptr`: an empty one is `""` wherever `ptr` points, a missing one
/// (`ptr` null) and non-UTF-8 are refused with the binding's own message.
fn arg<'a>(what: &str, ptr: *const u8, len: usize) -> Result<&'a str, String> {
    if len == 0 {
        return Ok("");
    }
    if ptr.is_null() {
        return Err(format!("{what}: the argument is missing"));
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).map_err(|error| format!("{what}: the argument is not UTF-8: {error}"))
}

/// Run a binding's body and answer it.
fn boundary(body: impl FnOnce() -> Result<String, String>) -> i32 {
    answer(body())
}

/// The clock the AI bodies are handed. Never read: with `NO_DEADLINE_MS` they ask for no clock.
fn no_clock() -> f64 {
    0.0
}

/// The setup checks, run before the engine is called: `create_game` panics on a bad setup (SURFACE §6.1),
/// and a panic in WebAssembly is a trap the host cannot recover from. R180's handicaps come first, so a
/// bad deck size is named as the handicap's fault, then each deck against its seat's size (R184); the
/// functions are the engine's own (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs, kind: SetupKind) -> Result<(), String> {
    // R915009: this module holds no Luau, so it plays no game with custom cards.
    if args.custom_cards.as_ref().is_some_and(|cards| !cards.is_empty()) {
        return Err("a game with custom cards is not played by this module (R915009)".to_owned());
    }
    wasm_core::check_decks(args, &IndexMap::new(), kind)
}

// The memory interface

/// A buffer of `len` bytes (at least one) for the host to write an argument into, at an address that is
/// never 0. Give it back with `jackioh_free` and the same `len`.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len.max(1)].into_boxed_slice()).cast::<u8>()
}

/// Free a buffer `jackioh_alloc` returned. NULL is ignored.
///
/// # Safety
///
/// `ptr` must be NULL or a pointer `jackioh_alloc(len)` returned with this very `len`, not freed before
/// and not read after.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jackioh_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len.max(1))));
        }
    }
}

/// Where the last binding's text starts. It is owned by the module and valid until the next binding
/// call; with `jackioh_result_len() == 0` there is nothing to read.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_result_ptr() -> *const u8 {
    RESULT.with(|result| result.borrow().as_ptr())
}

/// How many bytes of UTF-8 the last binding's text holds.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_result_len() -> usize {
    RESULT.with(|result| result.borrow().len())
}

/// Once, before any other binding: on WebAssembly a panic leaves its message in the result buffer before
/// the trap, and the catalog and the card scripts are registered with the engine
/// (`jackioh_cards::register_all`, idempotent).
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    // Off WebAssembly (a host `cargo test` run) the harness keeps its own hook.
    #[cfg(target_arch = "wasm32")]
    {
        use std::sync::Once;

        static HOOK: Once = Once::new();
        HOOK.call_once(|| {
            std::panic::set_hook(Box::new(|info| {
                let message = info.to_string().into_bytes();
                // A panic inside `answer` finds the buffer borrowed: then the host reads what was there.
                let _ = RESULT.try_with(|result| {
                    if let Ok(mut result) = result.try_borrow_mut() {
                        *result = message;
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

/// R767000: the shipped versions this module carries for replays, oldest first: those from
/// `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM` on.
#[unsafe(no_mangle)]
pub extern "C" fn carried_versions() -> i32 {
    answer(wasm_core::carried_versions())
}

// The dungeon run (docs/dungeon-run.md DR14 part C)

// A run-rule refusal is a returned value (`{ "error": … }`, or `run_check_setup`'s nullable reason),
// never `STATUS_FAILED`: only JSON that does not parse is one.

/// `RunContent`: `run_catalog_content` over the compiled-in catalog and the Sandbox's custom cards.
#[unsafe(no_mangle)]
pub extern "C" fn run_content(custom_cards_json_ptr: *const u8, custom_cards_json_len: usize) -> i32 {
    boundary(|| {
        let custom_cards = arg(
            "runContent: the custom cards",
            custom_cards_json_ptr,
            custom_cards_json_len,
        )?;
        wasm_core::run_content(custom_cards)
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

/// `ReduceResult`. An illegal action is refused in `error`, never `STATUS_FAILED` (SURFACE §6.1).
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
/// (`SetName[]`) count as sets that ship. Only a module built with the `preview` feature can preview;
/// any other refuses a set (#552's play-through). `STATUS_OK` with an empty result on success.
#[unsafe(no_mangle)]
pub extern "C" fn preview_sets(sets_json_ptr: *const u8, sets_json_len: usize) -> i32 {
    boundary(|| {
        wasm_core::preview_sets(arg("previewSets: the sets", sets_json_ptr, sets_json_len)?)
            .map(|()| String::new())
    })
}

// Duos co-op (#936 SPEC S4, S8): the wrappers over two duels. The bodies live in core behind its `duos`
// feature, which this crate's `duos` feature reaches; without it each answers the reason.

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

/// `{ state, events, error? }` for one `DuosAction`; an illegal one is refused in `error`.
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

/// Whether `seat` owes an action the AI should take now (R187, R188): the result is `true` or `false`.
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
/// module has no clock, so `deadline_ms` is ignored (it is here for the browser module's signature) and
/// the decision depends only on `(state, seed, cursor, budget)`: bound the search with `options.budget`.
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
/// the result is `true` or `false`. `deadline_ms` is ignored, as `ai_decide`'s is: the search is bounded
/// by its node budget alone.
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

/// Every number a client reads, one JSON object: the AI's (`AI_BUDGET`, `AI_GATE_BUDGET`,
/// `SHADOW_BAN_IDS`, `FIXTURE_BANNED_IDS`, `AI_DECK`), the dungeon run's (`RUN`) and the rules' by the
/// names `apps/web/src/wire/engineConfig.ts` gives them (`DECK_SIZE`, `HERO_HEALTH`, …), so a host states
/// none (CLAUDE.md rule 9).
#[unsafe(no_mangle)]
pub extern "C" fn constants() -> i32 {
    answer(wasm_core::constants())
}

/// R1011000: the learned evaluation's parity fixture
/// (`crates/ai/tests/fixtures/learned-parity.json`) as this build computes it.
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
/// and result, as JSON (SURFACE §10.1).
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

#[cfg(test)]
mod tests;
