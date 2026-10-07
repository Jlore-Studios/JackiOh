//! The Classic+ Fruit verbs (effects/fruit.ts): the Grapes C+ #65 Two Grapes and #66 Vine of Grapes roll
//! by `GRAPE_ODDS` with Lucky (R382, §6.1), C+ #65.2/#65.3's hit-or-heal and priced draw (§2.4, R58,
//! R65), and C+ #65.5 Mythic Grape's hand replaced card for card (R11, R60, R129, R387). Through fixture
//! scripts (fixtures/fruit.ts); the real cards' tests cover the same cases again.
//!
//! Port of `packages/engine/test/effects-fruit.test.ts`.

use jackioh_engine::effects::fruit::{
    add_rolled_grapes, damage_enemy_or_heal_friend, draw_priced, replace_hand_with_random, roll_grape,
};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_catalog;
use crate::rules::fixtures::combat::{combat_catalog, COMBAT_SCRIPTS};
use crate::rules::fixtures::fruit::{cast_on_draw, fruit_catalog, FRUIT_SCRIPTS, grape_roller, mythic, priced_draw, replacer};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};
use crate::rules::fixtures::scripts::{fixture_catalog, FIXTURE_SCRIPTS};

fn grape_ids() -> Vec<&'static str> {
    GRAPE_ODDS.iter().map(|grape| grape.def_id).collect()
}

/// TS `GRAPE_IDS.indexOf(id)`: -1 when absent.
fn grape_index(id: &str) -> i64 {
    grape_ids().iter().position(|grape| *grape == id).map_or(-1, |index| index as i64)
}

fn is_grape(id: &str) -> bool {
    grape_ids().contains(&id)
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_catalog(fruit_catalog(combat_catalog(fixture_catalog(vanilla_catalog(40, 1)))));
    let mut scripts = FIXTURE_SCRIPTS.clone();
    scripts.extend(COMBAT_SCRIPTS.clone());
    scripts.extend(FRUIT_SCRIPTS.clone());
    register_scripts(scripts);
    state.players[PlayerId::P1].hand = vec![];
    state.players[PlayerId::P2].hand = vec![];
    state
}

/// The card running the script, resolving as a Spell does (§10.5 step 4). Like TS's, it is in no pile.
fn resolving(state: &mut GameState, def_id: &str, radiant: bool, player: PlayerId) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Resolving { player });
    card.radiant = radiant;
    card
}

/// `resolving(state, defId)` with TS's defaults (base face, p1).
fn resolving_p1(state: &mut GameState, def_id: &str) -> CardInstance {
    resolving(state, def_id, false, PlayerId::P1)
}

/// TS `run(state, effect, self, targets, controller)`: one effect on a sink over `state`, the rng cursor
/// written back; its events.
fn run(
    state: &mut GameState,
    effect: Effect,
    self_: Option<&CardInstance>,
    targets: Vec<Selection>,
    controller: PlayerId,
) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            self_,
            HookOptions { controller: Some(controller), targets: Some(targets), ..Default::default() },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// `run(state, effect, self)` with TS's defaults (no targets, p1).
fn run_as(state: &mut GameState, effect: Effect, self_: &CardInstance) -> Vec<GameEvent> {
    run(state, effect, Some(self_), vec![], PlayerId::P1)
}

fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

fn zone_of(state: &GameState, id: &str) -> Option<ZoneName> {
    find_instance(state, id).map(|card| card.zone.z())
}

fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match serde_json::to_value(events).expect("events serialise") {
        Value::Array(items) => items.into_iter().map(|item| item.get(key).cloned().unwrap_or(Value::Null)).collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

/// `count` rolls from `rng` with `lucky`, as a share in percent per Grape id.
fn shares(seed: &str, lucky: i32, rolls: i32) -> IndexMap<String, f64> {
    let mut rng = Rng::new(seed, 0);
    let mut counts: IndexMap<String, i32> = IndexMap::new();
    for _ in 0..rolls {
        let id = roll_grape(&mut rng, lucky).to_string();
        *counts.entry(id).or_insert(0) += 1;
    }
    counts.into_iter().map(|(id, count)| (id, f64::from(count) / f64::from(rolls) * 100.0)).collect()
}

mod roll_grape_r382_build_2_grape_odds {
    use super::*;

    #[test]
    fn r382_the_table_is_the_five_grapes_worst_to_best_and_its_percents_sum_to_100() {
        assert_eq!(GRAPE_ODDS.iter().map(|grape| grape.percent).collect::<Vec<_>>(), vec![12, 60, 20, 7, 1]);
        assert_eq!(GRAPE_ODDS.iter().map(|grape| grape.percent).sum::<i32>(), 100);
    }

    #[test]
    fn r382_many_seeded_rolls_match_the_odds_12_60_20_7_1_percent() {
        let share = shares("grape-odds", 0, 20_000);
        for grape in GRAPE_ODDS {
            let got = share.get(grape.def_id).copied().unwrap_or(0.0);
            assert!((got - f64::from(grape.percent)).abs() < 1.5, "{}: {got}%", grape.def_id);
        }
    }

    /// TS: "§6.1 Lucky 1 rolls twice and keeps the better: Rotten falls to ~1.4%, Mythic rises to ~2%".
    #[test]
    fn lucky_1_rolls_twice_and_keeps_the_better_rotten_falls_to_1_4_percent_mythic_rises_to_2_percent() {
        let counts = shares("grape-lucky", 1, 20_000);
        let share = |id: &str| -> f64 { counts.get(id).copied().unwrap_or(0.0) };
        assert!(share(grape_ids()[0]) < 2.5);
        assert!(share(grape_ids()[4]) > 1.4);
        assert!(share(grape_ids()[4]) < 2.8);
    }

    /// TS: "§6.1 a Lucky roll is two draws of the rng, an ordinary one draw".
    #[test]
    fn a_lucky_roll_is_two_draws_of_the_rng_an_ordinary_one_draw() {
        let mut plain = Rng::new("grape-draws", 0);
        roll_grape(&mut plain, 0);
        assert_eq!(plain.cursor(), 1);
        let mut lucky = Rng::new("grape-draws", 0);
        roll_grape(&mut lucky, 1);
        assert_eq!(lucky.cursor(), 2);
    }

    /// TS: "§6.1 Lucky keeps the later entry: with a fixed seed the lucky pick is the best of the two plain picks".
    #[test]
    fn lucky_keeps_the_later_entry_with_a_fixed_seed_the_lucky_pick_is_the_best_of_the_two_plain_picks() {
        for i in 0..50 {
            let mut a = Rng::new(&format!("grape-best-{i}"), 0);
            let first = grape_index(&roll_grape(&mut a, 0).to_string());
            let second = grape_index(&roll_grape(&mut a, 0).to_string());
            let mut b = Rng::new(&format!("grape-best-{i}"), 0);
            assert_eq!(grape_index(&roll_grape(&mut b, 1).to_string()), first.max(second));
        }
    }
}

mod add_rolled_grapes_c_65_66 {
    use super::*;

    #[test]
    fn r60_adds_n_grapes_each_its_own_roll_to_the_controller_s_hand() {
        let mut state = game("grapes-three");
        let self_ = resolving_p1(&mut state, &grape_roller.id);
        let events = run_as(&mut state, add_rolled_grapes(json_as(json!({ "count": 3 }))), &self_);

        assert_eq!(state.players[PlayerId::P1].hand.len(), 3);
        assert!(state.players[PlayerId::P1].hand.iter().all(|card| is_grape(&card.def_id)));
        assert!(state.players[PlayerId::P1].hand.iter().all(|card| !card.radiant));
        assert_eq!(events_of_type(&events, GameEventType::AddedToHand).len(), 3);
        assert_eq!(state.rng_cursor, 3);
    }

    #[test]
    fn r74_radiant_every_grape_is_made_radiant() {
        let mut state = game("grapes-radiant");
        let self_ = resolving_p1(&mut state, &grape_roller.id);
        run_as(&mut state, add_rolled_grapes(json_as(json!({ "count": 3, "radiant": true, "lucky": 0 }))), &self_);
        assert!(state.players[PlayerId::P1].hand.iter().all(|card| card.radiant));
    }

    /// TS: "§6.1 the Lucky is the running card's own: the Radiant face's printed Lucky 1 is two draws a Grape".
    #[test]
    fn the_lucky_is_the_running_card_s_own_the_radiant_face_s_printed_lucky_1_is_two_draws_a_grape() {
        let mut state = game("grapes-lucky");
        let self_ = resolving(&mut state, &grape_roller.id, true, PlayerId::P1);
        run_as(&mut state, add_rolled_grapes(json_as(json!({ "count": 3, "radiant": true }))), &self_);
        assert_eq!(state.rng_cursor, 6);

        let mut plain = game("grapes-lucky");
        let self_ = resolving(&mut plain, &grape_roller.id, false, PlayerId::P1);
        run_as(&mut plain, add_rolled_grapes(json_as(json!({ "count": 3 }))), &self_);
        assert_eq!(plain.rng_cursor, 3);
    }

    /// TS: "§2.4 R4 a full hand burns each Grape that doesn't fit".
    #[test]
    fn r4_a_full_hand_burns_each_grape_that_doesn_t_fit() {
        let mut state = game("grapes-burn");
        in_hand(&mut state, "fx-1", PlayerId::P1, HAND_CAP - 1);
        let self_ = resolving_p1(&mut state, &grape_roller.id);
        let events = run_as(&mut state, add_rolled_grapes(json_as(json!({ "count": 3 }))), &self_);
        assert_eq!(state.players[PlayerId::P1].hand.len(), HAND_CAP as usize);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 2);
        assert_eq!(state.players[PlayerId::P1].graveyard.iter().filter(|card| is_grape(&card.def_id)).count(), 2);
    }

    /// TS: "§10.7 a fixed seed and cursor give fixed Grapes in a fixed order".
    #[test]
    fn a_fixed_seed_and_cursor_give_fixed_grapes_in_a_fixed_order() {
        let roll = || -> Vec<String> {
            let mut state = game("grapes-fixed");
            let self_ = resolving_p1(&mut state, &grape_roller.id);
            run_as(&mut state, add_rolled_grapes(json_as(json!({ "count": 5 }))), &self_);
            state.players[PlayerId::P1].hand.iter().map(|card| card.def_id.clone()).collect()
        };
        assert_eq!(roll(), roll());
    }

    #[test]
    fn a_count_of_0_adds_nothing_and_draws_nothing() {
        let mut state = game("grapes-none");
        let self_ = resolving_p1(&mut state, &grape_roller.id);
        run_as(&mut state, add_rolled_grapes(json_as(json!({ "count": 0 }))), &self_);
        assert_eq!(state.players[PlayerId::P1].hand, Vec::<CardInstance>::new());
        assert_eq!(state.rng_cursor, 0);
    }
}

mod damage_enemy_or_heal_friend_c_65_2_65_3 {
    use super::*;

    #[test]
    fn an_enemy_unit_takes_the_hit_from_the_card_running_the_script() {
        let mut state = game("hit-enemy");
        let enemy = put(&mut state, "fx-5", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        let events = run(
            &mut state,
            damage_enemy_or_heal_friend(json_as(json!({ "amount": 2 }))),
            Some(&self_),
            vec![Selection::Instance { instance_id: enemy.id.clone() }],
            PlayerId::P1,
        );
        assert_eq!(live(&state, &enemy.id).damage, 2);
        assert_eq!(pluck(&events_of_type(&events, GameEventType::Damage), "sourceId").first(), Some(&json!(self_.id)));
    }

    #[test]
    fn the_enemy_hero_takes_the_hit() {
        let mut state = game("hit-hero");
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        run(
            &mut state,
            damage_enemy_or_heal_friend(json_as(json!({ "amount": 2 }))),
            Some(&self_),
            vec![Selection::Hero { player: PlayerId::P2 }],
            PlayerId::P1,
        );
        assert_eq!(state.players[PlayerId::P2].hero.health, 28);
    }

    #[test]
    fn r19_a_friendly_unit_is_healed_never_hit() {
        let mut state = game("heal-friend");
        let friend = put(&mut state, "fx-5", slot(PlayerId::P1, Row::Units, 1), json!({}));
        live_mut(&mut state, &friend.id).damage = 1;
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        let events = run(
            &mut state,
            damage_enemy_or_heal_friend(json_as(json!({ "amount": 2 }))),
            Some(&self_),
            vec![Selection::Instance { instance_id: friend.id.clone() }],
            PlayerId::P1,
        );
        assert_eq!(live(&state, &friend.id).damage, 0);
        assert!(events_of_type(&events, GameEventType::Damage).is_empty());
    }

    #[test]
    fn r19_your_own_hero_is_healed_past_30_a_hero_has_no_maximum() {
        let mut state = game("heal-hero");
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        run(
            &mut state,
            damage_enemy_or_heal_friend(json_as(json!({ "amount": 4 }))),
            Some(&self_),
            vec![Selection::Hero { player: PlayerId::P1 }],
            PlayerId::P1,
        );
        assert_eq!(state.players[PlayerId::P1].hero.health, 34);
    }

    #[test]
    fn a_target_gone_by_resolution_is_nothing() {
        let mut state = game("hit-gone");
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        let events = run(
            &mut state,
            damage_enemy_or_heal_friend(json_as(json!({ "amount": 2 }))),
            Some(&self_),
            vec![Selection::Instance { instance_id: "c-missing".to_string() }],
            PlayerId::P1,
        );
        assert_eq!(events, Vec::<GameEvent>::new());
    }
}

mod draw_priced_c_65_2_65_3 {
    use super::*;

    #[test]
    fn r65_the_drawn_card_takes_the_discount_which_stacks_on_its_cost_mod() {
        let mut state = game("priced-mod");
        let Some(top) = set_library(&mut state, PlayerId::P1, &["fx-3".to_string(), "fx-4".to_string()]).into_iter().next()
        else {
            panic!("library");
        };
        live_mut(&mut state, &top.id).cost_mod = 1;
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        let events = run_as(&mut state, draw_priced(json_as(json!({ "costMod": -1 }))), &self_);
        assert_eq!(zone_of(&state, &top.id), Some(ZoneName::Hand));
        assert_eq!(live(&state, &top.id).cost_mod, 0);
        assert_eq!(pluck(&events_of_type(&events, GameEventType::CostChanged), "instanceId"), vec![json!(top.id)]);
    }

    #[test]
    fn r65_a_set_price_is_a_cost_override() {
        let mut state = game("priced-override");
        let top = set_library(&mut state, PlayerId::P1, &["fx-3".to_string()]).into_iter().next();
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        run_as(&mut state, draw_priced(json_as(json!({ "costOverride": 0 }))), &self_);
        assert_eq!(top.and_then(|top| live(&state, &top.id).cost_override), Some(0));
    }

    /// TS: "§2.4 R4 a burned card takes no price".
    #[test]
    fn r4_a_burned_card_takes_no_price() {
        let mut state = game("priced-burn");
        in_hand(&mut state, "fx-1", PlayerId::P1, HAND_CAP);
        let top = set_library(&mut state, PlayerId::P1, &["fx-3".to_string()]).into_iter().next().expect("library");
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        run_as(&mut state, draw_priced(json_as(json!({ "costMod": -1 }))), &self_);
        assert_eq!(zone_of(&state, &top.id), Some(ZoneName::Graveyard));
        assert_eq!(live(&state, &top.id).cost_mod, 0);
    }

    /// TS: "§2.4 a fatigue draw brings no card and prices nothing".
    #[test]
    fn a_fatigue_draw_brings_no_card_and_prices_nothing() {
        let mut state = game("priced-fatigue");
        state.players[PlayerId::P1].library = vec![];
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        let events = run_as(&mut state, draw_priced(json_as(json!({ "costMod": -1 }))), &self_);
        assert_eq!(events_of_type(&events, GameEventType::Fatigue).len(), 1);
        assert!(events_of_type(&events, GameEventType::CostChanged).is_empty());
    }

    #[test]
    fn r596_a_card_cast_on_draw_never_reaches_the_hand_so_neither_it_nor_the_card_the_draw_then_brings_is_priced() {
        let mut state = game("priced-cod");
        let library = set_library(&mut state, PlayerId::P1, &[cast_on_draw.id.clone(), "fx-3".to_string()]);
        let (cast, next) = (library[0].clone(), library[1].clone());
        let self_ = resolving_p1(&mut state, &priced_draw.id);
        run_as(&mut state, draw_priced(json_as(json!({ "costMod": -1 }))), &self_);
        assert_eq!(zone_of(&state, &cast.id), Some(ZoneName::Graveyard));
        assert_eq!(zone_of(&state, &next.id), Some(ZoneName::Hand));
        assert_eq!(live(&state, &next.id).cost_mod, 0);
    }
}

mod pick_generated_r382 {
    use super::*;

    #[test]
    fn passes_a_non_grape_pick_through_the_same_seed_draws_the_same_def_as_rng_pick_in_one_draw() {
        game("pick-through");
        let pool = query(&json_as(json!({ "type": "Spell", "notTags": ["Token"] })));
        assert!(pool.len() > 1);
        assert!(!pool.iter().any(|def| is_grape(&def.id)));
        let mut a = Rng::new("pick-through-seed", 0);
        let mut b = Rng::new("pick-through-seed", 0);
        assert_eq!(
            pick_generated(&mut a, &pool, None).map(|def| def.id.clone()),
            b.pick(&pool).map(|def| def.id.clone())
        );
        assert_eq!(a.cursor(), 1);
    }

    #[test]
    fn r382_a_grape_pick_is_re_rolled_by_grape_odds_a_pool_holding_only_one_grape_deals_the_odds_not_that_grape() {
        game("pick-grape");
        let Some(only) = find_def(None, grape_ids()[4]) else {
            panic!("fixture Mythic Grape");
        };
        let pool = vec![only];
        let mut counts: IndexMap<String, i32> = IndexMap::new();
        let seeds = 300;
        for i in 0..seeds {
            let got = pick_generated(&mut Rng::new(&format!("pick-grape-{i}"), 0), &pool, None);
            assert!(got.as_ref().is_none_or(|def| is_grape(&def.id)));
            let key = got.map(|def| def.id.clone()).unwrap_or_default();
            *counts.entry(key).or_insert(0) += 1;
        }
        let count = |id: &str| -> i32 { counts.get(id).copied().unwrap_or(0) };
        // The odds, not the pool's one entry: Normal (60%) is the plurality, and the pool's own Mythic
        // entry is not what comes out (it would be all 300 without the re-roll).
        assert!(count(grape_ids()[1]) > count(grape_ids()[2]));
        assert!(count(grape_ids()[1]) > count(grape_ids()[0]));
        assert!(count(grape_ids()[0]) >= 5);
        assert!(count(grape_ids()[3]) >= 1);
        assert!(count(grape_ids()[4]) <= 12);
        assert!(count(grape_ids()[4]) < seeds);
    }

    #[test]
    fn draws_one_extra_number_only_when_a_grape_was_picked_one_draw_else_two_for_a_grape() {
        game("pick-draws");
        let plain = query(&json_as(json!({ "type": "Spell", "notTags": ["Token"] })));
        let Some(grape) = find_def(None, grape_ids()[0]) else {
            panic!("fixture Rotten Grape");
        };
        let mut through = Rng::new("pick-draws", 0);
        pick_generated(&mut through, &plain, None);
        assert_eq!(through.cursor(), 1);
        let mut rerolled = Rng::new("pick-draws", 0);
        pick_generated(&mut rerolled, &[grape], None);
        assert_eq!(rerolled.cursor(), 2);
    }

    #[test]
    fn an_empty_pool_is_undefined_and_draws_nothing() {
        game("pick-empty");
        let mut rng = Rng::new("pick-empty", 0);
        let empty: Vec<&CardDef> = vec![];
        assert!(pick_generated(&mut rng, &empty, None).is_none());
        assert_eq!(rng.cursor(), 0);
    }
}

mod replace_hand_with_random_c_65_5 {
    use super::*;

    #[test]
    fn moves_every_hand_card_to_the_graveyard_not_a_discard_and_adds_as_many_cards_of_the_pool() {
        let mut state = game("replace-hand");
        let old = in_hand(&mut state, "fx-1", PlayerId::P1, 3);
        let self_ = resolving_p1(&mut state, &replacer.id);
        let events = run_as(
            &mut state,
            replace_hand_with_random(json_as(json!({ "query": { "rarity": "Mythic" }, "costOverride": 0 }))),
            &self_,
        );

        assert!(old.iter().all(|card| zone_of(&state, &card.id) == Some(ZoneName::Graveyard)));
        assert!(events_of_type(&events, GameEventType::Discarded).is_empty());
        assert_eq!(
            pluck(&events_of_type(&events, GameEventType::EnteredGraveyard), "instanceId"),
            old.iter().map(|card| json!(card.id)).collect::<Vec<_>>()
        );
        assert_eq!(state.players[PlayerId::P1].hand.len(), 3);
        assert!(state.players[PlayerId::P1]
            .hand
            .iter()
            .all(|card| card.def_id == mythic.id && card.cost_override == Some(0)));
    }

    #[test]
    fn r387_the_card_running_it_is_never_in_its_own_pool() {
        let mut state = game("replace-self");
        in_hand(&mut state, "fx-1", PlayerId::P1, 4);
        let self_ = resolving_p1(&mut state, &replacer.id);
        run_as(&mut state, replace_hand_with_random(json_as(json!({ "query": { "rarity": "Mythic" } }))), &self_);
        assert!(!state.players[PlayerId::P1].hand.iter().any(|card| card.def_id == replacer.id));
    }

    #[test]
    fn r74_radiant_the_new_cards_are_radiant() {
        let mut state = game("replace-radiant");
        in_hand(&mut state, "fx-1", PlayerId::P1, 2);
        let self_ = resolving_p1(&mut state, &replacer.id);
        run_as(
            &mut state,
            replace_hand_with_random(json_as(json!({ "query": { "rarity": "Mythic" }, "radiant": true }))),
            &self_,
        );
        assert!(state.players[PlayerId::P1].hand.iter().all(|card| card.radiant));
    }

    #[test]
    fn r11_a_unit_token_card_in_the_hand_ceases_to_exist_and_still_counts_as_a_card_replaced() {
        let mut state = game("replace-token");
        let token = in_hand(&mut state, "fx-token-rush", PlayerId::P1, 1).into_iter().next().expect("the token");
        in_hand(&mut state, "fx-1", PlayerId::P1, 1);
        let self_ = resolving_p1(&mut state, &replacer.id);
        run_as(&mut state, replace_hand_with_random(json_as(json!({ "query": { "rarity": "Mythic" } }))), &self_);
        assert_ne!(zone_of(&state, &token.id), Some(ZoneName::Graveyard));
        assert!(!state.players[PlayerId::P1].graveyard.iter().any(|card| card.id == token.id));
        assert_eq!(state.players[PlayerId::P1].hand.len(), 2);
    }

    #[test]
    fn r129_an_empty_hand_moves_nothing_adds_nothing_and_draws_nothing_from_the_rng() {
        let mut state = game("replace-empty");
        let self_ = resolving_p1(&mut state, &replacer.id);
        let events =
            run_as(&mut state, replace_hand_with_random(json_as(json!({ "query": { "rarity": "Mythic" } }))), &self_);
        assert_eq!(events, Vec::<GameEvent>::new());
        assert_eq!(state.rng_cursor, 0);
    }
}
