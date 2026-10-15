//! `jackioh-wasm`: the bindings the web client loads (docs/v0.3.0/SURFACE.md §10.1). JSON strings in
//! and out, nothing else crosses: the state is the `GameState` JSON object, which the client keeps
//! behind its opaque `EngineState` brand (CLAUDE.md rule 7) and hands back on every call.
//!
//! `apps/web/src/wasm/index.ts` is the only caller. It parses and stringifies, names each binding as
//! the TypeScript engine named it (`createGame`, `reduce`, `decide`, …), and is what the hotseat
//! port (`game/engine.ts`), the practice worker (`practice/core.ts`) and the `@jackioh/*` aliases
//! (`apps/web/src/wire/`) reach the engine, the AI and the validator through.
//!
//! The boundary is C, not `wasm-bindgen`: `mlua` (the Luau host, `crates/luau`) builds Luau for
//! WebAssembly only through `wasm32-unknown-emscripten`, which `wasm-bindgen` cannot target, so the
//! module links through `emcc` (`scripts/build-wasm.sh`) and every binding is a `#[no_mangle]
//! extern "C"` function over NUL-terminated UTF-8 C strings: string arguments in, an owned string
//! back, freed with `jackioh_free`. A binding that would have thrown returns NULL and stows its
//! message for `jackioh_last_error`. The function names and the JSON that crosses are the ones
//! SURFACE §10.1 gives, so nothing above `index.ts` changes.
//!
//! This crate is a binary, not a library: `rustc` only emits emscripten's JavaScript glue (the
//! `MODULARIZE` factory `index.ts` awaits) when the `emcc` link produces a program, and a `cdylib`
//! link produces a bare `.wasm` with no glue. `fn main` does nothing; the client calls `init()`
//! after instantiating, which registers the catalog and the card scripts, as before.
//!
//! Errors. A binding whose JSON argument does not parse, or whose setup TypeScript refused with a
//! `throw` (a deck or a handicap `createGame` would not take, an unknown validator call), returns
//! NULL: JavaScript reads `jackioh_last_error()` and throws an `Error` with TypeScript's own
//! message, where TypeScript threw. `reduce` never fails on an illegal action: its refusal is the
//! `error` of the `ReduceResult` (SURFACE §6.1). A panic is an engine invariant broken (SURFACE
//! §4.4.9); the panic hook prints it with `emscripten_console_error` before the abort, which the
//! glue raises as a `WebAssembly.RuntimeError` that `index.ts` names as a panic, as before.
//!
//! The state cache. Every call that takes a state parses its JSON, and the client hands the same
//! state over and over: a step of the practice worker or the hotseat asks `view_for` for both seats,
//! `legal_actions`, `seat_to_act` and `ai_to_act` of the state `reduce` just returned. So the bindings
//! keep the few states they parsed or produced last (`STATE_CACHE`), each under its exact JSON text,
//! and a call whose text is one of them reads that state instead of parsing it again. A text that
//! differs by one byte is parsed, so the answer never depends on the cache: parsing is deterministic,
//! and the engine only reads the state it is given. A state `reduce`, `begin_game` or `create_game`
//! returns is remembered under the text the binding wrote, which is the text `JSON.stringify` writes
//! of the object `JSON.parse` made of it (same keys, same order, integers only).
//!
//! The clock. The pure crates never read one (CLAUDE.md rule 4, SURFACE §3). The AI's wall-clock cap
//! is this crate's: `ai_decide` hands `decide` a `should_stop` that reads `SystemTime::now` (the
//! libc clock emscripten provides) against the deadline the page computed, as TypeScript's
//! `shouldStop` read `performance.now()`.

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

use jackioh_engine::{
    self as engine, Action, CardDefs, CraftRecipe, CreateGameArgs, DECK_SIZE, FoldArgs, GameState,
    PLAYER_IDS, PerPlayerOpt, PlayerId, ReduceResult, ReplayCheckpoints, ReplayRecord, Rng,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------------------------
// The C boundary: strings in, an owned string (or NULL) out
// ---------------------------------------------------------------------------------------------

// The message of the last binding call that failed, on this thread. WebAssembly runs one thread;
// `thread_local!` is how the module keeps it without a `static mut`. Valid until the next failing
// call; `index.ts` copies it at once.
thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("").expect("empty error"));
}

/// Stow `message` and answer NULL, the failure path every binding below returns through.
fn failed<T>(message: String) -> *mut T {
    LAST_ERROR.with(|last| {
        *last.borrow_mut() = CString::new(message).unwrap_or_else(|_| {
            CString::new("could not serialise the error message").expect("static message")
        });
    });
    std::ptr::null_mut()
}

/// The last failure's message, NUL-terminated UTF-8, valid until the next failing call.
#[unsafe(no_mangle)]
pub extern "C" fn jackioh_last_error() -> *const c_char {
    LAST_ERROR.with(|last| last.borrow().as_ptr())
}

/// Free a string this module returned, or an argument buffer `index.ts` is done with. Both live on
/// the one emscripten heap — Rust's allocator here is libc's `malloc`, the same `_malloc` the
/// glue's `stringToNewUTF8` writes through — and every argument the client passes is valid UTF-8
/// with no interior NUL (it is `JSON.stringify` output), so a buffer the glue allocated reads back
/// as exactly the `CString` this frees.
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

/// Run a binding's body: the owned string on success, NULL (with the message stowed) on failure.
fn boundary(body: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    out(body())
}

/// Answer a binding: the owned string on success, NULL (with the message stowed) on failure.
fn out(result: Result<String, String>) -> *mut c_char {
    match result {
        Ok(text) => CString::new(text)
            .map(|owned| owned.into_raw())
            .unwrap_or_else(|_| failed("could not serialise the result: an interior NUL".to_owned())),
        Err(message) => failed(message),
    }
}

// ---------------------------------------------------------------------------------------------
// JSON in, JSON out
// ---------------------------------------------------------------------------------------------

/// Parses one JSON argument, naming the binding and the argument when it does not parse.
fn parse<T: DeserializeOwned>(what: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|error| format!("{what}: {error}"))
}

/// The JSON text of a result.
fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("could not serialise the result: {error}"))
}

/// `"p1"` or `"p2"`; anything else is refused with `PlayerId`'s own message.
fn player(text: &str) -> Result<PlayerId, String> {
    PlayerId::from_str(text).map_err(|error| error.to_string())
}

/// How many states the bindings keep, by their JSON text (the module doc's "state cache"): the state a
/// call is asked about, the one `reduce` returned from it, and a little room for a caller that keeps
/// two games (the hotseat's replay check, a test's two cores).
const STATE_CACHE: usize = 4;

thread_local! {
    /// Most recently used first.
    static STATES: RefCell<Vec<(String, Rc<GameState>)>> = const { RefCell::new(Vec::new()) };
}

/// The state cached under exactly this text, moved to the front, if one is.
fn cached_state(state_json: &str) -> Option<Rc<GameState>> {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        let at = states.iter().position(|(text, _)| same_text(text, state_json))?;
        let entry = states.remove(at);
        let state = Rc::clone(&entry.1);
        states.insert(0, entry);
        Some(state)
    })
}

/// `a == b`, eight bytes at a time: WebAssembly's `memcmp` (compiler-builtins) goes byte by byte,
/// and the text a call is matched against is the whole ~80 KB state.
fn same_text(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let (left, right) = (a.as_chunks::<8>(), b.as_chunks::<8>());
    left.0
        .iter()
        .zip(right.0.iter())
        .all(|(x, y)| u64::from_ne_bytes(*x) == u64::from_ne_bytes(*y))
        && left.1 == right.1
}

/// Keep `state` under `text`, dropping the least recently used state past `STATE_CACHE`.
fn remember(text: String, state: Rc<GameState>) {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        states.retain(|(known, _)| *known != text);
        states.insert(0, (text, state));
        states.truncate(STATE_CACHE);
    });
}

/// How `apps/web/src/wasm/index.ts` wraps a state's JSON, `["\u{a0}",<state>]`: the bindings unwrap
/// it in `state_of`. The mark dates to the `wasm-bindgen` glue, whose string copy ran per character
/// up to the first non-ASCII character; emscripten's `stringToNewUTF8` walks the whole string
/// either way, so all the mark buys now is one shared shape between the two sides, which is reason
/// enough to keep it: no caller above `index.ts` ever sees it.
const STATE_ENVELOPE: (&str, &str) = ("[\"\u{a0}\",", "]");

/// A state argument's JSON, out of its envelope when it came in one.
fn unwrapped(state_json: &str) -> &str {
    let (head, tail) = STATE_ENVELOPE;
    state_json
        .strip_prefix(head)
        .and_then(|rest| rest.strip_suffix(tail))
        .unwrap_or(state_json)
}

/// The state a JSON argument holds: the cached one with this very text, else parsed (and cached).
fn state_of(what: &str, state_json: &str) -> Result<Rc<GameState>, String> {
    let state_json = unwrapped(state_json);
    if let Some(state) = cached_state(state_json) {
        return Ok(state);
    }
    let state: Rc<GameState> = Rc::new(parse(&format!("{what}: the state"), state_json)?);
    remember(state_json.to_owned(), Rc::clone(&state));
    Ok(state)
}

/// A state the bindings return: its JSON text, with the state cached under it.
fn returned_state(state: GameState) -> Result<String, String> {
    let text = to_json(&state)?;
    remember(text.clone(), Rc::new(state));
    Ok(text)
}

/// `ReduceResult`'s JSON, `{"state":…,"events":[…]}` plus `"error"` on a refusal, exactly as serde
/// writes the struct (`error` is skipped when absent), with the new state cached under its own text.
fn reduce_result_json(result: ReduceResult) -> Result<String, String> {
    let ReduceResult { state, events, error } = result;
    let events = to_json(&events)?;
    let error = match &error {
        Some(message) => format!(",\"error\":{}", to_json(message)?),
        None => String::new(),
    };
    // A refusal hands back the input state, which the cache holds already under the caller's text.
    let state = if error.is_empty() {
        returned_state(state)?
    } else {
        to_json(&state)?
    };
    Ok(format!("{{\"state\":{state},\"events\":{events}{error}}}"))
}

/// The checks TypeScript's `createGame` threw on, in its order (R180's handicaps first, so a bad
/// deck size is named as the handicap's fault, then each deck against its seat's size, R184), run
/// before the engine is called: Rust's `create_game` panics where TypeScript threw (SURFACE §6.1),
/// and a panic in WebAssembly is an abort the page cannot catch as the error TypeScript gave it. The
/// functions are the engine's own (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs) -> Result<(), String> {
    let catalog: &CardDefs = match &args.catalog {
        Some(catalog) => catalog,
        None => engine::registered_catalog(),
    };
    let no_handicaps = PerPlayerOpt::default();
    let handicaps = args.handicaps.as_ref().unwrap_or(&no_handicaps);
    for seat in PLAYER_IDS {
        if let Some(handicap) = handicaps.get(seat) {
            engine::validate_handicap(handicap, seat.as_str()).map_err(|error| error.to_string())?;
        }
    }
    for seat in PLAYER_IDS {
        let size = handicaps
            .get(seat)
            .map_or(DECK_SIZE, |handicap| handicap.deck_size);
        let deck = match seat {
            PlayerId::P1 => &args.decks.0,
            PlayerId::P2 => &args.decks.1,
        };
        engine::validate_deck(deck, catalog, seat.as_str(), size).map_err(|error| error.to_string())?;
    }
    Ok(())
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
    eprintln!("jackioh-wasm panic: {}", message.to_string_lossy());
}

/// Once, before any other binding: panics print on the console, and the catalog and the card
/// scripts are registered with the engine (`jackioh_cards::register_all`, idempotent). This is what
/// TypeScript's `registerAll()` calls did; the web has nothing left to register.
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

/// The registered catalog, `CardDefs` (TS `registeredCatalog()`).
#[unsafe(no_mangle)]
pub extern "C" fn catalog() -> *mut c_char {
    out(to_json(engine::registered_catalog()))
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
#[unsafe(no_mangle)]
pub extern "C" fn catalog_version() -> *mut c_char {
    out(Ok(jackioh_cards::catalog_version().to_string()))
}

/// R767000: the shipped versions this module carries for replays, oldest first: in the browser,
/// those from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM` on.
#[unsafe(no_mangle)]
pub extern "C" fn carried_versions() -> *mut c_char {
    out(to_json(&jackioh_cards::carried_versions()))
}

// ---------------------------------------------------------------------------------------------
// The engine (SURFACE §6.1)
// ---------------------------------------------------------------------------------------------

/// `GameState` (TS `createGame({ seed, decks, catalog?, handicaps?, lastBoards?, glitchBoards?, dealt? })`).
#[unsafe(no_mangle)]
pub extern "C" fn create_game(args_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let args: CreateGameArgs = parse("createGame", arg("createGame", args_json)?)?;
        check_setup(&args)?;
        returned_state(engine::create_game(&args))
    })
}

/// `ReduceResult`: `{ state, events, error? }`.
#[unsafe(no_mangle)]
pub extern "C" fn begin_game(state_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("beginGame", arg("beginGame", state_json)?)?;
        reduce_result_json(engine::begin_game(&state))
    })
}

/// `ReduceResult`. An illegal action is refused in `error`, never NULL (SURFACE §6.1).
#[unsafe(no_mangle)]
pub extern "C" fn reduce(state_json: *const c_char, action_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("reduce", arg("reduce: the state", state_json)?)?;
        let action: Action = parse("reduce: the action", arg("reduce: the action", action_json)?)?;
        reduce_result_json(engine::reduce(&state, &action))
    })
}

/// `ActionBody[]`: everything `player` may legally do now.
#[unsafe(no_mangle)]
pub extern "C" fn legal_actions(state_json: *const c_char, player_id: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("legalActions", arg("legalActions: the state", state_json)?)?;
        to_json(&engine::legal_actions(
            &state,
            player(arg("legalActions: the player", player_id)?)?,
        ))
    })
}

/// `PlayerView`: the only window the client has onto the game (SPEC §10.8).
#[unsafe(no_mangle)]
pub extern "C" fn view_for(state_json: *const c_char, player_id: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("viewFor", arg("viewFor: the state", state_json)?)?;
        to_json(&engine::view_for(
            &state,
            player(arg("viewFor: the player", player_id)?)?,
        ))
    })
}

/// `"p1"`, `"p2"`, or `""` when no seat owes an action.
#[unsafe(no_mangle)]
pub extern "C" fn seat_to_act(state_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("seatToAct", arg("seatToAct", state_json)?)?;
        Ok(engine::seat_to_act(&state).map_or_else(String::new, |seat| seat.as_str().to_string()))
    })
}

/// SURFACE §5.2's hash, eight lower-case hex digits: TypeScript's `hashState`, bit for bit.
#[unsafe(no_mangle)]
pub extern "C" fn hash_state(state_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("hashState", arg("hashState", state_json)?)?;
        Ok(engine::hash_state(&state))
    })
}

/// `{ state, errors: [{ nonce, error }] }` (TS `fold`). The setup is checked first, as TypeScript's
/// `fold` threw on it through `createGame`; a refused action in the log is collected, not failed.
#[unsafe(no_mangle)]
pub extern "C" fn fold(args_json: *const c_char) -> *mut c_char {
    boundary(|| {
        // The fold's input is `createGame`'s plus `log`, so the same JSON reads as both.
        let setup: CreateGameArgs = parse("fold", arg("fold", args_json)?)?;
        check_setup(&setup)?;
        let args: FoldArgs = parse("fold", arg("fold", args_json)?)?;
        to_json(&engine::fold(&args))
    })
}

/// R768: `ReplayOpen`. The setup is checked as `fold` checks it, under the catalog the game was
/// played on (R767000): this build's own or one it carries. A game of any other version is refused as
/// `earlier_patch` before its decks are read.
#[unsafe(no_mangle)]
pub extern "C" fn replay_open(args_json: *const c_char, record_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let record: ReplayRecord = parse(
            "replayOpen: the record",
            arg("replayOpen: the record", record_json)?,
        )?;
        let build = jackioh_cards::catalog_version();
        if let Ok(shipped) = engine::catalog_for(&record.catalog_version, build) {
            let setup: CreateGameArgs = parse("replayOpen", arg("replayOpen", args_json)?)?;
            engine::with_shipped(shipped, || check_setup(&setup))?;
        }
        let args: FoldArgs = parse("replayOpen", arg("replayOpen", args_json)?)?;
        to_json(&engine::replay_open(&args, &record, build))
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
        let checkpoints: ReplayCheckpoints = parse(
            "replayPage: the checkpoints",
            arg("replayPage: the checkpoints", checkpoints_json)?,
        )?;
        to_json(&engine::replay_page(
            &checkpoints,
            player(arg("replayPage: the seat", seat)?)?,
            from as usize,
            count as usize,
        ))
    })
}

/// `LastBoardEntry[]`: the board `seat` takes away (R417, R508).
#[unsafe(no_mangle)]
pub extern "C" fn last_board_for(state_json: *const c_char, seat: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("lastBoardFor", arg("lastBoardFor: the state", state_json)?)?;
        to_json(&engine::last_board_for(
            &state,
            player(arg("lastBoardFor: the seat", seat)?)?,
        ))
    })
}

/// `"p1"` or `"p2"`: the seat the player who began in `home` plays now (R677).
#[unsafe(no_mangle)]
pub extern "C" fn seat_played_by(state_json: *const c_char, home: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("seatPlayedBy", arg("seatPlayedBy: the state", state_json)?)?;
        Ok(
            engine::seat_played_by(&state, player(arg("seatPlayedBy: the home seat", home)?)?)
                .as_str()
                .to_string(),
        )
    })
}

/// `CardInstance` JSON, or `"null"` when no card has that id (`state.rs`'s `find_instance`).
#[unsafe(no_mangle)]
pub extern "C" fn find_instance(state_json: *const c_char, instance_id: *const c_char) -> *mut c_char {
    boundary(|| {
        let state = state_of("findInstance", arg("findInstance: the state", state_json)?)?;
        to_json(&engine::find_instance(
            &state,
            arg("findInstance: the instance", instance_id)?,
        ))
    })
}

/// `EmoteId[]`: the emote hand `seat` is dealt in the game seeded `seed` (R1341), so hotseat and
/// practice deal both seats exactly as the server's match actor does (R1342).
#[unsafe(no_mangle)]
pub extern "C" fn deal_emote_hand(seed: *const c_char, seat: *const c_char) -> *mut c_char {
    boundary(|| {
        to_json(&engine::deal_emote_hand(
            arg("dealEmoteHand: the seed", seed)?,
            player(arg("dealEmoteHand: the seat", seat)?)?,
        ))
    })
}

/// R1420's preview, in the browser: from this call on the module treats `sets` (`SetName[]`) as sets
/// that ship, as the testkit's `preview_sets` does on a test's thread, so a deck may hold their cards
/// and a pool that names no set reaches them. WebAssembly runs one thread, so the preview lasts the
/// module's life. Only a module built with the `preview` feature can preview (the engine's testkit:
/// `scripts/build-wasm.sh` builds it for development, the unit tests and the e2e client, and apps/web's
/// `prebuild` builds the production bundle without it); any other refuses a set. The hotseat route
/// calls it for its E2E injection's `preview`, in development builds only (#552's play-through).
/// Answers 0 on success, 1 with the message stowed for `jackioh_last_error` on failure.
#[unsafe(no_mangle)]
pub extern "C" fn preview_sets(sets_json: *const c_char) -> i32 {
    match (|| {
        let sets: Vec<engine::SetName> =
            parse("previewSets: the sets", arg("previewSets: the sets", sets_json)?)?;
        preview(&sets)
    })() {
        Ok(()) => 0,
        Err(message) => {
            failed::<()>(message);
            1
        }
    }
}

#[cfg(feature = "preview")]
fn preview(sets: &[engine::SetName]) -> Result<(), String> {
    // The guard ends the preview when it drops; the module keeps it instead.
    std::mem::forget(engine::testkit::preview_sets(sets));
    Ok(())
}

#[cfg(not(feature = "preview"))]
fn preview(sets: &[engine::SetName]) -> Result<(), String> {
    if sets.is_empty() {
        return Ok(());
    }
    Err(
        "previewSets: this module was built without the `preview` feature, so it previews no set (R1420)"
            .to_owned(),
    )
}

// ---------------------------------------------------------------------------------------------
// Duos co-op (#936 SPEC S4, S8): the wrapper over two duels, for `/dev/duos`'s four-seat hotseat
// ---------------------------------------------------------------------------------------------
//
// The five bindings mirror the 1v1 ones: a `DuosState` crosses as JSON in the same envelope as a
// `GameState` (`unwrapped`), a setup the engine would panic on is refused first with its own message,
// and an illegal action is the result's `error`, never NULL. Duos states are not cached: only the
// dev route plays them, and the cache above is sized for the 1v1 callers.
//
// Only the `duos` feature builds them: `/dev/duos` is the one caller, and a development build's
// alone (production serves it a 404), so `scripts/build-wasm.sh` turns the feature on for the dev
// server only (`JACKIOH_WASM_DUOS=1`, apps/web's `predev`), keeping them out of every other module:
// the one a player loads, the e2e client CI builds and the one #442 L12 weighs. Without it each binding is a stub that fails with the reason, as `preview_sets` does
// without `preview`, so the module exports the same functions either way.

#[cfg(feature = "duos")]
mod duos_bindings {
    use std::ffi::c_char;
    use std::str::FromStr;

    use jackioh_engine::duos::{self as duos_engine, DuosReduceResult, DuosState};
    use jackioh_engine::wire::{
        DUOS_SEATS, DuosAction, DuosCreateOptions, DuosDecks, DuosHandicaps, DuosSeat,
    };
    use jackioh_engine::{self as engine, CardDefs, DECK_SIZE, Handicap};

    use super::{arg, boundary, parse, to_json, unwrapped};

    /// `"a1"`, `"a2"`, `"b1"` or `"b2"`; anything else is refused with `DuosSeat`'s own message.
    fn duos_seat(text: &str) -> Result<DuosSeat, String> {
        DuosSeat::from_str(text)
    }

    /// The deck a seat's options name.
    fn duos_deck(decks: &DuosDecks, seat: DuosSeat) -> &[String] {
        match seat {
            DuosSeat::A1 => &decks.a1,
            DuosSeat::A2 => &decks.a2,
            DuosSeat::B1 => &decks.b1,
            DuosSeat::B2 => &decks.b2,
        }
    }

    /// The handicap a seat's options name, if any (§9.9; absent is a human's).
    fn duos_handicap(handicaps: Option<&DuosHandicaps>, seat: DuosSeat) -> Option<&Handicap> {
        let handicaps = handicaps?;
        match seat {
            DuosSeat::A1 => handicaps.a1.as_ref(),
            DuosSeat::A2 => handicaps.a2.as_ref(),
            DuosSeat::B1 => handicaps.b1.as_ref(),
            DuosSeat::B2 => handicaps.b2.as_ref(),
        }
    }

    /// `check_setup` for the four seats, in its order (every handicap, then every deck against its
    /// seat's size): each duel is a `create_game`, which panics where TypeScript threw, so a refusal is
    /// named here, with the engine's own message and the seat's name as its label.
    fn check_duos_setup(options: &DuosCreateOptions) -> Result<(), String> {
        let catalog: &CardDefs = match &options.catalog {
            Some(catalog) => catalog,
            None => engine::registered_catalog(),
        };
        let handicaps = options.handicaps.as_ref();
        for seat in DUOS_SEATS {
            if let Some(handicap) = duos_handicap(handicaps, seat) {
                engine::validate_handicap(handicap, seat.as_str()).map_err(|error| error.to_string())?;
            }
        }
        for seat in DUOS_SEATS {
            let size = duos_handicap(handicaps, seat).map_or(DECK_SIZE, |handicap| handicap.deck_size);
            engine::validate_deck(duos_deck(&options.decks, seat), catalog, seat.as_str(), size)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// A `DuosState` argument, out of its envelope.
    fn duos_state_of(what: &str, state_json: &str) -> Result<DuosState, String> {
        parse(&format!("{what}: the state"), unwrapped(state_json))
    }

    /// `DuosReduceResult`'s JSON as the 1v1 result's: `{"state":…,"events":[…]}` plus `"error"` on a refusal.
    fn duos_reduce_result_json(result: DuosReduceResult) -> Result<String, String> {
        let DuosReduceResult { state, events, error } = result;
        let state = to_json(&state)?;
        let events = to_json(&events)?;
        let error = match &error {
            Some(refusal) => format!(",\"error\":{}", to_json(&refusal.to_string())?),
            None => String::new(),
        };
        Ok(format!("{{\"state\":{state},\"events\":{events}{error}}}"))
    }

    /// `DuosState` (TS `duosCreate({ seed, decks: { a1, a2, b1, b2 }, catalog?, teams?, handicaps?, dealt? })`).
    #[unsafe(no_mangle)]
    pub extern "C" fn duos_create(options_json: *const c_char) -> *mut c_char {
        boundary(|| {
            let options: DuosCreateOptions = parse("duosCreate", arg("duosCreate", options_json)?)?;
            check_duos_setup(&options)?;
            to_json(&duos_engine::duos_create(&options))
        })
    }

    /// `{ state, events, error? }`: both duels set up, all four mulligans open (B1, B2).
    #[unsafe(no_mangle)]
    pub extern "C" fn duos_begin(state_json: *const c_char) -> *mut c_char {
        boundary(|| {
            let state = duos_state_of("duosBegin", arg("duosBegin", state_json)?)?;
            duos_reduce_result_json(duos_engine::duos_begin(&state))
        })
    }

    /// `{ state, events, error? }` for one `DuosAction`; an illegal one is refused in `error`, never NULL.
    #[unsafe(no_mangle)]
    pub extern "C" fn duos_reduce(state_json: *const c_char, action_json: *const c_char) -> *mut c_char {
        boundary(|| {
            let state = duos_state_of("duosReduce", arg("duosReduce: the state", state_json)?)?;
            let action: DuosAction = parse(
                "duosReduce: the action",
                arg("duosReduce: the action", action_json)?,
            )?;
            duos_reduce_result_json(duos_engine::duos_reduce(&state, &action))
        })
    }

    /// `DuosActionBody[]`: everything `seat` may do now.
    #[unsafe(no_mangle)]
    pub extern "C" fn duos_legal_actions(state_json: *const c_char, seat: *const c_char) -> *mut c_char {
        boundary(|| {
            let state = duos_state_of(
                "duosLegalActions",
                arg("duosLegalActions: the state", state_json)?,
            )?;
            to_json(&duos_engine::duos_legal_actions(
                &state,
                duos_seat(arg("duosLegalActions: the seat", seat)?)?,
            ))
        })
    }

    /// `DuosView`: the only window the client has onto a Duos game (B23).
    #[unsafe(no_mangle)]
    pub extern "C" fn duos_view_for(state_json: *const c_char, seat: *const c_char) -> *mut c_char {
        boundary(|| {
            let state = duos_state_of("duosViewFor", arg("duosViewFor: the state", state_json)?)?;
            to_json(&duos_engine::duos_view_for(
                &state,
                duos_seat(arg("duosViewFor: the seat", seat)?)?,
            ))
        })
    }
}

#[cfg(not(feature = "duos"))]
mod duos_bindings {
    use std::ffi::c_char;

    use super::boundary;

    /// What every Duos binding answers in a module built without them.
    fn absent(what: &str) -> Result<String, String> {
        Err(format!(
            "{what}: this module was built without the `duos` feature, which only development builds turn on (/dev/duos)"
        ))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn duos_create(_options_json: *const c_char) -> *mut c_char {
        boundary(|| absent("duosCreate"))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn duos_begin(_state_json: *const c_char) -> *mut c_char {
        boundary(|| absent("duosBegin"))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn duos_reduce(_state_json: *const c_char, _action_json: *const c_char) -> *mut c_char {
        boundary(|| absent("duosReduce"))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn duos_legal_actions(_state_json: *const c_char, _seat: *const c_char) -> *mut c_char {
        boundary(|| absent("duosLegalActions"))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn duos_view_for(_state_json: *const c_char, _seat: *const c_char) -> *mut c_char {
        boundary(|| absent("duosViewFor"))
    }
}

// ---------------------------------------------------------------------------------------------
// The AI (SURFACE §9)
// ---------------------------------------------------------------------------------------------

/// Whether `seat` owes an action the AI should take now (R187, R188): 1 for yes, 0 for no, and
/// -1 with the message stowed for `jackioh_last_error` on failure, which `index.ts` throws.
#[unsafe(no_mangle)]
pub extern "C" fn ai_to_act(state_json: *const c_char, seat: *const c_char) -> i32 {
    match (|| {
        let state = state_of("aiToAct", arg("aiToAct: the state", state_json)?)?;
        Ok(jackioh_ai::ai_to_act(
            &state,
            player(arg("aiToAct: the seat", seat)?)?,
        ))
    })() {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(message) => {
            failed::<()>(message);
            -1
        }
    }
}

/// `ai_decide`'s options: the AI's own stream, where it stands, and the budget (default AI_BUDGET).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecideOptions {
    rng_seed: String,
    rng_cursor: u32,
    #[serde(default)]
    budget: Option<jackioh_ai::SearchBudget>,
}

/// The wall clock in milliseconds, as `Date.now()` reads it: the libc clock emscripten provides.
fn wall_now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|age| age.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// `{ decision: Decision | null, rngCursor }`. The AI draws only from its own stream, rebuilt here
/// from `(rngSeed, rngCursor)`; the cursor it stopped at comes back so the caller resumes it there.
/// `deadline_ms` is a wall-clock time in milliseconds: the search stops and answers with its best so
/// far once the clock passes it (TS `shouldStop`). `deadline_ms <= 0` means no clock, so the decision
/// depends only on `(state, seed, cursor, budget)`.
#[unsafe(no_mangle)]
pub extern "C" fn ai_decide(
    state_json: *const c_char,
    seat: *const c_char,
    options_json: *const c_char,
    deadline_ms: f64,
) -> *mut c_char {
    boundary(|| {
        let state = state_of("decide", arg("decide: the state", state_json)?)?;
        let seat = player(arg("decide: the seat", seat)?)?;
        let request: DecideOptions = parse("decide: the options", arg("decide: the options", options_json)?)?;
        let clock = move || wall_now_ms() >= deadline_ms;
        let should_stop = if deadline_ms > 0.0 {
            Some(&clock as &dyn Fn() -> bool)
        } else {
            None
        };
        let mut options = jackioh_ai::AiOptions {
            rng: Rng::new(&request.rng_seed, request.rng_cursor),
            budget: request.budget.unwrap_or(jackioh_ai::AI_BUDGET),
            should_stop,
        };
        let decision = jackioh_ai::decide(&state, seat, &mut options);
        to_json(&json!({ "decision": decision, "rngCursor": options.rng.cursor() }))
    })
}

/// `{ deck, rngCursor }` (TS `buildAiDeck(createRng(rngSeed, rngCursor), size, options)`). Every key
/// but `rngSeed`, `rngCursor` and `size` is `AiDeckOptions`' own, read by its own serde, so an absent
/// `banned` (the shadow ban), an absent `theme` (roll one) and a `null` one (none) keep their meanings,
/// and so does an absent `leanSet` (no lean, R1370).
#[unsafe(no_mangle)]
pub extern "C" fn build_ai_deck(options_json: *const c_char) -> *mut c_char {
    boundary(|| {
        let mut request: serde_json::Map<String, Value> =
            parse("buildAiDeck", arg("buildAiDeck", options_json)?)?;
        let seed = match request.remove("rngSeed") {
            Some(Value::String(seed)) => seed,
            _ => return Err("buildAiDeck: \"rngSeed\" must be a string".to_owned()),
        };
        let cursor = match request.remove("rngCursor") {
            None => 0,
            Some(value) => match value.as_u64().and_then(|n| u32::try_from(n).ok()) {
                Some(cursor) => cursor,
                None => return Err("buildAiDeck: \"rngCursor\" must be a whole number".to_owned()),
            },
        };
        let size = match request.remove("size").and_then(|value| value.as_i64()) {
            Some(size) => {
                i32::try_from(size).map_err(|_| "buildAiDeck: \"size\" is out of range".to_owned())?
            }
            None => return Err("buildAiDeck: \"size\" must be a whole number".to_owned()),
        };
        let options: jackioh_ai::AiDeckOptions = serde_json::from_value(Value::Object(request))
            .map_err(|error| format!("buildAiDeck: the options: {error}"))?;
        let mut rng = Rng::new(&seed, cursor);
        let deck = jackioh_ai::build_ai_deck(&mut rng, size, &options);
        to_json(&json!({ "deck": deck, "rngCursor": rng.cursor() }))
    })
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy (TS
/// `subsystems.chooseAction(state, seat, rng)`, whose default skip set is `jackioh_ai::random_action`'s),
/// drawing from `(rng_seed, rng_cursor)`.
#[unsafe(no_mangle)]
pub extern "C" fn choose_action(
    state_json: *const c_char,
    seat: *const c_char,
    rng_seed: *const c_char,
    rng_cursor: f64,
) -> *mut c_char {
    boundary(|| {
        let state = state_of("chooseAction", arg("chooseAction: the state", state_json)?)?;
        let seat = player(arg("chooseAction: the seat", seat)?)?;
        if !((0.0..=f64::from(u32::MAX)).contains(&rng_cursor) && rng_cursor.fract() == 0.0) {
            return Err("chooseAction: the cursor must be a whole number".to_owned());
        }
        let mut rng = Rng::new(arg("chooseAction: the seed", rng_seed)?, rng_cursor as u32);
        let action = jackioh_ai::random_action(&state, seat, &mut rng);
        to_json(&json!({ "action": action, "rngCursor": rng.cursor() }))
    })
}

/// `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS, AI_DECK }` for practice, the tutorial harness, the
/// "More cards from the newest set" toggle (`AI_DECK.leanMinShare`, R1370) and their tests.
/// `SHADOW_BAN_IDS` is TypeScript's `Object.keys(SHADOW_BAN).sort()` (`jackioh_ai::SHADOW_BAN_IDS`).
#[unsafe(no_mangle)]
pub extern "C" fn constants() -> *mut c_char {
    out(to_json(&json!({
        "AI_BUDGET": jackioh_ai::AI_BUDGET,
        "AI_GATE_BUDGET": jackioh_ai::AI_GATE_BUDGET,
        "SHADOW_BAN_IDS": jackioh_ai::SHADOW_BAN_IDS,
        "AI_DECK": jackioh_ai::AI_DECK,
    })))
}

// ---------------------------------------------------------------------------------------------
// The engine's tables the web's tests read (`subsystems.*`)
// ---------------------------------------------------------------------------------------------

/// ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost —
/// a `CraftPreview` — so the block editor shows what the reducer will say and the client decides
/// nothing (CLAUDE.md rule 7).
#[unsafe(no_mangle)]
pub extern "C" fn craft_preview(recipe_json: *const c_char, cost: f64) -> *mut c_char {
    boundary(|| {
        let recipe: CraftRecipe = parse(
            "craftPreview: the recipe",
            arg("craftPreview: the recipe", recipe_json)?,
        )?;
        if !(0.0..=f64::from(i32::MAX)).contains(&cost) || cost.fract() != 0.0 {
            return Err("craftPreview: the cost must be a whole number".to_owned());
        }
        to_json(&engine::subsystems::craft::craft_preview(&recipe, cost as i32))
    })
}

/// `{ chaosEffects: [{label}], chaosPlusEffects: [{label}], chaosMedEffects: [{label}], heroPowerNames,
/// heroPowers: [{name, x, title, radiantTitle, label, radiantLabel}] }`. `heroPowerNames` is
/// TypeScript's `HERO_POWERS.map((power) => power.name)`.
#[unsafe(no_mangle)]
pub extern "C" fn engine_tables() -> *mut c_char {
    boundary(|| {
        use jackioh_engine::subsystems::{
            call_to_chaos, call_to_chaos_meditative, call_to_chaos_plus, hero_power,
        };

        let chaos_effects: Vec<Value> = call_to_chaos::CHAOS_EFFECTS
            .iter()
            .map(|effect| json!({ "label": effect.label }))
            .collect();
        let chaos_plus_effects: Vec<Value> = call_to_chaos_plus::CHAOS_PLUS_EFFECTS
            .iter()
            .map(|effect| json!({ "label": effect.label }))
            .collect();
        let chaos_med_effects: Vec<Value> = call_to_chaos_meditative::CHAOS_MED_EFFECTS
            .iter()
            .map(|effect| json!({ "label": effect.label }))
            .collect();
        let hero_powers: Vec<Value> = hero_power::HERO_POWERS
            .iter()
            .map(|power| {
                json!({
                    "name": power.name,
                    "x": power.x,
                    "title": power.title,
                    "radiantTitle": power.radiant_title,
                    "label": power.label,
                    "radiantLabel": power.radiant_label,
                })
            })
            .collect();
        let hero_power_names: Vec<Value> = hero_powers.iter().map(|power| power["name"].clone()).collect();
        to_json(&json!({
            "chaosEffects": chaos_effects,
            "chaosPlusEffects": chaos_plus_effects,
            "chaosMedEffects": chaos_med_effects,
            "heroPowerNames": hero_power_names,
            "heroPowers": hero_powers,
        }))
    })
}

// ---------------------------------------------------------------------------------------------
// The validator (SPEC §9.4: one module, client and server)
// ---------------------------------------------------------------------------------------------

/// `checkDeckDraft`'s input as it crosses: TypeScript's two predicates cannot, so the page answers
/// them first. `deckable` is every distinct card id of `cards` that `isDeckable` accepted;
/// `portraitKnown` is `isPortrait(portrait)`, absent when the page had no `isPortrait` or no portrait
/// to ask about (D5 then has nothing to check against, as in TypeScript).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeckDraftRequest {
    name: String,
    cards: Vec<String>,
    deckable: Vec<String>,
    #[serde(default)]
    portrait: Option<String>,
    #[serde(default)]
    portrait_known: Option<bool>,
    name_max_length: usize,
}

fn check_deck_draft(input_json: &str) -> Result<String, String> {
    use jackioh_engine::validator;

    let request: DeckDraftRequest = parse("checkDeckDraft", input_json)?;
    let deckable = request.deckable;
    let is_deckable = move |card_id: &str| deckable.iter().any(|id| id == card_id);
    let known = request.portrait_known.unwrap_or(true);
    let is_portrait = move |_portrait: &str| known;
    let input = validator::DeckDraftInput {
        name: request.name,
        cards: request.cards,
        is_deckable: &is_deckable,
        portrait: request.portrait,
        is_portrait: request
            .portrait_known
            .map(|_| &is_portrait as &dyn Fn(&str) -> bool),
        name_max_length: request.name_max_length,
    };
    to_json(&validator::check_deck_draft(&input))
}

/// A loadout input (`DeckInput`, `LoadoutInput`) whose `catalog.cards` may hold partial definitions.
///
/// The rules read three fields of a card's definition: `name` (its label in a message), `token` and
/// `tags` (L3, `validator.rs`); L6 asks only whether the id has one. TypeScript's validator took any
/// object with those, and the web's tests hand it `{ id, name, token, tags }` (`routes/play.test.tsx`),
/// while Rust's `CardDef` requires every field. So each definition that crosses is laid over a complete
/// one first: its own fields win, and the ones it lacks come from a registered definition with
/// `token: false` and no tags, which no rule reads. A full definition crosses unchanged.
fn parse_loadout_input<T: DeserializeOwned>(call: &str, input_json: &str) -> Result<T, String> {
    let mut input: Value = parse(call, input_json)?;
    if let Some(cards) = input.pointer_mut("/catalog/cards").and_then(Value::as_object_mut) {
        let mut template = engine::registered_catalog()
            .values()
            .next()
            .map(|def| serde_json::to_value(def).unwrap_or(Value::Null))
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        template.insert("token".to_owned(), Value::Bool(false));
        template.insert("tags".to_owned(), json!([]));
        for (card_id, def) in cards.iter_mut() {
            let Some(given) = def.as_object() else { continue };
            let mut complete = template.clone();
            complete.insert("id".to_owned(), Value::String(card_id.clone()));
            for (key, value) in given {
                complete.insert(key.clone(), value.clone());
            }
            *def = Value::Object(complete);
        }
    }
    serde_json::from_value(input).map_err(|error| format!("{call}: {error}"))
}

/// One validator call by its TypeScript name; the input and the output are that function's
/// argument and result, as JSON (SURFACE §10.1). `checkDeckDraft` takes `DeckDraftRequest` above.
#[unsafe(no_mangle)]
pub extern "C" fn validator(call: *const c_char, input_json: *const c_char) -> *mut c_char {
    boundary(|| {
        use jackioh_engine::validator;

        let call = arg("validator: the call", call)?;
        let input_json = arg("validator: the input", input_json)?;
        match call {
            "validateDeck" => {
                let input: validator::DeckInput = parse_loadout_input(call, input_json)?;
                to_json(&validator::validate_deck(&input))
            }
            "validateLoadout" => {
                let input: validator::LoadoutInput = parse_loadout_input(call, input_json)?;
                to_json(&validator::validate_loadout(&input))
            }
            "validateTrio" => {
                let input: validator::LoadoutInput = parse_loadout_input(call, input_json)?;
                to_json(&validator::validate_trio(&input))
            }
            "checkDeckDraft" => check_deck_draft(input_json),
            "checkTrioDraft" => {
                let input: validator::TrioDraftInput = parse(call, input_json)?;
                to_json(&validator::check_trio_draft(&input))
            }
            "checkImportRoom" => {
                let input: validator::ImportRoomInput = parse(call, input_json)?;
                to_json(&validator::check_import_room(&input))
            }
            "trioConflicts" => {
                let decks: Vec<validator::LoadoutDeck> = parse(call, input_json)?;
                to_json(&validator::trio_conflicts(&decks))
            }
            "normalizeName" => {
                let raw: String = parse(call, input_json)?;
                to_json(&validator::normalize_name(&raw))
            }
            other => Err(format!("validator: no such call {other:?}")),
        }
    })
}

/// The program's entry: it does nothing. The client instantiates the module, then calls `init()`,
/// which registers the catalog and the card scripts. (A `cdylib` link through `emcc` produces a
/// bare `.wasm` with no JavaScript glue; only a program link emits the `MODULARIZE` factory the
/// client loads, so the bindings live in a binary.)
fn main() {}
