//! A hero immune to damage (`makeHeroImmune`, `ModifierKind::HeroImmune`): every damage instance
//! to that hero is 0 — combat, effects and fatigue alike — so Lifesteal against it heals nothing,
//! while Lose health and Set health still apply. It lasts until its player's next turn begins.
//! Meditative #48 Tranquility (MD-C21, R1021) is its user.

use jackioh_engine::effects::{damage, draw, lose_health, make_hero_immune, set_health};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{act, grunt, playing, recorder};
use crate::rules::fixtures::harness::{put, sink_for, slot};

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn immune(state: &mut GameState, player: PlayerId) {
    let mut sink = sink_for(state);
    apply_effects(
        &[make_hero_immune(json_as(json!({})))],
        &mut make_context(&mut sink, None, by(player)),
    );
}

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn immune_mods(state: &GameState, player: PlayerId) -> usize {
    state.players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, jackioh_engine::state::ModifierKind::HeroImmune))
        .count()
}

mod r1021_a_hero_immune_to_damage {
    use super::*;

    #[test]
    fn r1021_every_hit_to_the_hero_is_0_combat_effect_and_fatigue() {
        let mut state = playing("dc-immune-hits");
        // p2's hero is immune; p1's attacker is active, so the combat hit plays at once.
        immune(&mut state, P2);
        assert_eq!(immune_mods(&state, P2), 1);
        let attacker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        // An effect hit.
        {
            let mut sink = sink_for(&mut state);
            apply_effects(
                &[damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 5 }),
                ))],
                &mut make_context(&mut sink, None, by(P1)),
            );
            assert_eq!(hero_health(sink.state, P2), 30);
            assert!(
                sink.events
                    .iter()
                    .all(|event| !matches!(event, GameEvent::Damage { .. }))
            );
        }
        // A combat hit.
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        assert_eq!(hero_health(game.state(), P2), 30);
        // Fatigue: an empty library draws nothing and takes no hit.
        let mut empty = game.state().clone();
        empty.players.p2.library.clear();
        let mut sink = sink_for(&mut empty);
        apply_effects(
            &[draw(json_as(json!({ "count": 1 })))],
            &mut make_context(&mut sink, None, by(P2)),
        );
        assert_eq!(hero_health(sink.state, P2), 30);
    }

    #[test]
    fn r1021_lifesteal_against_it_heals_nothing() {
        let mut state = playing("dc-immune-lifesteal");
        immune(&mut state, P1);
        // p2's hero is hurt first, so a heal would show.
        {
            let mut sink = sink_for(&mut state);
            apply_effects(
                &[lose_health(json_as(json!({ "player": "self", "amount": 5 })))],
                &mut make_context(&mut sink, None, by(P2)),
            );
        }
        assert_eq!(hero_health(&state, P2), 25);
        {
            let mut sink = sink_for(&mut state);
            apply_effects(
                &[damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 5, "lifesteal": true }),
                ))],
                &mut make_context(&mut sink, None, by(P2)),
            );
        }
        // The hit was 0, so Lifesteal healed nothing.
        assert_eq!(hero_health(&state, P1), 30);
        assert_eq!(hero_health(&state, P2), 25);
    }

    #[test]
    fn r1021_lose_health_and_set_health_still_apply() {
        let mut state = playing("dc-immune-set");
        immune(&mut state, P1);
        {
            let mut sink = sink_for(&mut state);
            apply_effects(
                &[lose_health(json_as(json!({ "player": "self", "amount": 5 })))],
                &mut make_context(&mut sink, None, by(P1)),
            );
        }
        assert_eq!(hero_health(&state, P1), 25);
        {
            let mut sink = sink_for(&mut state);
            apply_effects(
                &[set_health(json_as(
                    json!({ "to": { "of": "selfHero" }, "value": 10 }),
                ))],
                &mut make_context(&mut sink, None, by(P1)),
            );
        }
        assert_eq!(hero_health(&state, P1), 10);
    }

    #[test]
    fn r1021_it_ends_as_its_players_next_turn_begins() {
        let mut state = playing("dc-immune-turns");
        immune(&mut state, P1);
        // Through the opponent's turn it holds.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(state.active, P2);
        assert_eq!(immune_mods(&state, P1), 1);
        // Gone as its player's next turn begins.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(state.active, P1);
        assert_eq!(immune_mods(&state, P1), 0);
        // And a hit lands again.
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[damage(json_as(
                json!({ "to": { "of": "selfHero" }, "amount": 5 }),
            ))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        assert_eq!(hero_health(sink.state, P1), 25);
    }

    #[test]
    fn r1021_made_on_the_opponents_turn_it_ends_at_that_turns_cleanup() {
        let mut state = playing("dc-immune-theirs");
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(state.active, P2);
        immune(&mut state, P1);
        assert_eq!(immune_mods(&state, P1), 1);
        // The opponent's cleanup ends it: gone as its player's turn begins.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(state.active, P1);
        assert_eq!(immune_mods(&state, P1), 0);
    }

    #[test]
    fn r1021_the_view_shows_its_badge() {
        let mut state = playing("dc-immune-badge");
        immune(&mut state, P1);
        let seen = jackioh_engine::view_for(&state, P1);
        let labels: Vec<&str> = seen
            .you
            .modifiers
            .iter()
            .map(|modifier| modifier.label.as_str())
            .collect();
        assert!(
            labels.iter().any(|label| label.contains("immune to damage")),
            "{labels:?}"
        );
        // Both seats carry it (R169).
        let foe = jackioh_engine::view_for(&state, P2);
        assert!(
            foe.opponent
                .modifiers
                .iter()
                .any(|modifier| modifier.label.contains("immune to damage"))
        );
    }
}
