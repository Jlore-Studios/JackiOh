//! C #29 Book of Vital Kill (SPEC §8.6 row 29, §6.3 Set health; R18, R81, R317). Spell, Book, cost 1,
//! Epic.
//!   Base:    "Set a hero's health to 13."
//!   Radiant: "Set a hero's health to 13. Add a Book of Flame to your hand."
//!   Engine:  "Set health (§6.3) on either hero, a declared target (R81): no pipeline, not damage and
//!            not a heal, like R18's lose health. Radiant: adds C #16's base face (a card named without
//!            "Radiant" is its base face); the hand cap applies. Tunes: none."
//!
//! THE TARGET is declared (R81): one hero, either side, so it travels in the play and `legalActions`
//! offers both heroes. `setHealth` writes the number and emits `healthSet`; it runs no §4.4 step, so
//! Armor, a hit cap, Lifesteal, a lethal-hit replacement and every "takes damage" or "is healed"
//! trigger never see it, and the hero's Armor is left as it was.
//!
//! 13 is the card's own printed number, and the entry declares no `params` for it (a Degrade has
//! nothing to move: 13 is good or bad depending on whose hero it is), so it is written here.
//!
//! THE RADIANT BOOK OF FLAME is C #16 named without "Radiant", so its base face: `addToHand` makes a
//! fresh one (R57's radiant flag left off) through §2.4's pipeline, where a full hand burns it (R4,
//! R317). Once in the hand it follows R97 like any hand card.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-029";

/// The printed health the Book sets a hero to.
const VITAL_HEALTH: i32 = 13;

/// C #16 Book of Flame, which the Radiant face adds on its base face.
const BOOK_OF_FLAME: &str = "classic-016";

/// `setHealth({ to: { of: "chosen" }, value: VITAL_HEALTH })`.
fn set_vital_health() -> Effect {
    set_health(json_as(json!({ "to": { "of": "chosen" }, "value": VITAL_HEALTH })))
}

pub fn script() -> CardScripts {
    let targets = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["hero"] }))];

    let base = Script {
        targets: targets.clone(),
        cry: Some(hook(|_ctx| vec![set_vital_health()])),
        ..Script::default()
    };

    let radiant = Script {
        targets,
        cry: Some(hook(|_ctx| {
            vec![
                set_vital_health(),
                add_to_hand(json_as(json!({ "defId": BOOK_OF_FLAME }))),
            ]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #29 Book of Vital Kill — SPEC §8.6 row 29, BUILD M9 Classic row C 29: "A declared hero target,
// either side; its health becomes 13 from above or below, its Armor unchanged: not damage and not a
// heal, so Armor, hit caps, C #75, Lifesteal, C #52's lethal window and Fed Fauci's tokens never see
// it; a hero at 13 stays; `healthSet` is public; radiant: also add a Book of Flame (C #16, base face)
// to your hand, burned at a full hand (R317) and never named in the opponent's view once there (R97);
// no tuned numbers".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VITAL: &str = "classic-029";
    const FLAME: &str = "classic-016"; // C #16 Book of Flame, (1) Spell: "Deal {damage} damage."
    const GAMBIT: &str = "classic-052"; // C #52 Final Gambit, (2) Trap: a hit that would bring your hero to 0 or less.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell: "Your next Spell gains Echo +1."
    const FAUCI: &str = "core-091"; // (2) Unit: "Whenever this takes damage, it gets a Plague Counter."
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text.
    const ANTI_ONESHOT: &str = "core-073"; // (2) Field Spell: "Your hero can't take more than 5 damage at once."
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

        #[test]
        fn declares_one_hero_target_on_either_side_and_13_is_the_card_s_own_number_no_params() {
            crate::register_all();
            let def = crate::card_def(VITAL);
            assert_eq!(def.id, VITAL);
            assert!(def.params.is_none());
            let scripts = script();
            assert_eq!(
                serde_json::to_value(&scripts.base.targets).expect("declarations serialise"),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["hero"] } }])
            );
            assert_eq!(scripts.radiant.targets, scripts.base.targets);
        }

        mod base {
            use super::*;

            #[test]
            fn r81_legalactions_offers_the_play_at_each_hero_and_at_no_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VITAL, FILLER], "field": [VANILLA] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
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
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "health": 30 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                s.expect_health(P1, 30);
                s.expect_in_zone(VITAL, "graveyard");
            }

            #[test]
            fn sets_your_own_hero_s_health_up_to_13_from_below_not_a_heal() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER], "health": 4 }, "p2": { "hand": [FILLER] } }));
                play_at(&mut s, P1);
                s.expect_health(P1, 13);
                assert!(events_of(&s, GameEventType::Healed).is_empty());
            }

            #[test]
            fn a_hero_above_its_starting_health_comes_down_to_13_too() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "health": 45 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
            }

            #[test]
            fn a_hero_at_13_stays_at_13() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER], "health": 13 } }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert_eq!(events_of(&s, GameEventType::HealthSet).len(), 1);
            }

            #[test]
            fn e7_not_damage_its_armor_is_left_as_it_was_and_no_damage_event_is_made() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VITAL, FILLER] },
                    "p2": { "hand": [FILLER], "health": 30, "armor": 5 },
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
                    "p2": { "hand": [FILLER], "field": [FAUCI], "backrow": [{ "def": GAMBIT, "faceUp": false }], "health": 30 },
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
                    "p2": { "hand": [FILLER], "backrow": [ANTI_ONESHOT], "health": 30 },
                }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                assert!(events_of(&s, GameEventType::Damage).is_empty());
            }

            #[test]
            fn r97_healthset_is_public_both_players_read_it_naming_the_book_as_its_source() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER] } }));
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
                let mut s = scenario(json!({ "p1": { "hand": [VITAL, FILLER] }, "p2": { "hand": [FILLER] } }));
                play_at(&mut s, P2);
                assert_eq!(defs_of(&s.hand(P1)), vec![FILLER]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn sets_the_chosen_hero_s_health_to_13_and_adds_a_book_of_flame_on_its_base_face_to_your_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VITAL, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                play_at(&mut s, P2);
                s.expect_health(P2, 13);
                let flame = s.hand(P1).into_iter().find(|card| card.def_id == FLAME);
                assert!(flame.is_some());
                assert_eq!(flame.as_ref().map(|card| card.radiant), Some(false));
                assert_eq!(flame.as_ref().map(|card| card.owner), Some(P1));
            }

            #[test]
            fn sets_your_own_hero_too_and_still_adds_the_book_of_flame() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VITAL, "radiant": true }, FILLER], "health": 2 },
                    "p2": { "hand": [FILLER] },
                }));
                play_at(&mut s, P1);
                s.expect_health(P1, 13);
                assert_eq!(defs_of(&s.hand(P1)), vec![FILLER, FLAME]);
            }

            #[test]
            fn r97_the_opponent_s_view_never_names_the_book_of_flame_once_it_is_in_your_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VITAL, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                play_at(&mut s, P2);
                let flame = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == FLAME)
                    .expect("the Book of Flame should be in p1's hand");
                let theirs = serde_json::to_string(&s.view(P2)).expect("views serialise");
                assert!(!theirs.contains(&format!("\"{}\"", flame.id)));
                assert!(!theirs.contains(FLAME));
                // Its owner reads it.
                let mine = serde_json::to_string(&s.view(P1)).expect("views serialise");
                assert!(mine.contains(&format!("\"{}\"", flame.id)));
            }

            #[test]
            fn r317_a_full_hand_burns_the_book_of_flame_into_your_graveyard_and_both_players_read_which() {
                crate::register_all();
                // A Radiant Twinspell's Echo +2 resolves the Book three times: 8 cards left in hand, then three
                // Flames — the first two fill the hand to 10 and the third burns.
                let mut hand = vec![
                    json!({ "def": TWINSPELL, "radiant": true }),
                    json!({ "def": VITAL, "radiant": true }),
                ];
                hand.extend((0..8).map(|_| json!(FILLER)));
                let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));
                s.play(TWINSPELL, json!({}));
                play_at(&mut s, P2);
                // §6.2 Echo: each repeat asks its target afresh.
                for _repeat in 0..2 {
                    assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
                    s.answer(json!([{ "pick": "hero", "player": "p2" }]));
                }
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == FLAME).count(), 2);
                let burned = events_of(&s, GameEventType::Burned);
                assert_eq!(burned.len(), 1);
                let burned_flame = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == FLAME);
                assert!(burned_flame.is_some());
                for viewer in [P1, P2] {
                    let view = s.view(viewer);
                    let seen: Vec<&GameEvent> = view
                        .events
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::Burned)
                        .collect();
                    assert_eq!(seen.len(), 1);
                    assert!(serde_json::to_string(&seen).expect("events serialise").contains(FLAME));
                }
            }
        }
    }
}
