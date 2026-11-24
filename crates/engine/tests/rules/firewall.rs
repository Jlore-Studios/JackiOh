//! C7's board facts (IA #148, #157, #168; R4591000, R4591001, R4591004): a set Attack this
//! turn, Containment, and Units immune to damage — each applied straight through its verb on a
//! testkit `scenario` with stand-ins, as `hero_immune.rs` does.

use jackioh_engine::effects::{
    bounce, buff, contain_until_next_turn, damage, make_units_immune, set_attack_this_turn,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{by, sink_for};

const FIVE_FIVE: &str = "stand-in (1) 5/5";
const SHIELDED: &str = "stand-in (1) 4/4 Divine Shield";
const WEAK: &str = "stand-in (1) 1/1";

fn body() -> Value {
    json!({
        "seed": "firewall",
        "p1": { "field": [FIVE_FIVE], "hand": [WEAK], "library": [WEAK, WEAK, WEAK, WEAK] },
        "p2": { "field": [FIVE_FIVE], "hand": [WEAK], "library": [WEAK, WEAK, WEAK, WEAK] },
    })
}

/// `scenario` over `setup`: the real cards registered first, so the filler deck builds — the
/// engine's dev-dependency on the cards runs the real catalog here (as the golden binary does).
fn start(setup: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(setup)
}

fn instance_arg(id: &str) -> Value {
    json!({ "target": { "of": "instance", "instanceId": id } })
}

fn apply(state: &mut GameState, effect: Effect, controller: PlayerId) {
    let mut sink = sink_for(state);
    apply_effects(&[effect], &mut make_context(&mut sink, None, by(controller)));
}

fn set_attack(state: &mut GameState, id: &str, attack: i32, controller: PlayerId) {
    let mut args = instance_arg(id);
    args["attack"] = json!(attack);
    apply(state, set_attack_this_turn(json_as(args)), controller);
}

fn buffed(state: &mut GameState, id: &str, attack: i32, health: i32, controller: PlayerId) {
    let mut args = instance_arg(id);
    args["attack"] = json!(attack);
    args["health"] = json!(health);
    apply(state, buff(json_as(args)), controller);
}

fn contained(state: &mut GameState, id: &str, controller: PlayerId) {
    apply(
        state,
        contain_until_next_turn(json_as(instance_arg(id))),
        controller,
    );
}

fn units_immune(state: &mut GameState, controller: PlayerId) {
    apply(state, make_units_immune(json_as(json!({}))), controller);
}

fn hit(state: &mut GameState, id: &str, amount: i32, controller: PlayerId) {
    let mut args = instance_arg(id);
    args["to"] = args["target"].clone();
    args.as_object_mut().expect("args").remove("target");
    args["amount"] = json!(amount);
    apply(state, damage(json_as(args)), controller);
}

fn offers_attack(s: &Scenario, id: &str) -> bool {
    let unit = s.card(id).clone();
    legal_actions(s.state(), unit.controller)
        .iter()
        .any(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == unit.id))
}

fn cannot_be_attacked_by(state: &GameState, unit: &CardInstance, attacker: &CardInstance) -> bool {
    why_cannot_attack(
        state,
        attacker,
        &DamageTarget::Unit {
            instance: unit.clone(),
        },
    )
    .is_err()
}

mod r4591000_an_attack_set_this_turn_beats_buffs_and_ends_with_the_turn {
    use super::*;

    #[test]
    fn r4591000_a_5_5_buffed_2_2_reads_attack_1_then_7_after_the_turn() {
        let mut s = start(body());
        let id = s.unit(P1, 1).map(|unit| unit.id).unwrap();
        buffed(s.state_mut(), &id, 2, 2, P1);
        s.expect_stats(id.as_str(), json!({ "attack": 7, "health": 7 }));
        set_attack(s.state_mut(), &id, 1, P1);
        s.expect_stats(id.as_str(), json!({ "attack": 1, "health": 7 }));
        s.end_turn();
        s.expect_stats(id.as_str(), json!({ "attack": 7, "health": 7 }));
    }

    #[test]
    fn r4591000_leaving_the_field_clears_it() {
        let mut s = start(body());
        let id = s.unit(P1, 1).map(|unit| unit.id).unwrap();
        set_attack(s.state_mut(), &id, 1, P1);
        assert_eq!(
            s.card(id.as_str()).attack_this_turn.map(|set| set.attack),
            Some(1)
        );
        apply(s.state_mut(), bounce(json_as(instance_arg(&id))), P1);
        assert_eq!(s.card(id.as_str()).attack_this_turn, None);
        let bounced = s.card(id.as_str()).clone();
        s.play(&bounced, json!({ "zone": 1 }));
        s.expect_stats(id.as_str(), json!({ "attack": 5, "health": 5 }));
    }
}

mod r4591001_a_contained_unit_neither_attacks_nor_is_attacked_until_the_casters_next_turn {
    use super::*;

    #[test]
    fn r4591001_containment_bars_both_ways_then_ends() {
        let mut s = start(body());
        let mine = s.unit(P1, 1).map(|unit| unit.id).unwrap();
        let theirs = s.unit(P2, 1).map(|unit| unit.id).unwrap();
        contained(s.state_mut(), &theirs, P1);
        assert!(
            cannot_be_attacked_by(
                s.state(),
                &s.card(theirs.as_str()).clone(),
                &s.card(mine.as_str()).clone()
            ),
            "a contained unit is attacked by nothing"
        );
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert!(
            !offers_attack(&s, &theirs),
            "a contained unit attacks nothing on the intervening turn"
        );
        s.end_turn();
        assert_eq!(s.state().active, P1, "the caster's next turn begins");
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert!(offers_attack(&s, &theirs), "containment ends");
        assert!(
            !cannot_be_attacked_by(
                s.state(),
                &s.card(mine.as_str()).clone(),
                &s.card(theirs.as_str()).clone()
            ),
            "attackable again"
        );
    }
}

mod r4591004_an_immune_unit_takes_no_hit_and_keeps_divine_shield {
    use super::*;

    #[test]
    fn r4591004_no_damage_instance_so_the_shield_stays() {
        let mut s = start(json!({
            "seed": "firewall-immune",
            "p1": { "field": [SHIELDED], "library": [WEAK, WEAK, WEAK, WEAK] },
            "p2": { "field": [FIVE_FIVE], "library": [WEAK, WEAK, WEAK, WEAK] },
        }));
        let id = s.unit(P1, 1).map(|unit| unit.id).unwrap();
        units_immune(s.state_mut(), P1);
        hit(s.state_mut(), &id, 3, P2);
        assert_eq!(s.card(id.as_str()).damage, 0);
        assert_eq!(
            s.card(id.as_str()).divine_shield_spent,
            None,
            "no damage instance took the shield"
        );
    }

    #[test]
    fn r4591004_a_unit_summoned_later_is_immune_too_and_it_ends_as_the_next_turn_begins() {
        let mut s = start(body());
        let first = s.unit(P1, 1).map(|unit| unit.id).unwrap();
        units_immune(s.state_mut(), P1);
        s.play(WEAK, json!({ "zone": 2 }));
        let second = s.unit(P1, 2).map(|unit| unit.id).unwrap();
        hit(s.state_mut(), &first, 3, P2);
        hit(s.state_mut(), &second, 3, P2);
        assert_eq!(s.card(first.as_str()).damage, 0);
        assert_eq!(s.card(second.as_str()).damage, 0, "summoned into the window");
        s.end_turn();
        hit(s.state_mut(), &first, 3, P2);
        assert_eq!(
            s.card(first.as_str()).damage,
            0,
            "still holds on the opponent's turn"
        );
        s.end_turn();
        hit(s.state_mut(), &first, 3, P2);
        assert_eq!(s.card(first.as_str()).damage, 3, "gone as the next turn begins");
    }
}
