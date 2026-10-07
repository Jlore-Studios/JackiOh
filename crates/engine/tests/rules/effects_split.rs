//! B5 E37 random split damage (`effects/split.ts`): "deal N damage split among enemies" — N hits of 1,
//! each a damage instance of its own on a random enemy still standing (Classic+ #3's Death).
//!
//! Port of `packages/engine/test/effects-split.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::effects::damage_split;
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::EngineSink;
use jackioh_engine::state::{CardInstance, GameState, clone_state, find_instance_mut};
use jackioh_engine::state_check::state_check;

use super::fixtures::damage_combat::{bolt, grunt, playing, rattle, snake, wall, warded};
use super::fixtures::harness::{in_hand, put, slot};

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

/// A JSON string field, as an owned string.
fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}

/// A state check on a sink over `state` (`sinkFor`: the rng at the state's cursor), handing back the
/// events it emitted.
fn snake_dies(state: &mut GameState) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        state_check(&mut sink);
    }
    events
}

/// `applyEffects([effect], makeContext(sinkFor(state), self, options))`, handing back the events.
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
        let hits = of_type(&events, "damage");
        assert_eq!(hits.len(), 3);
        for hit in &hits {
            assert_eq!(text(hit, "sourceId"), one.snake_id);
            assert_eq!(hit["amount"], json!(1));
            assert!(one.ids.contains(&text(hit, "targetId")));
        }
        // The same seed and board draw the same enemies (R60: the match rng).
        let mut two = build();
        let again = of_type(&snake_dies(&mut two.state), "damage");
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
            let hits = of_type(&events, "damage");
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
            of_type(&events, "damage")
                .iter()
                .map(|hit| json!([hit["targetId"], hit["amount"]]))
                .collect::<Vec<_>>(),
            vec![
                json!([target.id, 2]),
                json!([target.id, 2]),
                json!([target.id, 1])
            ]
        );
        // With no enemy unit standing there is nothing to hit.
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
            of_type(&events, "damage")
                .iter()
                .map(|hit| text(hit, "targetId"))
                .collect::<Vec<_>>(),
            vec!["hero-p2", "hero-p2", "hero-p2", "hero-p2"]
        );
    }
}
