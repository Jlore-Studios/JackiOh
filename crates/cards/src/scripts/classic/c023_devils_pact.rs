//! C #23 Devil's Pact (SPEC §8.6 row 23). Field Spell, cost 2, Rare.
//!   Base:    "Cry: Discard {discards|card|cards}.
//!             Activate: This turn, each card you play is replaced by a Book of Flame."
//!   Radiant: the same with a Radiant Book of Flame. Discards: 666 base (the whole hand, the joke), 6 Radiant.
//!
//! The Cry discards {discards} random cards (R682); a hand holding that many or fewer all goes, a
//! discard all the same, so C #64 sees each card. Activate (R384), once per turn, installs a this-turn
//! player modifier (`replacePlays`, R449): at §10.5 step 3 each card you play, a cast included (R70),
//! is replaced by a new C #16 Book of Flame that resolves as that play, counts as one and asks its target
//! then, since the old card's choices were the old card's. The old card ceases to exist (R35), unread by
//! the opponent when it left a hand (R177); the price paid was the old card's; a Unit or trap replaced
//! this way takes no zone. Activating is not a play.

use jackioh_engine::effects::{add_player_modifier, discard_hand, discard_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-023";

/// C #16, the Book of Flame this card names (its `refs`).
const BOOK_OF_FLAME: &str = "classic-016";

/// "Discard {discards} cards": that many at random, or the whole hand when it holds no more.
fn discard_cards(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let count = param(&*ctx, "discards");
    let held = zone_count(&*ctx.state, ctx.controller, OffFieldZone::Hand);
    if held == 0 {
        return vec![];
    }
    if held <= count {
        return vec![discard_hand(json_as(json!({ "player": "self" })))];
    }
    vec![discard_random(json_as(json!({ "count": count })))]
}

fn pact(radiant: bool) -> Script {
    Script {
        cry: Some(hook(discard_cards)),
        activations: vec![ActivationDecl {
            id: "pact".to_string(),
            label: if radiant {
                "This turn, each card you play is replaced by a Radiant Book of Flame".to_string()
            } else {
                "This turn, each card you play is replaced by a Book of Flame".to_string()
            },
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(move |ctx| {
                vec![add_player_modifier(json_as(json!({
                    "mod": {
                        "kind": "replacePlays",
                        "defId": BOOK_OF_FLAME,
                        "radiant": radiant,
                        "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                    },
                })))]
            }),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pact(false),
        radiant: pact(true),
    }
}

// C #23 Devil's Pact — SPEC §8.6 row 23, BUILD M9 Classic row C 23: a play from the graveyard (R454)
// is replaced too; the modifier expires at cleanup; the opponent's view never names a replaced card
// (R177); its tuned number (discards) reads through `param()` (R386).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PACT: &str = "classic-023";
    const BOOK: &str = "classic-016"; // Book of Flame: Deal 4 damage (Radiant 8).
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const TIMMY: &str = "core-011"; // (1) Unit 3/3.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal 2.
    const BEAR: &str = "core-060"; // (1) Trap.
    const PANTHER: &str = "core-032"; // Unit 5/4 Rush: after it attacks and survives, draw 2 per Unit it destroyed.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw.
    const FILLER: &str = "core-010"; // (0) Spell Rapid Replenish.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const WIND: &str = "classic-028"; // Second Wind: Radiant Aura lets you play cards costing (1) or more from your graveyard.

    const EIGHT: [&str; 8] = [VANILLA, VANILLA, TIMMY, TIMMY, STOCKPILE, STOCKPILE, FILLER, BEAR];

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

    fn on_field(p1: Value, radiant_face: bool, p2: Value) -> Scenario {
        scenario(json!({
            "p1": spread(
                json!({
                    "hand": [VANILLA, STOCKPILE, FILLER],
                    "backrow": [{ "def": PACT, "radiant": radiant_face, "lane": 1 }],
                    "library": [TIMMY, TIMMY, TIMMY],
                }),
                p1,
            ),
            "p2": spread(json!({ "hand": [FILLER], "library": [FILLER, FILLER] }), p2),
        }))
    }

    fn count(events: &[GameEvent], kind: GameEventType) -> usize {
        events.iter().filter(|event| event.event_type() == kind).count()
    }

    fn answer_at_p2(state: &GameState, nonce: &str) -> Action {
        Action::new(
            ActionBody::Answer {
                choice_id: state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default(),
                selection: json_as(at_p2()),
            },
            P1,
            nonce,
        )
    }

    /// The `play` action `legalActions` offers for this instance.
    fn play_of(s: &Scenario, instance_id: &str) -> Option<ActionBody> {
        legal_actions(s.state(), P1)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
    }

    fn radiant_pact_and_eight() -> Value {
        let mut hand = vec![json!({ "def": PACT, "radiant": true })];
        hand.extend(EIGHT.iter().map(|id| json!(id)));
        Value::Array(hand)
    }

    fn pending_player(s: &Scenario) -> Option<PlayerId> {
        s.state().pending.as_ref().map(|pending| pending.player_id)
    }

    mod c_n23_devil_s_pact {
        use super::*;

        #[test]
        fn declares_a_cry_and_one_activate_once_per_turn_on_each_face() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.id, PACT);
            assert!(def.refs.unwrap_or_default().contains(&BOOK.to_string()));
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert!(face.cry.is_some());
                assert_eq!(face.activations[0].uses, ActivationUses::Count(1));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn cry_discard_666_cards_the_whole_hand_with_no_prompt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PACT, VANILLA, TIMMY, STOCKPILE], "library": [TIMMY] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(PACT, json!({ "zone": 1 }));

                assert_eq!(s.hand(P1).len(), 0);
                assert!(s.state().pending.is_none());
                assert_eq!(count(s.last_events(), GameEventType::Discarded), 3);
            }

            #[test]
            fn cry_with_an_empty_hand_discards_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PACT], "field": [VANILLA] }, "p2": { "hand": [FILLER] } }));

                s.play(PACT, json!({ "zone": 1 }));

                assert_eq!(count(s.last_events(), GameEventType::Discarded), 0);
                s.expect_in_zone(PACT, "field");
            }

            #[test]
            fn r449_after_the_activation_a_played_card_is_replaced_by_a_book_of_flame_which_asks_its_target_then() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));
                let vanilla = s.card(VANILLA).clone();

                s.activate(PACT, json!({}));
                s.play(&vanilla, json!({ "zone": 2 }));

                assert_eq!(pending_player(&s), Some(P1));
                s.answer(at_p2());

                s.expect_in_zone(&vanilla, "gone");
                assert!(s.unit(P1, 2).is_none());
                s.expect_health(P2, 26);
                s.expect_events(json!(["activated", "transformed", "cardAnnounced", "cardPlayed", "damage"]));
            }

            #[test]
            fn s9_3_the_replaced_play_paused_on_the_book_of_flame_s_target_survives_a_json_round_trip() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));
                s.activate(PACT, json!({}));
                s.play(VANILLA, json!({ "zone": 2 }));
                let paused = s.state().clone();
                let revived: GameState = serde_json::from_value(serde_json::to_value(&paused).unwrap()).unwrap();
                assert_eq!(revived, paused);

                let result = reduce(&revived, &answer_at_p2(&revived, "pact-book-round-trip"));

                assert!(result.error.is_none());
                assert!(result.state.pending.is_none());
                assert!(result.state.work.is_empty());
                assert_eq!(hero_of(&result.state, P2).health, 26);
            }

            #[test]
            fn r449_the_price_paid_is_the_old_card_s_not_the_book_s_a_3_cost_unit_replaced_costs_3() {
                crate::register_all();
                let mut s = on_field(json!({ "hand": [MENACE, FILLER] }), false, json!({}));

                s.activate(PACT, json!({}));
                s.play(MENACE, json!({ "zone": 2 })).answer(at_p2());

                match s.events().iter().find(|event| event.event_type() == GameEventType::CardPlayed) {
                    Some(GameEvent::CardPlayed { def_id, cost_paid, .. }) => {
                        assert_eq!(def_id, BOOK);
                        assert_eq!(*cost_paid, 3);
                    }
                    other => panic!("no cardPlayed: {other:?}"),
                }
                s.expect_mana(P1, 1);
                s.expect_health(P2, 26);
            }

            #[test]
            fn r449_the_replacement_is_played_as_a_book_of_flame_counted_as_one_play_the_price_was_the_old_card_s() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));

                s.activate(PACT, json!({}));
                s.play(VANILLA, json!({ "zone": 2 })).answer(at_p2());

                let played: Vec<&GameEvent> = s
                    .events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::CardPlayed)
                    .collect();
                assert_eq!(played.len(), 1);
                match played[0] {
                    GameEvent::CardPlayed { def_id, cost_paid, .. } => {
                        assert_eq!(def_id, BOOK);
                        assert_eq!(*cost_paid, 1);
                    }
                    other => panic!("not a cardPlayed: {other:?}"),
                }
                assert_eq!(s.state().players[P1].turn_log.cards_played, 1);
                s.expect_mana(P1, 3);
                let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
                assert!(graveyard.contains(&BOOK.to_string()));
            }

            #[test]
            fn r449_a_trap_replaced_this_way_takes_no_zone() {
                crate::register_all();
                let mut s = on_field(json!({ "hand": [BEAR, FILLER] }), false, json!({}));

                s.activate(PACT, json!({}));
                s.play(BEAR, json!({ "zone": 3 })).answer(at_p2());

                assert!(s.backrow(P1, 3).is_none());
                s.expect_health(P2, 26);
            }

            #[test]
            fn r70_a_cast_is_replaced_too_a_card_cast_on_draw_becomes_a_book_of_flame() {
                crate::register_all();
                let mut s = on_field(
                    json!({
                        "hand": [FILLER],
                        "field": [PANTHER],
                        "library": [{ "def": HINDER, "radiant": true }, TIMMY, TIMMY],
                    }),
                    false,
                    json!({ "field": [TIMMY] }),
                );
                let hinder = s.card(HINDER).clone();
                let prey = s.unit(P2, 1).expect("fixture");

                s.activate(PACT, json!({}));
                // Prem Panther kills the Timmy and survives, so it draws 2: the first draw finds Hinder, cast.
                s.attack(PANTHER, &prey);
                assert_eq!(pending_player(&s), Some(P1));
                s.answer(at_p2());

                s.expect_in_zone(&hinder, "gone");
                s.expect_health(P2, 26);
                assert_eq!(s.state().players[P2].mana.next_turn_mod, 0);
            }

            #[test]
            fn r449_r454_a_play_from_the_graveyard_is_replaced_too_it_plays_as_from_hand_so_its_card_becomes_a_book_of_flame() {
                crate::register_all();
                let mut s = on_field(
                    json!({
                        "hand": [FILLER],
                        "backrow": [{ "def": PACT, "lane": 1 }, { "def": WIND, "radiant": true, "lane": 2 }],
                        "graveyard": [VANILLA],
                    }),
                    false,
                    json!({}),
                );
                let vanilla = s.card(VANILLA).clone();
                s.activate(PACT, json!({}));
                let play = play_of(&s, &vanilla.id).expect("Second Wind offers no graveyard play");

                let played = reduce(s.state(), &Action::new(play, P1, "pact-graveyard-play"));
                assert!(played.error.is_none());
                assert!(played.events.iter().any(|event| event.event_type() == GameEventType::Transformed));
                assert_eq!(played.state.pending.as_ref().map(|pending| pending.player_id), Some(P1));
                let answered = reduce(&played.state, &answer_at_p2(&played.state, "pact-graveyard-answer"));

                assert!(answered.error.is_none());
                // R35: the graveyard card ceased to exist.
                assert!(find_instance(&answered.state, &vanilla.id).is_none());
                assert!(
                    !played
                        .events
                        .iter()
                        .chain(answered.events.iter())
                        .any(|event| event.event_type() == GameEventType::Summoned)
                );
                assert_eq!(hero_of(&answered.state, P2).health, 26);
                match answered.events.iter().find(|event| event.event_type() == GameEventType::CardPlayed) {
                    Some(GameEvent::CardPlayed { def_id, cost_paid, .. }) => {
                        assert_eq!(def_id, BOOK);
                        assert_eq!(*cost_paid, 1);
                    }
                    other => panic!("no cardPlayed: {other:?}"),
                }
                let graveyard: Vec<String> = zone_cards(&answered.state, P1, OffFieldZone::Graveyard)
                    .into_iter()
                    .map(|card| card.def_id)
                    .collect();
                assert_eq!(graveyard, vec![BOOK]);
            }

            #[test]
            fn r449_r454_two_live_pacts_replace_a_graveyard_play_in_turn_the_radiant_one_s_book_of_flame_resolves() {
                crate::register_all();
                let mut s = on_field(
                    json!({
                        "hand": [FILLER],
                        "backrow": [
                            { "def": PACT, "lane": 1 },
                            { "def": PACT, "radiant": true, "lane": 2 },
                            { "def": WIND, "radiant": true, "lane": 3 },
                        ],
                        "graveyard": [VANILLA],
                    }),
                    false,
                    json!({}),
                );
                let pacts: Vec<CardInstance> = s.state().players[P1]
                    .backrow
                    .iter()
                    .flatten()
                    .filter(|card| card.def_id == PACT)
                    .cloned()
                    .collect();
                let (Some(first), Some(second)) = (pacts.first().cloned(), pacts.get(1).cloned()) else {
                    panic!("fixture");
                };
                s.activate(&first, json!({})).activate(&second, json!({}));
                let vanilla = s.card(VANILLA).clone();
                let play = play_of(&s, &vanilla.id).expect("Second Wind offers no graveyard play");

                let played = reduce(s.state(), &Action::new(play, P1, "pacts-graveyard-play"));
                assert_eq!(count(&played.events, GameEventType::Transformed), 2);
                let answered = reduce(&played.state, &answer_at_p2(&played.state, "pacts-graveyard-answer"));

                assert!(answered.error.is_none());
                assert_eq!(hero_of(&answered.state, P2).health, 22);
                // The Book came from no graveyard: its play does not say it did.
                match answered.events.iter().find(|event| event.event_type() == GameEventType::CardPlayed) {
                    Some(GameEvent::CardPlayed { from, .. }) => assert!(from.is_none()),
                    other => panic!("no cardPlayed: {other:?}"),
                }
            }

            #[test]
            fn a_card_played_before_the_activation_is_not_replaced() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));

                s.play(VANILLA, json!({ "zone": 2 }));
                s.activate(PACT, json!({}));

                s.expect_in_zone(VANILLA, "field");
                assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(VANILLA.to_string()));
            }

            #[test]
            fn the_modifier_expires_at_cleanup_next_turn_a_play_is_itself_again() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));

                s.activate(PACT, json!({}));
                s.end_turn().end_turn();
                assert_eq!(s.state().active, P1);
                s.play(VANILLA, json!({ "zone": 2 }));

                assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(VANILLA.to_string()));
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r384_activating_is_not_a_play_and_is_once_per_turn() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));
                let played = s.state().players[P1].turn_log.cards_played;

                s.activate(PACT, json!({}));

                assert_eq!(s.state().players[P1].turn_log.cards_played, played);
                assert_eq!(count(s.events(), GameEventType::CardPlayed), 0);
                s.expect_refused(|s| s.activate(PACT, json!({})));
            }

            #[test]
            fn r177_the_opponent_s_view_never_names_a_card_replaced_out_of_your_hand() {
                crate::register_all();
                let mut s = on_field(json!({}), false, json!({}));

                s.activate(PACT, json!({}));
                s.play(VANILLA, json!({ "zone": 2 })).answer(at_p2());

                assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(VANILLA));
            }

            #[test]
            fn r386_a_degrade_of_its_discards_stays_the_whole_hand_an_upgrade_499_is_still_more_than_any_hand() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PACT, VANILLA, TIMMY], "library": [TIMMY] }, "p2": { "hand": [FILLER] } }));
                step_param(s.card_mut(PACT), "discards", -1);

                s.play(PACT, json!({ "zone": 1 }));

                assert_eq!(s.hand(P1).len(), 0);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r682_cry_discard_6_cards_at_random_with_no_prompt() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_pact_and_eight() }, "p2": { "hand": [FILLER] } }));

                s.play(PACT, json!({ "zone": 1 }));

                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 2);
                assert_eq!(count(s.last_events(), GameEventType::Discarded), 6);
            }

            #[test]
            fn r682_the_random_discards_come_from_the_match_rng_the_same_game_discards_the_same_cards() {
                crate::register_all();
                let mk = || scenario(json!({ "p1": { "hand": radiant_pact_and_eight() }, "p2": { "hand": [FILLER] } }));
                let mut first = mk();
                first.play(PACT, json!({ "zone": 1 }));
                let mut second = mk();
                second.play(PACT, json!({ "zone": 1 }));
                let ids = |s: &Scenario| -> Vec<String> {
                    s.last_events()
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
            fn r682_with_6_or_fewer_in_hand_it_discards_all_of_them_asking_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PACT, "radiant": true }, VANILLA, TIMMY] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(PACT, json!({ "zone": 1 }));

                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 0);
            }

            #[test]
            fn each_replacement_is_a_radiant_book_of_flame_8_damage() {
                crate::register_all();
                let mut s = on_field(json!({}), true, json!({}));

                s.activate(PACT, json!({}));
                s.play(VANILLA, json!({ "zone": 2 })).answer(at_p2());

                s.expect_health(P2, 22);
                match s.events().iter().find(|event| event.event_type() == GameEventType::CardPlayed) {
                    Some(GameEvent::CardPlayed { def_id, .. }) => assert_eq!(def_id, BOOK),
                    other => panic!("no cardPlayed: {other:?}"),
                }
            }

            #[test]
            fn r386_an_upgrade_of_its_discards_takes_one_step_of_167_the_base_face_s_so_the_radiant_6_goes_to_its_floor_of_1() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_pact_and_eight() }, "p2": { "hand": [FILLER] } }));
                step_param(s.card_mut(PACT), "discards", -1);

                s.play(PACT, json!({ "zone": 1 }));

                assert!(s.state().pending.is_none());
                assert_eq!(count(s.last_events(), GameEventType::Discarded), 1);
                assert_eq!(s.hand(P1).len(), 7);
            }
        }
    }
}
