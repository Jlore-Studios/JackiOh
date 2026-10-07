//! The AI cards' verbs (effects/datacenter.ts): T-AI-4 Chain of Thought's draw repeated while the card it
//! brought is cheap (§2.4, R58, R65, R596, CHAIN_OF_THOUGHT_REPEATS) and T-AI-6 Datacenter Fire's sweep
//! of Field Spells that hits the heroes once per Field Spell it dooms (§4.4, §4.5, R46, R59, R408).
//! Through fixture definitions (fixtures/datacenter.ts); the real cards' tests cover the same cases again.
//!
//! Port of `packages/engine/test/effects-datacenter.test.ts`.

use jackioh_engine::effects::{
    FieldSpellSide, SweepReader, destroy_field_spells_and_hit, draw_while_cheap, field_spells_doomed,
};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::{Backrow, Units},
};

use super::fixtures::catalog::vanilla_catalog;
use super::fixtures::combat::{combat_catalog, plain, scripts as combat_scripts};
use super::fixtures::datacenter::{
    DYING_FIELD_DAMAGE, GUARD_CAP, animated_field, cast_on_draw, datacenter_catalog, dying_field, field,
    field_trap, free, guard, hard_field, one, runner, scripts as datacenter_scripts, trap, two, x_cost,
};
use super::fixtures::field::{act_result, flush, playing, tower};
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, sink_for, slot};
use super::fixtures::scripts::{fixture_catalog, scripts as fixture_scripts};

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_catalog(datacenter_catalog(combat_catalog(fixture_catalog(vanilla_catalog(40, 1)))));
    let mut scripts = registered_scripts().clone();
    scripts.extend(fixture_scripts());
    scripts.extend(combat_scripts());
    scripts.extend(datacenter_scripts());
    register_scripts(scripts);
    state.players.p1.hand = vec![];
    state.players.p2.hand = vec![];
    state.active = P1;
    state
}

/// The Spell running the verb, resolving as a Spell does (§10.5 step 4).
fn resolving(state: &mut GameState, player: PlayerId) -> CardInstance {
    new_instance(state, &runner().id, player, Zone::Resolving { player })
}

/// Apply one effect as the resolving Spell, then the state check §10.5 runs after its list (§4.5).
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let me = resolving(state, controller);
    let mut sink = sink_for(state);
    {
        let mut ctx = make_context(
            sink.sink(),
            Some(me),
            HookOptions { controller: Some(controller), ..Default::default() },
        );
        (effect.apply)(&mut ctx);
    }
    state_check(&mut sink.sink());
    let cursor = sink.rng.cursor();
    let events = std::mem::take(&mut sink.events);
    state.rng_cursor = cursor;
    events
}

fn hero_hits(events: &[GameEvent], player: PlayerId) -> Vec<i32> {
    let hero = format!("hero-{player}");
    events_of_type(events, GameEventType::Damage)
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage { target_id, amount, .. } if *target_id == hero => Some(*amount),
            _ => None,
        })
        .collect()
}

/// `card.zone.z`, read off the card as it stands now (TS held the live object).
fn zone_of(state: &GameState, card: &CardInstance) -> ZoneName {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id)).zone.z()
}

fn chain(max_cost: i32) -> Effect {
    draw_while_cheap(json_as(json!({ "maxCost": max_cost, "repeats": CHAIN_OF_THOUGHT_REPEATS })))
}

fn fire(side: &str, damage_per: i32) -> Effect {
    destroy_field_spells_and_hit(json_as(json!({ "side": side, "damagePer": damage_per })))
}

mod draw_while_cheap_t_ai_4_chain_of_thought {
    use super::*;

    #[test]
    fn draws_1_and_again_while_the_card_it_brought_costs_max_cost_or_less_at_most_repeats_plus_1_draws() {
        let mut state = game("chain-five");
        let one_id = one().id;
        let library = set_library(&mut state, P1, &vec![one_id; 7]);
        run(&mut state, chain(1), P1);
        assert_eq!(state.players.p1.hand.len(), (CHAIN_OF_THOUGHT_REPEATS + 1) as usize);
        assert!(
            library[(CHAIN_OF_THOUGHT_REPEATS + 1) as usize..]
                .iter()
                .all(|card| zone_of(&state, card) == ZoneName::Library)
        );
    }

    #[test]
    fn stops_after_the_first_card_that_costs_more_which_is_still_drawn() {
        let mut state = game("chain-stop");
        let library = set_library(&mut state, P1, &[free().id, two().id, one().id]);
        run(&mut state, chain(1), P1);
        let zones: Vec<ZoneName> = library.iter().map(|card| zone_of(&state, card)).collect();
        assert_eq!(zones, vec![ZoneName::Hand, ZoneName::Hand, ZoneName::Library]);
    }

    #[test]
    fn a_higher_threshold_goes_on_through_it() {
        let mut state = game("chain-two");
        let library = set_library(&mut state, P1, &[free().id, two().id, one().id]);
        run(&mut state, chain(2), P1);
        assert_eq!(zone_of(&state, &library[2]), ZoneName::Hand);
    }

    #[test]
    fn r65_an_x_cost_card_reads_0_as_it_arrives() {
        let mut state = game("chain-x");
        let library = set_library(&mut state, P1, &[x_cost().id, one().id]);
        run(&mut state, chain(0), P1);
        assert_eq!(zone_of(&state, &library[1]), ZoneName::Hand);
    }

    #[test]
    fn r65_the_cards_current_cost_a_cost_mod_and_a_players_price_rule_both_count() {
        let mut state = game("chain-mod");
        let library = set_library(&mut state, P1, &[two().id, one().id, one().id]);
        find_instance_mut(&mut state, &library[0].id).expect("library").cost_mod = -1;
        run(&mut state, chain(1), P1);
        let zones: Vec<ZoneName> = library.iter().map(|card| zone_of(&state, card)).collect();
        assert_eq!(zones, vec![ZoneName::Hand, ZoneName::Hand, ZoneName::Hand]);

        let mut taxed = game("chain-tax");
        let taxed_library = set_library(&mut taxed, P1, &[one().id, one().id]);
        add_modifier(
            &mut sink_for(&mut taxed).sink(),
            P1,
            json_as(json!({ "kind": "costRule", "rule": { "amount": 1 }, "expiry": { "until": "never" } })),
        );
        run(&mut taxed, chain(1), P1);
        let zones: Vec<ZoneName> = taxed_library.iter().map(|card| zone_of(&taxed, card)).collect();
        assert_eq!(zones, vec![ZoneName::Hand, ZoneName::Library]);
    }

    #[test]
    fn r596_a_card_cast_on_draw_ends_the_chain_even_though_its_casts_own_repeat_brings_a_card() {
        let mut state = game("chain-cod");
        let library = set_library(&mut state, P1, &[cast_on_draw().id, one().id, one().id]);
        run(&mut state, chain(1), P1);
        assert_eq!(zone_of(&state, &library[0]), ZoneName::Graveyard);
        assert_eq!(zone_of(&state, &library[1]), ZoneName::Hand);
        assert_eq!(zone_of(&state, &library[2]), ZoneName::Library);
    }

    #[test]
    fn s2_4_r4_a_burned_card_ends_the_chain() {
        let mut state = game("chain-burn");
        in_hand(&mut state, "fx-1", P1, HAND_CAP);
        let library = set_library(&mut state, P1, &[one().id, one().id]);
        let events = run(&mut state, chain(1), P1);
        assert_eq!(zone_of(&state, &library[0]), ZoneName::Graveyard);
        assert_eq!(zone_of(&state, &library[1]), ZoneName::Library);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 1);
    }

    #[test]
    fn s2_4_a_fatigue_draw_ends_the_chain_after_one_hit() {
        let mut state = game("chain-fatigue");
        set_library(&mut state, P1, &[]);
        let events = run(&mut state, chain(1), P1);
        assert_eq!(events_of_type(&events, GameEventType::Fatigue).len(), 1);
    }

    #[test]
    fn r129_it_takes_no_rng_draw() {
        let mut state = game("chain-rng");
        set_library(&mut state, P1, &[one().id, one().id]);
        run(&mut state, chain(1), P1);
        assert_eq!(state.rng_cursor, 0);
    }
}

mod destroy_field_spells_and_hit_t_ai_6_datacenter_fire {
    use super::*;

    #[test]
    fn r59_destroys_every_field_spell_on_both_sides_together_and_no_trap_or_field_trap() {
        let mut state = game("fire-all");
        let mine = put(&mut state, &field().id, slot(P1, Backrow, 1), Default::default());
        let theirs = put(&mut state, &field().id, slot(P2, Backrow, 2), Default::default());
        let kept_trap = put(&mut state, &trap().id, slot(P2, Backrow, 3), Default::default());
        let kept_field_trap = put(&mut state, &field_trap().id, slot(P1, Backrow, 4), Default::default());
        run(&mut state, fire("any", 1), P1);
        assert_eq!(
            [zone_of(&state, &mine), zone_of(&state, &theirs)],
            [ZoneName::Graveyard, ZoneName::Graveyard]
        );
        assert_eq!(
            [zone_of(&state, &kept_trap), zone_of(&state, &kept_field_trap)],
            [ZoneName::Field, ZoneName::Field]
        );
    }

    #[test]
    fn s4_4_one_hit_per_hero_of_damage_per_times_the_count_the_active_players_hero_first_r68() {
        let mut state = game("fire-hits");
        put(&mut state, &field().id, slot(P1, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 2), Default::default());
        let events = run(&mut state, fire("any", 1), P1);
        assert_eq!(hero_hits(&events, P1), vec![3]);
        assert_eq!(hero_hits(&events, P2), vec![3]);
        let targets: Vec<String> = events_of_type(&events, GameEventType::Damage)
            .iter()
            .map(|event| match event {
                GameEvent::Damage { target_id, .. } => target_id.clone(),
                other => panic!("not a damage: {other:?}"),
            })
            .collect();
        assert_eq!(targets, vec!["hero-p1".to_string(), "hero-p2".to_string()]);
        assert_eq!(state.players.p1.hero.health, 27);
    }

    #[test]
    fn s4_4_e6_each_heros_hit_is_one_instance_a_per_hit_cap_of_1_caps_the_whole_sweep_at_1() {
        let mut state = game("fire-cap");
        put(&mut state, &guard().id, slot(P1, Units, 1), Default::default());
        put(&mut state, &field().id, slot(P1, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 2), Default::default());
        run(&mut state, fire("any", 1), P1);
        assert_eq!(state.players.p1.hero.health, 30 - GUARD_CAP);
        assert_eq!(state.players.p2.hero.health, 27);
    }

    #[test]
    fn r46_an_indestructible_field_spell_survives_and_doesnt_count() {
        let mut state = game("fire-hard");
        let hard = put(&mut state, &hard_field().id, slot(P1, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 1), Default::default());
        let events = run(&mut state, fire("any", 1), P1);
        assert_eq!(zone_of(&state, &hard), ZoneName::Field);
        assert_eq!(hero_hits(&events, P1), vec![1]);
    }

    #[test]
    fn r129_none_doomed_no_damage_at_all() {
        let mut state = game("fire-none");
        put(&mut state, &hard_field().id, slot(P1, Backrow, 1), Default::default());
        put(&mut state, &trap().id, slot(P2, Backrow, 1), Default::default());
        let events = run(&mut state, fire("any", 2), P1);
        assert!(events_of_type(&events, GameEventType::Damage).is_empty());
    }

    #[test]
    fn the_enemy_side_only_your_field_spells_stay_and_only_the_enemy_hero_is_hit() {
        let mut state = game("fire-enemy");
        let mine = put(&mut state, &field().id, slot(P1, Backrow, 1), Default::default());
        let theirs = put(&mut state, &field().id, slot(P2, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 2), Default::default());
        let events = run(&mut state, fire("enemy", 2), P1);
        assert_eq!(zone_of(&state, &mine), ZoneName::Field);
        assert_eq!(zone_of(&state, &theirs), ZoneName::Graveyard);
        assert_eq!(hero_hits(&events, P2), vec![4]);
        assert_eq!(hero_hits(&events, P1), Vec::<i32>::new());
    }

    #[test]
    fn s4_5_a_destroyed_field_spell_that_prints_death_fires_it() {
        let mut state = game("fire-death");
        put(&mut state, &dying_field().id, slot(P2, Backrow, 1), Default::default());
        run(&mut state, fire("any", 1), P1);
        // 1 from the fire, and the dying Field Spell's Death hits its enemy, p1, for 3.
        assert_eq!(state.players.p1.hero.health, 30 - 1 - DYING_FIELD_DAMAGE);
        assert_eq!(state.players.p2.hero.health, 29);
    }

    #[test]
    fn r588_r383_an_animated_field_spell_standing_in_a_unit_zone_is_a_unit_there_neither_destroyed_nor_counted() {
        let mut state = game("fire-animated");
        let animated = put(&mut state, &animated_field().id, slot(P2, Backrow, 1), Default::default());
        assert!(animate_card(&mut sink_for(&mut state).sink(), &animated, Default::default()));
        let backrow = put(&mut state, &field().id, slot(P2, Backrow, 2), Default::default());
        let events = run(&mut state, fire("any", 1), P1);
        let standing = find_instance(&state, &animated.id).expect("the animated Field Spell");
        assert!(matches!(standing.zone, Zone::Field { row: Row::Units, .. }), "{:?}", standing.zone);
        assert_eq!(zone_of(&state, &backrow), ZoneName::Graveyard);
        assert_eq!(hero_hits(&events, P2), vec![1]);
    }

    #[test]
    fn r418_r446_a_carrier_field_spell_holding_a_unit_is_destroyed_and_counted_and_the_unit_it_holds_is_not() {
        let mut begun = playing("fire-tower");
        put(&mut begun, &tower().id, slot(P1, Backrow, 2), Default::default());
        let rider = in_hand(&mut begun, &plain().id, P1, 1).into_iter().next();
        flush(&mut begun, P1, 10);
        let rider_id = rider.as_ref().map(|card| card.id.clone()).unwrap_or_default();
        let played = act_result(
            &begun,
            json_as(json!({ "type": "play", "instanceId": rider_id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1" })),
        );
        assert_eq!(played.error, None);
        let mut state = played.state;
        let mut catalog = registered_catalog().clone();
        let runner_def = runner();
        catalog.insert(runner_def.id.clone(), runner_def);
        register_catalog(catalog);
        let events = run(&mut state, fire("any", 1), P1);
        let destroyed: Vec<String> = events_of_type(&events, GameEventType::Destroyed)
            .iter()
            .map(|event| match event {
                GameEvent::Destroyed { def_id, .. } => def_id.clone(),
                other => panic!("not a destroyed: {other:?}"),
            })
            .collect();
        assert_eq!(destroyed, vec![tower().id]);
        assert_eq!(hero_hits(&events, P1), vec![1]);
        assert!(state.players.p1.units.iter().flatten().flatten().any(|card| card.id == rider_id));
    }

    #[test]
    fn r280_field_spells_doomed_is_the_count_the_sweep_hits_with_read_without_writing() {
        let mut state = game("fire-read");
        put(&mut state, &field().id, slot(P1, Backrow, 1), Default::default());
        put(&mut state, &hard_field().id, slot(P2, Backrow, 1), Default::default());
        put(&mut state, &field().id, slot(P2, Backrow, 2), Default::default());
        let me = resolving(&mut state, P1);
        let reader = SweepReader { state: &state, self_: Some(&me), def_id: None, radiant: false, controller: P1 };
        let before = serde_json::to_string(&state).expect("serialises");
        assert_eq!(field_spells_doomed(&reader, FieldSpellSide::Any).len(), 2);
        assert_eq!(field_spells_doomed(&reader, FieldSpellSide::Enemy).len(), 1);
        assert_eq!(serde_json::to_string(&state).expect("serialises"), before);
    }
}
