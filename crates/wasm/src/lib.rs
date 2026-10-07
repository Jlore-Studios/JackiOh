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
//! The clock. The pure crates never read one (CLAUDE.md rule 4, SURFACE §3). The AI's wall-clock cap
//! is this crate's: `ai_decide` hands `decide` a `should_stop` that reads `js_sys::Date::now()`
//! against the deadline the page computed, as TypeScript's `shouldStop` read `performance.now()`.

use std::str::FromStr;

use jackioh_engine::{
    self as engine, Action, CardDefs, CreateGameArgs, DECK_SIZE, FoldArgs, GameState, PLAYER_IDS, PerPlayerOpt,
    PlayerId, Rng,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------------------------
// JSON in, JSON out
// ---------------------------------------------------------------------------------------------

/// Parses one JSON argument, naming the binding and the argument when it does not parse.
fn parse<T: DeserializeOwned>(what: &str, text: &str) -> Result<T, JsError> {
    serde_json::from_str(text).map_err(|error| JsError::new(&format!("{what}: {error}")))
}

/// The JSON text of a result.
fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<String, JsError> {
    serde_json::to_string(value).map_err(|error| JsError::new(&format!("could not serialise the result: {error}")))
}

/// `"p1"` or `"p2"`; anything else is refused with `PlayerId`'s own message.
fn player(text: &str) -> Result<PlayerId, JsError> {
    PlayerId::from_str(text).map_err(|error| JsError::new(&error))
}

fn state_of(what: &str, state_json: &str) -> Result<GameState, JsError> {
    parse(&format!("{what}: the state"), state_json)
}

/// The checks TypeScript's `createGame` threw on, in its order (R180's handicaps first, so a bad
/// deck size is named as the handicap's fault, then each deck against its seat's size, R184), run
/// before the engine is called: Rust's `create_game` panics where TypeScript threw (SURFACE §6.1),
/// and a panic in WebAssembly is a trap the page cannot catch as the error TypeScript gave it. The
/// functions are the engine's own (`state.rs`), so the messages are the engine's verbatim.
fn check_setup(args: &CreateGameArgs) -> Result<(), JsError> {
    let catalog: &CardDefs = match &args.catalog {
        Some(catalog) => catalog,
        None => engine::registered_catalog(),
    };
    let no_handicaps = PerPlayerOpt::default();
    let handicaps = args.handicaps.as_ref().unwrap_or(&no_handicaps);
    for seat in PLAYER_IDS {
        if let Some(handicap) = handicaps.get(seat) {
            engine::validate_handicap(handicap, seat.as_str())?;
        }
    }
    for seat in PLAYER_IDS {
        let size = handicaps.get(seat).map_or(DECK_SIZE, |handicap| handicap.deck_size);
        let deck = match seat {
            PlayerId::P1 => &args.decks.0,
            PlayerId::P2 => &args.decks.1,
        };
        engine::validate_deck(deck, catalog, seat.as_str(), size)?;
    }
    Ok(())
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
    to_json(engine::registered_catalog())
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
#[wasm_bindgen]
pub fn catalog_version() -> String {
    jackioh_cards::catalog_version().to_string()
}

// ---------------------------------------------------------------------------------------------
// The engine (SURFACE §6.1)
// ---------------------------------------------------------------------------------------------

/// `GameState` (TS `createGame({ seed, decks, catalog?, handicaps?, lastBoards?, glitchBoards?, dealt? })`).
#[wasm_bindgen]
pub fn create_game(args_json: &str) -> Result<String, JsError> {
    let args: CreateGameArgs = parse("createGame", args_json)?;
    check_setup(&args)?;
    to_json(&engine::create_game(&args))
}

/// `ReduceResult`: `{ state, events, error? }`.
#[wasm_bindgen]
pub fn begin_game(state_json: &str) -> Result<String, JsError> {
    let state = state_of("beginGame", state_json)?;
    to_json(&engine::begin_game(&state))
}

/// `ReduceResult`. An illegal action is refused in `error`, never thrown (SURFACE §6.1).
#[wasm_bindgen]
pub fn reduce(state_json: &str, action_json: &str) -> Result<String, JsError> {
    let state = state_of("reduce", state_json)?;
    let action: Action = parse("reduce: the action", action_json)?;
    to_json(&engine::reduce(&state, &action))
}

/// `ActionBody[]`: everything `player` may legally do now.
#[wasm_bindgen]
pub fn legal_actions(state_json: &str, player_id: &str) -> Result<String, JsError> {
    let state = state_of("legalActions", state_json)?;
    to_json(&engine::legal_actions(&state, player(player_id)?))
}

/// `PlayerView`: the only window the client has onto the game (SPEC §10.8).
#[wasm_bindgen]
pub fn view_for(state_json: &str, player_id: &str) -> Result<String, JsError> {
    let state = state_of("viewFor", state_json)?;
    to_json(&engine::view_for(&state, player(player_id)?))
}

/// `"p1"`, `"p2"`, or `""` when no seat owes an action.
#[wasm_bindgen]
pub fn seat_to_act(state_json: &str) -> Result<String, JsError> {
    let state = state_of("seatToAct", state_json)?;
    Ok(engine::seat_to_act(&state).map_or_else(String::new, |seat| seat.as_str().to_string()))
}

/// SURFACE §5.2's hash, eight lower-case hex digits: TypeScript's `hashState`, bit for bit.
#[wasm_bindgen]
pub fn hash_state(state_json: &str) -> Result<String, JsError> {
    let state = state_of("hashState", state_json)?;
    Ok(engine::hash_state(&state))
}

/// `{ state, errors: [{ nonce, error }] }` (TS `fold`). The setup is checked first, as TypeScript's
/// `fold` threw on it through `createGame`; a refused action in the log is collected, not thrown.
#[wasm_bindgen]
pub fn fold(args_json: &str) -> Result<String, JsError> {
    // The fold's input is `createGame`'s plus `log`, so the same JSON reads as both.
    let setup: CreateGameArgs = parse("fold", args_json)?;
    check_setup(&setup)?;
    let args: FoldArgs = parse("fold", args_json)?;
    to_json(&engine::fold(&args))
}

/// `LastBoardEntry[]`: the board `seat` takes away (R417, R508).
#[wasm_bindgen]
pub fn last_board_for(state_json: &str, seat: &str) -> Result<String, JsError> {
    let state = state_of("lastBoardFor", state_json)?;
    to_json(&engine::last_board_for(&state, player(seat)?))
}

/// `"p1"` or `"p2"`: the seat the player who began in `home` plays now (R677).
#[wasm_bindgen]
pub fn seat_played_by(state_json: &str, home: &str) -> Result<String, JsError> {
    let state = state_of("seatPlayedBy", state_json)?;
    Ok(engine::seat_played_by(&state, player(home)?).as_str().to_string())
}

/// `CardInstance` JSON, or `"null"` when no card has that id (`state.rs`'s `find_instance`).
#[wasm_bindgen]
pub fn find_instance(state_json: &str, instance_id: &str) -> Result<String, JsError> {
    let state = state_of("findInstance", state_json)?;
    to_json(&engine::find_instance(&state, instance_id))
}

// ---------------------------------------------------------------------------------------------
// The AI (SURFACE §9)
// ---------------------------------------------------------------------------------------------

/// Whether `seat` owes an action the AI should take now (R187, R188).
#[wasm_bindgen]
pub fn ai_to_act(state_json: &str, seat: &str) -> Result<bool, JsError> {
    let state = state_of("aiToAct", state_json)?;
    Ok(jackioh_ai::ai_to_act(&state, player(seat)?))
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

/// `{ decision: Decision | null, rngCursor }`. The AI draws only from its own stream, rebuilt here
/// from `(rngSeed, rngCursor)`; the cursor it stopped at comes back so the caller resumes it there.
/// `deadline_ms` is a `Date.now()` time: the search stops and answers with its best so far once the
/// clock passes it (TS `shouldStop`). `deadline_ms <= 0` means no clock, so the decision depends
/// only on `(state, seed, cursor, budget)`.
#[wasm_bindgen]
pub fn ai_decide(state_json: &str, seat: &str, options_json: &str, deadline_ms: f64) -> Result<String, JsError> {
    let state = state_of("decide", state_json)?;
    let seat = player(seat)?;
    let request: DecideOptions = parse("decide: the options", options_json)?;
    let clock = move || js_sys::Date::now() >= deadline_ms;
    let should_stop = if deadline_ms > 0.0 { Some(&clock as &dyn Fn() -> bool) } else { None };
    let mut options = jackioh_ai::AiOptions {
        rng: Rng::new(&request.rng_seed, request.rng_cursor),
        budget: request.budget.unwrap_or(jackioh_ai::AI_BUDGET),
        should_stop,
    };
    let decision = jackioh_ai::decide(&state, seat, &mut options);
    to_json(&json!({ "decision": decision, "rngCursor": options.rng.cursor() }))
}

/// `{ deck, rngCursor }` (TS `buildAiDeck(createRng(rngSeed, rngCursor), size, options)`). Every key
/// but `rngSeed`, `rngCursor` and `size` is `AiDeckOptions`' own, read by its own serde, so an absent
/// `banned` (the shadow ban), an absent `theme` (roll one) and a `null` one (none) keep their meanings.
#[wasm_bindgen]
pub fn build_ai_deck(options_json: &str) -> Result<String, JsError> {
    let mut request: serde_json::Map<String, Value> = parse("buildAiDeck", options_json)?;
    let seed = match request.remove("rngSeed") {
        Some(Value::String(seed)) => seed,
        _ => return Err(JsError::new("buildAiDeck: \"rngSeed\" must be a string")),
    };
    let cursor = match request.remove("rngCursor") {
        None => 0,
        Some(value) => match value.as_u64().and_then(|n| u32::try_from(n).ok()) {
            Some(cursor) => cursor,
            None => return Err(JsError::new("buildAiDeck: \"rngCursor\" must be a whole number")),
        },
    };
    let size = match request.remove("size").and_then(|value| value.as_i64()) {
        Some(size) => i32::try_from(size).map_err(|_| JsError::new("buildAiDeck: \"size\" is out of range"))?,
        None => return Err(JsError::new("buildAiDeck: \"size\" must be a whole number")),
    };
    let options: jackioh_ai::AiDeckOptions = serde_json::from_value(Value::Object(request))
        .map_err(|error| JsError::new(&format!("buildAiDeck: the options: {error}")))?;
    let mut rng = Rng::new(&seed, cursor);
    let deck = jackioh_ai::build_ai_deck(&mut rng, size, &options);
    to_json(&json!({ "deck": deck, "rngCursor": rng.cursor() }))
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy (TS
/// `subsystems.chooseAction(state, seat, rng)`, whose default skip set is `jackioh_ai::random_action`'s),
/// drawing from `(rng_seed, rng_cursor)`.
#[wasm_bindgen]
pub fn choose_action(state_json: &str, seat: &str, rng_seed: &str, rng_cursor: f64) -> Result<String, JsError> {
    let state = state_of("chooseAction", state_json)?;
    let seat = player(seat)?;
    if !((0.0..=f64::from(u32::MAX)).contains(&rng_cursor) && rng_cursor.fract() == 0.0) {
        return Err(JsError::new("chooseAction: the cursor must be a whole number"));
    }
    let mut rng = Rng::new(rng_seed, rng_cursor as u32);
    let action = jackioh_ai::random_action(&state, seat, &mut rng);
    to_json(&json!({ "action": action, "rngCursor": rng.cursor() }))
}

/// `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS }` for practice, the tutorial harness and their tests.
/// `SHADOW_BAN_IDS` is TypeScript's `Object.keys(SHADOW_BAN).sort()` (`jackioh_ai::SHADOW_BAN_IDS`).
#[wasm_bindgen]
pub fn constants() -> Result<String, JsError> {
    to_json(&json!({
        "AI_BUDGET": jackioh_ai::AI_BUDGET,
        "AI_GATE_BUDGET": jackioh_ai::AI_GATE_BUDGET,
        "SHADOW_BAN_IDS": jackioh_ai::SHADOW_BAN_IDS,
    }))
}

// ---------------------------------------------------------------------------------------------
// The engine's tables the web's tests read (`subsystems.*`)
// ---------------------------------------------------------------------------------------------

/// `{ chaosEffects: [{label}], chaosPlusEffects: [{label}], heroPowerNames, heroPowers: [{name, x,
/// title, radiantTitle, label, radiantLabel}] }`. `heroPowerNames` is TypeScript's
/// `HERO_POWERS.map((power) => power.name)`.
#[wasm_bindgen]
pub fn engine_tables() -> Result<String, JsError> {
    use jackioh_engine::subsystems::{call_to_chaos, call_to_chaos_plus, hero_power};

    let chaos_effects: Vec<Value> =
        call_to_chaos::CHAOS_EFFECTS.iter().map(|effect| json!({ "label": effect.label })).collect();
    let chaos_plus_effects: Vec<Value> =
        call_to_chaos_plus::CHAOS_PLUS_EFFECTS.iter().map(|effect| json!({ "label": effect.label })).collect();
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
        "heroPowerNames": hero_power_names,
        "heroPowers": hero_powers,
    }))
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

fn check_deck_draft(input_json: &str) -> Result<String, JsError> {
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
        is_portrait: request.portrait_known.map(|_| &is_portrait as &dyn Fn(&str) -> bool),
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
fn parse_loadout_input<T: DeserializeOwned>(call: &str, input_json: &str) -> Result<T, JsError> {
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
    serde_json::from_value(input).map_err(|error| JsError::new(&format!("{call}: {error}")))
}

/// One validator call by its TypeScript name; the input and the output are that function's
/// argument and result, as JSON (SURFACE §10.1). `checkDeckDraft` takes `DeckDraftRequest` above.
#[wasm_bindgen]
pub fn validator(call: &str, input_json: &str) -> Result<String, JsError> {
    use jackioh_engine::validator;

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
        other => Err(JsError::new(&format!("validator: no such call {other:?}"))),
    }
}
