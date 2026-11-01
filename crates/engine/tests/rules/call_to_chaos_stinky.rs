//! Stinkyscape #150 Call to Chaos (Stinky Edition)'s table (`subsystems/call_to_chaos_stinky.rs`), rolled
//! by Core #95's subsystem (R28, R87, R423, R436): the ten entries in the card's order, those that add or
//! summon the set's cards by index against `fixtures::call_to_chaos_stinkyscape`, the freeze (R1882003),
//! and the rest-of-game death watch with its cap (R1882004) and its casts a turn (R2176001).

use jackioh_engine::config::{
    CHAOS_STINKY_DEATH_WATCH_CAP, CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN, CHAOS_STINKYSCAPE_COST,
};
use jackioh_engine::subsystems::call_to_chaos_stinky::{
    CHAOS_DEATH_WATCH_HOOK, CHAOS_DEATH_WATCH_LABEL, CHAOS_STINKY_EFFECTS, death_watch_casts_this_turn,
    death_watch_step, death_watches_held,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::work::script_step_for;

use crate::rules::fixtures::call_to_chaos_stinkyscape::{
    ARCTIC_CITADEL, DANDY_LAD, ETHICS, GOLEM, VANGUARD, stinkyscape_game as game,
};
use crate::rules::fixtures::harness::{put, slot};

fn entry(name: &str) -> Effect {
    let found = CHAOS_STINKY_EFFECTS
        .iter()
        .find(|effect| effect.name == name)
        .unwrap_or_else(|| panic!("no entry {name}"));
    (found.build)()
}

fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let me = new_instance(&mut *state, ETHICS, P1, Zone::Resolving { player: P1 });
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        let options = HookOptions {
            controller: Some(P1),
            ..HookOptions::default()
        };
        {
            let mut ctx = make_context(&mut sink, Some(&me), options);
            apply_effects(&[effect], &mut ctx);
        }
        state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// p1's hand, as `(def, radiant, costOverride)`.
fn hand(state: &GameState) -> Vec<(String, bool, Option<i32>)> {
    state
        .players
        .p1
        .hand
        .iter()
        .map(|card| (card.def_id.clone(), card.radiant, card.cost_override))
        .collect()
}

mod the_stinky_editions_table {
    use super::*;

    #[test]
    fn is_the_ten_entries_of_s8_10_row_150_in_the_cards_order() {
        let names: Vec<&str> = CHAOS_STINKY_EFFECTS.iter().map(|effect| effect.name).collect();
        assert_eq!(
            names,
            vec![
                "ethics",
                "dandy",
                "quests",
                "acclaimed",
                "freeze",
                "humans",
                "allure",
                "deathWatch",
                "golem",
                "recast"
            ]
        );
        assert_eq!(CHAOS_STINKY_EFFECTS[7].label, CHAOS_DEATH_WATCH_LABEL);
        assert_eq!(CHAOS_STINKY_EFFECTS[9].label, "Cast a random Call to Chaos");
    }

    #[test]
    fn the_ethics_entry_adds_a_radiant_ethics_then_a_radiant_vanguard_coalition() {
        let mut state = game("stinky-ethics");
        run(&mut state, entry("ethics"));
        assert_eq!(
            hand(&state),
            vec![
                (ETHICS.to_string(), true, None),
                (VANGUARD.to_string(), true, None)
            ]
        );
    }

    #[test]
    fn the_dandy_entry_adds_a_very_dandy_lad_then_an_arctic_citadel_at_zero() {
        let mut state = game("stinky-dandy");
        run(&mut state, entry("dandy"));
        let at_zero = Some(CHAOS_STINKYSCAPE_COST);
        assert_eq!(
            hand(&state),
            vec![
                (DANDY_LAD.to_string(), false, at_zero),
                (ARCTIC_CITADEL.to_string(), false, at_zero)
            ]
        );
    }

    #[test]
    fn the_golem_entry_summons_a_stinkyscape_golem() {
        let mut state = game("stinky-golem");
        run(&mut state, entry("golem"));
        let golems = state.players.p1.units.iter().flatten().flatten();
        assert_eq!(golems.filter(|card| card.def_id == GOLEM).count(), 1);
    }
}

mod r1882003_the_freeze {
    use super::*;

    #[test]
    fn r1882003_every_enemy_unit_goes_to_attack_position_and_is_frozen_and_the_casters_are_not() {
        let mut state = game("stinky-freeze");
        let guard = put(&mut state, VANGUARD, slot(P2, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &guard.id)
            .expect("on the field")
            .position = Some(Position::Def);
        let enemy = put(&mut state, VANGUARD, slot(P2, Row::Units, 2), json!({}));
        let own = put(&mut state, VANGUARD, slot(P1, Row::Units, 1), json!({}));
        run(&mut state, entry("freeze"));
        for id in [&guard.id, &enemy.id] {
            let card = find_instance(&state, id).expect("on the field");
            assert_eq!(card.position, Some(Position::Atk), "{id}");
            assert!(card.frozen.is_some(), "{id}");
        }
        assert_eq!(find_instance(&state, &own.id).and_then(|card| card.frozen), None);
    }
}

mod r1882004_the_death_watch {
    use super::*;

    #[test]
    fn r1882004_the_watch_stacks_on_its_caster_up_to_the_cap_and_a_roll_past_it_adds_nothing() {
        let mut state = game("stinky-watch");
        for held in 1..=CHAOS_STINKY_DEATH_WATCH_CAP {
            run(&mut state, entry("deathWatch"));
            assert_eq!(death_watches_held(&state, P1), held);
        }
        let mods = state.players.p1.mods.len();
        run(&mut state, entry("deathWatch"));
        assert_eq!(state.players.p1.mods.len(), mods);
        assert_eq!(death_watches_held(&state, P1), CHAOS_STINKY_DEATH_WATCH_CAP);
        assert_eq!(death_watches_held(&state, P2), 0);
    }

    #[test]
    fn r1882004_the_engine_answers_the_watch_hook_for_any_script() {
        let resume = Resume {
            def_id: "fx-1".to_string(),
            hook: CHAOS_DEATH_WATCH_HOOK.to_string(),
            step: "chaosDeathWatch".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        };
        assert!(script_step_for(&Script::default(), &resume).is_some());
    }

    #[test]
    fn r2176001_the_watch_casts_at_most_its_casts_a_turn_for_a_player_and_again_the_next_turn() {
        let mut state = game("stinky-watch-casts");
        let step = death_watch_step();
        let casts = |state: &mut GameState| -> usize {
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            let options = HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            };
            let mut ctx = make_context(&mut sink, None, options);
            step(&mut ctx).len()
        };
        for cast in 1..=CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN {
            assert_eq!(casts(&mut state), 1);
            assert_eq!(death_watch_casts_this_turn(&state, P1), cast);
        }
        assert_eq!(casts(&mut state), 0, "the terminator");
        assert_eq!(death_watch_casts_this_turn(&state, P2), 0);
        state.turn += 1;
        assert_eq!(death_watch_casts_this_turn(&state, P1), 0);
        assert_eq!(casts(&mut state), 1);
    }
}
