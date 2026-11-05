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
    // The keys that name a card by id; what the card is follows its Radiant face (R3112000's test).
    let ids = |sparse: &[(String, i32)]| -> Vec<(String, i32)> {
        sparse
            .iter()
            .filter(|(key, _)| card_id_of_key(key).is_some())
            .cloned()
            .collect()
    };
    let others: Vec<_> = ids(&after.sparse)
        .into_iter()
        .filter(|pair| *pair != key)
        .collect();
    assert_eq!(others, ids(&before.sparse));

    assert_eq!(
        features_of(&plain, PlayerId::P2),
        features_of(&radiant, PlayerId::P2)
    );
}

/// R3112000, R185: beside its id, each card the seat can name has the keys of what it is, read from its
/// definition and its running face: its type, printed cost, tags, keywords, printed attack and health
/// (clamped at `ATTRIBUTE_TOP`) and its Radiant face. The enemy's hand and face-down trap give none; the
/// seat that owns the trap reads it.
#[test]
fn r3112000_r185_the_features_name_what_each_card_the_seat_can_read_is() {
    let state = build("features-attr", p2_a(), p1_side());
    let def = |id: &str| jackioh_engine::find_def(Some(&state), id).expect("a catalog card");
    let top = |value: i32| value.clamp(0, ATTRIBUTE_TOP);
    let found = features_of(&state, AI);
    let keys: Vec<&str> = found.sparse.iter().map(|(key, _)| key.as_str()).collect();
    let has = |key: String| assert!(keys.contains(&key.as_str()), "{key} in {keys:?}");

    let timmy = def("core-011");
    has("board.type:own:Unit".to_string());
    has(format!("board.cost:own:{}", top(query_cost(timmy))));
    for tag in &timmy.tags {
        has(format!("board.tag:own:{}", tag.as_str()));
    }
    for keyword in &timmy.base.keywords {
        has(format!("board.kw:own:{}", keyword.kind().as_str()));
    }
    has(format!(
        "board.attack:own:{}",
        top(timmy.base.attack.expect("a Unit's attack"))
    ));
    has(format!(
        "board.health:own:{}",
        top(timmy.base.health.expect("a Unit's health"))
    ));
    has("board.type:enemy:Unit".to_string());
    assert_eq!(def("core-008").type_, CardType::Unit);
    assert_eq!(def("core-035").type_, CardType::Spell);
    has("hand.type:own:Unit".to_string());
    has("hand.type:own:Spell".to_string());
    assert!(
        !keys.iter().any(|key| key.starts_with("backrow")),
        "the face-down trap gives no key: {keys:?}"
    );
    assert!(
        !keys
            .iter()
            .any(|key| key.starts_with("hand") && key.contains(":enemy:")),
        "the enemy's hand gives no key: {keys:?}"
    );
    assert!(!keys.iter().any(|key| key.contains("Radiant")), "{keys:?}");

    let theirs = features_of(&state, PlayerId::P2);
    assert!(
        theirs
            .sparse
            .iter()
            .any(|(key, _)| key == "backrow.type:own:Trap"),
        "{:?}",
        theirs.sparse
    );

    // A Radiant card in hand is read by its Radiant face, and the other seat sees no change.
    let mut radiant = clone(&state);
    let at = radiant.players[AI]
        .hand
        .iter()
        .position(|card| card.def_id == "core-008")
        .expect("core-008 is in hand");
    radiant.players[AI].hand[at].radiant = true;
    let glowing = features_of(&radiant, AI);
    let keys: Vec<&str> = glowing.sparse.iter().map(|(key, _)| key.as_str()).collect();
    let face = &def("core-008").radiant;
    for key in [
        "hand.face:own:Radiant".to_string(),
        format!("hand.attack:own:{}", top(face.attack.expect("a Unit's attack"))),
        format!("hand.health:own:{}", top(face.health.expect("a Unit's health"))),
    ] {
        assert!(keys.contains(&key.as_str()), "{key} in {keys:?}");
    }
    assert_eq!(theirs, features_of(&radiant, PlayerId::P2));
}
