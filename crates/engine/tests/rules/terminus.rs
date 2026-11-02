// The verbs Stinkyscape's Terminus cards are written with (issue #1870), each through a fixture script
// (`fixtures/terminus.rs`) before a card uses it: a win a card carries (R1870000), an exile that takes
// a Stack pile whole (R1870001), the Echo a resolving card gives itself up to its cap (R1870002), "your
// Units have <keyword>" through the next turn (R1870004) and a keyword a card keeps for good (R1870007).

use jackioh_engine::config::SELF_ECHO_GAIN_CAP;
use jackioh_engine::effects::WIN_ON_PLAY_KEY;
use jackioh_engine::query::lucky_on;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::terminus::*;

/// `p1`'s turn 9, its main phase, each side's setup as given, with plain cards to draw, and no turn
/// that ends by itself (R345), so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_terminus();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn in_hand(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
    s.hand(player)
        .into_iter()
        .find(|card| card.def_id == def_id)
        .unwrap_or_else(|| panic!("no {def_id} in {player:?}'s hand"))
}

fn may_attack_hero(s: &Scenario, attacker: &str) -> bool {
    let player = s.card(attacker).controller;
    let hero = format!("hero-{}", opponent_of(player));
    legal_actions(s.state(), player).into_iter().any(|action| {
        matches!(action, ActionBody::Attack { attacker_id, target_id } if attacker_id == attacker && target_id == hero)
    })
}

mod r1870000_win_on_play {
    use super::*;

    #[test]
    fn r1870000_a_card_carrying_the_win_wins_as_it_is_played() {
        let mut s = board("r1870000-crown", json!({ "hand": [CROWN] }), json!({}));
        s.play(CROWN, json!({}));
        let carrier = in_hand(&s, P1, PLAIN);
        assert_eq!(
            carrier.memory.get(WIN_ON_PLAY_KEY),
            Some(&Value::Bool(true)),
            "the pick carries the win in its hand"
        );
        assert!(s.state().result.is_none(), "holding it wins nothing");
        s.play(carrier.id.as_str(), json!({}));
        let result = s.state().result.expect("the play wins");
        assert_eq!(result.winner, Winner::P1);
        assert_eq!(result.reason, GameOverReason::WonByEffect);
        assert_eq!(s.state().players[P1].won_by_effect, Some(true));
    }

    #[test]
    fn r1870000_without_the_key_the_play_wins_nothing() {
        let mut s = board("r1870000-plain", json!({ "hand": [PLAIN] }), json!({}));
        s.play(PLAIN, json!({}));
        assert!(s.state().result.is_none());
        assert_eq!(s.state().players[P1].won_by_effect, None);
    }

    #[test]
    fn r1870000_a_hero_at_zero_still_loses_first() {
        let mut s = board(
            "r1870000-zero",
            json!({ "hand": [CROWN] }),
            json!({ "health": 1 }),
        );
        s.play(CROWN, json!({}));
        let carrier = in_hand(&s, P1, PLAIN);
        s.state_mut().players[P1].hero.health = 0;
        s.play(carrier.id.as_str(), json!({}));
        let result = s.state().result.expect("the game is over");
        assert_eq!(
            result.winner,
            Winner::P2,
            "a hero at 0 loses before a win is held"
        );
    }
}

mod r1870001_exile_all_piles {
    use super::*;

    #[test]
    fn r1870001_exile_all_piles_takes_a_stack_pile_whole() {
        let mut s = board(
            "r1870001-pile",
            json!({ "hand": [SWEEP], "field": [GRUNT] }),
            json!({ "field": [GRUNT, { "def": GRUNT, "stack": true }] }),
        );
        let ours = s.unit(P1, 1).expect("p1's grunt").id;
        let top = s.unit(P2, 1).expect("p2's top grunt").id;
        let under = s.state().players[P2].units[0]
            .as_ref()
            .and_then(|pile| pile.last())
            .expect("the grunt beneath")
            .id
            .clone();
        assert_ne!(top, under, "a pile of two");
        s.play(SWEEP, json!({}));
        for id in [&ours, &top, &under] {
            s.expect_in_zone(id.as_str(), "exile");
        }
        assert!(
            !s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::Destroyed),
            "an exile fires no Death"
        );
    }

    #[test]
    fn r1870001_a_dormant_card_stays() {
        let mut s = board(
            "r1870001-dormant",
            json!({ "hand": [SWEEP] }),
            json!({ "field": [{ "def": GRUNT, "lane": 2, "slumber": { "turns": 2 } }, { "def": GRUNT, "lane": 3 }] }),
        );
        let dormant = s.unit(P2, 2).expect("the dormant grunt").id;
        let awake = s.unit(P2, 3).expect("the awake grunt").id;
        s.play(SWEEP, json!({}));
        s.expect_in_zone(dormant.as_str(), "field");
        s.expect_in_zone(awake.as_str(), "exile");
    }
}

mod r1870002_gain_echo {
    use super::*;

    #[test]
    fn r1870002_gain_echo_repeats_until_the_cap() {
        let mut s = board("r1870002-cap", json!({ "hand": [EXAM] }), json!({}));
        let exam = in_hand(&s, P1, EXAM).id;
        s.play(EXAM, json!({}));
        s.expect_health(P2, HERO_HEALTH - EXAM_DAMAGE * (1 + SELF_ECHO_GAIN_CAP));
        s.expect_in_zone(exam.as_str(), "graveyard");
        assert!(s.state().echo_queue.is_empty(), "nothing is owed once it lands");
        assert!(
            !s.card(exam.as_str())
                .memory
                .contains_key(jackioh_engine::effects::ECHO_GAINED_KEY),
            "its count goes with R215's reset"
        );
    }

    #[test]
    fn r1870002_a_second_play_counts_from_nothing() {
        let dealt = EXAM_DAMAGE * (1 + SELF_ECHO_GAIN_CAP);
        let health = 2 * dealt + 1;
        let mut s = board(
            "r1870002-again",
            json!({ "hand": [EXAM, EXAM] }),
            json!({ "health": health }),
        );
        s.play(EXAM, json!({}));
        s.play(EXAM, json!({}));
        s.expect_health(P2, health - 2 * dealt);
    }
}

mod r1870004_units_keyword {
    use super::*;

    #[test]
    fn r1870004_units_keyword_charges_only_on_the_next_turn() {
        let mut s = board(
            "r1870004-charge",
            json!({ "hand": [RALLY, GRUNT, GRUNT, GRUNT] }),
            json!({ "hand": [PLAIN] }),
        );
        s.play(RALLY, json!({}));
        let first = in_hand(&s, P1, GRUNT).id;
        s.play(first.as_str(), json!({}));
        assert!(!may_attack_hero(&s, &first), "not on the turn it was given");

        s.end_turn();
        s.end_turn();
        assert!(may_attack_hero(&s, &first), "a Unit that stood there has it too");
        let second = in_hand(&s, P1, GRUNT).id;
        s.play(second.as_str(), json!({}));
        assert!(
            unit_has(s.state(), s.card(second.as_str()), KeywordKind::Charge),
            "a Unit that lands has Charge"
        );
        assert!(
            may_attack_hero(&s, &second),
            "and may attack the hero the turn it lands"
        );

        s.end_turn();
        s.end_turn();
        let third = in_hand(&s, P1, GRUNT).id;
        s.play(third.as_str(), json!({}));
        assert!(!may_attack_hero(&s, &third), "gone after that turn");
        assert!(!unit_has(s.state(), s.card(first.as_str()), KeywordKind::Charge));
        assert!(
            s.state().players[P1].mods.is_empty(),
            "the modifier ends at that turn's cleanup"
        );
    }

    #[test]
    fn r1870004_the_enemys_units_have_none_of_it() {
        let mut s = board(
            "r1870004-enemy",
            json!({ "hand": [RALLY, GRUNT] }),
            json!({ "field": [GRUNT], "hand": [PLAIN] }),
        );
        let theirs = s.unit(P2, 1).expect("p2's grunt").id;
        s.play(RALLY, json!({}));
        s.end_turn();
        assert!(!unit_has(s.state(), s.card(theirs.as_str()), KeywordKind::Charge));
        s.end_turn();
        assert!(!unit_has(s.state(), s.card(theirs.as_str()), KeywordKind::Charge));
    }
}

mod r1870007_keyword_for_good {
    use super::*;

    #[test]
    fn r1870007_a_keyword_kept_for_good_survives_the_graveyard() {
        let mut s = board("r1870007-charm", json!({ "hand": [CHARM] }), json!({}));
        let charm = in_hand(&s, P1, CHARM).id;
        s.play(CHARM, json!({}));
        s.expect_in_zone(charm.as_str(), "graveyard");
        let landed = s.card(charm.as_str());
        assert_eq!(lucky_on(s.state(), landed), 1, "R215's reset keeps tuning");
        assert!(
            s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::KeywordGranted { instance_id, .. } if *instance_id == charm)),
            "reported as a grant"
        );
    }
}
