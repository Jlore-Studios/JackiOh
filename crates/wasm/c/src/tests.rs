//! The C boundary's native tests (#3093): each export, called through the memory interface with JSON
//! text, answers what `jackioh-wasm-core`'s body answers for the same JSON, with this module's own setup
//! check where the body takes one. The browser module (wasm-bindgen) and the Luau module wrap the same
//! bodies, so this holds the third wrapper to them, and `crates/wasm/tests/export_parity.rs` holds its
//! list of exports to the browser module's.

use serde_json::{Value, json};

use super::*;

/// What a binding returned: its status and the text it left in the result buffer.
#[derive(Debug, PartialEq)]
struct Answer {
    status: i32,
    text: String,
}

/// What a body's answer should read as: `STATUS_OK` and its text, or `STATUS_FAILED` and its message.
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

/// The result buffer, read as a host reads it: `jackioh_result_len()` bytes at `jackioh_result_ptr()`.
fn answered(status: i32) -> Answer {
    let len = jackioh_result_len();
    let bytes = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(jackioh_result_ptr(), len) }.to_vec()
    };
    Answer {
        status,
        text: String::from_utf8(bytes).expect("the result is UTF-8"),
    }
}

/// Calls an export with each text as its `(ptr, len)` pair and then each number, and reads its answer.
/// `call!(export; text, text; number, number)`.
macro_rules! call {
    ($export:ident; $($text:expr),* ; $($number:expr),*) => {{
        answered($export($($text.as_ptr(), $text.len(),)* $($number),*))
    }};
}

/// A game as the AI's decks deal it, begun, with the seat that owes an action and a legal action of it.
struct Game {
    /// `create_game`'s arguments.
    args: String,
    /// The state `begin_game` returned.
    state: String,
    seat: String,
    /// A legal action of `seat`, as `reduce` takes it.
    action: String,
}

/// The `Answer`'s text, which must have answered.
fn ok(answer: Answer) -> String {
    assert_eq!(answer.status, STATUS_OK, "{}", answer.text);
    answer.text
}

fn game() -> Game {
    init();
    let deck = |seed: &str| -> Value {
        let options = json!({ "rngSeed": seed, "size": jackioh_engine::config::DECK_SIZE }).to_string();
        let built: Value = serde_json::from_str(&ok(call!(build_ai_deck; options;))).expect("JSON");
        built["deck"].clone()
    };
    let args = json!({ "seed": "c-test", "decks": [deck("c-deck-a"), deck("c-deck-b")] }).to_string();
    let created = ok(call!(create_game; args;));
    let begun: Value = serde_json::from_str(&ok(call!(begin_game; created;))).expect("JSON");
    let state = begun["state"].to_string();
    let seat = ok(call!(seat_to_act; state;));
    assert!(
        seat == "p1" || seat == "p2",
        "a begun game owes a seat an action: {seat:?}"
    );
    let legal: Value = serde_json::from_str(&ok(call!(legal_actions; state, seat;))).expect("JSON");
    let mut action = legal[0].clone();
    assert!(
        action.is_object(),
        "a begun game offers its seat an action: {legal}"
    );
    action["playerId"] = json!(seat);
    action["nonce"] = json!("c-1");
    Game {
        args,
        state,
        seat,
        action: action.to_string(),
    }
}

#[test]
fn engine_exports_answer_what_the_core_bodies_answer() {
    let game = game();
    let Game {
        args,
        state,
        seat,
        action,
    } = &game;
    assert_eq!(
        call!(create_game; args;),
        expect(wasm_core::create_game(args, check_setup))
    );
    assert_eq!(call!(begin_game; state;), expect(wasm_core::begin_game(state)));
    let reduced = call!(reduce; state, action;);
    assert_eq!(reduced, expect(wasm_core::reduce(state, action)));
    assert_eq!(reduced.status, STATUS_OK);
    assert!(
        !reduced.text.contains("\"error\""),
        "the legal action is taken: {}",
        reduced.text
    );
    assert_eq!(
        call!(legal_actions; state, seat;),
        expect(wasm_core::legal_actions(state, seat))
    );
    assert_eq!(
        call!(view_for; state, seat;),
        expect(wasm_core::view_for(state, seat))
    );
    assert_eq!(call!(seat_to_act; state;), expect(wasm_core::seat_to_act(state)));
    assert_eq!(call!(hash_state; state;), expect(wasm_core::hash_state(state)));
    let folded: Value = serde_json::from_str(args).expect("JSON");
    let folded = json!({ "seed": folded["seed"], "decks": folded["decks"], "log": [] }).to_string();
    assert_eq!(
        call!(fold; folded;),
        expect(wasm_core::fold(&folded, check_setup))
    );
    assert_eq!(
        call!(last_board_for; state, seat;),
        expect(wasm_core::last_board_for(state, seat))
    );
    assert_eq!(
        call!(seat_played_by; state, "p1";),
        expect(wasm_core::seat_played_by(state, "p1"))
    );
    assert_eq!(
        call!(find_instance; state, "no-such-card";),
        expect(wasm_core::find_instance(state, "no-such-card"))
    );
    assert_eq!(
        call!(deal_emote_hand; "c-seed", "p2";),
        expect(wasm_core::deal_emote_hand("c-seed", "p2"))
    );
    // A replay is opened from a record and its pages read from checkpoints, which `{}` is neither.
    let opened = call!(replay_open; "{}", "{}";);
    assert_eq!(opened, expect(wasm_core::replay_open("{}", "{}", check_setup)));
    assert_eq!(opened.status, STATUS_FAILED);
    let paged = call!(replay_page; "{}", "p1"; 0, 10);
    assert_eq!(paged, expect(wasm_core::replay_page("{}", "p1", 0, 10)));
    assert_eq!(paged.status, STATUS_FAILED);
}

#[test]
fn a_game_with_custom_cards_is_refused_before_the_engine_is_called() {
    let game = game();
    let mut args: Value = serde_json::from_str(&game.args).expect("JSON");
    args["customCards"] = json!([{
        "id": "custom-c",
        "version": 1,
        "frame": { "name": "C", "type": "Unit", "cost": 1, "rarity": "Common", "attack": 1, "health": 1 },
    }]);
    let args = args.to_string();
    for answer in [call!(create_game; args;), call!(fold; args;)] {
        assert_eq!(answer.status, STATUS_FAILED);
        assert!(answer.text.contains("R915009"), "{}", answer.text);
    }
}

#[test]
fn ai_exports_answer_what_the_core_bodies_answer() {
    let game = game();
    let Game { state, seat, .. } = &game;
    let owes = call!(ai_to_act; state, seat;);
    assert_eq!(
        owes,
        expect(wasm_core::ai_to_act(state, seat).map(|owes| owes.to_string()))
    );
    assert!(owes.text == "true" || owes.text == "false", "{}", owes.text);
    // A small budget, so the debug build's search is quick; the options carry no clock.
    let options = json!({
        "rngSeed": "c-ai",
        "rngCursor": 0,
        "budget": {
            "nodes": 40, "lethalNodes": 10, "determinizations": 1, "beamWidth": 2,
            "rootBranching": 3, "branching": 2, "maxDepth": 2, "finalists": 1,
        },
    })
    .to_string();
    // This module has no clock: `deadline_ms` changes nothing.
    let decided = call!(ai_decide; state, seat, options; 5000.0);
    assert_eq!(
        decided,
        expect(wasm_core::ai_decide(state, seat, &options, 0.0, || 0.0))
    );
    assert_eq!(decided, call!(ai_decide; state, seat, options; 0.0));
    assert_eq!(decided.status, STATUS_OK, "{}", decided.text);
    let sure = call!(lethal_hint; state, seat; 5000.0);
    assert_eq!(
        sure,
        expect(wasm_core::lethal_hint(state, seat, 0.0, || 0.0).map(|sure| sure.to_string()))
    );
    assert!(sure.text == "true" || sure.text == "false", "{}", sure.text);
    let deck_options = json!({ "rngSeed": "c-deck", "size": jackioh_engine::config::DECK_SIZE }).to_string();
    let deck = call!(build_ai_deck; deck_options;);
    assert_eq!(deck, expect(wasm_core::build_ai_deck(&deck_options)));
    assert_eq!(deck.status, STATUS_OK, "{}", deck.text);
    assert_eq!(
        call!(choose_action; state, seat, "c-choose"; 0.0),
        expect(wasm_core::choose_action(state, seat, "c-choose", 0.0))
    );
    // A cursor that is not a whole number is refused by the body, as the status here.
    let refused = call!(choose_action; state, seat, "c-choose"; 0.5);
    assert_eq!(
        refused,
        expect(wasm_core::choose_action(state, seat, "c-choose", 0.5))
    );
    assert_eq!(refused.status, STATUS_FAILED);
}

#[test]
fn static_exports_answer_what_the_core_bodies_answer() {
    init();
    assert_eq!(call!(catalog;;), expect(wasm_core::catalog()));
    assert_eq!(call!(catalog;;).status, STATUS_OK);
    assert_eq!(call!(catalog_version;;), expect(Ok(wasm_core::catalog_version())));
    assert_eq!(call!(carried_versions;;), expect(wasm_core::carried_versions()));
    let constants = call!(constants;;);
    assert_eq!(constants, expect(wasm_core::constants()));
    // A host reads a number by the name the web reads it by, and the AI's and the run's beside them.
    let constants: Value = serde_json::from_str(&ok(constants)).expect("JSON");
    assert_eq!(constants["DECK_SIZE"], json!(jackioh_engine::config::DECK_SIZE));
    assert!(constants["AI_BUDGET"].is_object() && constants["RUN"].is_object());
    assert_eq!(call!(engine_tables;;), expect(wasm_core::engine_tables()));
    let validated = call!(validator; "normalizeName", "\"ab\"";);
    assert_eq!(validated, expect(wasm_core::validator("normalizeName", "\"ab\"")));
    assert_eq!(validated.status, STATUS_OK, "{}", validated.text);
    // A recipe and a fixture that are not ones are refused by the bodies, as the status here.
    let crafted = answered(craft_preview("{}".as_ptr(), 2, 1.0, "{}".as_ptr(), 2));
    assert_eq!(crafted, expect(wasm_core::craft_preview("{}", 1.0, "{}")));
    assert_eq!(crafted.status, STATUS_FAILED);
    let parity = call!(learned_parity; "{}";);
    assert_eq!(parity, expect(wasm_core::learned_parity("{}")));
    assert_eq!(parity.status, STATUS_FAILED);
    // No set to preview is no preview, which every module takes: the result is empty.
    assert_eq!(
        call!(preview_sets; "[]";),
        Answer {
            status: STATUS_OK,
            text: String::new()
        }
    );
    assert_eq!(jackioh_result_len(), 0);
}

#[test]
fn run_exports_answer_what_the_core_bodies_answer() {
    init();
    let content = call!(run_content; "[]";);
    assert_eq!(content, expect(wasm_core::run_content("[]")));
    let content = ok(content);
    let numbers = call!(run_constants;;);
    assert_eq!(numbers, expect(wasm_core::run_constants()));
    let numbers: Value = serde_json::from_str(&ok(numbers)).expect("JSON");
    let seed = call!(run_daily_seed; "\"2026-11-01\"", "1";);
    assert_eq!(seed, expect(wasm_core::run_daily_seed("\"2026-11-01\"", "1")));
    let seed: Value = serde_json::from_str(&ok(seed)).expect("JSON");
    assert_eq!(
        call!(run_seed; "\"c-source\"";),
        expect(wasm_core::run_seed("\"c-source\""))
    );

    // The setup the run's own tests use: the first cards the starter may hold, at the ladder's top.
    let access: Value = serde_json::from_str(&content).expect("JSON");
    let starter = numbers["starterSize"].as_u64().expect("starterSize") as usize;
    let setup = json!({
        "seed": seed,
        "starter": access["starterAccess"].as_array().expect("starterAccess")[..starter],
    })
    .to_string();
    let level = numbers["slotLadder"].as_array().expect("slotLadder").len();
    let unlocks = json!({ "level": level, "cards": [], "artifacts": [] }).to_string();

    let checked = call!(run_check_setup; setup, unlocks, content;);
    assert_eq!(
        checked,
        expect(wasm_core::run_check_setup(&setup, &unlocks, &content))
    );
    assert_eq!(checked.text, "null", "the setup may start a run");
    let created = call!(run_create; setup, unlocks, content;);
    assert_eq!(created, expect(wasm_core::run_create(&setup, &unlocks, &content)));
    let created: Value = serde_json::from_str(&ok(created)).expect("JSON");
    let state = created["state"].to_string();
    assert_ne!(state, "null", "{created}");

    let action = json!({ "type": "go", "col": 0 }).to_string();
    let reduced = call!(run_reduce; state, content, action;);
    assert_eq!(reduced, expect(wasm_core::run_reduce(&state, &content, &action)));
    let reduced: Value = serde_json::from_str(&ok(reduced)).expect("JSON");
    let after = reduced["state"].to_string();
    assert_ne!(after, "null", "{reduced}");

    let file = json!({
        "setup": serde_json::from_str::<Value>(&setup).expect("JSON"),
        "unlocks": serde_json::from_str::<Value>(&unlocks).expect("JSON"),
        "log": [serde_json::from_str::<Value>(&action).expect("JSON")],
    })
    .to_string();
    assert_eq!(
        call!(run_fold; file, content;),
        expect(wasm_core::run_fold(&file, &content))
    );
    let fight = call!(run_fight_setup; after, content;);
    assert_eq!(fight, expect(wasm_core::run_fight_setup(&after, &content)));
    assert_ne!(ok(fight), "null", "the first move meets a fight");
    // A game with no result to report is refused in the returned value, not as the status.
    let human = "\"p1\"";
    assert_eq!(
        call!(run_fight_report; "{}", human;),
        expect(wasm_core::run_fight_report("{}", human))
    );
    assert_eq!(
        call!(run_score; state, content;),
        expect(wasm_core::run_score(&state, &content))
    );
    assert_eq!(call!(run_embers; "250";), expect(wasm_core::run_embers("250")));
    assert_eq!(
        call!(run_max_hp; state, content;),
        expect(wasm_core::run_max_hp(&state, &content))
    );
    assert_eq!(
        call!(run_level_cost; "1";),
        expect(wasm_core::run_level_cost("1"))
    );
}

#[test]
fn duos_exports_answer_what_the_core_bodies_answer() {
    init();
    // Built without the `duos` feature each body answers the reason, which the status carries.
    assert_eq!(call!(duos_create; "{}";), expect(wasm_core::duos_create("{}")));
    assert_eq!(call!(duos_begin; "{}";), expect(wasm_core::duos_begin("{}")));
    assert_eq!(
        call!(duos_reduce; "{}", "{}";),
        expect(wasm_core::duos_reduce("{}", "{}"))
    );
    assert_eq!(
        call!(duos_legal_actions; "{}", "a1";),
        expect(wasm_core::duos_legal_actions("{}", "a1"))
    );
    assert_eq!(
        call!(duos_view_for; "{}", "a1";),
        expect(wasm_core::duos_view_for("{}", "a1"))
    );
    assert_eq!(call!(duos_create; "{}";).status, STATUS_FAILED);
}

#[test]
fn memory_interface_round_trips() {
    let game = game();
    let bytes = game.state.as_bytes();
    let ptr = jackioh_alloc(bytes.len());
    assert!(!ptr.is_null());
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
    let seat = answered(seat_to_act(ptr, bytes.len()));
    unsafe { jackioh_free(ptr, bytes.len()) };
    assert_eq!(
        seat,
        Answer {
            status: STATUS_OK,
            text: game.seat
        }
    );
    // A buffer for no bytes still has an address, and a null address frees nothing.
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
    let missing = answered(seat_to_act(std::ptr::null(), 3));
    assert_eq!(
        missing,
        Answer {
            status: STATUS_FAILED,
            text: "seatToAct: the argument is missing".to_owned()
        }
    );
    let bad = [0x7b_u8, 0xff, 0xfe];
    let refused = answered(seat_to_act(bad.as_ptr(), bad.len()));
    assert_eq!(refused.status, STATUS_FAILED);
    assert!(
        refused.text.starts_with("seatToAct: the argument is not UTF-8"),
        "{}",
        refused.text
    );
    // No length is the empty text, whatever the address: the body then refuses it as JSON that is not.
    let empty = answered(seat_to_act(std::ptr::null(), 0));
    assert_eq!(empty, expect(wasm_core::seat_to_act("")));
    assert_eq!(empty.status, STATUS_FAILED);
}

#[test]
fn the_next_call_replaces_the_result() {
    init();
    let version = call!(catalog_version;;);
    assert_eq!(version, expect(Ok(wasm_core::catalog_version())));
    assert_eq!(jackioh_result_len(), version.text.len());
    let refused = call!(seat_to_act; "";);
    assert_eq!(refused.status, STATUS_FAILED);
    assert_ne!(refused.text, version.text);
    assert_eq!(jackioh_result_len(), refused.text.len());
    // A long answer does not leave its tail behind a short one.
    assert!(call!(constants;;).text.len() > version.text.len());
    assert_eq!(call!(catalog_version;;), version);
}

/// Whether `text` calls or runs `name` as a whole word, as `call!(name; …)` or `name(…)` does.
fn mentions(text: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        before.is_none_or(|c| !word(c)) && matches!(after, Some(';' | '('))
    })
}

#[test]
fn every_export_has_a_test() {
    let lib = include_str!("lib.rs");
    let tests = include_str!("tests.rs");
    let lines: Vec<&str> = lib.lines().collect();
    let mut names = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == "#[unsafe(no_mangle)]" {
            let next = lines[index + 1];
            let rest = &next[next.find("fn ").expect("a fn") + "fn ".len()..];
            names.push(rest[..rest.find('(').expect("a (")].trim());
        }
    }
    assert!(names.len() >= 52, "{} exports scanned", names.len());
    let untested: Vec<&&str> = names.iter().filter(|name| !mentions(tests, name)).collect();
    assert!(untested.is_empty(), "exports with no test: {untested:?}");
    assert!(mentions("call!(reduce; a;)", "reduce") && !mentions("call!(run_reduce; a;)", "reduce"));
}
