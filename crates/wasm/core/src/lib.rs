//! `jackioh-wasm-core`: bodies both browser modules bind (#1505). JSON strings in and out,
//! with state kept behind opaque `EngineState` brand (CLAUDE.md rule 7).
//!
//! State cache (`STATE_CACHE`): parses JSON once and keeps recent states under exact text.
//! Pure crates read no clock (`ai_decide` takes `now_ms`; CLAUDE.md rule 4, SURFACE §3).
//! Setup check (`CheckSetup`): normal module refuses custom cards (R915009), Luau judges them (R915005, R915006).

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use jackioh_engine::prelude::IndexMap;
use jackioh_engine::{
    self as engine, Action, CardDef, CardDefs, CraftRecipe, CreateGameArgs, FoldArgs, GameState, PlayerId,
    ReduceResult, ReplayCheckpoints, ReplayRecord, Rng, SetupKind,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

// JSON in, JSON out

/// Parses one JSON argument, naming the binding and the argument when it does not parse.
pub fn parse<T: DeserializeOwned>(what: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|error| format!("{what}: {error}"))
}

/// The JSON text of a result.
pub fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("could not serialize the result: {error}"))
}

/// `"p1"` or `"p2"`; anything else is refused with `PlayerId`'s own message.
pub fn player(text: &str) -> Result<PlayerId, String> {
    PlayerId::from_str(text).map_err(|error| error.to_string())
}

/// Number of states kept in `STATE_CACHE`.
const STATE_CACHE: usize = 4;

thread_local! {
    /// Most recently used first. WebAssembly runs one thread; `thread_local!` keeps data without `static mut`.
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

/// 8-byte chunk equality comparison for state cache text matching.
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

/// Envelope mark `["\u{a0}",<state>]` to accelerate string copying via `TextEncoder.encodeInto`.
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

/// `ReduceResult` JSON with new state cached under its text.
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

// The setup check each module runs before the engine sees a setup

/// Setup check run before engine sees setup (R915009, R915005, R915006, R1705003).
pub type CheckSetup = fn(&CreateGameArgs, SetupKind) -> Result<(), String>;

/// Checks setup before `create_game` (SURFACE §6.1), ordering R180 handicaps before R184 deck sizes (R1705003).
pub fn check_decks(
    args: &CreateGameArgs,
    customs: &IndexMap<String, CardDef>,
    kind: SetupKind,
) -> Result<(), String> {
    let catalog: &CardDefs = match &args.catalog {
        Some(catalog) => catalog,
        None => engine::registered_catalog(),
    };
    engine::validate_setup(args, catalog, customs, kind).map_err(|error| error.to_string())
}

// Boot and the catalog

/// The registered catalog, `CardDefs`.
pub fn catalog() -> Result<String, String> {
    to_json(engine::registered_catalog())
}

/// The catalog version compiled in from `crates/cards/patches/patches.json` (SURFACE §11.3).
pub fn catalog_version() -> String {
    jackioh_cards::catalog_version().to_string()
}

/// R767000: shipped versions this module carries for replays, oldest first.
pub fn carried_versions() -> Result<String, String> {
    to_json(&jackioh_cards::carried_versions())
}

// The engine (SURFACE §6.1)

/// `GameState` from JSON arguments.
pub fn create_game(args_json: &str, check_setup: CheckSetup) -> Result<String, String> {
    let args: CreateGameArgs = parse("createGame", args_json)?;
    check_setup(&args, SetupKind::New)?;
    returned_state(engine::create_game(&args))
}

/// `ReduceResult`: `{ state, events, error? }`.
pub fn begin_game(state_json: &str) -> Result<String, String> {
    let state = state_of("beginGame", state_json)?;
    reduce_result_json(engine::begin_game(&state))
}

/// `ReduceResult`. An illegal action is refused in `error`, never thrown (SURFACE §6.1).
pub fn reduce(state_json: &str, action_json: &str) -> Result<String, String> {
    let state = state_of("reduce", state_json)?;
    let action: Action = parse("reduce: the action", action_json)?;
    reduce_result_json(engine::reduce(&state, &action))
}

/// `ActionBody[]`: everything `player` may legally do now.
pub fn legal_actions(state_json: &str, player_id: &str) -> Result<String, String> {
    let state = state_of("legalActions", state_json)?;
    to_json(&engine::legal_actions(&state, player(player_id)?))
}

/// `PlayerView`: the only window the client has onto the game (SPEC §10.8).
pub fn view_for(state_json: &str, player_id: &str) -> Result<String, String> {
    let state = state_of("viewFor", state_json)?;
    to_json(&engine::view_for(&state, player(player_id)?))
}

/// `"p1"`, `"p2"`, or `""` when no seat owes an action.
pub fn seat_to_act(state_json: &str) -> Result<String, String> {
    let state = state_of("seatToAct", state_json)?;
    Ok(engine::seat_to_act(&state).map_or_else(String::new, |seat| seat.as_str().to_string()))
}

/// SURFACE §5.2's hash, eight lower-case hex digits.
pub fn hash_state(state_json: &str) -> Result<String, String> {
    let state = state_of("hashState", state_json)?;
    Ok(engine::hash_state(&state))
}

/// `{ state, errors: [{ nonce, error }] }`. Setup is checked first; refused actions are collected.
pub fn fold(args_json: &str, check_setup: CheckSetup) -> Result<String, String> {
    // The fold's input is `createGame`'s plus `log`, so the same JSON reads as both.
    let setup: CreateGameArgs = parse("fold", args_json)?;
    check_setup(&setup, SetupKind::Recorded)?;
    let args: FoldArgs = parse("fold", args_json)?;
    to_json(&engine::fold(&args))
}

/// R768: `ReplayOpen`. Setup is checked under the catalog the game was played on (R767000).
pub fn replay_open(args_json: &str, record_json: &str, check_setup: CheckSetup) -> Result<String, String> {
    let record: ReplayRecord = parse("replayOpen: the record", record_json)?;
    let build = jackioh_cards::catalog_version();
    if let Ok(shipped) = engine::catalog_for(&record.catalog_version, build) {
        let setup: CreateGameArgs = parse("replayOpen", args_json)?;
        engine::with_shipped(shipped, || check_setup(&setup, SetupKind::Recorded))?;
    }
    let args: FoldArgs = parse("replayOpen", args_json)?;
    to_json(&engine::replay_open(&args, &record, build))
}

/// R768: `ReplayPage`, `{ from, steps: [{ step, turn, view }], reduces }`.
pub fn replay_page(checkpoints_json: &str, seat: &str, from: u32, count: u32) -> Result<String, String> {
    let checkpoints: ReplayCheckpoints = parse("replayPage: the checkpoints", checkpoints_json)?;
    to_json(&engine::replay_page(
        &checkpoints,
        player(seat)?,
        from as usize,
        count as usize,
    ))
}

/// `LastBoardEntry[]`: the board `seat` takes away (R417, R508).
pub fn last_board_for(state_json: &str, seat: &str) -> Result<String, String> {
    let state = state_of("lastBoardFor", state_json)?;
    to_json(&engine::last_board_for(&state, player(seat)?))
}

/// `"p1"` or `"p2"`: the seat the player who began in `home` plays now (R677).
pub fn seat_played_by(state_json: &str, home: &str) -> Result<String, String> {
    let state = state_of("seatPlayedBy", state_json)?;
    Ok(engine::seat_played_by(&state, player(home)?).as_str().to_string())
}

/// `CardInstance` JSON, or `"null"` when no card has that id.
pub fn find_instance(state_json: &str, instance_id: &str) -> Result<String, String> {
    let state = state_of("findInstance", state_json)?;
    to_json(&engine::find_instance(&state, instance_id))
}

/// `EmoteId[]`: emote hand `seat` is dealt for `seed` (R1341), matching the match actor (R1342).
pub fn deal_emote_hand(seed: &str, seat: &str) -> Result<String, String> {
    to_json(&engine::deal_emote_hand(seed, player(seat)?))
}

/// R1420 preview: treats `sets` as shipped sets for module lifetime (#552).
pub fn preview_sets(sets_json: &str) -> Result<(), String> {
    let sets: Vec<engine::SetName> = parse("previewSets: the sets", sets_json)?;
    preview(&sets)
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

// The AI (SURFACE §9)

/// Whether `seat` owes an action the AI should take now (R187, R188).
pub fn ai_to_act(state_json: &str, seat: &str) -> Result<bool, String> {
    let state = state_of("aiToAct", state_json)?;
    Ok(jackioh_ai::ai_to_act(&state, player(seat)?))
}

/// `ai_decide`'s options: AI stream, state, and search budget.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecideOptions {
    rng_seed: String,
    rng_cursor: u32,
    #[serde(default)]
    budget: Option<jackioh_ai::SearchBudget>,
}

/// `{ decision: Decision | null, rngCursor }`. Search stops once clock passes `deadline_ms`.
pub fn ai_decide(
    state_json: &str,
    seat: &str,
    options_json: &str,
    deadline_ms: f64,
    now_ms: fn() -> f64,
) -> Result<String, String> {
    let state = state_of("decide", state_json)?;
    let seat = player(seat)?;
    let request: DecideOptions = parse("decide: the options", options_json)?;
    let clock = move || now_ms() >= deadline_ms;
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
}

/// R2102000: whether `seat` has a sure win this turn by what it can see (lethal hint).
pub fn lethal_hint(
    state_json: &str,
    seat: &str,
    deadline_ms: f64,
    now_ms: fn() -> f64,
) -> Result<bool, String> {
    let state = state_of("lethalHint", state_json)?;
    let seat = player(seat)?;
    let clock = move || now_ms() >= deadline_ms;
    let should_stop = if deadline_ms > 0.0 {
        Some(&clock as &dyn Fn() -> bool)
    } else {
        None
    };
    Ok(jackioh_ai::lethal_hint(&state, seat, should_stop))
}

/// `{ deck, rngCursor }`: builds an AI deck. An absent `leanSet` implies no lean (R1370).
pub fn build_ai_deck(options_json: &str) -> Result<String, String> {
    let mut request: serde_json::Map<String, Value> = parse("buildAiDeck", options_json)?;
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
        Some(size) => i32::try_from(size).map_err(|_| "buildAiDeck: \"size\" is out of range".to_owned())?,
        None => return Err("buildAiDeck: \"size\" must be a whole number".to_owned()),
    };
    let options: jackioh_ai::AiDeckOptions = serde_json::from_value(Value::Object(request))
        .map_err(|error| format!("buildAiDeck: the options: {error}"))?;
    let mut rng = Rng::new(&seed, cursor);
    let deck = jackioh_ai::build_ai_deck(&mut rng, size, &options);
    to_json(&json!({ "deck": deck, "rngCursor": rng.cursor() }))
}

/// `{ action: ActionBody | null, rngCursor }`: §10.7's random policy drawing from `(rng_seed, rng_cursor)`.
pub fn choose_action(
    state_json: &str,
    seat: &str,
    rng_seed: &str,
    rng_cursor: f64,
) -> Result<String, String> {
    let state = state_of("chooseAction", state_json)?;
    let seat = player(seat)?;
    if !((0.0..=f64::from(u32::MAX)).contains(&rng_cursor) && rng_cursor.fract() == 0.0) {
        return Err("chooseAction: the cursor must be a whole number".to_owned());
    }
    let mut rng = Rng::new(rng_seed, rng_cursor as u32);
    let action = jackioh_ai::random_action(&state, seat, &mut rng);
    to_json(&json!({ "action": action, "rngCursor": rng.cursor() }))
}

/// Constants for practice, tutorial harness, leanMinShare (R1370), and fixtures (#1113).
pub fn constants() -> Result<String, String> {
    to_json(&json!({
        "AI_BUDGET": jackioh_ai::AI_BUDGET,
        "AI_GATE_BUDGET": jackioh_ai::AI_GATE_BUDGET,
        "SHADOW_BAN_IDS": jackioh_ai::SHADOW_BAN_IDS,
        "FIXTURE_BANNED_IDS": jackioh_ai::FIXTURE_BANNED_IDS,
        "AI_DECK": jackioh_ai::AI_DECK,
    }))
}

/// R1011000: learned evaluation parity fixture as computed by this build.
pub fn learned_parity(fixture_json: &str) -> Result<String, String> {
    let fixture: serde_json::Value = serde_json::from_str(fixture_json)
        .map_err(|error| format!("the parity fixture is not JSON: {error}"))?;
    let answers = jackioh_ai::parity_answers(&fixture)?;
    to_json(&answers)
}

// The engine's tables the web's tests read (`subsystems.*`)

/// ME-CRAFT (Meditative #17, R880): engine verdict on recipe at cost (CLAUDE.md rule 7, R1563007).
pub fn craft_preview(recipe_json: &str, cost: f64, prompt_json: &str) -> Result<String, String> {
    let recipe: CraftRecipe = parse("craftPreview: the recipe", recipe_json)?;
    if !(0.0..=f64::from(i32::MAX)).contains(&cost) || cost.fract() != 0.0 {
        return Err("craftPreview: the cost must be a whole number".to_owned());
    }
    let prompt: CraftPrompt = parse("craftPreview: the prompt", prompt_json)?;
    to_json(&engine::subsystems::craft::craft_preview_for(
        &recipe,
        cost as i32,
        prompt.craft_mode,
        prompt.craft_offer.as_ref(),
    ))
}

/// Slice of craft prompt `craft_preview` reads.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CraftPrompt {
    #[serde(default)]
    craft_mode: Option<engine::CraftMode>,
    #[serde(default)]
    craft_offer: Option<engine::CraftOffer>,
}

/// Engine tables for chaos effects and hero powers.
pub fn engine_tables() -> Result<String, String> {
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
}

// The validator (SPEC §9.4: one module, client and server)

/// `checkDeckDraft`'s input as it crosses from the client.
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

/// A loadout input whose `catalog.cards` may hold partial definitions laid over templates.
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

/// One validator call by name, with JSON argument and result (SURFACE §10.1).
pub fn validator(call: &str, input_json: &str) -> Result<String, String> {
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
        // R915001, R915000: custom card frame validation and definition.
        "validateCustomCard" => {
            let card: jackioh_engine::CustomCard = parse(call, input_json)?;
            to_json(&jackioh_engine::custom::validate_custom_card(&card))
        }
        "customDef" => {
            let card: jackioh_engine::CustomCard = parse(call, input_json)?;
            to_json(&jackioh_engine::custom::custom_def(&card))
        }
        other => Err(format!("validator: no such call {other:?}")),
    }
}

// Duos co-op (#936 SPEC S4, S8): bodies over two duels for `/dev/duos` hotseat.
// Only the `duos` feature builds them; without it each body returns the reason.

#[cfg(feature = "duos")]
mod duos_bodies {
    use std::str::FromStr;

    use jackioh_engine::duos::{self as duos_engine, DuosReduceResult, DuosState};
    use jackioh_engine::wire::{
        DUOS_SEATS, DuosAction, DuosCreateOptions, DuosDecks, DuosHandicaps, DuosSeat,
    };
    use jackioh_engine::{self as engine, CardDefs, DECK_SIZE, Handicap};

    use super::{parse, to_json, unwrapped};

    /// `"a1"`, `"a2"`, `"b1"` or `"b2"`; anything else is refused with `DuosSeat`'s own message.
    fn duos_seat(text: &str) -> Result<DuosSeat, String> {
        DuosSeat::from_str(text).map_err(|error| error.to_string())
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

    /// Setup check for four seats, validating handicaps then deck sizes.
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

    /// `DuosState` from options JSON.
    pub fn duos_create(options_json: &str) -> Result<String, String> {
        let options: DuosCreateOptions = parse("duosCreate", options_json)?;
        check_duos_setup(&options)?;
        to_json(&duos_engine::duos_create(&options))
    }

    /// `{ state, events, error? }`: both duels set up, all four mulligans open (B1, B2).
    pub fn duos_begin(state_json: &str) -> Result<String, String> {
        let state = duos_state_of("duosBegin", state_json)?;
        duos_reduce_result_json(duos_engine::duos_begin(&state))
    }

    /// `{ state, events, error? }` for one `DuosAction`; an illegal one is refused in `error`, never thrown.
    pub fn duos_reduce(state_json: &str, action_json: &str) -> Result<String, String> {
        let state = duos_state_of("duosReduce", state_json)?;
        let action: DuosAction = parse("duosReduce: the action", action_json)?;
        duos_reduce_result_json(duos_engine::duos_reduce(&state, &action))
    }

    /// `DuosActionBody[]`: everything `seat` may do now.
    pub fn duos_legal_actions(state_json: &str, seat: &str) -> Result<String, String> {
        let state = duos_state_of("duosLegalActions", state_json)?;
        to_json(&duos_engine::duos_legal_actions(&state, duos_seat(seat)?))
    }

    /// `DuosView`: the only window the client has onto a Duos game (B23).
    pub fn duos_view_for(state_json: &str, seat: &str) -> Result<String, String> {
        let state = duos_state_of("duosViewFor", state_json)?;
        to_json(&duos_engine::duos_view_for(&state, duos_seat(seat)?))
    }
}

#[cfg(not(feature = "duos"))]
mod duos_bodies {
    /// What every Duos body answers in a module built without them.
    fn absent(what: &str) -> String {
        format!(
            "{what}: this module was built without the `duos` feature, which only development builds turn on (/dev/duos)"
        )
    }

    pub fn duos_create(_options_json: &str) -> Result<String, String> {
        Err(absent("duosCreate"))
    }

    pub fn duos_begin(_state_json: &str) -> Result<String, String> {
        Err(absent("duosBegin"))
    }

    pub fn duos_reduce(_state_json: &str, _action_json: &str) -> Result<String, String> {
        Err(absent("duosReduce"))
    }

    pub fn duos_legal_actions(_state_json: &str, _seat: &str) -> Result<String, String> {
        Err(absent("duosLegalActions"))
    }

    pub fn duos_view_for(_state_json: &str, _seat: &str) -> Result<String, String> {
        Err(absent("duosViewFor"))
    }
}

pub use duos_bodies::{duos_begin, duos_create, duos_legal_actions, duos_reduce, duos_view_for};
