//! The learned evaluation's features (#1006, `docs/learned-eval.md` D5): they read only what the
//! seat may know (R185, R1006000), they never panic on a redacted view's hidden cards, and their
//! layout is the one `dense_names` names.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, clone};

fn p1_side() -> Value {
    json!({
        "hand": ["core-008", "core-035"],
        "field": ["core-011"],
        "library": ["core-020", "core-053", "core-025"],
    })
}

/// The pair `observe.rs` names (B9): p2's hand, face-down trap and library differ.
fn p2_a() -> Value {
    json!({
        "hand": ["core-002", "core-011"],
        "field": ["core-019"],
        "backrow": [{ "def": "core-041", "faceUp": false }],
        "library": ["core-005", "core-016", "core-047"],
        "health": 20,
    })
}

fn p2_b() -> Value {
    json!({
        "hand": ["core-053", "core-020"],
        "field": ["core-019"],
        "backrow": [{ "def": "core-060", "faceUp": false }],
        "library": ["core-063", "core-044", "core-072"],
        "health": 20,
    })
}

fn build(seed: &str, p2: Value, p1: Value) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// R185, R1006000: two states that differ only in p2's hidden hand, face-down trap and library, p1's
/// library order and the seed give p1 the same features.
#[test]
fn r1006000_r185_two_states_differing_only_in_hidden_cards_give_the_same_features() {
    let a = build("features-a", p2_a(), p1_side());
    let mut b = clone(&build("features-b", p2_b(), p1_side()));
    b.players[PlayerId::P1].library.reverse();
    assert_ne!(hash_state(&a), hash_state(&b));
    assert_eq!(features_of(&a, AI), features_of(&b, AI));
}

/// R1006000: what the seat can read does change them, and the enemy's face-down card is counted
/// without being named.
#[test]
fn r1006000_the_features_see_the_board_and_the_own_hand_and_count_the_hidden_card() {
    let state = build("features-c", p2_a(), p1_side());
    let found = features_of(&state, AI);
    let names = dense_names();
    assert_eq!(found.dense.len(), names.len());
    let value = |name: &str| found.dense[names.iter().position(|n| n == name).unwrap()];
    assert_eq!(value("own.hand"), 2);
    assert_eq!(value("enemy.hand"), 2);
    assert_eq!(value("enemy.life"), 20);
    assert_eq!(value("enemy.backrow"), 1);
    assert_eq!(value("enemy.backrow_hidden"), 1);
    assert_eq!(value("own.units"), 1);
    let keys: Vec<&str> = found.sparse.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"board:own:core-011"), "{keys:?}");
    assert!(keys.contains(&"board:enemy:core-019"), "{keys:?}");
    assert!(keys.contains(&"hand:own:core-008"), "{keys:?}");
    assert!(
        !keys.iter().any(|key| key.contains("core-041")),
        "the face-down trap is not named: {keys:?}"
    );
    assert!(!keys.iter().any(|key| key.starts_with("hand:enemy")), "{keys:?}");

    // The other seat reads its own trap and hand.
    let theirs = features_of(&state, PlayerId::P2);
    let keys: Vec<&str> = theirs.sparse.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"backrow:own:core-041"), "{keys:?}");
    assert!(keys.contains(&"hand:own:core-002"), "{keys:?}");
}

/// R1012004: a Radiant card in the seat's own hand is an input of its own, a count and a second key
/// beside the card's own, and nothing else it reads moves; the other seat, which cannot see that
/// hand, reads the same features either way.
#[test]
fn r1012004_a_radiant_card_in_the_seats_own_hand_is_an_input_of_its_own() {
    let plain = build("features-radiant", p2_a(), p1_side());
    let mut radiant = clone(&plain);
    let at = radiant.players[AI]
        .hand
        .iter()
        .position(|card| card.def_id == "core-008")
        .expect("core-008 is in hand");
    radiant.players[AI].hand[at].radiant = true;

    let names = dense_names();
    let count = names
        .iter()
        .position(|name| name == "own.hand_radiant")
        .expect("a dense name");
    let (before, after) = (features_of(&plain, AI), features_of(&radiant, AI));
    assert_eq!((before.dense[count], after.dense[count]), (0, 1));
    let mut rest = after.dense.clone();
    rest[count] = 0;
    assert_eq!(rest, before.dense);

    let key = ("hand_radiant:own:core-008".to_string(), 1);
    assert!(after.sparse.contains(&key), "{:?}", after.sparse);
    assert!(
        after.sparse.contains(&("hand:own:core-008".to_string(), 1)),
        "{:?}",
        after.sparse
    );
    let others: Vec<_> = after
        .sparse
        .iter()
        .filter(|pair| **pair != key)
        .cloned()
        .collect();
    assert_eq!(others, before.sparse);

    assert_eq!(
        features_of(&plain, PlayerId::P2),
        features_of(&radiant, PlayerId::P2)
    );
}
