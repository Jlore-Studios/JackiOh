//! Evolve and Evolves (issue #3536, R3536000–R3536005): the once-only Evolve text, the standing
//! and trigger "Evolves when" conditions, the deck mark, the multiplier, the counter and the view,
//! proved on the fixtures of `fixtures/evolution.rs` with no card yet.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::evolution::*;

/// p1's turn 9, its main phase; each side's setup as given, with plain cards to draw, and no turn
/// that ends by itself (R345), so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_evolution_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn evolved_events(events: &[GameEvent]) -> Vec<&GameEvent> {
    events
        .iter()
        .filter(|event| event.event_type() == GameEventType::Evolved)
        .collect()
}

fn evolutions(s: &Scenario, player: PlayerId) -> i32 {
    query::evolutions_of(s.state(), player)
}

fn is_evolved(s: &Scenario, id: &str) -> bool {
    s.card(id).evolved == Some(true)
}

mod r3536000_once {
    use super::*;

    #[test]
    fn r3536000_a_second_evolve_does_nothing_and_counts_nothing() {
        let mut s = board(
            "r3536000-second",
            json!({ "hand": [EVOLVER, EVOLVER], "field": [PINGER] }),
            json!({}),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&pinger));
        s.expect_health(P2, HERO_HEALTH - EV_PING);
        assert!(is_evolved(&s, &pinger));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        // A second evolve of the same permanent runs nothing and counts nothing.
        s.play(EVOLVER, pick(&pinger));
        s.expect_health(P2, HERO_HEALTH - EV_PING);
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
    }

    #[test]
    fn r3536000_a_card_back_from_its_hand_may_evolve_again() {
        let mut s = board(
            "r3536000-bounce",
            json!({ "hand": [EVOLVER, EVOLVER, BOUNCER], "field": [PINGER] }),
            json!({}),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&pinger));
        assert!(is_evolved(&s, &pinger));
        // R78's reset takes the mark off with the card leaving the field.
        s.play(BOUNCER, pick(&pinger));
        assert!(!is_evolved(&s, &pinger));
        s.play(PINGER, json!({ "zone": 1 }));
        let back = id_at(&s, P1, 1);
        assert!(!is_evolved(&s, &back));
        s.play(EVOLVER, pick(&back));
        s.expect_health(P2, HERO_HEALTH - 2 * EV_PING);
        assert_eq!(evolutions(&s, P1), 2);
        assert_eq!(evolved_events(s.events()).len(), 2);
    }

    #[test]
    fn r3536000_a_copy_is_a_new_card_and_may_evolve() {
        let mut s = board(
            "r3536000-copy",
            json!({ "hand": [EVOLVER, EVOLVER, COPIER], "field": [PINGER] }),
            json!({}),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&pinger));
        s.play(COPIER, pick(&pinger));
        let copy = id_at(&s, P1, 2);
        assert!(!is_evolved(&s, &copy), "a copy is a new card (R57)");
        s.play(EVOLVER, pick(&copy));
        assert!(is_evolved(&s, &copy));
        s.expect_health(P2, HERO_HEALTH - 2 * EV_PING);
        assert_eq!(evolutions(&s, P1), 2);
        assert_eq!(evolved_events(s.events()).len(), 2);
    }

    #[test]
    fn r3536000_a_vanilla_unit_evolves_into_nothing() {
        let mut s = board(
            "r3536000-vanilla",
            json!({ "hand": [EVOLVER], "field": [PLAIN] }),
            json!({}),
        );
        let plain = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&plain));
        s.expect_health(P2, HERO_HEALTH);
        assert!(!is_evolved(&s, &plain));
        assert_eq!(evolutions(&s, P1), 0);
        assert!(evolved_events(s.events()).is_empty());
    }
}

mod r3536001_evolves_when {
    use super::*;

    #[test]
    fn r3536001_a_hand_of_three_zero_cost_cards() {
        // Two zero-cost cards plus a dear one hold nothing.
        let mut s = board(
            "r3536001-zero-few",
            json!({ "hand": [PLAIN, PLAIN, DEAR], "field": [ZERO] }),
            json!({}),
        );
        let zero = id_at(&s, P1, 1);
        s.switch_position(zero.as_str());
        assert!(!is_evolved(&s, &zero));
        assert_eq!(evolutions(&s, P1), 0);
        s.expect_health(P2, HERO_HEALTH);
        // Three zero-cost cards evolve it at the settled check after it enters.
        let mut s = board(
            "r3536001-zero",
            json!({ "hand": [ZERO, PLAIN, PLAIN, PLAIN] }),
            json!({}),
        );
        s.play(ZERO, json!({ "zone": 1 }));
        let zero = id_at(&s, P1, 1);
        assert!(is_evolved(&s, &zero));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_all_unit_zones_full() {
        let mut s = board(
            "r3536001-full",
            json!({ "hand": [PLAIN], "field": [FULL, PLAIN, PLAIN, PLAIN] }),
            json!({}),
        );
        let full = id_at(&s, P1, 1);
        assert!(!is_evolved(&s, &full), "four of five zones hold nothing");
        assert_eq!(evolutions(&s, P1), 0);
        s.play(PLAIN, json!({ "zone": 5 }));
        assert!(is_evolved(&s, &full));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        s.switch_position(full.as_str());
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_below_half_health_but_not_a_lethal_hit() {
        // Five off eight is below half: it evolves.
        let mut s = board(
            "r3536001-half",
            json!({ "hand": [BOLT], "field": [HALF] }),
            json!({}),
        );
        let half = id_at(&s, P1, 1);
        s.play(BOLT, pick(&half));
        assert!(is_evolved(&s, &half));
        assert_eq!(evolutions(&s, P1), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
        // Two off eight is not below half: nothing evolves.
        let mut s = board(
            "r3536001-half-whole",
            json!({ "field": [HALF] }),
            json!({ "field": [PLAIN] }),
        );
        let half = id_at(&s, P1, 1);
        let plain = id_at(&s, P2, 1);
        s.attack(half.as_str(), plain.as_str());
        assert_eq!(s.card(&half).damage, EV_HALF);
        assert!(!is_evolved(&s, &half));
        assert_eq!(evolutions(&s, P1), 0);
        assert!(evolved_events(s.events()).is_empty());
        // A lethal hit kills it instead: the dead card evolves nothing.
        let mut s = board(
            "r3536001-half-lethal",
            json!({ "field": [HALF] }),
            json!({ "field": [DEAR] }),
        );
        let half = id_at(&s, P1, 1);
        let dear = id_at(&s, P2, 1);
        s.attack(half.as_str(), dear.as_str());
        assert_eq!(s.card(&half).zone.z(), ZoneName::Graveyard);
        assert!(!is_evolved(&s, &half));
        assert_eq!(evolutions(&s, P1), 0);
        assert!(evolved_events(s.events()).is_empty());
    }

    #[test]
    fn r3536001_stacked_on_a_unit() {
        let mut s = board(
            "r3536001-stacked",
            json!({ "field": [PLAIN, { "def": STACKER, "stack": true }] }),
            json!({}),
        );
        let stacker = id_at(&s, P1, 1);
        assert!(is_evolved(&s, &stacker), "the setup's state check evolves it");
        assert_eq!(evolutions(&s, P1), 1);
        // The setup's Evolved event stays in setup, which the log does not keep; the queued run
        // still drains on the first step.
        assert!(evolved_events(s.events()).is_empty());
        s.switch_position(stacker.as_str());
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_temporary_s_discard_says_so_and_the_card_evolves_in_its_graveyard() {
        let mut s = board("r3536001-cheese", json!({ "hand": [CHEESE] }), json!({}));
        let cheese = s.hand(P1)[0].id.clone();
        s.end_turn();
        assert_eq!(s.card(&cheese).zone.z(), ZoneName::Graveyard);
        assert!(is_evolved(&s, &cheese), "it evolves where it lies");
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        let discarded = s
            .events()
            .iter()
            .find(|event| event.event_type() == GameEventType::Discarded)
            .unwrap_or_else(|| panic!("a discard reports the Temporary countdown"));
        let GameEvent::Discarded { by_temporary, .. } = discarded else {
            panic!("a discard reports the Temporary countdown");
        };
        assert_eq!(*by_temporary, Some(true));
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_your_opponent_plays_a_unit() {
        let mut s = board(
            "r3536001-watcher",
            json!({ "field": [WATCHER] }),
            json!({ "hand": [PLAIN, PLAIN] }),
        );
        let watcher = id_at(&s, P1, 1);
        s.end_turn();
        // Twice still evolves it once.
        s.play(PLAIN, json!({ "zone": 1 }));
        assert!(is_evolved(&s, &watcher));
        s.play(PLAIN, json!({ "zone": 2 }));
        s.expect_health(P2, HERO_HEALTH - EV_PING);
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
    }

    #[test]
    fn r3536001_defense_to_attack() {
        let mut s = board(
            "r3536001-riser",
            json!({ "field": [{ "def": RISER, "position": "DEF" }] }),
            json!({}),
        );
        let riser = id_at(&s, P1, 1);
        assert!(!is_evolved(&s, &riser));
        s.switch_position(riser.as_str());
        assert!(is_evolved(&s, &riser));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_survives_damage() {
        let mut s = board(
            "r3536001-survivor",
            json!({ "hand": [BOLT], "field": [SURVIVOR] }),
            json!({}),
        );
        let survivor = id_at(&s, P1, 1);
        s.play(BOLT, pick(&survivor));
        assert!(is_evolved(&s, &survivor), "five off six lives and evolves");
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_a_friendly_unit_dies() {
        let mut s = board(
            "r3536001-mourner",
            json!({ "hand": [BOLT], "field": [MOURNER, PLAIN] }),
            json!({}),
        );
        let plain = id_at(&s, P1, 2);
        let mourner = id_at(&s, P1, 1);
        s.play(BOLT, pick(&plain));
        assert_eq!(s.card(&plain).zone.z(), ZoneName::Graveyard);
        assert!(is_evolved(&s, &mourner));
        assert!(!is_evolved(&s, &plain), "the dead card evolves nothing");
        assert_eq!(evolutions(&s, P1), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_this_destroys_a_unit() {
        let mut s = board(
            "r3536001-killer",
            json!({ "field": [KILLER] }),
            json!({ "field": [PLAIN] }),
        );
        let killer = id_at(&s, P1, 1);
        let plain = id_at(&s, P2, 1);
        s.attack(killer.as_str(), plain.as_str());
        assert_eq!(s.card(&plain).zone.z(), ZoneName::Graveyard);
        assert!(is_evolved(&s, &killer));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        // The combat was unit on unit, so the only hero damage is the Evolve text's.
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_start_of_turn() {
        let mut s = board("r3536001-dawn", json!({ "field": [DAWN] }), json!({}));
        let dawn = id_at(&s, P1, 1);
        s.end_turn();
        assert!(!is_evolved(&s, &dawn), "the other seat's turn holds nothing");
        s.end_turn();
        assert!(is_evolved(&s, &dawn));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536001_you_launch_a_starship() {
        use crate::rules::fixtures::combat::plain as combat_plain;
        use crate::rules::fixtures::starship as ss;
        ss::register_starship_fixtures();
        let mut catalog: CardDefs = registered_catalog().clone();
        for def in defs() {
            if def.id == SHIP {
                catalog.insert(def.id.clone(), def);
            }
        }
        register_catalog(catalog);
        let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
        merged.extend(scripts().into_iter().filter(|(id, _)| id == SHIP));
        register_scripts(merged);
        let library = json!([combat_plain.id, combat_plain.id, combat_plain.id, combat_plain.id]);
        let mut s = scenario(json!({
            "seed": "r3536001-launch",
            "active": "p1",
            "p1": {
                "mana": 4,
                "hand": [ss::bolt.id],
                "field": [{ "def": ss::booster.id, "lane": 1 }, { "def": SHIP, "lane": 2 }],
                "library": library,
            },
            "p2": { "hand": [combat_plain.id], "field": [{ "def": combat_plain.id, "lane": 1 }], "library": library },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        let ship = id_at(&s, P1, 2);
        let booster = id_at(&s, P1, 1);
        s.play(ss::bolt.id.as_str(), pick(&booster));
        let hull = jackioh_engine::starship::dormant_starship_of(s.state(), P1)
            .unwrap_or_else(|| panic!("the piece's death summons a Dormant Starship"))
            .id
            .clone();
        s.activate(
            hull.as_str(),
            json!({ "ability": jackioh_engine::starship::STARSHIP_LAUNCH_ABILITY }),
        );
        assert!(is_evolved(&s, &ship));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        // The launched piece's own text hits too, then the Evolve text's ping.
        s.expect_health(P2, HERO_HEALTH - ss::LAUNCHED_DAMAGE - EV_PING);
    }
}

mod r3536002_enters_evolved {
    use super::*;

    #[test]
    fn r3536002_evolve_deck_marks_only_cards_with_evolve_text() {
        let mut s = board(
            "r3536002-marks",
            json!({ "hand": [BREEDER], "library": [PINGER, PLAIN, PINGER] }),
            json!({}),
        );
        s.play(BREEDER, json!({}));
        let library = &s.state().players[P1].library;
        assert_eq!(library.len(), 3);
        for card in library {
            if card.def_id == PINGER {
                assert_eq!(card.enters_evolved, Some(true));
            } else {
                assert_eq!(card.enters_evolved, None, "no Evolve text, no mark");
            }
            assert_eq!(card.evolved, None, "a mark counts nothing");
        }
        assert_eq!(evolutions(&s, P1), 0);
        assert!(evolved_events(s.events()).is_empty());
    }

    #[test]
    fn r3536002_a_marked_card_evolves_once_after_it_enters() {
        let mut s = board(
            "r3536002-marked",
            json!({ "hand": [BREEDER], "library": [PINGER, PLAIN, PLAIN, PLAIN] }),
            json!({}),
        );
        s.play(BREEDER, json!({}));
        s.end_turn();
        s.end_turn();
        s.play(PINGER, json!({ "zone": 1 }));
        let pinger = id_at(&s, P1, 1);
        assert!(is_evolved(&s, &pinger), "the state check after entry evolves it");
        assert_eq!(s.card(&pinger).enters_evolved, None, "the mark is spent");
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
        s.expect_health(P2, HERO_HEALTH - EV_PING);
    }

    #[test]
    fn r3536002_a_marked_card_with_no_evolve_text_loses_the_mark() {
        let mut s = board(
            "r3536002-markless",
            json!({ "hand": [BREEDER], "library": [PLAIN, PLAIN, PLAIN, PLAIN] }),
            json!({}),
        );
        s.play(BREEDER, json!({}));
        s.end_turn();
        s.end_turn();
        s.play(PLAIN, json!({ "zone": 1 }));
        let plain = id_at(&s, P1, 1);
        assert!(!is_evolved(&s, &plain));
        assert_eq!(s.card(&plain).enters_evolved, None);
        assert_eq!(evolutions(&s, P1), 0);
        assert!(evolved_events(s.events()).is_empty());
        s.expect_health(P2, HERO_HEALTH);
    }
}

mod r3536003_multiplier {
    use super::*;

    #[test]
    fn r3536003_twice_runs_the_text_twice_and_counts_one() {
        let mut s = board(
            "r3536003-twice",
            json!({ "hand": [EVOLVER], "field": [PINGER], "backrow": [TWICE] }),
            json!({}),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&pinger));
        assert!(is_evolved(&s, &pinger));
        s.expect_health(P2, HERO_HEALTH - 2 * EV_PING);
        assert_eq!(evolutions(&s, P1), 1, "the once-only rule counts evolvings");
        assert_eq!(evolved_events(s.events()).len(), 1);
    }

    #[test]
    fn r3536003_the_highest_holds() {
        let mut s = board(
            "r3536003-highest",
            json!({ "hand": [EVOLVER], "field": [PINGER], "backrow": [TWICE, THRICE] }),
            json!({}),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&pinger));
        s.expect_health(P2, HERO_HEALTH - 3 * EV_PING);
        assert_eq!(evolutions(&s, P1), 1);
    }

    #[test]
    fn r3536003_a_standing_evolution_queues_every_run() {
        let mut s = board(
            "r3536003-standing",
            json!({ "hand": [ZERO, PLAIN, PLAIN, PLAIN], "backrow": [TWICE] }),
            json!({}),
        );
        s.play(ZERO, json!({ "zone": 1 }));
        let zero = id_at(&s, P1, 1);
        assert!(is_evolved(&s, &zero));
        s.expect_health(P2, HERO_HEALTH - 2 * EV_PING);
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 1);
    }
}

mod r3536004_count {
    use super::*;

    #[test]
    fn r3536004_each_evolution_counts_for_its_controller_and_a_mark_counts_nothing() {
        let mut s = board(
            "r3536004-count",
            json!({ "hand": [BREEDER, EVOLVER], "field": [PINGER], "library": [PINGER] }),
            json!({ "hand": [EVOLVER], "field": [PINGER] }),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(BREEDER, json!({}));
        assert_eq!(evolutions(&s, P1), 0, "a mark counts nothing");
        s.play(EVOLVER, pick(&pinger));
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolutions(&s, P2), 0);
        s.end_turn();
        let foe = id_at(&s, P2, 1);
        s.play(EVOLVER, pick(&foe));
        assert_eq!(evolutions(&s, P2), 1);
        assert_eq!(evolutions(&s, P1), 1);
        assert_eq!(evolved_events(s.events()).len(), 2);
    }
}

mod r3536005_view {
    use super::*;

    #[test]
    fn r3536005_both_seats_see_the_mark_and_the_event_and_the_owner_sees_a_marked_hand_card() {
        let mut s = board(
            "r3536005-view",
            json!({ "hand": [EVOLVER, BREEDER], "field": [PINGER], "library": [PINGER] }),
            json!({}),
        );
        let pinger = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&pinger));
        for viewer in [P1, P2] {
            let seen = s
                .view(viewer)
                .you
                .units
                .iter()
                .flatten()
                .chain(s.view(viewer).opponent.units.iter().flatten())
                .find(|unit| unit.instance_id == pinger)
                .unwrap_or_else(|| panic!("{viewer:?} sees the evolved unit"))
                .clone();
            assert_eq!(seen.evolved, Some(true));
            assert!(
                s.view(viewer).events.iter().any(|event| matches!(
                    event,
                    GameEvent::Evolved { instance_id, def_id, .. }
                    if instance_id == &pinger && def_id == PINGER
                )),
                "{viewer:?} sees the Evolved event with its names on"
            );
        }
        // A marked card shows the mark in its owner's hand.
        s.play(BREEDER, json!({}));
        s.end_turn();
        s.end_turn();
        let hand = match &s.view(P1).you.hand {
            HandView::Cards(cards) => cards.clone(),
            HandView::Count { .. } => panic!("the owner reads their own hand"),
        };
        assert!(
            hand.iter()
                .any(|card| card.def_id == PINGER && card.evolved == Some(true)),
            "the drawn marked card wears the mark"
        );
    }
}
