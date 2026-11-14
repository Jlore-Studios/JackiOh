//! effects/perks.rs: C+ #46 Felinor Flagbearer's Armor for the rest of the game and C+ #49 Jay Fungus's
//! discount on a random hand card. Applied straight to fixture cards; the real cards' tests cover the
//! same cases again.

use jackioh_engine::testkit::*;

use jackioh_engine::damage::hero_hit_amount;
use jackioh_engine::effects::player_mods::add_player_modifier;
use jackioh_engine::effects::{discount_random_in_hand, gain_hero_armor};
use jackioh_engine::mana::effective_cost;
use jackioh_engine::script::Effect;
use jackioh_engine::state::{CardInstance, GameState};
use jackioh_engine::testkit::golden::{events_text, view_texts};

use super::fixtures::catalog::{spell_def, unit_def, vanilla_catalog};
use super::fixtures::harness::{by, events_json, in_hand, live, new_game, run_effect, run_on_sink};

fn free() -> CardDef {
    unit_def(801, json!({ "cost": 0 }))
}

fn x_spell() -> CardDef {
    spell_def(802, json!({ "cost": "X" }))
}

fn pricey() -> CardDef {
    unit_def(803, json!({ "cost": 3 }))
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = vanilla_catalog(2 * DECK_SIZE, 1);
    for def in [free(), x_spell(), pricey()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.players.p1.hand = Vec::new();
    state
}

/// One effect applied as p1's, on a sink whose rng starts at the state's cursor; the cursor is
/// written back, and the events it emitted are handed back.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    run_effect(state, effect, PlayerId::P1)
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card was put in hand")
}

mod gain_hero_armor_c_plus_c46 {
    use super::*;

    #[test]
    fn r124_adds_to_the_heros_own_armor_stacks_and_s4_4_step_2_takes_it_off_each_hit_but_a_pierce_one() {
        let mut state = game("perks-armor");
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": 1 }))));
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": 2 }))));
        assert_eq!(state.players.p1.hero.armor, 3);
        assert_eq!(state.players.p2.hero.armor, 0);
        assert_eq!(hero_hit_amount(&state, PlayerId::P1, 5, false), 2);
        assert_eq!(hero_hit_amount(&state, PlayerId::P1, 5, true), 5);
    }

    #[test]
    fn names_the_other_hero_with_player_enemy_and_ignores_a_non_positive_amount() {
        let mut state = game("perks-armor-enemy");
        run(
            &mut state,
            gain_hero_armor(json_as(json!({ "amount": 2, "player": "enemy" }))),
        );
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": 0 }))));
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": -3 }))));
        assert_eq!(state.players.p2.hero.armor, 2);
        assert_eq!(state.players.p1.hero.armor, 0);
    }

    /// R3857000: each gain above 0 reports `armorGained`, its amount and the hero's Armor after it.
    #[test]
    fn r3857000_a_gain_reports_its_amount_and_the_heros_armor_after() {
        let mut state = game("r3857000-gain");
        let events = run(&mut state, gain_hero_armor(json_as(json!({ "amount": 1 }))));
        assert_eq!(
            events_json(&events, GameEventType::ArmorGained),
            vec![json!({ "type": "armorGained", "player": "p1", "amount": 1, "armor": 1 })]
        );
        let events = run(&mut state, gain_hero_armor(json_as(json!({ "amount": 2 }))));
        assert_eq!(
            events_json(&events, GameEventType::ArmorGained),
            vec![json!({ "type": "armorGained", "player": "p1", "amount": 2, "armor": 3 })]
        );
    }

    /// R3857000: a gain of 0 or less changes nothing and reports nothing; `player: enemy` names the
    /// other hero.
    #[test]
    fn r3857000_none_for_zero_or_less_and_player_enemy_names_the_other_hero() {
        let mut state = game("r3857000-none");
        for amount in [0, -3] {
            let events = run(&mut state, gain_hero_armor(json_as(json!({ "amount": amount }))));
            assert_eq!(
                events_json(&events, GameEventType::ArmorGained),
                Vec::<Value>::new()
            );
        }
        let events = run(
            &mut state,
            gain_hero_armor(json_as(json!({ "amount": 2, "player": "enemy" }))),
        );
        assert_eq!(
            events_json(&events, GameEventType::ArmorGained),
            vec![json!({ "type": "armorGained", "player": "p2", "amount": 2, "armor": 2 })]
        );
    }

    /// R3857000: `armor` is the Armor the view shows (`hero_armor_of`, §10.8), Armor Up's modifier
    /// counted, not the stored field alone.
    #[test]
    fn r3857000_the_armor_after_counts_armor_up_s_modifier() {
        let mut state = game("r3857000-modifier");
        run(
            &mut state,
            add_player_modifier(json_as(json!({
                "mod": { "kind": "heroArmor", "amount": 2, "expiry": { "until": "never" } }
            }))),
        );
        let events = run(&mut state, gain_hero_armor(json_as(json!({ "amount": 3 }))));
        assert_eq!(
            events_json(&events, GameEventType::ArmorGained),
            vec![json!({ "type": "armorGained", "player": "p1", "amount": 3, "armor": 5 })]
        );
        assert_eq!(state.players.p1.hero.armor, 3);
    }

    /// R3857000: nothing answers the report, so the trigger loop never takes it and it takes no
    /// number from `state.next_seq` (R68), which keeps a recorded game's hash (R768).
    #[test]
    fn r3857000_the_report_takes_no_number_in_r68_s_order() {
        let mut state = game("r3857000-seq");
        let seq = state.next_seq;
        let ((), events) = run_on_sink(&mut state, |sink| {
            (gain_hero_armor(json_as(json!({ "amount": 2 }))).apply)(&mut make_context(
                sink,
                None,
                by(PlayerId::P1),
            ));
            settle(sink, SettleOptions::default());
            assert!(sink.state.dispatch.is_empty());
        });
        assert_eq!(state.next_seq, seq);
        assert_eq!(events_json(&events, GameEventType::ArmorGained).len(), 1);
    }

    /// R3857000: the report takes no room in R168's floor, so the view's window holds what it held
    /// before the report existed, the reports among it; and the golden texts leave it out, so a trace
    /// recorded before it replays as it did (R2783000's way, R1956001).
    #[test]
    fn r3857000_takes_no_room_in_the_views_window_and_the_golden_texts_leave_it_out() {
        let gain = GameEvent::ArmorGained {
            player: PlayerId::P1,
            amount: 2,
            armor: 2,
        };
        let mut state = game("r3857000-window");
        let older: Vec<GameEvent> = (0..VIEW_EVENT_LIMIT)
            .flat_map(|turn| {
                [
                    gain.clone(),
                    GameEvent::TurnStarted {
                        player: PlayerId::P2,
                        turn: turn as i32,
                        extra: None,
                    },
                ]
            })
            .collect();
        let newest = vec![GameEvent::DrawOffered { player: PlayerId::P1 }, gain.clone()];
        let mut bare = state.clone();
        for (nonce, events) in [("older", older), ("newest", newest)] {
            bare.applied.push(AppliedAction {
                nonce: nonce.into(),
                events: events
                    .iter()
                    .filter(|event| !matches!(event, GameEvent::ArmorGained { .. }))
                    .cloned()
                    .collect(),
            });
            state.applied.push(AppliedAction {
                nonce: nonce.into(),
                events,
            });
        }

        for viewer in [PlayerId::P1, PlayerId::P2] {
            let all = view_for(&state, viewer).events;
            let without: Vec<GameEvent> = all
                .iter()
                .filter(|event| !matches!(event, GameEvent::ArmorGained { .. }))
                .cloned()
                .collect();
            assert_eq!(without, view_for(&bare, viewer).events);
            assert_eq!(without.len(), VIEW_EVENT_LIMIT);
            assert_eq!(all.last(), Some(&gain));
        }
        assert_eq!(view_texts(&state), view_texts(&bare));
        assert_eq!(
            events_text(&[gain.clone(), GameEvent::DrawOffered { player: PlayerId::P1 }]),
            events_text(&[GameEvent::DrawOffered { player: PlayerId::P1 }])
        );
    }
}

mod discount_random_in_hand_c_plus_c49 {
    use super::*;

    #[test]
    fn r65_only_a_card_above_0_that_is_not_x_cost_is_discounted_by_cost_mod() {
        let mut state = game("perks-discount");
        let zero = first(in_hand(&mut state, &free().id, PlayerId::P1, 1));
        let x = first(in_hand(&mut state, &x_spell().id, PlayerId::P1, 1));
        let three = first(in_hand(&mut state, &pricey().id, PlayerId::P1, 1));
        let events = run(
            &mut state,
            discount_random_in_hand(json_as(json!({ "amount": 2 }))),
        );
        assert_eq!(live(&state, &three.id).cost_mod, -2);
        assert_eq!(live(&state, &zero.id).cost_mod, 0);
        assert_eq!(live(&state, &x.id).cost_mod, 0);
        assert_eq!(
            events_json(&events, "costChanged"),
            vec![json!({ "type": "costChanged", "instanceId": three.id, "cost": 1 })]
        );
    }

    #[test]
    fn s2_3_a_discount_past_the_price_floors_the_cost_at_0() {
        let mut state = game("perks-floor");
        let three = first(in_hand(&mut state, &pricey().id, PlayerId::P1, 1));
        run(
            &mut state,
            discount_random_in_hand(json_as(json!({ "amount": 20 }))),
        );
        assert_eq!(live(&state, &three.id).cost_mod, -20);
        assert_eq!(
            effective_cost(&state, &live(&state, &three.id), Default::default()),
            0
        );
    }

    #[test]
    fn r129_a_hand_with_nothing_to_discount_draws_no_random_number() {
        let mut state = game("perks-none");
        in_hand(&mut state, &free().id, PlayerId::P1, 1);
        in_hand(&mut state, &x_spell().id, PlayerId::P1, 1);
        let before = state.rng_cursor;
        assert_eq!(
            run(
                &mut state,
                discount_random_in_hand(json_as(json!({ "amount": 2 })))
            ),
            Vec::<GameEvent>::new()
        );
        assert_eq!(state.rng_cursor, before);
    }

    #[test]
    fn r60_the_pick_is_uniform_over_the_cards_it_can_make_cheaper() {
        let mut hits: IndexMap<String, i32> = IndexMap::new();
        for i in 0..300 {
            let mut state = game(&format!("perks-uniform-{i}"));
            let cards = in_hand(&mut state, &pricey().id, PlayerId::P1, 3);
            run(
                &mut state,
                discount_random_in_hand(json_as(json!({ "amount": 1 }))),
            );
            let hit = cards
                .iter()
                .position(|card| live(&state, &card.id).cost_mod == -1)
                .map_or(-1, |at| at as i64);
            *hits.entry(hit.to_string()).or_insert(0) += 1;
        }
        let mut keys: Vec<String> = hits.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec!["0", "1", "2"]);
        for count in hits.values() {
            assert!(*count > 60);
        }
    }
}
