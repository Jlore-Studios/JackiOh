//! C #20 The Power to Punish (SPEC §8.6 row 20). Field Spell, cost 2, Rare.
//!   Base:    "Activate: Choose one: Deal {damage} damage; your opponent discards {discards|card|cards};
//!             or choose a Unit, which is destroyed at the start of your next turn."
//!   Radiant: "Activate: Choose one: Deal {damage} damage; your opponent discards {discards|card|cards};
//!             or all enemy Units are destroyed at the start of your next turn."
//!
//! Activate (R384): once per turn, and not a play. The mode and its targets travel in the `activate`
//! action (R81), each target bound to its mode (`forModes`, R90). The discard is random (R682), so no
//! prompt opens. The delayed destroy (§10.1, `destroyAtNextTurnStart`) is keyed to the Unit's stay: it
//! fizzles if the Unit left, even if it came back (R174). The Radiant face takes every enemy Unit on the
//! field when it resolves. Either face resolves with the start-of-turn effects (R62, R68) even if this card
//! left (as R76), and as a destroy it spares an Indestructible Unit (R46). Units wear the red mark while it
//! waits (R437; Radiant, every enemy Unit, R750).

use jackioh_engine::effects::{damage, destroy_at_next_turn_start, discard_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-020";

const DAMAGE_MODE: &str = "deal damage";
const DISCARD_MODE: &str = "opponent discards";
const DOOM_MODE: &str = "destroy a Unit at the start of your next turn";
const DOOM_ALL_MODE: &str = "destroy all enemy Units at the start of your next turn";

fn damage_target() -> TargetDecl {
    json_as(json!({
        "kind": "target",
        "min": 1,
        "max": 1,
        "filter": { "of": ["unit", "hero"] },
        "forModes": [DAMAGE_MODE],
    }))
}

fn doom_target() -> TargetDecl {
    json_as(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "of": ["unit"] }, "forModes": [DOOM_MODE] }))
}

fn punish(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    match ctx.modes.first().map(String::as_str) {
        Some(DAMAGE_MODE) => vec![damage(json_as(
            json!({ "to": { "of": "chosen" }, "amount": param(ctx, "damage") }),
        ))],
        Some(DISCARD_MODE) => vec![discard_random(json_as(
            json!({ "count": param(ctx, "discards"), "player": "enemy" }),
        ))],
        Some(DOOM_MODE) => {
            // The unit marked for death wears #50 K-Pop Fanatic's aura, in red (R437).
            if radiant {
                vec![]
            } else {
                vec![destroy_at_next_turn_start(json_as(json!({
                    "target": { "of": "chosen" },
                    "mark": { "mark": "destroy", "color": "red" },
                })))]
            }
        }
        Some(DOOM_ALL_MODE) => {
            if radiant {
                vec![destroy_at_next_turn_start(json_as(json!({
                    "scope": { "side": "enemy" },
                    "mark": { "mark": "destroy", "color": "red" },
                })))]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

fn face(radiant: bool) -> Script {
    let ability = ActivationDecl {
        id: "punish".to_string(),
        label: "Choose one".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: if radiant {
            vec![damage_target()]
        } else {
            vec![damage_target(), doom_target()]
        },
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![
                DAMAGE_MODE.to_string(),
                DISCARD_MODE.to_string(),
                (if radiant { DOOM_ALL_MODE } else { DOOM_MODE }).to_string(),
            ],
        }],
        can_activate: None,
        has: None,
        run: hook(move |ctx| punish(ctx, radiant)),
    };
    Script {
        activations: vec![ability],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// BUILD M9 Classic row C 20: 2 damage; the opponent discards 1 at random (an empty hand: nothing); or a
// Unit, either side, is destroyed at the start of your next turn; radiant: 4 damage; discards 2; or every
// enemy Unit there then. Tuned numbers (damage, discards) read through `param()` (R386).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PUNISH: &str = "classic-020";
    const DAMAGE: &str = "deal damage";
    const DISCARD: &str = "opponent discards";
    const DOOM: &str = "destroy a Unit at the start of your next turn";
    const DOOM_ALL: &str = "destroy all enemy Units at the start of your next turn";

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: draw 2, heal 2.
    const VANILLA: &str = "core-008"; // 4/4.
    const TIMMY: &str = "core-011"; // (1) 3/3.
    const MENACE: &str = "core-019"; // 9/9.
    const POINTMASTER: &str = "core-020"; // 7/1.
    const ROCK: &str = "core-066"; // 10/10 Indestructible.
    const BIG: &str = "core-025"; // 7/7 Armor 7; Radiant adds Reborn.
    const CUBE: &str = "core-022"; // (3) Cry: Tribute one of your other Units.
    const COLLATERAL: &str = "core-034"; // (4) Exile target permanent and a random card from their deck.

    fn at_p2() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    /// The side's own keys over the defaults.
    fn spread(mut defaults: Value, side: Value) -> Value {
        if let (Some(into), Some(from)) = (defaults.as_object_mut(), side.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        defaults
    }

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": spread(
                json!({
                    "hand": [FILLER, FILLER],
                    "library": [FILLER, FILLER, FILLER],
                    "backrow": [{ "def": PUNISH, "radiant": radiant_face }],
                }),
                p1,
            ),
            "p2": spread(json!({ "hand": [FILLER, FILLER], "library": [FILLER, FILLER, FILLER] }), p2),
        }))
    }

    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// Activate the card in that mode, with those targets.
    fn activate_punish<'a>(s: &'a mut Scenario, mode: &str, targets: Option<Value>) -> &'a mut Scenario {
        let mut options = json!({ "modes": [mode] });
        if let Some(targets) = targets {
            options["targets"] = targets;
        }
        s.activate(PUNISH, options)
    }

    /// p1 ends the turn, p2 takes theirs and ends it, and p1's next turn starts.
    fn to_next_turn(s: &mut Scenario) -> &mut Scenario {
        s.end_turn();
        assert_eq!(s.state().active, P2);
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s
    }

    fn activations(s: &Scenario, player: PlayerId) -> Vec<ActionBody> {
        let card = s.card(PUNISH).id.clone();
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if *instance_id == card))
            .collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// The marks a seat's view shows on the unit with this id, anywhere on the board, as JSON.
    fn marks_in(s: &Scenario, seat: PlayerId, id: &str) -> Value {
        let view = s.view(seat);
        let card = view
            .you
            .units
            .iter()
            .chain(view.opponent.units.iter())
            .flatten()
            .find(|unit| unit.instance_id == id);
        serde_json::to_value(card.and_then(|unit| unit.marks.clone()).unwrap_or_default()).unwrap()
    }

    fn marked(events: &[GameEvent], was_added: bool) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Marked { instance_id, added, .. } if *added == was_added => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod c_n20_the_power_to_punish {
        use super::*;

        #[test]
        fn r384_declares_one_activate_once_per_turn_with_three_modes_on_each_face() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, PUNISH);
            assert_eq!(scripts.base.activations[0].uses, ActivationUses::Count(1));
            assert_eq!(scripts.base.activations[0].modes[0].options, vec![DAMAGE, DISCARD, DOOM]);
            assert_eq!(scripts.radiant.activations[0].uses, ActivationUses::Count(1));
            assert_eq!(scripts.radiant.activations[0].modes[0].options, vec![DAMAGE, DISCARD, DOOM_ALL]);
        }

        mod base {
            use super::*;

            #[test]
            fn r81_deal_damage_the_mode_and_its_target_travel_in_the_action_2_damage_to_a_hero() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                activate_punish(&mut s, DAMAGE, Some(at_p2()));

                s.expect_health(P2, 28);
                s.expect_events(json!(["activated", "damage"]));
            }

            #[test]
            fn deal_damage_reaches_a_unit_on_either_side_from_this_card() {
                crate::register_all();
                let mut s = setup(json!({ "field": [VANILLA] }), json!({ "field": [POINTMASTER] }), false);
                let point = s.card(POINTMASTER).clone();

                activate_punish(&mut s, DAMAGE, Some(at(&point)));

                s.expect_in_zone(&point, "graveyard");
                let punish_id = s.card(PUNISH).id.clone();
                let hit = s.events().iter().find(|event| event.event_type() == GameEventType::Damage).cloned();
                match hit {
                    Some(GameEvent::Damage { source_id, amount, .. }) => {
                        assert_eq!(source_id, Some(punish_id));
                        assert_eq!(amount, 2);
                    }
                    other => panic!("no damage event: {other:?}"),
                }
            }

            #[test]
            fn r90_the_damage_mode_refuses_an_activation_with_no_target() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                s.expect_refused(|s| activate_punish(s, DAMAGE, None));
                s.expect_health(P2, 30);
            }

            #[test]
            fn r682_the_opponent_discards_a_card_at_random_no_prompt_opens() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [VANILLA, TIMMY, MENACE] }), false);

                activate_punish(&mut s, DISCARD, None);

                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P2).len(), 2);
                let grave = def_ids(&s.pile(P2, "graveyard"));
                assert_eq!(grave.len(), 1);
                assert!([VANILLA, TIMMY, MENACE].contains(&grave[0].as_str()));
                s.expect_events(json!(["activated", "discarded"]));
            }

            #[test]
            fn r177_your_view_names_none_of_the_opponent_s_remaining_hand() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [VANILLA, TIMMY, MENACE] }), false);

                activate_punish(&mut s, DISCARD, None);

                let seen = serde_json::to_string(&s.view(P1)).unwrap();
                for card in s.hand(P2) {
                    assert!(!seen.contains(&card.id));
                }
                // The discard itself is public: the graveyard names it.
                for card in s.pile(P2, "graveyard") {
                    assert!(seen.contains(&card.id));
                }
            }

            #[test]
            fn r682_the_random_discard_comes_from_the_match_rng_the_same_game_discards_the_same_card() {
                crate::register_all();
                let mut first = setup(json!({}), json!({ "hand": [VANILLA, TIMMY, MENACE] }), false);
                activate_punish(&mut first, DISCARD, None);
                let mut second = setup(json!({}), json!({ "hand": [VANILLA, TIMMY, MENACE] }), false);
                activate_punish(&mut second, DISCARD, None);
                let ids = |s: &Scenario| -> Vec<String> {
                    s.events()
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::Discarded { instance_id, .. } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r682_an_opponent_with_an_empty_hand_discards_nothing_and_is_asked_nothing() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [], "field": [VANILLA] }), false);

                activate_punish(&mut s, DISCARD, None);

                assert!(s.state().pending.is_none());
                assert_eq!(s.pile(P2, "graveyard").len(), 0);
            }

            #[test]
            fn r62_the_chosen_enemy_unit_is_destroyed_at_the_start_of_your_next_turn_and_not_before() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [VANILLA] }), false);
                let vanilla = s.card(VANILLA).clone();

                activate_punish(&mut s, DOOM, Some(at(&vanilla)));
                s.end_turn();
                s.expect_in_zone(&vanilla, "field");
                s.end_turn();

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_events(json!(["turnStarted", "destroyed"]));
                // R62: start-of-turn delayed effects come before the turn's draw.
                let types: Vec<GameEventType> = s.last_events().iter().map(|event| event.event_type()).collect();
                let drawn = s
                    .last_events()
                    .iter()
                    .position(|event| matches!(event, GameEvent::Drawn { player, .. } if *player == P1))
                    .map_or(-1, |index| index as i64);
                let destroyed = types
                    .iter()
                    .position(|kind| *kind == GameEventType::Destroyed)
                    .map_or(-1, |index| index as i64);
                assert!(drawn > destroyed);
            }

            #[test]
            fn the_delayed_destroy_may_name_one_of_your_own_units() {
                crate::register_all();
                let mut s = setup(json!({ "field": [VANILLA] }), json!({}), false);
                let vanilla = s.card(VANILLA).clone();

                activate_punish(&mut s, DOOM, Some(at(&vanilla)));
                to_next_turn(&mut s);

                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn r437_the_unit_marked_for_death_wears_n50_s_aura_in_red_in_both_seats_views_until_it_dies() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [VANILLA] }), false);
                let vanilla = s.card(VANILLA).clone();

                activate_punish(&mut s, DOOM, Some(at(&vanilla)));

                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Marked { mark, color, .. } if mark == "destroy" && color == "red"
                )));
                // Vanilla is p2's: the opponent's side from p1's seat, its own side from p2's.
                let red = json!([{ "mark": "destroy", "color": "red" }]);
                let from_p1 = s
                    .view(P1)
                    .opponent
                    .units
                    .into_iter()
                    .flatten()
                    .find(|card| card.instance_id == vanilla.id);
                assert_eq!(serde_json::to_value(from_p1.and_then(|card| card.marks)).unwrap(), red);
                let from_p2 = s
                    .view(P2)
                    .you
                    .units
                    .into_iter()
                    .flatten()
                    .find(|card| card.instance_id == vanilla.id);
                assert_eq!(serde_json::to_value(from_p2.and_then(|card| card.marks)).unwrap(), red);
                to_next_turn(&mut s);
                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn r46_an_indestructible_unit_survives_it() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [ROCK] }), false);
                let rock = s.card(ROCK).clone();

                activate_punish(&mut s, DOOM, Some(at(&rock)));
                to_next_turn(&mut s);

                s.expect_in_zone(&rock, "field");
            }

            #[test]
            fn r174_it_fizzles_when_the_unit_has_left_the_field_even_though_it_came_back_reborn() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [CUBE, FILLER], "field": [{ "def": BIG, "radiant": true }] }), json!({}), false);
                let big = s.card(BIG).clone();

                activate_punish(&mut s, DOOM, Some(at(&big)));
                // The Cube's Cry tributes the 7/7, whose Reborn brings it back: a new stay (R83).
                s.play(CUBE, json!({ "targets": at(&big) }));
                s.expect_in_zone(&big, "field");
                to_next_turn(&mut s);

                s.expect_in_zone(&big, "field");
                s.expect_stats(&big, json!({ "health": 1 }));
            }

            #[test]
            fn r76_it_still_fires_after_the_power_to_punish_has_left_the_field() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [VANILLA], "hand": [COLLATERAL, FILLER] }), false);
                let vanilla = s.card(VANILLA).clone();
                let card = s.card(PUNISH).clone();

                activate_punish(&mut s, DOOM, Some(at(&vanilla)));
                s.end_turn();
                s.play(COLLATERAL, json!({ "targets": at(&card) }));
                s.expect_in_zone(&card, "exile");
                s.end_turn();

                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn r384_once_per_turn_a_second_activation_is_refused_and_not_listed_it_is_back_next_turn() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                activate_punish(&mut s, DAMAGE, Some(at_p2()));
                assert_eq!(activations(&s, P1).len(), 0);
                s.expect_refused(|s| activate_punish(s, DAMAGE, Some(at_p2())));
                s.expect_health(P2, 28);

                to_next_turn(&mut s);
                assert!(!activations(&s, P1).is_empty());
                activate_punish(&mut s, DAMAGE, Some(at_p2()));
                s.expect_health(P2, 26);
            }

            #[test]
            fn r384_only_its_controller_in_their_own_turn_on_the_opponent_s_turn_it_is_not_listed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "backrow": [PUNISH] },
                    "p2": { "hand": [FILLER] },
                }));
                assert_eq!(activations(&s, P1).len(), 0);
                s.expect_refused(|s| activate_punish(s, DAMAGE, Some(at_p2())));
            }

            #[test]
            fn r384_activating_is_not_a_play() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let played = s.state().players[P1].turn_log.cards_played;

                activate_punish(&mut s, DAMAGE, Some(at_p2()));

                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::CardPlayed));
                assert_eq!(s.state().players[P1].turn_log.cards_played, played);
            }

            #[test]
            fn r386_an_upgrade_of_its_damage_deals_3_an_upgrade_of_its_discards_takes_2_cards() {
                crate::register_all();
                let mut hit = setup(json!({}), json!({}), false);
                step_param(hit.card_mut(PUNISH), "damage", 1);
                activate_punish(&mut hit, DAMAGE, Some(at_p2()));
                hit.expect_health(P2, 27);

                let mut two = setup(json!({}), json!({ "hand": [VANILLA, TIMMY, MENACE] }), false);
                step_param(two.card_mut(PUNISH), "discards", 1);
                activate_punish(&mut two, DISCARD, None);
                assert!(two.state().pending.is_none());
                assert_eq!(two.hand(P2).len(), 1);
                assert_eq!(two.pile(P2, "graveyard").len(), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn deals_4() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);

                activate_punish(&mut s, DAMAGE, Some(at_p2()));

                s.expect_health(P2, 26);
            }

            #[test]
            fn r682_the_opponent_discards_2_cards_at_random() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [VANILLA, TIMMY, MENACE] }), true);

                activate_punish(&mut s, DISCARD, None);
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P2).len(), 1);
                assert_eq!(s.pile(P2, "graveyard").len(), 2);
            }

            #[test]
            fn r682_with_one_card_in_hand_the_opponent_discards_that_one() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [TIMMY], "field": [VANILLA] }), true);

                activate_punish(&mut s, DISCARD, None);
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P2).len(), 0);
                assert_eq!(def_ids(&s.pile(P2, "graveyard")), vec![TIMMY]);
            }

            #[test]
            fn the_third_mode_takes_no_target() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [VANILLA] }), true);
                let vanilla = s.card(VANILLA).clone();
                s.expect_refused(|s| activate_punish(s, DOOM_ALL, Some(at(&vanilla))));
            }

            #[test]
            fn destroys_every_enemy_unit_on_the_field_at_the_start_of_your_next_turn_the_units_there_then() {
                crate::register_all();
                let mut s = setup(json!({ "field": [MENACE] }), json!({ "field": [VANILLA], "hand": [TIMMY, FILLER] }), true);
                let vanilla = s.card(VANILLA).clone();
                let menace = s.card(MENACE).clone();

                activate_punish(&mut s, DOOM_ALL, None);
                s.end_turn();
                // A Unit the opponent plays after the activation is there when it resolves, so it goes too.
                s.play(TIMMY, json!({ "zone": 2 }));
                let timmy = s.card(TIMMY).clone();
                s.expect_in_zone(&vanilla, "field");
                s.end_turn();

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_in_zone(&timmy, "graveyard");
                s.expect_in_zone(&menace, "field");
            }

            #[test]
            fn r750_every_enemy_unit_wears_the_red_destroy_mark_while_the_destroy_waits_one_played_later_too_and_the_marks_go_when_it_resolves() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [MENACE], "hand": [COLLATERAL, FILLER], "mana": 10 }),
                    json!({ "field": [VANILLA, POINTMASTER], "hand": [TIMMY, FILLER] }),
                    true,
                );
                let vanilla = s.card(VANILLA).clone();
                let pointmaster = s.card(POINTMASTER).clone();
                let menace = s.card(MENACE).clone();
                let red = json!([{ "mark": "destroy", "color": "red" }]);

                activate_punish(&mut s, DOOM_ALL, None);
                // The enemy Units, in both seats' views; p1's own Menace is not one.
                assert_eq!(marked(s.last_events(), true), vec![vanilla.id.clone(), pointmaster.id.clone()]);
                for seat in [P1, P2] {
                    assert_eq!(marks_in(&s, seat, &vanilla.id), red);
                    assert_eq!(marks_in(&s, seat, &pointmaster.id), red);
                    assert_eq!(marks_in(&s, seat, &menace.id), json!([]));
                }
                // One that leaves the field is not one it will destroy, and loses its mark.
                s.play(COLLATERAL, json!({ "targets": at(&pointmaster) }));
                s.expect_in_zone(&pointmaster, "exile");
                assert_eq!(marked(s.last_events(), false), vec![pointmaster.id.clone()]);
                assert_eq!(marked(s.last_events(), true), Vec::<String>::new());

                s.end_turn();
                // A Unit played while it waits is one it will destroy, so it is marked as it lands.
                s.play(TIMMY, json!({ "zone": 3 }));
                let timmy = s.card(TIMMY).clone();
                assert_eq!(marked(s.last_events(), true), vec![timmy.id.clone()]);
                assert_eq!(marks_in(&s, P1, &timmy.id), red);
                assert_eq!(marks_in(&s, P2, &timmy.id), red);
                s.end_turn();

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_in_zone(&timmy, "graveyard");
                s.expect_in_zone(&menace, "field");
                let mut unmarked = marked(s.last_events(), false);
                unmarked.sort();
                let mut expected = vec![vanilla.id.clone(), timmy.id.clone()];
                expected.sort();
                assert_eq!(unmarked, expected);
                assert!(s.state().marks.is_none());
            }

            #[test]
            fn r46_an_indestructible_enemy_unit_survives_it() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [ROCK, VANILLA] }), true);

                activate_punish(&mut s, DOOM_ALL, None);
                to_next_turn(&mut s);

                s.expect_in_zone(ROCK, "field");
                s.expect_in_zone(VANILLA, "graveyard");
            }

            #[test]
            fn r386_an_upgrade_of_its_damage_deals_5() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                step_param(s.card_mut(PUNISH), "damage", 1);

                activate_punish(&mut s, DAMAGE, Some(at_p2()));

                s.expect_health(P2, 25);
            }
        }
    }
}
