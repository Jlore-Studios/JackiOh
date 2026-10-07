//! effects/perks.ts: C+ #46 Felinor Flagbearer's Armor for the rest of the game and C+ #49 Jay Fungus's
//! discount on a random hand card. Applied straight to fixture cards; the real cards' tests cover the
//! same cases again (packages/cards/test/classic-plus/046-felinor-flagbearer.test.ts, 049-jay-fungus.test.ts).
//!
//! Port of `packages/engine/test/effects-perks.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::damage::hero_hit_amount;
use jackioh_engine::effects::{discount_random_in_hand, gain_hero_armor};
use jackioh_engine::mana::effective_cost;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{EngineSink, Effect};
use jackioh_engine::state::{CardInstance, GameState, find_instance};

use super::fixtures::catalog::{spell_def, unit_def, vanilla_catalog};
use super::fixtures::harness::{in_hand, new_game};

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
    let mut catalog = vanilla_catalog(40, 1);
    for def in [free(), x_spell(), pricey()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.players.p1.hand = Vec::new();
    state
}

/// One effect applied as p1's, on a sink whose rng starts at the state's cursor (`sinkFor`); the
/// cursor is written back, and the events it emitted are handed back.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

/// The card under `id` as it stands in the state now (TS read the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
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
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": 2, "player": "enemy" }))));
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": 0 }))));
        run(&mut state, gain_hero_armor(json_as(json!({ "amount": -3 }))));
        assert_eq!(state.players.p2.hero.armor, 2);
        assert_eq!(state.players.p1.hero.armor, 0);
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
        let events = run(&mut state, discount_random_in_hand(json_as(json!({ "amount": 2 }))));
        assert_eq!(live(&state, &three.id).cost_mod, -2);
        assert_eq!(live(&state, &zero.id).cost_mod, 0);
        assert_eq!(live(&state, &x.id).cost_mod, 0);
        assert_eq!(
            of_type(&events, "costChanged"),
            vec![json!({ "type": "costChanged", "instanceId": three.id, "cost": 1 })]
        );
    }

    #[test]
    fn s2_3_a_discount_past_the_price_floors_the_cost_at_0() {
        let mut state = game("perks-floor");
        let three = first(in_hand(&mut state, &pricey().id, PlayerId::P1, 1));
        run(&mut state, discount_random_in_hand(json_as(json!({ "amount": 20 }))));
        assert_eq!(live(&state, &three.id).cost_mod, -20);
        assert_eq!(effective_cost(&state, live(&state, &three.id), Default::default()), 0);
    }

    #[test]
    fn r129_a_hand_with_nothing_to_discount_draws_no_random_number() {
        let mut state = game("perks-none");
        in_hand(&mut state, &free().id, PlayerId::P1, 1);
        in_hand(&mut state, &x_spell().id, PlayerId::P1, 1);
        let before = state.rng_cursor;
        assert_eq!(
            run(&mut state, discount_random_in_hand(json_as(json!({ "amount": 2 })))),
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
            run(&mut state, discount_random_in_hand(json_as(json!({ "amount": 1 }))));
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

