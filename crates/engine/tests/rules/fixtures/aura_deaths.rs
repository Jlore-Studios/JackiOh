//! The board of fuzz seed 6180's shape (R4318000, R4379000): an attack whose other hit, a Cleave
//! neighbour's (§4.4 step 10) or a sweep's (R3554008), kills the Unit whose aura holds its target up, so
//! the target falls a pass of the state check later (§4.5 step 1). The combat fixtures' cards, with the
//! scripts these cases give them registered over the engine's own.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::combat::{big_body, charger};
use super::harness::{both, new_game, put, slot};

/// `script` registered for `def_id`, over the scripts already registered. After `new_game`, whose
/// `setup_catalog` registers the engine's own afresh.
pub fn register_script(def_id: &str, script: Script) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(def_id.to_string(), both(script));
    register_scripts(scripts);
}

/// `def_id` as a sweeper whose attacks weigh 1× in its lane and beside it (R3554008).
pub fn register_sweeper(def_id: &str) {
    register_script(
        def_id,
        Script {
            static_flags: Some(StaticFlags {
                sweep_lane: Some(1),
                sweep_adjacent: Some(1),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
    );
}

/// p1's main phase on turn 4, `attacker_def` in p1's lane 1 facing a Big Body in p2's, 8 damage on it,
/// and a Charger (3/3) in p2's lane `aura_lane` whose aura gives p2's other Units +2 Health: the Big Body
/// stands at 5/12 with 4 left only while the Charger does. R345: p1's turn stays open after the attack.
/// The ids of the attacker, the Big Body and the Charger.
pub fn aura_board(seed: &str, attacker_def: &str, aura_lane: i32) -> (GameState, String, String, String) {
    let mut state = new_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players[P1].auto_end_turn = Some(false);
    register_script(
        &charger.id,
        Script {
            aura: Some(aura_hook(|args| {
                vec![AuraEntry {
                    applies: Box::new(move |unit: &CardInstance| {
                        unit.controller == args.self_.controller && unit.id != args.self_.id
                    }),
                    mod_: StatMod {
                        max_health: Some(2),
                        ..StatMod::default()
                    },
                }]
            })),
            ..Script::default()
        },
    );
    let attacker = put(&mut state, attacker_def, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, &big_body.id, slot(P2, Row::Units, 1), json!({}));
    let aura = put(
        &mut state,
        &charger.id,
        slot(P2, Row::Units, aura_lane),
        json!({}),
    );
    find_instance_mut(&mut state, &defender.id)
        .expect("on the field")
        .damage = 8;
    (state, attacker.id, defender.id, aura.id)
}

/// A fresh p2 `def_id` stacked on top of the unit zone `at` (§3.2), whatever lies there now dormant
/// beneath it (R1829000). Its id.
pub fn stack_on(state: &mut GameState, def_id: &str, at: ZoneSlot) -> String {
    let mut top = new_instance(state, def_id, P2, Zone::Hand { player: P2 });
    assert!(
        place_on_field(state, &mut top, at, json_as(json!({ "stack": true }))),
        "{def_id} lands on top of the pile"
    );
    assert!(!beneath_at(state, at).is_empty(), "a card lies beneath it");
    top.id
}
