//! #16 Hit Job (SPEC §8.1): "Destroy target unit", radiant "Destroy target unit and the units
//! adjacent to it on its side". Engine cell: "Adjacency 3.1; Indestructible survives".
//!
//! The target is destroyed on both faces (§8 Conventions); only the neighbours are radiant-only.
//! Nothing here knows about Indestructible: §6.3 Destroy only marks, and §4.5 step 1 collects at the
//! next state check, where R46 spares an Indestructible unit. Both destroys are one effect list, so
//! the target and its neighbours die together (R59). §3.1's adjacency is N-1 and N+1 on the same side
//! and row; `destroy_adjacent_to` fizzles silently when the target is off the field or alone.

use jackioh_engine::effects::{destroy, destroy_adjacent_to};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-016";

/// R81: an unnarrowed "target" is any unit on either side (§8 Conventions), picked in the `play`
/// action. On an empty board R90 keeps the play legal and the spell fizzles, still counted as played.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
        ..Script::default()
    };
    let radiant = Script {
        targets: targets(),
        cry: Some(hook(|_ctx| {
            vec![
                destroy(json_as(json!({ "target": { "of": "chosen" } }))),
                destroy_adjacent_to(json_as(json!({ "target": { "of": "chosen" } }))),
            ]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #16 Hit Job — SPEC §8.1 row 16, BUILD M4-T4 row 16. Must-pass. (R46, §3.1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// Reach the radiant face by setting the flag on the hand instance, the state §5.2's `set_radiant`
    /// leaves behind on a card in hand (R60), so the play that follows is a real one.
    fn make_radiant<'a>(s: &'a mut Scenario, card: &str) -> &'a mut Scenario {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
        s
    }

    mod n16_hit_job_base {
        use super::*;

        #[test]
        fn destroys_the_target_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016", "core-010"], "mana": 4 },
                "p2": { "field": ["core-019"] }
            }));
            let victim = s.unit_at(PlayerId::P2, 1);

            s.play("core-016", json!({ "targets": pick::instance(&victim) }));

            s.expect_in_zone(&victim, "graveyard");
            assert!(s.unit(PlayerId::P2, 1).is_none());
            s.expect_events(json!(["cardPlayed", "destroyed", "enteredGraveyard"]));
        }

        #[test]
        fn r46_an_indestructible_unit_ignores_the_destroy_mark_and_survives() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016", "core-010"], "mana": 4 },
                // #66 The Rock is Indestructible (§8.3 row 66).
                "p2": { "field": ["core-066"] }
            }));
            let survivor = s.unit_at(PlayerId::P2, 1);

            s.play("core-016", json!({ "targets": pick::instance(&survivor) }));

            s.expect_in_zone(&survivor, "field");
            s.expect_stats(&survivor, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
        }

        #[test]
        fn destroys_an_ally_as_readily_as_an_enemy_target_unit_is_narrowed_to_neither_side() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016", "core-010"], "field": ["core-012"], "mana": 4 },
                "p2": { "field": ["core-019"] }
            }));
            let mine = s.unit_at(PlayerId::P1, 1);

            s.play("core-016", json!({ "targets": pick::instance(&mine) }));

            s.expect_in_zone(&mine, "graveyard");
            s.expect_in_zone("core-019", "field");
        }

        #[test]
        fn fizzles_with_no_unit_on_the_board_and_the_spell_still_counts_as_played_s8_conventions() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-016", "core-010"], "mana": 4 }, "p2": {} }));

            s.play("core-016", json!({}));

            s.expect_in_zone("core-016", "graveyard");
            s.expect_mana(PlayerId::P1, 1);
            s.expect_events(json!(["cardPlayed"]));
        }
    }

    mod n16_hit_job_radiant {
        use super::*;

        #[test]
        fn s3_1_also_destroys_the_units_adjacent_to_the_target_on_its_own_side() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016"], "field": ["core-011", "core-025", "core-012"], "mana": 4 },
                // lanes 1-4: the target sits in lane 2, so lanes 1 and 3 are adjacent and lane 4 is not.
                "p2": { "field": ["core-002", "core-019", "core-020", "core-008"] }
            }));
            make_radiant(&mut s, "core-016");
            let left = s.unit_at(PlayerId::P2, 1);
            let target = s.unit_at(PlayerId::P2, 2);
            let right = s.unit_at(PlayerId::P2, 3);
            let far = s.unit_at(PlayerId::P2, 4);

            s.play("core-016", json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            s.expect_in_zone(&left, "graveyard");
            s.expect_in_zone(&right, "graveyard");
            s.expect_in_zone(&far, "field");
            // R59: one effect, one state check, so all three deaths land together.
            s.expect_events(json!(["destroyed", "destroyed", "destroyed"]));
        }

        #[test]
        fn s3_1_never_reaches_across_the_center_line() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016"], "field": ["core-011", "core-025", "core-012"], "mana": 4 },
                "p2": { "field": ["core-002", "core-019", "core-020"] }
            }));
            make_radiant(&mut s, "core-016");
            let across_left = s.unit_at(PlayerId::P1, 1);
            let across_target = s.unit_at(PlayerId::P1, 2);
            let across_right = s.unit_at(PlayerId::P1, 3);

            let target = s.unit_at(PlayerId::P2, 2);
            s.play("core-016", json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&across_left, "field");
            s.expect_in_zone(&across_target, "field");
            s.expect_in_zone(&across_right, "field");
        }

        #[test]
        fn r46_spares_an_indestructible_neighbour_and_kills_the_rest() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016"], "mana": 4 },
                "p2": { "field": [{ "def": "core-025", "radiant": true }, "core-019", "core-012"] }
            }));
            make_radiant(&mut s, "core-016");
            let indestructible = s.unit_at(PlayerId::P2, 1);
            let target = s.unit_at(PlayerId::P2, 2);
            let neighbour = s.unit_at(PlayerId::P2, 3);

            s.play("core-016", json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&indestructible, "field");
            s.expect_in_zone(&target, "graveyard");
            s.expect_in_zone(&neighbour, "graveyard");
        }

        #[test]
        fn s3_1_on_its_side_cuts_both_ways_an_ally_target_takes_your_own_neighbours_with_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016"], "field": ["core-011", "core-025", "core-012"], "mana": 4 },
                "p2": { "field": ["core-002", "core-019", "core-020"] }
            }));
            make_radiant(&mut s, "core-016");
            let my_left = s.unit_at(PlayerId::P1, 1);
            let my_target = s.unit_at(PlayerId::P1, 2);
            let my_right = s.unit_at(PlayerId::P1, 3);
            let theirs = s.unit_at(PlayerId::P2, 2);

            s.play("core-016", json!({ "targets": pick::instance(&my_target) }));

            s.expect_in_zone(&my_target, "graveyard");
            s.expect_in_zone(&my_left, "graveyard");
            s.expect_in_zone(&my_right, "graveyard");
            s.expect_in_zone(&theirs, "field");
        }

        #[test]
        fn kills_the_target_alone_when_its_neighbouring_lanes_are_empty() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-016", "core-010"], "mana": 4 },
                "p2": { "field": [{ "def": "core-019", "lane": 3 }] }
            }));
            make_radiant(&mut s, "core-016");
            let target = s.unit_at(PlayerId::P2, 3);

            s.play("core-016", json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
        }
    }
}
