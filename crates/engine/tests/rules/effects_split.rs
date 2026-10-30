//! B5 E37 random split damage: "deal N damage split among enemies" — N hits of 1,
//! each a damage instance of its own on a random enemy still standing (Classic+ #3's Death).

use jackioh_engine::testkit::*;

use jackioh_engine::effects::{damage_random_enemies, damage_split};
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::EngineSink;
use jackioh_engine::state::{CardInstance, GameState, clone_state, find_instance_mut};
use jackioh_engine::state_check::state_check;

use super::fixtures::damage_combat::{bolt, grunt, playing, rattle, snake, wall, warded};
use super::fixtures::harness::{events_json, in_hand, put, slot};

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}

/// A state check over `state`, the rng at its cursor; its events.
fn snake_dies(state: &mut GameState) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        state_check(&mut sink);
    }
    events
}

fn split_from(
    state: &mut GameState,
    self_: Option<&CardInstance>,
    options: HookOptions,
    args: Value,
) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, self_, options);
        apply_effects(&[damage_split(json_as(args))], &mut ctx);
    }
    events
}

/// As `split_from`, for ST-S03-5's `damageRandomEnemies` (R1860004).
fn random_enemies_from(
    state: &mut GameState,
    self_: Option<&CardInstance>,
    options: HookOptions,
    args: Value,
) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, self_, options);
        apply_effects(&[damage_random_enemies(json_as(args))], &mut ctx);
    }
    events
}

mod e37_damage_split {
    use super::*;

    struct Built {
        state: GameState,
        snake_id: String,
        ids: Vec<String>,
    }

    fn build() -> Built {
        let mut state = playing("dc-split");
        let dying = put(
            &mut state,
            &snake.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        {
            let live = find_instance_mut(&mut state, &dying.id).expect("the snake");
            live.counters.plague = Some(3);
            live.marked_destroyed = Some(true);
        }
        let a = put(&mut state, &wall.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let b = put(&mut state, &wall.id, slot(PlayerId::P2, Row::Units, 2), json!({}));
        Built {
            state,
            snake_id: dying.id,
            ids: vec![a.id, b.id, "hero-p2".to_string()],
        }
    }

    #[test]
    fn classic_plus_3s_death_one_1_damage_hit_per_token_on_a_random_enemy_deterministic_from_the_seed() {
        let mut one = build();
        let events = snake_dies(&mut one.state);
        let hits = events_json(&events, "damage");
        assert_eq!(hits.len(), 3);
        for hit in &hits {
            assert_eq!(text(hit, "sourceId"), one.snake_id);
            assert_eq!(hit["amount"], json!(1));
            assert!(one.ids.contains(&text(hit, "targetId")));
        }
        // The same seed and board draw the same enemies (R60: the match rng).
        let mut two = build();
        let again = events_json(&snake_dies(&mut two.state), "damage");
        assert_eq!(
            again.iter().map(|hit| text(hit, "targetId")).collect::<Vec<_>>(),
            hits.iter().map(|hit| text(hit, "targetId")).collect::<Vec<_>>()
        );
    }

    #[test]
    fn an_enemy_an_earlier_hit_killed_is_no_longer_standing_so_no_later_hit_lands_on_it() {
        for seed in ["dc-split-a", "dc-split-b", "dc-split-c", "dc-split-d"] {
            let mut state = playing(seed);
            let source = put(
                &mut state,
                &grunt.id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            let victim = put(
                &mut state,
                &rattle.id,
                slot(PlayerId::P2, Row::Units, 1),
                json!({}),
            );
            let events = split_from(
                &mut state,
                Some(&source),
                HookOptions::default(),
                json!({ "amount": 6, "among": "enemies" }),
            );
            let hits = events_json(&events, "damage");
            assert_eq!(hits.len(), 6);
            assert!(
                hits.iter()
                    .filter(|hit| text(hit, "targetId") == victim.id)
                    .count()
                    <= 1
            );
            let on_hero = hits
                .iter()
                .filter(|hit| text(hit, "targetId") == "hero-p2")
                .count() as i32;
            assert_eq!(state.players.p2.hero.health, 30 - on_hero);
        }
    }

    #[test]
    fn hits_of_more_than_1_the_last_taking_what_is_left_an_enemy_unit_pool_leaves_the_hero_out() {
        let mut state = playing("dc-split-per-hit");
        let source = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let target = put(&mut state, &wall.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let events = split_from(
            &mut state,
            Some(&source),
            HookOptions::default(),
            json!({ "amount": 5, "perHit": 2, "among": "enemyUnits" }),
        );
        assert_eq!(
            events_json(&events, "damage")
                .iter()
                .map(|hit| json!([hit["targetId"], hit["amount"]]))
                .collect::<Vec<_>>(),
            vec![
                json!([target.id, 2]),
                json!([target.id, 2]),
                json!([target.id, 1])
            ]
        );
        let mut empty = clone_state(&state);
        for zone in empty.players.p2.units.iter_mut() {
            *zone = None;
        }
        let quiet = split_from(
            &mut empty,
            None,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
            json!({ "amount": 3, "among": "enemyUnits" }),
        );
        assert_eq!(quiet, Vec::<GameEvent>::new());
    }

    #[test]
    fn e35_a_spells_split_passes_a_unit_immune_to_spells_by() {
        let mut state = playing("dc-split-immune");
        put(
            &mut state,
            &warded.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let spell = in_hand(&mut state, &bolt.id, PlayerId::P1, 1)
            .into_iter()
            .next()
            .expect("no spell");
        let events = split_from(
            &mut state,
            Some(&spell),
            HookOptions::default(),
            json!({ "amount": 4, "among": "enemies" }),
        );
        assert_eq!(
            events_json(&events, "damage")
                .iter()
                .map(|hit| text(hit, "targetId"))
                .collect::<Vec<_>>(),
            vec!["hero-p2", "hero-p2", "hero-p2", "hero-p2"]
        );
    }
}

mod s03_damage_random_enemies {
    use super::*;

    /// Two walls (1/8) and the hero stand: count 2 hits two different enemies, one instance each.
    fn two_walls() -> (GameState, CardInstance) {
        let mut state = playing("dc-random-enemies");
        let source = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        put(&mut state, &wall.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        put(&mut state, &wall.id, slot(PlayerId::P2, Row::Units, 2), json!({}));
        (state, source)
    }

    #[test]
    fn r1860004_count_2_hits_two_different_enemies_one_instance_each() {
        let (mut state, source) = two_walls();
        let events = random_enemies_from(
            &mut state,
            Some(&source),
            HookOptions::default(),
            json!({ "count": 2, "amount": 3 }),
        );
        let hits = events_json(&events, "damage");
        assert_eq!(hits.len(), 2);
        for hit in &hits {
            assert_eq!(text(hit, "sourceId"), source.id);
            assert_eq!(hit["amount"], json!(3));
        }
        let targets: Vec<String> = hits.iter().map(|hit| text(hit, "targetId")).collect();
        let mut unique = targets.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 2, "no enemy takes both hits: {targets:?}");
        let (mut again, source) = two_walls();
        let redrawn = events_json(
            &random_enemies_from(
                &mut again,
                Some(&source),
                HookOptions::default(),
                json!({ "count": 2, "amount": 3 }),
            ),
            "damage",
        );
        assert_eq!(
            redrawn
                .iter()
                .map(|hit| text(hit, "targetId"))
                .collect::<Vec<_>>(),
            targets
        );
    }

    #[test]
    fn r1860004_one_standing_takes_one_fewer_when_fewer_stand() {
        let (mut state, source) = two_walls();
        for zone in state.players.p2.units.iter_mut() {
            *zone = None;
        }
        let events = random_enemies_from(
            &mut state,
            Some(&source),
            HookOptions::default(),
            json!({ "count": 2, "amount": 3 }),
        );
        assert_eq!(
            events_json(&events, "damage")
                .iter()
                .map(|hit| text(hit, "targetId"))
                .collect::<Vec<_>>(),
            vec!["hero-p2".to_string()]
        );
        assert_eq!(state.players.p2.hero.health, 30 - 3);
    }

    #[test]
    fn r1860004_zero_amount_or_zero_count_hits_nothing() {
        let (mut state, source) = two_walls();
        for args in [
            json!({ "count": 0, "amount": 3 }),
            json!({ "count": 2, "amount": 0 }),
        ] {
            let events = random_enemies_from(&mut state, Some(&source), HookOptions::default(), args);
            assert!(events_json(&events, "damage").is_empty());
        }
        assert_eq!(state.players.p2.hero.health, 30);
    }
}
