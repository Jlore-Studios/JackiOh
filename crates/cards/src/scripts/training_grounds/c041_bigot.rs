//! SPEC §8.11 Training Grounds #41 Bigot, the snapshot of Core #2 Bigot (`core-002`),
//! a card of the held Training Grounds set (R1420, R2194000), built by #2195.
//! Snapshot code not re-exported (R2195003); shares Core name (R2195001), deck holds one (R2195000).
//! Core #2 (SPEC §8, §8.1): Cry destroy target non-Human (R81, R90); radiant destroy all in one state check (R59).

use jackioh_engine::effects::{destroy, destroy_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-041";

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"], "notTags": ["Human"] }),
        )],
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![destroy_all(json_as(json!({
                "side": "enemy",
                "rows": ["units"],
                "notTags": ["Human"]
            })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// BUILD M4-T4 row 2: destroys enemy non-Human (#25, #12); Humans (#8, #20) not targetable / survive.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    mod n2_bigot {
        use super::*;

        #[test]
        fn destroys_the_chosen_enemy_non_human_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["traininggrounds-041"] },
                "p2": { "field": ["core-025", "core-008"] }
            }));
            let target = s.card("core-025").id.clone();
            s.play("traininggrounds-041", json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
            s.expect_in_zone("core-025", "graveyard");
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("traininggrounds-041", "field");
        }

        #[test]
        fn r90_a_human_is_not_targetable_the_play_is_refused() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["traininggrounds-041"] },
                "p2": { "field": ["core-008"] }
            }));
            let human = s.card("core-008").id.clone();
            s.expect_refused(|s| {
                s.play("traininggrounds-041", json!({ "targets": [{ "pick": "instance", "instanceId": human }] }))
            });
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("traininggrounds-041", "hand");
        }

        #[test]
        fn medina_enforcer_is_human_so_bigot_cannot_target_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["traininggrounds-041"] },
                "p2": { "field": ["meditative-083"] }
            }));
            let human = s.card("meditative-083").id.clone();
            s.expect_refused(|s| {
                s.play("traininggrounds-041", json!({ "targets": [{ "pick": "instance", "instanceId": human }] }))
            });
            s.expect_in_zone("meditative-083", "field");
            s.expect_in_zone("traininggrounds-041", "hand");
        }

        #[test]
        fn r90_no_legal_target_the_play_is_still_legal_the_unit_enters_and_the_cry_fizzles() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["traininggrounds-041"] },
                "p2": { "field": ["core-008"] }
            }));
            s.play("traininggrounds-041", json!({}));
            s.expect_in_zone("traininggrounds-041", "field");
            s.expect_in_zone("core-008", "field");
        }

        #[test]
        fn radiant_clears_every_enemy_non_human_while_the_humans_survive() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "traininggrounds-041", "radiant": true }] },
                "p2": { "field": ["core-025", "core-008", "core-012", "core-020"] }
            }));
            s.play("traininggrounds-041", json!({}));
            s.expect_in_zone("core-025", "graveyard");
            s.expect_in_zone("core-012", "graveyard");
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("core-020", "field");
            s.expect_in_zone("traininggrounds-041", "field");
        }

        #[test]
        fn radiant_needs_no_target_and_destroys_nothing_when_every_enemy_unit_is_human() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "traininggrounds-041", "radiant": true }] },
                "p2": { "field": ["core-008", "core-020"] }
            }));
            s.play("traininggrounds-041", json!({}));
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("core-020", "field");
            s.expect_stats("traininggrounds-041", json!({ "attack": 8, "maxHealth": 2 }));
        }
    }

    /// R2195003: the snapshot plays exactly as core-002, on both faces.
    #[test]
    fn r2195003_plays_exactly_as_core_002() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-002",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }] }, "p2": { "field": ["core-025", "core-008", "core-012"] } }),
                |s, id| {
                    let t = s.card("core-025").id.clone();
                    let pick = if radiant { json!({}) } else { json!({ "targets": [{ "pick": "instance", "instanceId": t }] }) };
                    s.play(id, pick);
                },
            );
        }
    }
}
