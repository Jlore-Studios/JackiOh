//! SPEC §8.1 #2 Speciesist — 4/1 → 8/2 Unit, Human, cost 2 (7/1 → 14/2, cost 3 from issue #3837 until
//! issue #4353; 4/1 → 8/2, cost 2 until issue #3837).
//! Base: "Cry: destroy target enemy non-Human unit. You have {mana} less mana next turn." Radiant: "Cry:
//! destroy all enemy non-Human units. You have {mana} less mana next turn." — the radiant cell restates
//! the clause, so it replaces the base one (§8 Conventions) and the radiant form declares no target at
//! all.
//!
//! The pick travels in the `play` action, never a prompt (R81), checked against this declaration and
//! the board (R90); with no legal target the unit still enters and the destroy fizzles. The radiant
//! `destroy_all` marks every match as `destroy` does, so all die in one state check (R59).
//!
//! Issue #4353 gave both faces' Cry "You have {mana} less mana next turn" (R4353021): the self side of #21
//! Hinder's next-refresh modifier (§6.3 Mana, `next_turn_mana`, as M #18 Expedition12's), the declared
//! `mana` (R386, 1 on both faces, read by the Cry alone), its controller's next refresh giving that much
//! less, never below 0 (§2.3). It is the Cry's, so it comes with every Cry that runs, whatever the destroy
//! finds, and never with a summon, which runs none; through v0.6.8b the Cry lost nothing, which
//! `script_v0_6_8b` keeps (R767000).

use jackioh_engine::effects::{destroy, destroy_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-002";

/// R4353021: the Cry's "You have {mana} less mana next turn", its controller's own next refresh.
fn less_mana_next_turn(ctx: &EffectContext<'_>) -> Effect {
    next_turn_mana(json_as(json!({ "amount": -param(ctx, "mana") })))
}

/// Both faces; `loses`: whether the Cry also costs its controller the declared `mana` next turn
/// (R4353021, since issue #4353).
fn speciesist(loses: bool) -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"], "notTags": ["Human"] }),
        )],
        cry: Some(hook(move |ctx| {
            let mut effects = vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))];
            if loses {
                effects.push(less_mana_next_turn(ctx));
            }
            effects
        })),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![destroy_all(json_as(json!({
                "side": "enemy",
                "rows": ["units"],
                "notTags": ["Human"]
            })))];
            if loses {
                effects.push(less_mana_next_turn(ctx));
            }
            effects
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    speciesist(true)
}

/// R767000: through v0.6.8b its Cry lost no mana (issue #4353); `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_6_8b() -> CardScripts {
    speciesist(false)
}

// BUILD M4-T4 row 2.
// Non-Human: #25 and #12 (its Cry never fires on a placed board). Human: #8 and #20.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    mod n2_speciesist {
        use super::*;

        #[test]
        fn destroys_the_chosen_enemy_non_human_unit() {
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
        fn medina_enforcer_is_human_so_speciesist_cannot_target_it() {
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

    /// R4353021: issue #4353's "You have {mana} less mana next turn", on both faces.
    mod n2_speciesist_less_mana_next_turn_r4353021 {
        use super::*;

        /// The declared `mana`, 1 on both faces.
        const LOSS: i32 = 1;
        const FILLER: &str = "core-005"; // Stockpile: a card kept in hand.

        fn board(radiant: bool, enemies: Value) -> Scenario {
            scenario(json!({
                "p1": {
                    "hand": [{ "def": "core-002", "radiant": radiant }, FILLER],
                    "library": [FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "field": enemies, "library": [FILLER, FILLER, FILLER] },
            }))
        }

        /// The play that destroys the enemy `target`: the base face names it, the Radiant face names none.
        fn destroying(radiant: bool, target: &str) -> Value {
            if radiant {
                json!({})
            } else {
                pick::targets(pick::id(target))
            }
        }

        #[test]
        fn r4353021_both_faces_cry_loses_1_mana_next_turn_and_the_turn_after_is_whole() {
            for radiant in [false, true] {
                let mut s = board(radiant, json!(["core-025"]));
                let target = s.card("core-025").id.clone();
                s.play("core-002", destroying(radiant, &target));
                s.expect_in_zone("core-025", "graveyard");
                assert_eq!(s.state().players.p1.mana.next_turn_mod, -LOSS, "radiant {radiant}");
                assert_eq!(s.state().players.p2.mana.next_turn_mod, 0, "radiant {radiant}: its own side");

                crate::next_turn_of(&mut s, P1);
                let side = &s.state().players.p1;
                assert_eq!(side.mana.current, side.mana.max - LOSS, "radiant {radiant}");
                assert_eq!(side.mana.next_turn_mod, 0, "radiant {radiant}");
                crate::next_turn_of(&mut s, P1);
                let side = &s.state().players.p1;
                assert_eq!(side.mana.current, side.mana.max, "radiant {radiant}");
            }
        }

        /// R4353021: the loss is the Cry's, so it comes when the destroy finds nothing (R90).
        #[test]
        fn r4353021_the_cry_loses_the_mana_when_its_destroy_finds_no_target() {
            for radiant in [false, true] {
                let mut s = board(radiant, json!(["core-008"]));
                s.play("core-002", json!({}));
                s.expect_in_zone("core-008", "field");
                assert_eq!(s.state().players.p1.mana.next_turn_mod, -LOSS, "radiant {radiant}");
            }
        }

        /// R4353021: a summon runs no Cry, so it loses nothing.
        #[test]
        fn r4353021_a_summoned_speciesist_loses_nothing() {
            let mut s = board(false, json!(["core-025"]));
            crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({ "defId": "core-002" }))),
                P1,
            );
            assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
            s.expect_in_zone("core-025", "field");
        }

        /// R386: the loss is the declared `mana`, read through `param`: a Nerf in hand makes it 2.
        #[test]
        fn r386_the_loss_reads_through_param() {
            let mut s = board(true, json!(["core-025"]));
            let loss = crate::degrade_number(&mut s, "core-002", "mana");
            assert_eq!(loss, LOSS + 1);
            s.play("core-002", json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -loss);
        }

        /// R767000: through v0.6.8b the Cry lost no mana (issue #4353).
        #[test]
        fn r767000_the_script_kept_for_v0_6_8b_loses_no_mana() {
            crate::under_kept(super::super::ID, super::super::script_v0_6_8b(), || {
                for radiant in [false, true] {
                    let mut s = board(radiant, json!(["core-025"]));
                    let target = s.card("core-025").id.clone();
                    s.play("core-002", destroying(radiant, &target));
                    s.expect_in_zone("core-025", "graveyard");
                    assert_eq!(s.state().players.p1.mana.next_turn_mod, 0, "radiant {radiant}");
                }
            });
        }
    }
}
