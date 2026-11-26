//! `redact` of two worlds that differ only in a card the seat cannot see (R185, R4804000). Each test
//! builds both worlds through `reduce`, checks that the seat's view of them is one view, and lists
//! every JSON path at which their redactions differ: README, "`decide` reads only `redact`", has "two
//! states that differ only in hidden cards give the same decision under the same rng".
//!
//! The play counts (R451) are written as a card is played, a face-down Trap's by the card's tags, its
//! single-player set and the type it was played as, while the other seat saw only a Trap being set
//! (R33, R448). An interrupted trap firing (R113) parks its event on `state.work` under
//! `data.firing.event`, a hidden card's id and definition among what it names.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act_with_nonce, js};

const SHEEPISH: &str = "core-041";
const CROP_DUSTING: &str = "classic-063";
const BREAD_AND_BUTTER: &str = "core-018";
const OVERFLOW_ERROR: &str = "infiniteassault-014";
const PESTILENT_SLIME: &str = "classic-027";
const VANILLA: &str = "core-008";
const STOCKPILE: &str = "core-005";

/// Every path at which two JSON values differ.
fn diff_paths(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                match (x.get(key), y.get(key)) {
                    (Some(p), Some(q)) => diff_paths(p, q, &format!("{path}.{key}"), out),
                    _ => out.push(format!("{path}.{key} (one side only)")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}[] (len {} vs {})", x.len(), y.len()));
                return;
            }
            for (i, (p, q)) in x.iter().zip(y.iter()).enumerate() {
                diff_paths(p, q, &format!("{path}[{i}]"), out);
            }
        }
        _ => {
            if a != b {
                out.push(format!("{path}: {a} vs {b}"));
            }
        }
    }
}

/// Where the AI's redactions of two worlds differ.
fn differences(a: &GameState, b: &GameState) -> Vec<String> {
    let mut out = Vec::new();
    diff_paths(&js(redact(a, AI)), &js(redact(b, AI)), "", &mut out);
    out
}

/// The AI's view of a world, without its events and the definitions they name.
fn shown(state: &GameState) -> Value {
    let mut view = js(view_for(state, AI));
    if let Some(map) = view.as_object_mut() {
        map.remove("events");
        map.remove("defs");
    }
    view
}

/// The first legal play of `player`'s hand card of `def_id`.
fn play_of(state: &GameState, player: PlayerId, def_id: &str) -> ActionBody {
    let id = state.players[player]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in hand");
    legal_actions(state, player)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .expect("the card can be played")
}

/// On turn 8, p2 holds `hand` and plays each of `plays` in order; with `end`, p2 then ends its turn
/// (unless R82 ended it), and the AI (p1) is to act.
fn p2_plays(hand: &[&str], plays: &[&str], end: bool) -> GameState {
    jackioh_cards::register_all();
    let mut state = scenario(json!({
        "seed": "redact-two-worlds",
        "active": "p2",
        "turn": 8,
        "p1": { "field": ["core-011"], "hand": [VANILLA], "library": ["core-020", "core-053", "core-019"] },
        "p2": { "hand": hand, "mana": 5, "library": [STOCKPILE, "core-016", "core-012"] },
    }))
    .state()
    .clone();
    for (i, def_id) in plays.iter().enumerate() {
        let body = play_of(&state, HUMAN, def_id);
        state = act_with_nonce(&state, HUMAN, body, &format!("play-{i}"));
    }
    if end && state.active == HUMAN {
        state = act_with_nonce(&state, HUMAN, ActionBody::EndTurn, "end");
    }
    assert_eq!(state.active, if end { AI } else { HUMAN }, "whose turn it is");
    state
}

/// The opponent set a (1) Trap face-down: Sheepish (no tag) in one world, Crop Dusting (Plague) in the
/// other. The true count says Plague; the redaction says nothing, and redacting it again changes nothing.
#[test]
fn r4804000_redact_hides_the_tags_of_the_opponents_face_down_trap() {
    let sheepish = p2_plays(&[SHEEPISH], &[SHEEPISH], true);
    let dusting = p2_plays(&[CROP_DUSTING], &[CROP_DUSTING], true);
    assert_eq!(
        shown(&sheepish),
        shown(&dusting),
        "the two worlds look alike to the seat"
    );
    let differs = differences(&sheepish, &dusting);
    assert!(
        differs.is_empty(),
        "redact tells the two worlds apart at: {differs:#?}"
    );

    assert_eq!(
        played_this_game_with_tag(&dusting, HUMAN, Tag::Plague),
        1,
        "the true count"
    );
    let redacted = redact(&dusting, AI);
    assert_eq!(
        played_this_game_with_tag(&redacted, HUMAN, Tag::Plague),
        0,
        "the seat's count"
    );
    assert_eq!(
        hash_state(&redact(&redacted, AI)),
        hash_state(&redacted),
        "redacting twice is redacting once"
    );
}

/// The opponent set a (1) Trap this turn: Sheepish, a Trap, in one world, Bread and Butter, a Field Trap,
/// in the other. This turn's plays by type count it as a Trap, as the seat saw it set.
#[test]
fn r4804000_redact_counts_the_opponents_face_down_play_this_turn_as_a_trap() {
    let trap = p2_plays(&[SHEEPISH, VANILLA], &[SHEEPISH], false);
    let field_trap = p2_plays(&[BREAD_AND_BUTTER, VANILLA], &[BREAD_AND_BUTTER], false);
    assert_eq!(
        shown(&trap),
        shown(&field_trap),
        "the two worlds look alike to the seat"
    );
    let differs = differences(&trap, &field_trap);
    assert!(
        differs.is_empty(),
        "redact tells the two worlds apart at: {differs:#?}"
    );

    assert_eq!(
        field_trap.players[HUMAN].turn_log.played_by_type,
        Some(IndexMap::from([(CardType::FieldTrap, 1)])),
        "the true count"
    );
    assert_eq!(
        redact(&field_trap, AI).players[HUMAN].turn_log.played_by_type,
        Some(IndexMap::from([(CardType::Trap, 1)])),
        "the seat's count"
    );
}

/// The opponent set a (1) Trap face-down: Overflow Error, an Infinite Assault AI card (R4593000), in one
/// world, Sheepish in the other. Neither its set nor its tag is the seat's to count.
#[test]
fn r4804000_redact_hides_the_set_of_the_opponents_face_down_trap() {
    let overflow = p2_plays(&[OVERFLOW_ERROR], &[OVERFLOW_ERROR], true);
    let sheepish = p2_plays(&[SHEEPISH], &[SHEEPISH], true);
    assert_eq!(
        shown(&overflow),
        shown(&sheepish),
        "the two worlds look alike to the seat"
    );
    let differs = differences(&overflow, &sheepish);
    assert!(
        differs.is_empty(),
        "redact tells the two worlds apart at: {differs:#?}"
    );

    assert_eq!(
        played_this_game_from_set(&overflow, HUMAN, SetName::InfiniteAssault),
        1,
        "the true set count"
    );
    assert_eq!(
        played_this_game_with_tag(&overflow, HUMAN, Tag::Ai),
        1,
        "the true tag count"
    );
    let redacted = redact(&overflow, AI);
    let log = redacted.players[HUMAN].game_log.as_ref().expect("p2 has played");
    assert_eq!(log.played_by_set, None, "the seat's set count");
    assert_eq!(
        played_this_game_with_tag(&redacted, HUMAN, Tag::Ai),
        0,
        "the seat's tag count"
    );
}

/// The opponent played Pestilent Slime face-up, then set a Trap: Sheepish in one world, Crop Dusting in
/// the other. The Slime's Plague and its Unit stay, as the seat watched it played, and the seat's own
/// counts are its own.
#[test]
fn r4804000_redact_keeps_the_opponents_face_up_plays_and_the_seats_own_counts() {
    let own = GameLog {
        played_by_tag: IndexMap::from([(Tag::Plague, 2)]),
        ..Default::default()
    };
    let world = |trap: &str| {
        let mut state = p2_plays(&[PESTILENT_SLIME, trap, VANILLA], &[PESTILENT_SLIME, trap], false);
        state.players[AI].game_log = Some(own.clone());
        state
    };
    let sheepish = world(SHEEPISH);
    let dusting = world(CROP_DUSTING);
    assert_eq!(
        shown(&sheepish),
        shown(&dusting),
        "the two worlds look alike to the seat"
    );
    let differs = differences(&sheepish, &dusting);
    assert!(
        differs.is_empty(),
        "redact tells the two worlds apart at: {differs:#?}"
    );

    for state in [&sheepish, &dusting] {
        let redacted = redact(state, AI);
        assert_eq!(
            played_this_game_with_tag(&redacted, HUMAN, Tag::Plague),
            1,
            "the Slime's Plague"
        );
        assert_eq!(
            played_this_turn_of_type(&redacted, HUMAN, &[CardType::Unit]),
            1,
            "the Slime's Unit"
        );
        assert_eq!(
            redacted.players[AI].game_log,
            Some(own.clone()),
            "the seat's own counts"
        );
    }
}

/// README, "`decide` reads only `redact`": `redact` "strips from the events its queues keep what
/// `view_for` strips of a hidden card". The AI's face-down C #9 Income Tax fires on the opponent's
/// second draw of a turn, and the opponent's keep prompt interrupts the firing, so `traps::owe_firing`
/// parks the rest of it on `state.work` as `@trapFiring` with the whole `drawn` event under
/// `resume.data.firing.event`: the drawn card's id and definition, a card in the opponent's hand. The
/// two worlds differ in the card p2 drew second.
#[test]
fn r185_redact_scrubs_the_event_an_interrupted_trap_firing_keeps() {
    jackioh_cards::register_all();
    let world = |second: &str| {
        let start = scenario(json!({
            "seed": "audit-hidden-owed-firing",
            "active": "p2",
            "turn": 8,
            "p1": {
                "hand": [VANILLA],
                "backrow": [{ "def": "classic-009", "faceUp": false }],
                "library": ["core-020", "core-053", "core-019"],
            },
            "p2": { "hand": [STOCKPILE, "core-011"], "mana": 5, "library": ["core-012", second, "core-012"] },
        }))
        .state()
        .clone();
        let body = play_of(&start, HUMAN, STOCKPILE);
        let drawn = act_with_nonce(&start, HUMAN, body, "stockpile");
        assert_eq!(
            drawn.pending.as_ref().map(|pending| pending.player_id),
            Some(HUMAN),
            "the keep prompt"
        );
        assert!(
            drawn
                .work
                .iter()
                .any(|item| item.resume.data.contains_key("firing")),
            "the owed firing"
        );
        drawn
    };
    let a = world("core-016");
    let b = world("core-053");
    let differs = differences(&a, &b);
    assert!(
        differs.is_empty(),
        "redact tells the two worlds apart at: {differs:#?}"
    );

    let redacted = redact(&a, AI);
    let firing = redacted
        .work
        .iter()
        .find_map(|item| item.resume.data.get("firing"))
        .expect("the owed firing is kept");
    assert_eq!(
        firing["event"]["defId"],
        json!(HIDDEN_DEF_ID),
        "the drawn card's definition"
    );
}

/// B5 E21: a face-down Trap the opponent set lies beneath another in a backrow pile, Sheepish in one world
/// and Crop Dusting in the other, after a face-up Plague play both seats watched. Only the hidden card's
/// Plague comes off the opponent's count; the watched one stays.
#[test]
fn r4804000_redact_hides_the_tags_of_the_opponents_face_down_trap_beneath_a_pile() {
    jackioh_cards::register_all();
    let world = |beneath: &str, plagues: i32| {
        let mut state = scenario(json!({
            "seed": "redact-two-worlds-pile",
            "p2": {
                "backrow": [
                    { "def": beneath, "faceUp": false },
                    { "def": SHEEPISH, "faceUp": false, "stack": true },
                ],
            },
        }))
        .state()
        .clone();
        state.players[HUMAN].game_log = Some(GameLog {
            played_by_tag: IndexMap::from([(Tag::Plague, plagues)]),
            ..Default::default()
        });
        state
    };
    let sheepish = world(SHEEPISH, 1);
    let dusting = world(CROP_DUSTING, 2);
    assert!(
        dusting.players[HUMAN]
            .backrow_piles
            .iter()
            .flatten()
            .flatten()
            .any(|card| card.def_id == CROP_DUSTING),
        "Crop Dusting lies beneath the pile's top"
    );
    assert_eq!(
        shown(&sheepish),
        shown(&dusting),
        "the two worlds look alike to the seat"
    );
    let differs = differences(&sheepish, &dusting);
    assert!(
        differs.is_empty(),
        "redact tells the two worlds apart at: {differs:#?}"
    );
    assert_eq!(
        played_this_game_with_tag(&redact(&dusting, AI), HUMAN, Tag::Plague),
        1,
        "the watched Plague play"
    );
}
