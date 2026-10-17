//! C #29 Book of Vital Kill (SPEC §8.6 row 29, §6.3 Set health; R18, R81, R317). Spell, Book, cost 1,
//! Epic.
//!   Base:    "Set health of a hero to 13.\nEnd the turn."
//!   Radiant: "Set health of a hero to 13."
//!   Engine:  "Set health (§6.3) on either hero, a declared target (R81): no pipeline, not damage and
//!            not a heal, like R18's lose health." Patch v0.3.10 (#831): the base face then ends the
//!            turn (§2.2, as C+ #26 Tommy Tempo's cast on draw does), and the Radiant face no longer
//!            adds a Book of Flame: its edge is the turn it leaves you.
//!
//! The target is declared (R81): one hero, either side. `setHealth` emits `healthSet` and runs no §4.4
//! step, so Armor, hit caps, Lifesteal and every "takes damage" or "is healed" trigger never see it.
//! The entry declares no `params`: a Degrade has nothing to move, as 13 is good or bad depending on
//! whose hero it is.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-029";

const VITAL_HEALTH: i32 = 13;

fn set_vital_health() -> Effect {
    set_health(json_as(json!({ "to": { "of": "chosen" }, "value": VITAL_HEALTH })))
}

pub fn script() -> CardScripts {
    let targets = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["hero"] }))];

    let base = Script {
        targets: targets.clone(),
        cry: Some(hook(|_ctx| vec![set_vital_health(), end_turn(json_as(json!({})))])),
        ..Script::default()
    };

    let radiant = Script {
        targets,
        cry: Some(hook(|_ctx| vec![set_vital_health()])),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #29 Book of Vital Kill — SPEC §8.6 row 29, BUILD M9 Classic row C 29: a declared hero target, either
// side, set to 13 from above or below with Armor unchanged: not damage and not a heal, so hit caps, C #75,
// Lifesteal, C #52's lethal window and Fed Fauci's tokens never see it; `healthSet` is public; the base
// face then ends the turn, the Radiant face does not (patch v0.3.10).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VITAL: &str = "classic-029";
    const GAMBIT: &str = "classic-052"; // C #52 Final Gambit, (2) Trap: a hit that would bring your hero to 0 or less.
    const FAUCI: &str = "core-091"; // (2) Unit: "Whenever this takes damage, place a Plague Counter on this."
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text.
    const ANTI_ONESHOT: &str = "core-073"; // (2) Field Spell: "Your hero can't take more than 5 damage at a time."
    const FILLER: &str = "core-005"; // (1) Spell, a spare card so a hand never runs out (§2.5).

    fn events_of(s: &Scenario, kind: GameEventType) -> Vec<GameEvent> {
        s.events().iter().filter(|event| event.event_type() == kind).cloned().collect()
    }

    fn play_at(s: &mut Scenario, player: PlayerId) -> &mut Scenario {
        s.play(VITAL, json!({ "targets": [{ "pick": "hero", "player": player }] }))
    }

    fn defs_of(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    mod c_n29_book_of_vital_kill {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r81_legalactions_offers_the_play_at_each_hero_and_at_no_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VITAL, FILLER], "field": [VANILLA] },
                    "p2": { "hand": [FILLER], "library": [FILLER], "field": [VANILLA] },
                }));
                let book = s.card(VITAL).clone();
                let targets: Vec<Option<Vec<Selection>>> = legal_actions(s.state(), P1)
                    .into_iter()
                    .filter_map(|action| match action {
                        ActionBody::Play { instance_id, targets, .. } if instance_id == book.id => Some(targets),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    targets,
                    vec![
                        Some(vec![Selection::Hero { player: P1 }]),
                        Some(vec![Selection::Hero { player: P2 }]),
                    ]
                );
                let unit = s.unit(P2, 1).expect("p2's Vanilla should be on the board");
                s.expect_refused(|s| {
                    s.play(VITAL, json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] }))
                });
            }

            #[test]
            fn sets_the_enemy_hero_s_health_down_to_13_from_above() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "library": [FILLER], "health": 30 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                s.expect_health(P1, 30);
                s.expect_in_zone(VITAL, "graveyard");
            }

            #[test]
            fn sets_your_own_hero_s_health_up_to_13_from_below_not_a_heal() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER], "health": 4 }, "p2": { "hand": [FILLER], "library": [FILLER] } }));
                play_at(&mut s, P1);
                s.expect_health(P1, 13);
                assert!(events_of(&s, GameEventType::Healed).is_empty());
            }

            #[test]
            fn a_hero_above_its_starting_health_comes_down_to_13_too() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "library": [FILLER], "health": 45 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
            }

            #[test]
            fn a_hero_at_13_stays_at_13() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "library": [FILLER], "health": 13 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert_eq!(events_of(&s, GameEventType::HealthSet).len(), 1);
            }

            #[test]
            fn e7_not_damage_its_armor_is_left_as_it_was_and_no_damage_event_is_made() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VITAL, FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER], "health": 30, "armor": 5 },
                }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert_eq!(hero_of(s.state(), P2).armor, 5);
                assert!(events_of(&s, GameEventType::Damage).is_empty());
                assert!(events_of(&s, GameEventType::Healed).is_empty());
                assert!(events_of(&s, GameEventType::Redirected).is_empty());
            }

            #[test]
            fn e7_nothing_that_answers_a_hit_or_a_heal_sees_it_a_set_final_gambit_stays_set_fed_fauci_gets_no_token() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VITAL, FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER], "field": [FAUCI], "backrow": [{ "def": GAMBIT, "faceUp": false }], "health": 30 },
                }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                let gambit = s.backrow(P2, 1);
                assert_eq!(gambit.as_ref().map(|card| card.def_id.clone()), Some(GAMBIT.to_string()));
                assert_eq!(gambit.as_ref().and_then(|card| card.face_up), Some(false));
                assert!(events_of(&s, GameEventType::TrapFired).is_empty());
                let fauci = s.unit(P2, 1);
                assert_eq!(fauci.and_then(|card| card.counters.plague).unwrap_or(0), 0);
            }

            #[test]
            fn e7_a_hit_cap_never_sees_it_anti_oneshot_armor_s_5_does_not_stop_30_13() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VITAL, FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER], "backrow": [ANTI_ONESHOT], "health": 30 },
                }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert!(events_of(&s, GameEventType::Damage).is_empty());
            }

            #[test]
            fn r97_healthset_is_public_both_players_read_it_naming_the_book_as_its_source() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "library": [FILLER] } }));
                let book = s.card(VITAL).clone();
                play_at(&mut s, P2);
                let expected = json!([{ "type": "healthSet", "player": "p2", "health": 13, "sourceId": book.id }]);
                for viewer in [P1, P2] {
                    let view = s.view(viewer);
                    let seen: Vec<&GameEvent> = view
                        .events
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::HealthSet)
                        .collect();
                    assert_eq!(serde_json::to_value(&seen).expect("events serialise"), expected);
                }
            }

            #[test]
            fn the_base_face_adds_nothing_to_the_hand() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "library": [FILLER] } }));
                play_at(&mut s, P2);
                assert_eq!(defs_of(&s.hand(P1)), vec![FILLER]);
            }

            #[test]
            fn then_ends_the_turn_so_the_opponent_is_up() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "library": [FILLER], "health": 30 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert_eq!(events_of(&s, GameEventType::TurnEnded).len(), 1);
                assert_eq!(s.state().active, P2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn sets_the_chosen_hero_s_health_to_13_adds_nothing_and_leaves_the_turn_going() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VITAL, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert_eq!(defs_of(&s.hand(P1)), vec![FILLER]);
                assert!(events_of(&s, GameEventType::TurnEnded).is_empty());
                assert_eq!(s.state().active, P1);
            }

            #[test]
            fn sets_your_own_hero_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VITAL, "radiant": true }, FILLER], "health": 2 },
                    "p2": { "hand": [FILLER] },
                }));
                play_at(&mut s, P1);
                s.expect_health(P1, 13);
                assert_eq!(s.state().active, P1);
            }
        }
    }
}
