//! R2271000 (issue #2271): a trigger's printed condition (`TriggerDef.when`, R99) holds when its card is
//! no trap. The traps' dispatch reads a trap's; every other trigger is queued (§10.3, R68), and its
//! condition is read as it resolves, so a Unit or a Field Spell that answers "your opponent" never
//! answers its own controller. A fusion that is no trap reads each ingredient's in its place (R102).
//!
//! Fixtures are this file's own: `fx-` defs indexed from 22710 (BUILD §0).

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::harness::{both, setup_catalog};

/// Damage the warden deals its controller's enemy hero when it answers.
const STING: i32 = 1;

/// "After your opponent plays a Unit: deal 1 damage to the enemy hero", its condition only in `when`,
/// as Meditative #97.8 Prison's is.
fn warden() -> CardDef {
    unit_def(22710, json!({ "attack": 0, "health": 5 }))
}

/// A vanilla Unit to play, and to fuse the warden's text onto.
fn body() -> CardDef {
    unit_def(22711, json!({}))
}

fn warden_script() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("qw-sting", &[GameEventType::CardResolved], |_ctx, _event| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": STING }),
                ))]
            })
            .with_when(|ctx, event| {
                let GameEvent::CardResolved {
                    player, permanent, ..
                } = event
                else {
                    return false;
                };
                *player != ctx.controller && *permanent
            }),
        ],
        ..Script::default()
    }
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for def in [warden(), body()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.insert(warden().id, both(warden_script()));
    register_scripts(scripts);
}

/// Fuses a warden from p's hand onto p's body in unit lane 1, which stays a Unit (R102's type).
fn fuse_warden_onto_body(s: &mut Scenario, player: PlayerId) {
    let state = s.state_mut();
    let hand = &mut state.players[player].hand;
    let at = hand
        .iter()
        .position(|held| held.def_id == warden().id)
        .expect("a warden in the hand");
    let ingredient = hand.remove(at);
    let target = jackioh_engine::zones::active_units_of(state, player)
        .into_iter()
        .find(|card| card.def_id == body().id)
        .cloned()
        .expect("a body on the field");
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    fuse(
        &mut sink,
        FuseArgs {
            ingredients: vec![ingredient],
            target: Some(target),
            ..FuseArgs::default()
        },
    )
    .expect("the warden fuses onto the body");
}

mod r2271000_a_queued_trigger_reads_its_condition {
    use super::*;

    #[test]
    fn r2271000_a_units_condition_admits_only_its_opponents_play_though_it_is_no_trap() {
        register();
        let mut s = scenario(json!({
            "seed": "r2271000-unit",
            "p1": { "field": [warden().id], "hand": [body().id] },
            "p2": { "field": [warden().id] },
        }));
        s.play(body().id.as_str(), json!({ "zone": 2 }));
        // p2's warden answers its opponent's play; p1's own warden lets its controller's play pass.
        s.expect_health(P1, HERO_HEALTH - STING);
        s.expect_health(P2, HERO_HEALTH);
    }

    #[test]
    fn r2271000_a_fusion_that_is_no_trap_reads_each_ingredients_condition() {
        register();
        let mut s = scenario(json!({
            "seed": "r2271000-fused",
            "p1": { "field": [body().id], "hand": [warden().id, body().id] },
            "p2": { "field": [body().id], "hand": [warden().id] },
        }));
        fuse_warden_onto_body(&mut s, P1);
        fuse_warden_onto_body(&mut s, P2);
        s.play(body().id.as_str(), json!({ "zone": 2 }));
        s.expect_health(P1, HERO_HEALTH - STING);
        s.expect_health(P2, HERO_HEALTH);
    }
}
