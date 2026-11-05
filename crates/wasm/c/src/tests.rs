//! Native tests: each export, called with JSON the way a host calls it (bytes in, a status out, the text in
//! the result buffer), answers what `jackioh-wasm-core`'s body answers for the same JSON. The bodies are
//! the logic; these hold the wrapper's argument names, its status, its result buffer and its memory
//! interface to them. `crates/wasm/tests/export_parity.rs` holds the export list to the browser module's.

use serde_json::{Value, json};

use super::*;

/// What a binding said: its status and the text it left in the result buffer.
#[derive(Debug, PartialEq)]
struct Answer {
    status: i32,
    text: String,
}

/// What a body's answer should read as through an export.
fn expect(result: Result<String, String>) -> Answer {
    match result {
        Ok(text) => Answer {
            status: STATUS_OK,
            text,
        },
        Err(text) => Answer {
            status: STATUS_FAILED,
            text,
        },
    }
}

/// The last binding's answer, read as a host reads it: through the result accessors.
fn answered(status: i32) -> Answer {
    let (ptr, len) = (jackioh_result_ptr(), jackioh_result_len());
    let bytes = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
    };
    Answer {
        status,
        text: String::from_utf8(bytes).expect("the result is UTF-8"),
    }
}

/// An export called with text arguments (each as its pointer and length), then number arguments.
macro_rules! call {
    ($export:ident; $($text:expr),* ; $($number:expr),*) => {{
        answered($export($($text.as_ptr(), $text.len(),)* $($number),*))
    }};
}

/// The export said what the body says, and it is the status the case is about.
#[track_caller]
fn agrees(export: Answer, body: Result<String, String>, status: i32) {
    let said: String = export.text.chars().take(300).collect();
    assert_eq!(export.status, status, "the export said: {said}");
    assert_eq!(export, expect(body));
}

fn value(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("{error}: {text}"))
}

/// A game begun, and what a host would ask of it.
struct Game {
    /// `createGame`'s JSON, `fold`'s without `log`.
    args: String,
    /// `GameState` before `begin_game`.
    created: String,
    /// `GameState` after it.
    state: String,
    /// The seat that owes an action.
    seat: String,
    /// A legal action of that seat, `ActionBody` plus `playerId` and `nonce`.
    action: String,
}

fn game() -> Game {
    init();
    let deck = |seed: &str| {
        let options = json!({ "rngSeed": seed, "size": jackioh_engine::config::DECK_SIZE }).to_string();
        value(&wasm_core::build_ai_deck(&options).expect("a deck"))["deck"].clone()
    };
    let args = json!({ "seed": "c-test", "decks": [deck("c-test-a"), deck("c-test-b")] }).to_string();
    let created = wasm_core::create_game(&args, check_setup).expect("a game");
    let begun = value(&wasm_core::begin_game(&created).expect("a begun game"));
    let state = begun["state"].to_string();
    let seat = wasm_core::seat_to_act(&state).expect("a seat");
    let bodies = value(&wasm_core::legal_actions(&state, &seat).expect("legal actions"));
    let mut action = bodies[0].clone();
    action["playerId"] = json!(seat);
    action["nonce"] = json!("n-1");
    Game {
        args,
        created,
        state,
        seat,
        action: action.to_string(),
    }
}

// One test per area: every export against the body it wraps

#[test]
fn engine_exports_answer_what_the_core_bodies_answer() {
    let g = game();
    agrees(
        call!(create_game; g.args;),
        wasm_core::create_game(&g.args, check_setup),
        STATUS_OK,
    );
    agrees(
        call!(begin_game; g.created;),
        wasm_core::begin_game(&g.created),
        STATUS_OK,
    );
    agrees(
        call!(reduce; g.state, g.action;),
        wasm_core::reduce(&g.state, &g.action),
        STATUS_OK,
    );
    agrees(
        call!(legal_actions; g.state, g.seat;),
        wasm_core::legal_actions(&g.state, &g.seat),
        STATUS_OK,
    );
    agrees(
        call!(view_for; g.state, g.seat;),
        wasm_core::view_for(&g.state, &g.seat),
        STATUS_OK,
    );
    agrees(
        call!(seat_to_act; g.state;),
        wasm_core::seat_to_act(&g.state),
        STATUS_OK,
    );
    agrees(
        call!(hash_state; g.state;),
        wasm_core::hash_state(&g.state),
        STATUS_OK,
    );
    let mut folded = value(&g.args);
    folded["log"] = json!([]);
    let folded = folded.to_string();
    agrees(
        call!(fold; folded;),
        wasm_core::fold(&folded, check_setup),
        STATUS_OK,
    );
    agrees(
        call!(last_board_for; g.state, g.seat;),
        wasm_core::last_board_for(&g.state, &g.seat),
        STATUS_OK,
    );
    agrees(
        call!(seat_played_by; g.state, g.seat;),
        wasm_core::seat_played_by(&g.state, &g.seat),
        STATUS_OK,
    );
    agrees(
        call!(find_instance; g.state, "no-such-card";),
        wasm_core::find_instance(&g.state, "no-such-card"),
        STATUS_OK,
    );
    agrees(
        call!(deal_emote_hand; "c-test", g.seat;),
        wasm_core::deal_emote_hand("c-test", &g.seat),
        STATUS_OK,
    );
    // A record that is not one is refused, not folded.
    agrees(
        call!(replay_open; "{}", "{}";),
        wasm_core::replay_open("{}", "{}", check_setup),
        STATUS_FAILED,
    );
    agrees(
        call!(replay_page; "{}", g.seat; 0, 8),
        wasm_core::replay_page("{}", &g.seat, 0, 8),
        STATUS_FAILED,
    );
}

#[test]
fn a_game_with_custom_cards_is_refused_at_setup() {
    let mut args = value(&game().args);
    let frame = json!({
        "name": "X", "type": "Unit", "cost": 1, "rarity": "Common", "attack": 1, "health": 1, "text": ""
    });
    args["customCards"] = json!([{ "id": "custom-x", "version": 1, "frame": frame, "source": "" }]);
    let args = args.to_string();
    let answer = call!(create_game; args;);
    assert_eq!(answer.status, STATUS_FAILED);
    assert!(answer.text.contains("R915009"), "{}", answer.text);
}

#[test]
fn ai_exports_answer_what_the_core_bodies_answer() {
    let g = game();
    let to_act = call!(ai_to_act; g.state, g.seat;);
    assert!(
        matches!(to_act.text.as_str(), "true" | "false"),
        "{}",
        to_act.text
    );
    agrees(
        to_act,
        wasm_core::ai_to_act(&g.state, &g.seat).map(|owes| owes.to_string()),
        STATUS_OK,
    );

    // The deadline is ignored: 5000 ms and none answer alike, from the same stream.
    let options = json!({ "rngSeed": "s", "rngCursor": 0 }).to_string();
    let decided = call!(ai_decide; g.state, g.seat, options; 5000.0);
    assert!(value(&decided.text)["decision"].is_object(), "{}", decided.text);
    agrees(
        decided,
        wasm_core::ai_decide(&g.state, &g.seat, &options, 0.0, || 0.0),
        STATUS_OK,
    );

    let hint = call!(lethal_hint; g.state, g.seat; 5000.0);
    assert!(matches!(hint.text.as_str(), "true" | "false"), "{}", hint.text);
    agrees(
        hint,
        wasm_core::lethal_hint(&g.state, &g.seat, 0.0, || 0.0).map(|sure| sure.to_string()),
        STATUS_OK,
    );

    let deck = json!({ "rngSeed": "c-deck", "size": jackioh_engine::config::DECK_SIZE }).to_string();
    agrees(
        call!(build_ai_deck; deck;),
        wasm_core::build_ai_deck(&deck),
        STATUS_OK,
    );
    agrees(
        call!(choose_action; g.state, g.seat, "c-policy"; 0.0),
        wasm_core::choose_action(&g.state, &g.seat, "c-policy", 0.0),
        STATUS_OK,
    );
    // A cursor that is not a whole number is the body's refusal.
    agrees(
        call!(choose_action; g.state, g.seat, "c-policy"; 0.5),
        wasm_core::choose_action(&g.state, &g.seat, "c-policy", 0.5),
        STATUS_FAILED,
    );
}

#[test]
fn static_exports_answer_what_the_core_bodies_answer() {
    init();
    agrees(call!(catalog;;), wasm_core::catalog(), STATUS_OK);
    agrees(
        call!(catalog_version;;),
        Ok(wasm_core::catalog_version()),
        STATUS_OK,
    );
    agrees(
        call!(carried_versions;;),
        wasm_core::carried_versions(),
        STATUS_OK,
    );
    agrees(call!(constants;;), wasm_core::constants(), STATUS_OK);
    agrees(call!(engine_tables;;), wasm_core::engine_tables(), STATUS_OK);
    agrees(
        call!(validator; "normalizeName", "\"ab\"";),
        wasm_core::validator("normalizeName", "\"ab\""),
        STATUS_OK,
    );
    agrees(
        call!(validator; "noSuchCall", "{}";),
        wasm_core::validator("noSuchCall", "{}"),
        STATUS_FAILED,
    );
    // A recipe that is not one, and a fixture that is not one, are refused.
    let craft = craft_preview("{}".as_ptr(), 2, 3.0, "{}".as_ptr(), 2);
    agrees(
        answered(craft),
        wasm_core::craft_preview("{}", 3.0, "{}"),
        STATUS_FAILED,
    );
    agrees(
        call!(learned_parity; "{}";),
        wasm_core::learned_parity("{}"),
        STATUS_FAILED,
    );
    // Previewing no set is allowed in every build; the answer is empty.
    let none = call!(preview_sets; "[]";);
    assert_eq!(
        none,
        Answer {
            status: STATUS_OK,
            text: String::new()
        }
    );
}

#[test]
fn constants_carry_the_numbers_a_host_reads() {
    init();
    let all = value(&call!(constants;;).text);
    assert_eq!(all["DECK_SIZE"], json!(jackioh_engine::config::DECK_SIZE));
    assert!(all["AI_BUDGET"]["nodes"].is_number());
    assert!(all["RUN"]["starterSize"].is_number());
}

#[test]
fn run_exports_answer_what_the_core_bodies_answer() {
    init();
    agrees(call!(run_content; "[]";), wasm_core::run_content("[]"), STATUS_OK);
    agrees(call!(run_constants;;), wasm_core::run_constants(), STATUS_OK);

    let constants = value(&wasm_core::run_constants().expect("run constants"));
    let content = value(&wasm_core::run_content("[]").expect("run content"));
    let seed = value(&wasm_core::run_daily_seed("\"2026-11-01\"", "1").expect("a seed"));
    let starter_size = constants["starterSize"].as_u64().expect("a starter size") as usize;
    let setup = json!({
        "seed": seed,
        "starter": content["starterAccess"].as_array().expect("starter access")[..starter_size],
        "artifacts": [],
        "curses": [],
    })
    .to_string();
    let unlocks = json!({
        "level": constants["slotLadder"].as_array().expect("a slot ladder").len(),
        "cards": [],
        "artifacts": [],
    })
    .to_string();
    let content = content.to_string();

    agrees(
        call!(run_check_setup; setup, unlocks, content;),
        wasm_core::run_check_setup(&setup, &unlocks, &content),
        STATUS_OK,
    );
    let created = call!(run_create; setup, unlocks, content;);
    let state = value(&created.text)["state"].to_string();
    assert_ne!(state, "null", "{}", created.text);
    agrees(
        created,
        wasm_core::run_create(&setup, &unlocks, &content),
        STATUS_OK,
    );

    let go = json!({ "type": "go", "col": 0 }).to_string();
    let went = call!(run_reduce; state, content, go;);
    let after = value(&went.text)["state"].to_string();
    assert_ne!(after, "null", "{}", went.text);
    agrees(went, wasm_core::run_reduce(&state, &content, &go), STATUS_OK);
    // An action the run refuses is a returned `{ error }`, not a failed call.
    let heal = json!({ "type": "heal" }).to_string();
    let refused = call!(run_reduce; state, content, heal;);
    assert!(value(&refused.text)["error"].is_string(), "{}", refused.text);
    agrees(refused, wasm_core::run_reduce(&state, &content, &heal), STATUS_OK);

    let file = json!({ "setup": value(&setup), "unlocks": value(&unlocks), "log": [value(&go)] }).to_string();
    agrees(
        call!(run_fold; file, content;),
        wasm_core::run_fold(&file, &content),
        STATUS_OK,
    );
    agrees(
        call!(run_fight_setup; after, content;),
        wasm_core::run_fight_setup(&after, &content),
        STATUS_OK,
    );
    let fight = value(&wasm_core::run_fight_setup(&after, &content).expect("a fight"));
    let game = json!({
        "seed": fight["seed"],
        "decks": [fight["humanDeck"], fight["enemyDeck"]],
        "handicaps": { "p1": fight["humanHandicap"], "p2": fight["enemyHandicap"] },
        "deckMarks": { "p1": fight["humanMarks"] },
        "log": [],
    })
    .to_string();
    agrees(
        call!(run_fight_report; game, "\"p1\"";),
        wasm_core::run_fight_report(&game, "\"p1\""),
        STATUS_OK,
    );
    agrees(
        call!(run_score; state, content;),
        wasm_core::run_score(&state, &content),
        STATUS_OK,
    );
    agrees(call!(run_embers; "250";), wasm_core::run_embers("250"), STATUS_OK);
    agrees(
        call!(run_max_hp; state, content;),
        wasm_core::run_max_hp(&state, &content),
        STATUS_OK,
    );
    agrees(
        call!(run_level_cost; "1";),
        wasm_core::run_level_cost("1"),
        STATUS_OK,
    );
    agrees(
        call!(run_seed; "\"anything\"";),
        wasm_core::run_seed("\"anything\""),
        STATUS_OK,
    );
    agrees(
        call!(run_daily_seed; "\"2026-11-01\"", "2";),
        wasm_core::run_daily_seed("\"2026-11-01\"", "2"),
        STATUS_OK,
    );
}

#[test]
fn duos_exports_answer_what_the_core_bodies_answer() {
    // Without the `duos` feature each answers the reason it is absent; with it, the input is refused.
    init();
    agrees(
        call!(duos_create; "{}";),
        wasm_core::duos_create("{}"),
        STATUS_FAILED,
    );
    agrees(
        call!(duos_begin; "{}";),
        wasm_core::duos_begin("{}"),
        STATUS_FAILED,
    );
    agrees(
        call!(duos_reduce; "{}", "{}";),
        wasm_core::duos_reduce("{}", "{}"),
        STATUS_FAILED,
    );
    agrees(
        call!(duos_legal_actions; "{}", "a1";),
        wasm_core::duos_legal_actions("{}", "a1"),
        STATUS_FAILED,
    );
    agrees(
        call!(duos_view_for; "{}", "a1";),
        wasm_core::duos_view_for("{}", "a1"),
        STATUS_FAILED,
    );
}

// The memory interface

#[test]
fn memory_interface_round_trips() {
    let g = game();
    // The host writes the argument into a buffer the module handed out, calls, reads the result, frees.
    let len = g.state.len();
    let ptr = jackioh_alloc(len);
    assert!(!ptr.is_null());
    unsafe { std::ptr::copy_nonoverlapping(g.state.as_ptr(), ptr, len) };
    let said = answered(seat_to_act(ptr, len));
    unsafe { jackioh_free(ptr, len) };
    assert_eq!(said, expect(wasm_core::seat_to_act(&g.state)));
    assert_eq!(said.text, g.seat);

    // A buffer of no bytes is still a buffer, and never at address 0; NULL frees as nothing.
    let empty = jackioh_alloc(0);
    assert!(!empty.is_null());
    unsafe {
        jackioh_free(empty, 0);
        jackioh_free(std::ptr::null_mut(), 0);
    }
}

#[test]
fn a_missing_or_non_utf8_argument_is_refused() {
    init();
    let missing = answered(seat_to_act(std::ptr::null(), 5));
    assert_eq!(
        missing,
        Answer {
            status: STATUS_FAILED,
            text: "seatToAct: the argument is missing".to_owned()
        }
    );

    let bytes = [0x7b_u8, 0xff, 0xfe, 0x7d];
    let bad = answered(seat_to_act(bytes.as_ptr(), bytes.len()));
    assert_eq!(bad.status, STATUS_FAILED);
    assert!(
        bad.text.starts_with("seatToAct: the argument is not UTF-8: "),
        "{}",
        bad.text
    );

    // An empty argument is the empty text wherever its pointer points: the body's own refusal, not "missing".
    let empty = answered(seat_to_act(std::ptr::null(), 0));
    assert_eq!(empty, expect(wasm_core::seat_to_act("")));
    assert!(!empty.text.contains("missing"), "{}", empty.text);
}

#[test]
fn the_next_call_replaces_the_result() {
    init();
    let long = call!(run_constants;;);
    assert_eq!(long.status, STATUS_OK);
    assert!(long.text.len() > "null".len());
    let short = call!(run_level_cost; "999";);
    assert_eq!(
        short,
        Answer {
            status: STATUS_OK,
            text: "null".to_owned()
        }
    );
    assert_eq!(jackioh_result_len(), "null".len());
    // A success with nothing to say leaves nothing behind it, and an allocation disturbs nothing.
    let none = call!(preview_sets; "[]";);
    let ptr = jackioh_alloc(64);
    assert_eq!(answered(none.status).text, "");
    unsafe { jackioh_free(ptr, 64) };
    // A failure's message is the result too.
    let failed = call!(run_level_cost; "x";);
    assert_eq!(failed.status, STATUS_FAILED);
    assert_eq!(answered(failed.status), failed);
}

// Every export is tested

/// The names a source exports: for each line that trims to `#[unsafe(no_mangle)]`, the `fn` after it.
fn exported_names(source: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    lines
        .windows(2)
        .filter(|pair| pair[0].trim() == "#[unsafe(no_mangle)]")
        .map(|pair| {
            let rest = pair[1].split("fn ").nth(1).expect("a fn after the attribute");
            rest[..rest.find('(').expect("a `(` after the name")]
                .trim()
                .to_owned()
        })
        .collect()
}

/// Whether `source` names `name` as a call, `name(` or the macro's `name;`, not as the end of a longer
/// name (`run_reduce(` is not a call of `reduce`).
fn names_call(source: &str, name: &str) -> bool {
    ["(", ";"].iter().any(|after| {
        let needle = format!("{name}{after}");
        source.match_indices(&needle).any(|(at, _)| {
            source[..at]
                .chars()
                .next_back()
                .is_none_or(|before| !(before.is_alphanumeric() || before == '_'))
        })
    })
}

#[test]
fn every_export_has_a_test() {
    let names = exported_names(include_str!("lib.rs"));
    assert!(
        names.len() >= 50,
        "the scan of lib.rs found {} exports",
        names.len()
    );
    let tests = include_str!("tests.rs");
    let untested: Vec<&String> = names.iter().filter(|name| !names_call(tests, name)).collect();
    assert!(untested.is_empty(), "exports no test calls: {untested:?}");
}

#[test]
fn the_scan_finds_a_call_and_not_a_longer_name() {
    assert!(names_call("let a = reduce(x);", "reduce"));
    assert!(names_call("call!(reduce; a;)", "reduce"));
    assert!(!names_call("run_reduce(x); run_reduce;", "reduce"));
}
