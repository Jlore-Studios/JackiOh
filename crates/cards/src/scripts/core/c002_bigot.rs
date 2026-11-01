//! SPEC §8.1 #2 Bigot — 4/1 → 8/2 Unit, Human, cost 2.
//! Base: "Cry: destroy target enemy non-Human unit". Radiant: "Cry: destroy all enemy non-Human
//! units" — the radiant cell restates the clause, so it replaces the base one (§8 Conventions) and
//! the radiant form declares no target at all.
//!
//! The pick travels in the `play` action, never a prompt (R81), checked against this declaration and
//! the board (R90); with no legal target the unit still enters and the Cry fizzles. The radiant
//! `destroy_all` marks every match as `destroy` does, so all die in one state check (R59).

use jackioh_engine::effects::{destroy, destroy_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-002";

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

// BUILD M4-T4 row 2.
// Non-Human: #25 and #12 (its Cry never fires on a placed board). Human: #8 and #20.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    mod n2_bigot {
        use super::*;

        #[test]
        fn destroys_the_chosen_enemy_non_human_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-002"] },
                "p2": { "field": ["core-025", "core-008"] }
            }));
            let target = s.card("core-025").id.clone();
            s.play("core-002", json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
            s.expect_in_zone("core-025", "graveyard");
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("core-002", "field");
        }

        #[test]
        fn r90_a_human_is_not_targetable_the_play_is_refused() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-002"] },
                "p2": { "field": ["core-008"] }
            }));
            let human = s.card("core-008").id.clone();
            s.expect_refused(|s| {
                s.play("core-002", json!({ "targets": [{ "pick": "instance", "instanceId": human }] }))
            });
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("core-002", "hand");
        }

        #[test]
        fn medina_enforcer_is_human_so_bigot_cannot_target_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-002"] },
                "p2": { "field": ["meditative-083"] }
            }));
            let human = s.card("meditative-083").id.clone();
            s.expect_refused(|s| {
                s.play("core-002", json!({ "targets": [{ "pick": "instance", "instanceId": human }] }))
            });
            s.expect_in_zone("meditative-083", "field");
            s.expect_in_zone("core-002", "hand");
        }

        #[test]
        fn r90_no_legal_target_the_play_is_still_legal_the_unit_enters_and_the_cry_fizzles() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-002"] },
                "p2": { "field": ["core-008"] }
            }));
            s.play("core-002", json!({}));
            s.expect_in_zone("core-002", "field");
            s.expect_in_zone("core-008", "field");
        }

        #[test]
        fn radiant_clears_every_enemy_non_human_while_the_humans_survive() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-002", "radiant": true }] },
                "p2": { "field": ["core-025", "core-008", "core-012", "core-020"] }
            }));
            s.play("core-002", json!({}));
            s.expect_in_zone("core-025", "graveyard");
            s.expect_in_zone("core-012", "graveyard");
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("core-020", "field");
            s.expect_in_zone("core-002", "field");
        }

        #[test]
        fn radiant_needs_no_target_and_destroys_nothing_when_every_enemy_unit_is_human() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-002", "radiant": true }] },
                "p2": { "field": ["core-008", "core-020"] }
            }));
            s.play("core-002", json!({}));
            s.expect_in_zone("core-008", "field");
            s.expect_in_zone("core-020", "field");
            s.expect_stats("core-002", json!({ "attack": 8, "maxHealth": 2 }));
        }
    }
}
