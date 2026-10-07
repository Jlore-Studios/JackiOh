//! C+ #44 Simplicity Audit and #45 Complexity Audit's sweep (`subsystems/audit.ts`, SPEC §8.7 rows 44
//! and 45, E36), through test-only definitions carrying their own `loc`. The real cards are proved in
//! packages/cards (test/classic-plus/044-simplicity-audit.test.ts, 045-complexity-audit.test.ts).
//!
//! Port of `packages/engine/test/audit.test.ts`.

use jackioh_engine::subsystems::audit::{AuditTargetsArgs, audit_targets, lines_of_code};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{new_game, put, slot};

/// TS `{ ...base, ...extra }`: a shallow merge of two object literals.
fn merged(base: Value, extra: Value) -> Value {
    let mut out = base;
    if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    out
}

fn unit(id: &str, loc: Option<i32>, extra: Value) -> CardDef {
    let mut base = json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": id },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": id },
    });
    if let Some(loc) = loc {
        base["loc"] = json!(loc);
    }
    json_as(merged(base, extra))
}

fn small() -> CardDef {
    unit("au-3", Some(3), json!({}))
}
fn even() -> CardDef {
    unit("au-10", Some(10), json!({}))
}
fn big() -> CardDef {
    unit("au-20", Some(20), json!({}))
}
fn none() -> CardDef {
    unit("au-none", None, json!({}))
}
fn stacker() -> CardDef {
    unit(
        "au-stack",
        Some(40),
        json!({
            "base": { "attack": 1, "health": 1, "keywords": [{ "kind": "Stack" }], "text": "stack" },
            "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Stack" }], "text": "stack" },
        }),
    )
}
fn trap() -> CardDef {
    unit(
        "au-trap",
        Some(4),
        json!({ "type": "Trap", "base": { "keywords": [], "text": "trap" }, "radiant": { "keywords": [], "text": "trap" } }),
    )
}

fn board() -> GameState {
    let mut state = new_game("audit");
    let mut catalog = registered_catalog().clone();
    for def in [small(), even(), big(), none(), stacker(), trap()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.active = P1;
    state
}

fn ids<C: std::borrow::Borrow<CardInstance>>(cards: &[C]) -> Vec<String> {
    cards.iter().map(|card| card.borrow().def_id.clone()).collect()
}

fn controllers<C: std::borrow::Borrow<CardInstance>>(cards: &[C]) -> Vec<PlayerId> {
    cards.iter().map(|card| card.borrow().controller).collect()
}

fn args(controller: PlayerId, active: PlayerId, loc: i32, more: bool, enemy_only: bool) -> AuditTargetsArgs {
    AuditTargetsArgs {
        controller,
        active,
        loc,
        more,
        enemy_only,
    }
}

mod the_audits_sweep_e36 {
    use super::*;

    #[test]
    fn e36_fewer_every_permanent_below_the_audits_loc_on_both_sides_the_active_side_first_an_equal_one_stays() {
        let mut state = board();
        put(&mut state, &small().id, slot(P2, Row::Units, 1));
        put(&mut state, &even().id, slot(P1, Row::Units, 1));
        put(&mut state, &small().id, slot(P1, Row::Units, 2));
        put(&mut state, &big().id, slot(P2, Row::Units, 2));
        assert_eq!(
            ids(&audit_targets(&state, args(P2, P1, 10, false, false))),
            vec![small().id, small().id]
        );
        let targets = audit_targets(&state, args(P2, P1, 10, false, false));
        assert_eq!(controllers(&targets), vec![P1, P2]);
    }

    #[test]
    fn e36_more_every_permanent_above_it_an_equal_one_stays() {
        let mut state = board();
        put(&mut state, &small().id, slot(P1, Row::Units, 1));
        put(&mut state, &even().id, slot(P2, Row::Units, 1));
        put(&mut state, &big().id, slot(P2, Row::Units, 2));
        assert_eq!(ids(&audit_targets(&state, args(P1, P1, 10, true, false))), vec![big().id]);
    }

    #[test]
    fn e36_only_the_opponents_the_controllers_own_permanents_are_left_out() {
        let mut state = board();
        put(&mut state, &small().id, slot(P1, Row::Units, 1));
        put(&mut state, &small().id, slot(P2, Row::Units, 1));
        let targets = audit_targets(&state, args(P1, P1, 10, false, true));
        assert_eq!(controllers(&targets), vec![P2]);
    }

    #[test]
    fn s3_2_face_down_backrow_cards_count_a_card_dormant_under_a_stack_is_not_on_the_field() {
        let mut state = board();
        put(&mut state, &trap().id, slot(P2, Row::Backrow, 1));
        put(&mut state, &small().id, slot(P2, Row::Units, 1));
        let top = new_instance(&mut state, &stacker().id, P2, Zone::Hand { player: P2 });
        assert!(place_on_field(
            &mut state,
            top,
            slot(P2, Row::Units, 1),
            PlaceOnFieldOptions { stack: Some(true) }
        ));
        assert_eq!(ids(&audit_targets(&state, args(P1, P1, 10, false, false))), vec![trap().id]);
        assert_eq!(ids(&audit_targets(&state, args(P1, P1, 10, true, false))), vec![stacker().id]);
    }

    #[test]
    fn r77_e36_a_card_with_no_loc_reads_0_a_fused_cards_loc_is_its_ingredients_sum() {
        let mut state = board();
        assert_eq!(lines_of_code(&state, &none().id), 0);
        let kept = put(&mut state, &small().id, slot(P1, Row::Units, 1));
        let other = new_instance(&mut state, &big().id, P1, Zone::Gone { player: P1 });
        let fused = {
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            fuse(
                &mut sink,
                FuseArgs {
                    ingredients: vec![kept.clone(), other.clone()],
                    target: Some(kept.clone()),
                    ..FuseArgs::default()
                },
            )
        };
        let fused_def = fused.as_ref().map(|card| card.def_id.clone()).unwrap_or_default();
        assert_eq!(lines_of_code(&state, &fused_def), 23);
        assert_eq!(ids(&audit_targets(&state, args(P2, P1, 23, false, false))), Vec::<String>::new());
        assert_eq!(audit_targets(&state, args(P2, P1, 24, false, false)).len(), 1);
    }
}
