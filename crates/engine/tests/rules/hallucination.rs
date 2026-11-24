//! The Hallucination verbs (Infinite Assault #25–#48, R4586000–R4586008): the random set cost,
//! the face-down ward, the top-N pile look and the bottom move, the `unitsAttack` lend, the attacker
//! a `trapFired` names, and the hand swap. Each is one effect under `run_effect`, except the firing,
//! which needs a real attack.

use jackioh_engine::effects::{
    choose_pick, destroy, destroy_all, give_units_attack, put_on_library_bottom, reveal,
    set_cost_override_random, swap_hands,
};
use jackioh_engine::modifiers::expire_modifiers;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    anime_armor, gambit, grunt, playing, recorder, veteran, voidwalker, wall, watcher,
};
use crate::rules::fixtures::harness::{
    Bench, by, ids, in_hand, input, live, put, run_effect, set_library, sink_for, slot,
};

/// A Unit whose static flags carry the face-down ward (Infinite Assault #42 Paranoia Engine's
/// shape), registered beside the fixtures by the ward test.
fn warden_def() -> CardDef {
    json_as(json!({
        "id": "hx-warden",
        "index": "4801",
        "name": "Warden (hallucination fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "attack": 2, "health": 6, "keywords": [], "text": "warden" },
        "radiant": { "attack": 4, "health": 12, "keywords": [], "text": "warden" },
    }))
}

fn warden_scripts() -> CardScripts {
    let ward = Script {
        static_flags: Some(StaticFlags {
            face_down_ward: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        base: ward.clone(),
        radiant: ward,
    }
}

fn register_warden() {
    let mut defs = registered_catalog().clone();
    defs.insert(warden_def().id.clone(), warden_def());
    register_catalog(defs);
    let mut scripts = registered_scripts().clone();
    scripts.insert(warden_def().id.clone(), warden_scripts());
    register_scripts(scripts);
}

fn marked(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| card.marked_destroyed == Some(true))
}

#[test]
fn r4586000_set_cost_override_random_stays_in_range_and_draws_once() {
    let mut state = playing("hx-cost-random");
    let unit = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    let before = state.rng_cursor;
    let events = run_effect(
        &mut state,
        set_cost_override_random(json_as(json!({
            "target": { "of": "instance", "instanceId": unit.id },
            "low": 1,
            "high": 4,
        }))),
        P1,
    );
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::CostChanged { instance_id, .. } if *instance_id == unit.id
    )));
    let cost = live(&state, &unit.id).cost_override.expect("a rolled override");
    assert!((1..=4).contains(&cost), "rolled {cost}, outside 1 to 4");
    // One uniform draw, no more: the cursor advances by exactly one draw.
    assert_eq!(state.rng_cursor, before + 1);
}

#[test]
fn r4586000_a_high_below_the_low_clamps_to_the_low_with_no_extra_draw() {
    let mut state = playing("hx-cost-clamp");
    let unit = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    let before = state.rng_cursor;
    run_effect(
        &mut state,
        set_cost_override_random(json_as(json!({
            "target": { "of": "instance", "instanceId": unit.id },
            "low": 5,
            "high": 3,
        }))),
        P1,
    );
    assert_eq!(live(&state, &unit.id).cost_override, Some(5));
    assert_eq!(state.rng_cursor, before + 1);
}

#[test]
fn r4586004_a_ward_stops_the_opponents_destroy_destroy_all_and_reveal_not_the_controllers() {
    let mut state = playing("hx-ward");
    register_warden();
    put(&mut state, &warden_def().id, slot(P1, Row::Units, 1), json!({}));
    let first = put(&mut state, &gambit.id, slot(P1, Row::Backrow, 1), json!({}));
    // Both traps sit under the ward: the opponent's effects pass them over.
    let second = put(&mut state, &gambit.id, slot(P1, Row::Backrow, 2), json!({}));
    assert!(is_face_down(&state, &live(&state, &first.id)));
    // The opponent's `destroy` passes the warded face-down card by; the controller's own lands.
    run_effect(
        &mut state,
        destroy(json_as(
            json!({ "target": { "of": "instance", "instanceId": first.id } }),
        )),
        P2,
    );
    assert!(!marked(&state, &first.id));
    run_effect(
        &mut state,
        destroy(json_as(
            json!({ "target": { "of": "instance", "instanceId": first.id } }),
        )),
        P1,
    );
    assert!(marked(&state, &first.id));
    // The opponent's `destroyAll` passes both warded cards by; the controller's own marks.
    run_effect(
        &mut state,
        destroy_all(json_as(json!({ "side": "enemy", "rows": ["backrow"] }))),
        P2,
    );
    assert!(!marked(&state, &second.id));
    run_effect(
        &mut state,
        destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] }))),
        P1,
    );
    assert!(marked(&state, &first.id));
    // The opponent's `reveal` leaves it face-down; the controller's own shows it.
    run_effect(
        &mut state,
        reveal(json_as(
            json!({ "target": { "of": "instance", "instanceId": second.id } }),
        )),
        P2,
    );
    assert!(is_face_down(&state, &live(&state, &second.id)));
    run_effect(
        &mut state,
        reveal(json_as(
            json!({ "target": { "of": "instance", "instanceId": second.id } }),
        )),
        P1,
    );
    assert!(!is_face_down(&state, &live(&state, &second.id)));
}

#[test]
fn r4586005_pile_top_offers_the_top_n_in_order_and_bottom_moves_the_picks() {
    let mut state = playing("hx-top-n");
    let dealt = set_library(
        &mut state,
        P2,
        &[&grunt.id, &wall.id, &veteran.id, &voidwalker.id],
    );
    // The top two, in library order: the third and fourth are not offered.
    run_effect(
        &mut state,
        choose_pick(json_as(json!({
            "step": "narrator",
            "from": [{ "zone": "library", "player": "enemy", "top": 2 }],
            "min": 0,
            "max": 2,
        }))),
        P1,
    );
    let pending = state.pending.clone().expect("the pick prompt");
    let offered: Vec<String> = pending
        .options
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(offered, vec![dealt[0].id.clone(), dealt[1].id.clone()]);
    // The picked two go to the bottom in pick order; the rest keep theirs.
    let mut sink = sink_for(&mut state);
    let mut ctx = make_context(&mut sink, None, by(P1));
    ctx.targets = vec![
        Selection::Instance {
            instance_id: dealt[1].id.clone(),
        },
        Selection::Instance {
            instance_id: dealt[0].id.clone(),
        },
    ];
    apply_effects(&[put_on_library_bottom(json_as(json!({})))], &mut ctx);
    let order: Vec<String> = ids(&sink.state.players[P2].library);
    assert_eq!(
        order,
        vec![
            dealt[2].id.clone(),
            dealt[3].id.clone(),
            dealt[1].id.clone(),
            dealt[0].id.clone(),
        ]
    );
}

#[test]
fn r4586006_units_attack_reaches_later_units_and_expires_at_the_next_turn() {
    let mut state = playing("hx-units-attack");
    let first = put(&mut state, &voidwalker.id, slot(P2, Row::Units, 1), json!({}));
    run_effect(
        &mut state,
        give_units_attack(json_as(json!({ "player": "enemy", "amount": -2 }))),
        P1,
    );
    // Cast on the caster's turn: it lasts through the affected side's next turn.
    let expiry = state.players[P2]
        .mods
        .iter()
        .find(|modifier| matches!(modifier.kind, ModifierKind::UnitsAttack { .. }))
        .expect("the debuff modifier")
        .expiry
        .clone();
    assert!(matches!(
        expiry,
        ModifierExpiry::NextTurnOf { player, from_turn } if player == P2 && from_turn == state.turn
    ));
    assert_eq!(unit_view(&state, &live(&state, &first.id)).attack, 4);
    // A Unit arriving later reads the same lend, floored at 0.
    let later = put(&mut state, &anime_armor.id, slot(P2, Row::Units, 2), json!({}));
    assert_eq!(unit_view(&state, &live(&state, &later.id)).attack, 2);
    let small = put(&mut state, &grunt.id, slot(P2, Row::Units, 3), json!({}));
    assert_eq!(unit_view(&state, &live(&state, &small.id)).attack, 0);
    // The affected side's cleanup past the turn it was made ends it.
    state.turn += 1;
    state.active = P2;
    let mut bench = Bench::new(&state);
    bench.run(&mut state, |sink| expire_modifiers(sink, P2));
    assert_eq!(unit_view(&state, &live(&state, &first.id)).attack, 6);
    assert_eq!(unit_view(&state, &live(&state, &later.id)).attack, 4);
}

#[test]
fn r4586007_trap_fired_names_the_attacker_of_the_attack_it_answered() {
    let mut state = playing("hx-attacker");
    put(&mut state, &watcher.id, slot(P2, Row::Backrow, 1), json!({}));
    let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    let mut game = recorder(&state);
    let result = game.play(input(json!({
        "type": "attack", "attackerId": striker.id, "targetId": "hero-p2", "playerId": "p1"
    })));
    assert!(result.error.is_none(), "the attack resolves: {:?}", result.error);
    let fired: Vec<&GameEvent> = result
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::TrapFired { .. }))
        .collect();
    assert_eq!(fired.len(), 1, "the watcher fired once");
    assert!(
        matches!(
            fired[0],
            GameEvent::TrapFired { attacker_id: Some(attacker), .. } if *attacker == striker.id
        ),
        "the firing names its attacker: {:?}",
        fired[0]
    );
}

#[test]
fn r4586008_swap_hands_swaps_full_hands_without_burning() {
    let mut state = playing("hx-swap");
    state.players[P1].hand.clear();
    state.players[P2].hand.clear();
    let mine = in_hand(&mut state, &grunt.id, P1, 10);
    let theirs = in_hand(&mut state, &wall.id, P2, 10);
    let mine_ids: Vec<String> = ids(&mine);
    let theirs_ids: Vec<String> = ids(&theirs);
    let events = run_effect(&mut state, swap_hands(json_as(json!({ "costMod": -1 }))), P1);
    // Both hands move whole: ten and ten, nothing burned, every card stolen over.
    assert_eq!(state.players[P1].hand.len(), 10);
    assert_eq!(state.players[P2].hand.len(), 10);
    assert_eq!(ids(&state.players[P1].hand), theirs_ids);
    assert_eq!(ids(&state.players[P2].hand), mine_ids);
    for card in state.players[P1].hand.iter().chain(state.players[P2].hand.iter()) {
        let holder = match card.zone {
            Zone::Hand { player } => player,
            _ => panic!("a swapped card is in no hand"),
        };
        assert_eq!(card.owner, holder);
        assert_eq!(card.controller, holder);
    }
    // The controller's taken cards carry the discount; the other side's keep their price.
    for card in &state.players[P1].hand {
        assert_eq!(card.cost_mod, -1, "the taken card costs 1 less");
    }
    for card in &state.players[P2].hand {
        assert_eq!(card.cost_mod, 0);
    }
    let stolen = events
        .iter()
        .filter(|event| matches!(event, GameEvent::Stolen { .. }))
        .count();
    assert_eq!(stolen, 20, "each moved card is stolen over");
}
