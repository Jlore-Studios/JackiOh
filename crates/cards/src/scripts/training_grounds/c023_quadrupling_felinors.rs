//! SPEC §8.11 Training Grounds 23 Quadrupling Felinors: (3) Unit, Felinor, Rare, 3/4 → 6/8, a card of the
//! held Training Grounds set (R1420, R2194000).
//!
//! Base:    "Cry: Summon 3 copies of this."
//! Radiant: "Cry: Summon 3 copies of this."
//! Engine: Core #12 Duplicating Felinors' `summon_copy` of itself, `copies` times: each copy is R57's
//! (Radiant flag and buffs kept, damage reset), in the leftmost free zone (R64) or silently none (§3.2).
//! Copies are summons and fire no Cry (R1), so the chain ends after one round.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-023";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            (0..param(&*ctx, "copies"))
                .map(|_| summon_copy(json_as(json!({ "of": { "of": "self" }, "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    };
    // The same script: the copies of a Radiant one are Radiant (R57).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #23 Quadrupling Felinors — SPEC §8.11 row 23, BUILD M13 row TG 23 (R57).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const FELINORS: &str = super::ID;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const FILLER: &str = "core-005";

    fn felinors(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FELINORS, "radiant": radiant }, FILLER], "field": field },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn row(s: &Scenario) -> Vec<Option<String>> {
        (1..=UNIT_ZONES).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect()
    }

    fn some(id: &str) -> Option<String> {
        Some(id.to_string())
    }

    fn copies_of(s: &Scenario) -> Vec<CardInstance> {
        (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).filter(|card| card.def_id == FELINORS).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r64_on_an_empty_row_four_3_4_felinors() {
            let mut s = felinors(false, json!([]));
            s.play(FELINORS, json!({ "zone": 3 }));
            assert_eq!(row(&s), vec![some(FELINORS); 4].into_iter().chain([None]).collect::<Vec<_>>());
            for card in copies_of(&s) {
                s.expect_stats(&card, json!({ "attack": 3, "health": 4 }));
            }
        }

        #[test]
        fn r1_the_copies_fire_no_cry_so_exactly_three_are_made() {
            let mut s = felinors(false, json!([]));
            s.play(FELINORS, json!({}));
            let summons = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Summoned)
                .count();
            // The played one's own arrival, then its three copies.
            assert_eq!(summons, 4);
            assert_eq!(copies_of(&s).len(), 4);
        }

        #[test]
        fn s3_2_with_two_free_zones_left_two_copies_and_no_error() {
            let mut s = felinors(false, json!([{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 2 }]));
            s.play(FELINORS, json!({ "zone": 3 }));
            assert_eq!(row(&s), vec![some(BODY), some(BODY), some(FELINORS), some(FELINORS), some(FELINORS)]);
        }

        #[test]
        fn r57_a_buff_on_the_hand_instance_is_carried_its_damage_is_not() {
            let mut s = felinors(false, json!([]));
            {
                let hand = s.card_mut(FELINORS);
                hand.buffs = AttackHealth { attack: 2, health: 2 };
                hand.damage = 1;
            }
            s.play(FELINORS, json!({ "zone": 1 }));
            for lane in 2..=4 {
                let copy = s.unit_at(P1, lane);
                s.expect_stats(&copy, json!({ "attack": 5, "health": 6, "damage": 0 }));
            }
        }

        #[test]
        fn r1_summoned_itself_it_makes_no_copies() {
            let mut s = felinors(false, json!([]));
            crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({ "defId": FELINORS, "player": "self", "lane": 3 }))),
                P1,
            );
            assert_eq!(row(&s), vec![None, None, some(FELINORS), None, None]);
        }

        #[test]
        fn r386_an_upgrade_makes_four_copies_a_degrade_two() {
            for (step, bodies) in [(1, 5), (-1, 3)] {
                let mut s = felinors(false, json!([]));
                step_param(s.card_mut(FELINORS), "copies", step);
                s.play(FELINORS, json!({}));
                assert_eq!(copies_of(&s).len(), bodies, "step {step}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r57_four_radiant_6_8s() {
            let mut s = felinors(true, json!([]));
            s.play(FELINORS, json!({}));
            let all = copies_of(&s);
            assert_eq!(all.len(), 4);
            for card in all {
                assert!(s.card(&card).radiant);
                s.expect_stats(&card, json!({ "attack": 6, "health": 8 }));
            }
        }
    }
}
