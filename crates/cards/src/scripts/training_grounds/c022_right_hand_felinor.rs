//! SPEC §8.11 Training Grounds 22 Right-Hand Felinor: (2) Unit, Felinor, Rare, 3/3 → 6/6, a card of the
//! held Training Grounds set (R1420, R2194000).
//!
//! Base:    "Cry: If this is to the right of a Unit, give this and that Unit +3/+3."
//! Radiant: "Cry: If this is to the right of a Unit, give this and that Unit +6/+6."
//! Engine: §3's lanes run 1 to 5 left to right from the owner's seat, so the Unit to its left is the pile
//! top one lane lower on its own side and row (R2196002), read as the Cry resolves; never a Dormant one
//! (R1839000), and beside a Felinor standing in the backrow only a standing Unit (`is_adjacent_card`,
//! R1081000). The Cry fires only on a play (R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-022";

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(me) = ctx.live_self() else {
        return vec![];
    };
    let Some(at) = slot_of(&*ctx.state, me) else {
        return vec![];
    };
    let left = ZoneSlot { lane: at.lane - 1, ..at };
    if left.lane < 1 || !in_units_row_or_standing(&*ctx.state, me) {
        return vec![];
    }
    let Some(neighbour) = card_at(&*ctx.state, left)
        .filter(|card| !is_slumbering(card) && is_adjacent_card(&*ctx.state, me, card))
        .map(|card| card.id.clone())
    else {
        return vec![];
    };
    let amount = param(&*ctx, "buff");
    vec![
        buff(json_as(json!({ "target": { "of": "self" }, "attack": amount, "health": amount }))),
        buff(json_as(json!({
            "target": { "of": "instance", "instanceId": neighbour },
            "attack": amount,
            "health": amount,
        }))),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };
    // The same script: the Radiant face's 6 is its declared `buff`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #22 Right-Hand Felinor — SPEC §8.11 row 22, BUILD M13 row TG 22 (R1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FELINOR: &str = super::ID;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const TOP: &str = "core-092"; // Felinor Fiender, Stack
    const FILLER: &str = "core-005";

    fn felinor(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FELINOR, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn unbuffed(s: &mut Scenario, card: &CardInstance) {
        assert_eq!(s.card(card).buffs, AttackHealth { attack: 0, health: 0 }, "{}", card.def_id);
    }

    mod base {
        use super::*;

        #[test]
        fn r2196002_with_a_friendly_unit_in_the_lane_to_its_left_both_gain_3_3() {
            let mut s = felinor(false, json!([{ "def": BODY, "lane": 2 }]), json!([]));
            s.play(FELINOR, json!({ "zone": 3 }));
            s.expect_stats(FELINOR, json!({ "attack": 6, "health": 6 }));
            let body = s.unit_at(P1, 2);
            s.expect_stats(&body, json!({ "attack": 7, "health": 7, "maxHealth": 7 }));
        }

        #[test]
        fn r2196002_a_unit_only_to_its_right_gives_nothing() {
            let mut s = felinor(false, json!([{ "def": BODY, "lane": 4 }]), json!([]));
            s.play(FELINOR, json!({ "zone": 3 }));
            let felinor = s.card(FELINOR).clone();
            unbuffed(&mut s, &felinor);
            let body = s.unit_at(P1, 4);
            unbuffed(&mut s, &body);
        }

        #[test]
        fn r2196002_in_lane_1_nothing() {
            let mut s = felinor(false, json!([{ "def": BODY, "lane": 2 }]), json!([]));
            s.play(FELINOR, json!({ "zone": 1 }));
            s.expect_stats(FELINOR, json!({ "attack": 3, "health": 3 }));
            let body = s.unit_at(P1, 2);
            unbuffed(&mut s, &body);
        }

        #[test]
        fn r2196002_an_enemy_unit_in_the_lane_to_its_left_is_never_beside_it() {
            let mut s = felinor(false, json!([]), json!([{ "def": BODY, "lane": 2 }]));
            s.play(FELINOR, json!({ "zone": 3 }));
            s.expect_stats(FELINOR, json!({ "attack": 3, "health": 3 }));
            let enemy = s.unit_at(P2, 2);
            unbuffed(&mut s, &enemy);
        }

        #[test]
        fn r13_the_pile_top_to_its_left_is_buffed_not_the_unit_beneath() {
            let mut s = felinor(
                false,
                json!([{ "def": BODY, "lane": 2 }, { "def": TOP, "lane": 2, "stack": true }]),
                json!([]),
            );
            let pile: Vec<CardInstance> =
                pile_at(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 2 }).expect("a pile").to_vec();
            s.play(FELINOR, json!({ "zone": 3 }));
            let (top, beneath) = (&pile[0], &pile[1]);
            assert_eq!(top.def_id, TOP);
            assert_eq!(s.card(top).buffs, AttackHealth { attack: 3, health: 3 });
            unbuffed(&mut s, beneath);
        }

        #[test]
        fn r1_summoned_rather_than_played_it_buffs_nothing() {
            let mut s = felinor(false, json!([{ "def": BODY, "lane": 2 }]), json!([]));
            crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({ "defId": FELINOR, "player": "self", "lane": 3 }))),
                P1,
            );
            let summoned = s.unit_at(P1, 3);
            unbuffed(&mut s, &summoned);
            let body = s.unit_at(P1, 2);
            unbuffed(&mut s, &body);
        }

        #[test]
        fn r1_a_copy_of_it_beside_a_friendly_unit_buffs_nothing() {
            // The source is unbuffed (a copy keeps buffs, R57); the copy lands right of the 4/4.
            let mine = json!([{ "def": BODY, "lane": 2 }, { "def": FELINOR, "lane": 5 }]);
            let mut s = felinor(false, mine, json!([]));
            let source = s.unit_at(P1, 5);
            crate::run_as(
                &mut s,
                jackioh_engine::effects::summon_copy(json_as(json!({
                    "of": { "of": "instance", "instanceId": source.id },
                    "lane": 3,
                }))),
                P1,
            );
            let copy = s.unit_at(P1, 3);
            assert_eq!(copy.def_id, FELINOR);
            assert_ne!(copy.id, source.id);
            unbuffed(&mut s, &copy);
            s.expect_stats(&copy, json!({ "attack": 3, "health": 3 }));
            let body = s.unit_at(P1, 2);
            unbuffed(&mut s, &body);
        }

        #[test]
        fn r386_an_upgrade_gives_4_4_a_degrade_2_2() {
            for (step, gain) in [(1, 4), (-1, 2)] {
                let mut s = felinor(false, json!([{ "def": BODY, "lane": 2 }]), json!([]));
                step_param(s.card_mut(FELINOR), "buff", step);
                s.play(FELINOR, json!({ "zone": 3 }));
                s.expect_stats(FELINOR, json!({ "attack": 3 + gain, "health": 3 + gain }));
                let body = s.unit_at(P1, 2);
                s.expect_stats(&body, json!({ "attack": 4 + gain }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2196002_a_6_6_that_gives_both_6_6() {
            let mut s = felinor(true, json!([{ "def": BODY, "lane": 4 }]), json!([]));
            s.play(FELINOR, json!({ "zone": 5 }));
            s.expect_stats(FELINOR, json!({ "attack": 12, "health": 12 }));
            let body = s.unit_at(P1, 4);
            s.expect_stats(&body, json!({ "attack": 10, "health": 10 }));
        }
    }
}
