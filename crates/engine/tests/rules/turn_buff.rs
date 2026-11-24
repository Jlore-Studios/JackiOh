//! R4587000: "+N Attack and <keywords> this turn" (`buffThisTurn`): it holds through the turn
//! given and stacks within it, is gone at the next turn, is cleared by R78's reset, is kept by a
//! copy (R57), and loses its Attack — but keeps its keywords — to a Vanilla (R781003).

use jackioh_engine::effects::transform::vanilla;
use jackioh_engine::effects::turn_buff::{buff_this_turn, turn_buff_of};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{Bench, by, live, live_mut, put, slot};
use crate::rules::fixtures::instance_data::instance_main_phase;

const UNIT: &str = "stand-in (1) 2/2";

/// Apply `buffThisTurn` with this attack and these keywords to the unit, as its controller.
/// Keywords are written as their kinds: `Rush` here is `{ "kind": "Rush" }`.
fn buff(state: &mut GameState, bench: &mut Bench, id: &str, attack: i32, keywords: Vec<Value>) {
    let target = json!({ "of": "instance", "instanceId": id });
    bench.run(state, |sink| {
        let mut ctx = make_context(sink, None, by(PlayerId::P1));
        (buff_this_turn(json_as(json!({ "target": target, "attack": attack, "keywords": keywords }))).apply)(
            &mut ctx,
        );
    });
}

fn rush() -> Vec<Value> {
    vec![json!({ "kind": "Rush" })]
}

fn attack_of(state: &GameState, id: &str) -> i32 {
    unit_view(state, &live(state, id)).attack
}

fn has_rush(state: &GameState, id: &str) -> bool {
    has_keyword(&unit_view(state, &live(state, id)).keywords, KeywordKind::Rush)
}

#[test]
fn r4587000_it_holds_this_turn_and_is_gone_at_the_next() {
    let mut state = instance_main_phase("turn-buff");
    let unit = put(&mut state, UNIT, slot(PlayerId::P1, Row::Units, 1), json!({}));
    let mut bench = Bench::new(&state);
    buff(&mut state, &mut bench, &unit.id, 2, rush());
    assert_eq!(attack_of(&state, &unit.id), 4);
    assert!(has_rush(&state, &unit.id));

    state.turn += 1;
    assert_eq!(attack_of(&state, &unit.id), 2);
    assert!(!has_rush(&state, &unit.id));
    // The stale record stays on the card until something clears it (R78).
    assert!(live(&state, &unit.id).turn_buff.is_some());
}

#[test]
fn r4587000_two_in_one_turn_add_up_and_an_older_one_is_replaced() {
    let mut state = instance_main_phase("turn-buff-stack");
    let unit = put(&mut state, UNIT, slot(PlayerId::P1, Row::Units, 1), json!({}));
    let mut bench = Bench::new(&state);
    buff(&mut state, &mut bench, &unit.id, 2, rush());
    buff(&mut state, &mut bench, &unit.id, 3, rush());
    assert_eq!(attack_of(&state, &unit.id), 7);
    assert_eq!(turn_buff_of(&state, &live(&state, &unit.id)).map(|held| held.keywords.len()), Some(1));

    // An older record, made on a past turn, is replaced rather than added to.
    state.turn += 1;
    buff(&mut state, &mut bench, &unit.id, 3, rush());
    assert_eq!(attack_of(&state, &unit.id), 5);
    assert_eq!(turn_buff_of(&state, &live(&state, &unit.id)).map(|held| held.turn), Some(state.turn));
}

#[test]
fn r4587000_leaving_the_field_clears_it() {
    let mut state = instance_main_phase("turn-buff-reset");
    let unit = put(&mut state, UNIT, slot(PlayerId::P1, Row::Units, 1), json!({}));
    let mut bench = Bench::new(&state);
    buff(&mut state, &mut bench, &unit.id, 2, rush());
    assert!(live(&state, &unit.id).turn_buff.is_some());
    let mut copy = live(&state, &unit.id);
    jackioh_engine::zones::reset_instance(&mut copy);
    *live_mut(&state, &unit.id) = copy;
    assert!(live(&state, &unit.id).turn_buff.is_none());
    assert_eq!(attack_of(&state, &unit.id), 2);
    assert!(!has_rush(&state, &unit.id));
}

#[test]
fn r4587000_a_vanilla_takes_its_attack_and_leaves_its_keywords() {
    let mut state = instance_main_phase("turn-buff-vanilla");
    let unit = put(&mut state, UNIT, slot(PlayerId::P1, Row::Units, 1), json!({}));
    let mut bench = Bench::new(&state);
    buff(&mut state, &mut bench, &unit.id, 2, rush());
    bench.run(&mut state, |sink| {
        let mut ctx = make_context(sink, None, by(PlayerId::P1));
        (vanilla(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id.clone() } }))).apply)(
            &mut ctx,
        );
    });
    assert_eq!(attack_of(&state, &unit.id), 2);
    assert!(has_rush(&state, &unit.id));
}

#[test]
fn r4587000_a_copy_keeps_it() {
    use jackioh_engine::effects::summon_copy;
    use jackioh_engine::zones::card_at;

    let mut state = instance_main_phase("turn-buff-copy");
    let unit = put(&mut state, UNIT, slot(PlayerId::P1, Row::Units, 1), json!({}));
    let mut bench = Bench::new(&state);
    buff(&mut state, &mut bench, &unit.id, 2, rush());
    bench.run(&mut state, |sink| {
        let mut ctx = make_context(sink, None, by(PlayerId::P1));
        (summon_copy(json_as(json!({ "of": { "of": "instance", "instanceId": unit.id } }))).apply)(
            &mut ctx,
        );
    });
    let lane_two = card_at(&state, slot(PlayerId::P1, Row::Units, 2))
        .expect("the copy in lane 2")
        .clone();
    assert_ne!(unit.id, lane_two.id);
    assert_eq!(attack_of(&state, &lane_two.id), 4);
    assert!(has_rush(&state, &lane_two.id));
}
