// The rest-of-game play watcher (issue #1863, R1863000): "for the rest of the game, the first time you
// play each …" is a `never` player modifier on the caster (`watch_first_plays`) that answers its owner's
// first resolved play of each card it names, a cast included (R70), once each. Each through a fixture
// script (`fixtures/play_watch.rs`) before a card uses it: Stinkyscape #44 Great Replacement Theory.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::play_watch::{
    ALPHA, BETA, CASTER, GAMMA, LABEL, WATCH, register_play_watch_fixtures,
};

/// `p1`'s main phase; each side's setup as given, with plain cards to draw so no turn ends by itself
/// (§2.5).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_play_watch_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id]);
        }
    }
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

/// The cards of `player`'s hand of `def_id`, as their faces (`true` for Radiant).
fn held(s: &Scenario, player: PlayerId, def_id: &str) -> Vec<bool> {
    s.hand(player)
        .into_iter()
        .filter(|card| card.def_id == def_id)
        .map(|card| card.radiant)
        .collect()
}

/// The definitions each of `player`'s play watches still names, in the order they were made.
fn watched(s: &Scenario, player: PlayerId) -> Vec<Vec<String>> {
    s.state().players[player]
        .mods
        .iter()
        .filter_map(|modifier| match &modifier.kind {
            ModifierKind::PlayWatch { def_ids, .. } => Some(def_ids.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn r1863000_the_first_resolved_play_of_each_watched_card_answers_once() {
    let mut s = board(
        "r1863000-once",
        json!({ "hand": [WATCH.id, ALPHA.id] }),
        json!({}),
    );
    s.play(WATCH.id.as_str(), json!({}));
    assert_eq!(watched(&s, P1), vec![vec![ALPHA.id.clone(), BETA.id.clone()]]);

    // The first alpha comes back once …
    s.play(ALPHA.id.as_str(), json!({}));
    assert_eq!(held(&s, P1, &ALPHA.id), vec![false], "the first alpha adds one");
    assert_eq!(
        watched(&s, P1),
        vec![vec![BETA.id.clone()]],
        "alpha comes off the watch"
    );

    // … and the second adds nothing.
    s.play(ALPHA.id.as_str(), json!({}));
    assert!(
        held(&s, P1, &ALPHA.id).is_empty(),
        "the second alpha adds nothing"
    );
    assert_eq!(watched(&s, P1), vec![vec![BETA.id.clone()]]);
}

#[test]
fn r1863000_a_cast_counts_and_unwatched_or_enemy_plays_do_not() {
    let mut s = board(
        "r1863000-cast",
        json!({ "hand": [WATCH.id, GAMMA.id, CASTER.id] }),
        json!({ "hand": [ALPHA.id] }),
    );
    s.play(WATCH.id.as_str(), json!({}));

    // A card the watch does not name answers nothing.
    s.play(GAMMA.id.as_str(), json!({}));
    assert!(held(&s, P1, &GAMMA.id).is_empty());
    assert_eq!(watched(&s, P1), vec![vec![ALPHA.id.clone(), BETA.id.clone()]]);

    // A cast is a play (R70): the beta the caster casts answers.
    s.play(CASTER.id.as_str(), json!({}));
    assert_eq!(held(&s, P1, &BETA.id), vec![false], "the cast beta adds one");
    assert_eq!(watched(&s, P1), vec![vec![ALPHA.id.clone()]]);

    // The opponent's alpha is no play of the watcher's: neither side gains a card and alpha stays.
    s.end_turn();
    let before = s.hand(P1).len();
    s.play(ALPHA.id.as_str(), json!({}));
    assert_eq!(s.hand(P1).len(), before, "the watcher gains nothing");
    assert!(held(&s, P2, &ALPHA.id).is_empty(), "nor does the opponent");
    assert_eq!(watched(&s, P1), vec![vec![ALPHA.id.clone()]]);
}

#[test]
fn r1863000_two_watches_each_answer_under_their_own_face() {
    let mut s = board(
        "r1863000-two",
        json!({ "hand": [WATCH.id, { "def": WATCH.id, "radiant": true }, ALPHA.id] }),
        json!({}),
    );
    s.play(WATCH.id.as_str(), json!({}));
    s.play(WATCH.id.as_str(), json!({}));
    assert_eq!(watched(&s, P1).len(), 2, "each cast keeps its own record");

    s.play(ALPHA.id.as_str(), json!({}));
    let mut faces = held(&s, P1, &ALPHA.id);
    faces.sort();
    assert_eq!(faces, vec![false, true], "one base alpha and one Radiant");
    assert_eq!(
        watched(&s, P1),
        vec![vec![BETA.id.clone()], vec![BETA.id.clone()]]
    );
}

#[test]
fn r1863000_the_watch_is_badged_in_both_views() {
    let mut s = board(
        "r1863000-badge",
        json!({ "hand": [WATCH.id, ALPHA.id, BETA.id] }),
        json!({}),
    );
    s.play(WATCH.id.as_str(), json!({}));
    let labels = |view: &[ModifierView]| -> Vec<String> {
        view.iter().map(|modifier| modifier.label.clone()).collect()
    };
    assert_eq!(labels(&s.view(P1).you.modifiers), vec![LABEL.to_string()]);
    assert_eq!(labels(&s.view(P2).opponent.modifiers), vec![LABEL.to_string()]);

    // Every card it names answered, the watch and its badge stay for the rest of the game.
    s.play(ALPHA.id.as_str(), json!({}));
    s.play(BETA.id.as_str(), json!({}));
    assert_eq!(watched(&s, P1), vec![Vec::<String>::new()]);
    s.end_turn();
    s.end_turn();
    assert_eq!(labels(&s.view(P1).you.modifiers), vec![LABEL.to_string()]);
    assert_eq!(labels(&s.view(P2).opponent.modifiers), vec![LABEL.to_string()]);
}
