//! #43 Big Felinor (SPEC §8.2). Unit 3/10 → 6/20, Felinor, cost 4, Rare.
//!   Base:    "Cry: Destroy {destroys} random non-Felinor Units." — destroys 3, among both sides' Units
//!   Radiant: the same among the enemy's Units only (§8 Conventions: a restated clause replaces the base one).
//!   Engine:  "Destroy {destroys} different Units at random, all together, one state check."
//! Three different Units, uniform and without replacement (R60), or all of them when fewer stand. The hook
//! lists the scope with `cards_in_scope_aimed` and draws each pick with `ctx.rng` as the Cry resolves
//! (R3139003), one `destroy` per pick: marks collected in one state check (§10.3, R59; §6.3, §4.5).
//! Dormant under Stack spared (R13); Indestructible spares (R46, R69; #12, #20, #21, #25, #66, R11, R82, §5), but a pick
//! of it is still one of the three. Through v0.5.11o (`script_v0_5_11o`, kept, R767000) the Cry was a
//! `destroy_all`: every non-Felinor Unit of the scope.

use jackioh_engine::effects::{cards_in_scope_aimed, destroy, destroy_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-043";

/// v0.5.11o's Cry, "destroy all non-Felinor Units": the faces differ only in which sides the destroy
/// reaches (`"any"` or `"enemy"`).
fn big_felinor_v0_5_11o(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![destroy_all(json_as(json!({ "side": side, "notTags": ["Felinor"] })))]
        })),
        ..Script::default()
    }
}

/// "Destroy {destroys} random non-Felinor Units" over `side`'s Units (`"any"` or `"enemy"`): the scope's
/// Units are listed once; each pick is drawn from those left and removed, so no Unit is picked twice, and a
/// scope with fewer than `destroys` Units is destroyed whole (R60). Nothing is drawn from an empty scope.
fn big_felinor(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let scope: BoardScope = json_as(json!({ "side": side, "notTags": ["Felinor"] }));
            let mut left = cards_in_scope_aimed(&*ctx, &scope, TargetAim::Harm);
            let mut picked = Vec::new();
            for _ in 0..param(&*ctx, "destroys") {
                if left.is_empty() {
                    break;
                }
                let at = ctx.rng.int(left.len() as i32) as usize;
                picked.push(left.remove(at));
            }
            picked
                .iter()
                .map(|unit| destroy(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id } }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: big_felinor("any"),
        radiant: big_felinor("enemy"),
    }
}

/// R767000: the script v0.5.11o and the versions before it ran, destroying every non-Felinor Unit of the
/// scope; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: big_felinor_v0_5_11o("any"),
        radiant: big_felinor_v0_5_11o("enemy"),
    }
}

// #43 Big Felinor (SPEC §8.2, §4.5, §6.3 Destroy; R11, R46, R59, R60, R69, R3139003; BUILD M4-T4 #43).
// Non-Felinors (#20, #25) destroyed in single state check (R59), three at most (R3139003); Felinors (#12,
// Felinor Token) survive.
#[cfg(test)]
mod tests {
    use super::script_v0_5_11o;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// Big Felinor waits in p1's hand; #21 Hinder rides along so the turn does not auto-end (R82).
    /// `radiant_felinor` flags the hand card Radiant.
    fn board(indestructible_enemy: bool, radiant_felinor: bool) -> Scenario {
        scenario(json!({
            "seed": "big-felinor",
            "p1": {
                "hand": [{ "def": "core-043", "radiant": radiant_felinor }, "core-021"],
                "field": [
                    { "def": "core-020", "lane": 2 },
                    { "def": "core-t-felinor", "lane": 4 },
                ],
            },
            "p2": {
                "field": [
                    { "def": "core-012", "lane": 1 },
                    // #66 The Rock is Indestructible; #25 4-mana 7/7 is not.
                    { "def": if indestructible_enemy { "core-066" } else { "core-025" }, "lane": 2 },
                ],
            },
        }))
    }

    fn def_at(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.unit(player, lane).map(|card| card.def_id)
    }

    const POINTMASTER: &str = "core-020"; // (2) Unit.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const SEVEN_SEVEN: &str = "core-025"; // (4) Unit 7/7.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ROCK: &str = "core-066"; // (4) Unit 10/10, Indestructible.
    const FELINOR_TOKEN: &str = "core-t-felinor";
    const HINDER: &str = "core-021"; // (0) Spell: a card in hand, so no turn ends by itself (R82).

    /// Three non-Felinor Units of p1's and four of p2's, a Felinor on each side; Big Felinor waits in p1's
    /// hand. The enemy's Units are `enemy` and p1's are the Pointmaster, the Menace and a Vanilla.
    fn wide_board(seed: &str, radiant: bool, enemy: [&str; 4]) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": "core-043", "radiant": radiant }, HINDER],
                "field": [
                    { "def": POINTMASTER, "lane": 1 },
                    { "def": MENACE, "lane": 2 },
                    { "def": VANILLA, "lane": 4 },
                    { "def": FELINOR_TOKEN, "lane": 5 },
                ],
            },
            "p2": {
                "field": [
                    { "def": enemy[0], "lane": 1 },
                    { "def": enemy[1], "lane": 2 },
                    { "def": enemy[2], "lane": 3 },
                    { "def": enemy[3], "lane": 4 },
                    { "def": "core-012", "lane": 5 },
                ],
            },
        }))
    }

    /// The ids of the Units on either side, p1's then p2's lanes in order.
    fn standing(s: &Scenario) -> Vec<String> {
        [PlayerId::P1, PlayerId::P2]
            .into_iter()
            .flat_map(|player| (1..=5).filter_map(move |lane| s.unit(player, lane)).collect::<Vec<_>>())
            .map(|card| card.id)
            .collect()
    }

    /// The ids of the non-Felinor Units a Big Felinor played at zone 3 destroyed, in the order they died.
    fn destroyed_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r59_r60_with_fewer_than_3_it_destroys_every_non_felinor_unit_on_both_sides_in_one_state_check() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-020", "graveyard");
            s.expect_in_zone("core-025", "graveyard");
            // Two deaths out of one Cry: neither died before the other was marked.
            s.expect_events(json!(["cardPlayed", "destroyed", "destroyed"]));
            assert!(s.unit(PlayerId::P1, 2).is_none());
            assert!(s.unit(PlayerId::P2, 2).is_none());
        }

        /// Issue #3139, R3139003, R60: three different Units, no more, among the seven non-Felinors of both
        /// sides, in one state check; every one of them is picked on some seed and each seed picks again alike.
        #[test]
        fn r3139003_r60_r59_it_destroys_three_different_non_felinor_units_of_either_side_in_one_state_check() {
            let mut ever: IndexSet<String> = IndexSet::new();
            for n in 0..40 {
                let seed = format!("big-felinor-three-{n}");
                let mut s = wide_board(&seed, false, [SEVEN_SEVEN, VANILLA, MENACE, POINTMASTER]);
                let before = standing(&s);
                s.play("core-043", json!({ "zone": 3 }));

                let dead = destroyed_ids(&s);
                assert_eq!(dead.len(), 3, "{seed}");
                let distinct: IndexSet<&String> = dead.iter().collect();
                assert_eq!(distinct.len(), 3, "{seed}: three different Units");
                assert_eq!(standing(&s).len(), before.len() + 1 - 3, "{seed}: Big Felinor lands, three go");
                for id in &dead {
                    assert_ne!(s.card(id).def_id, "core-012", "{seed}: a Felinor was destroyed");
                    assert_ne!(s.card(id).def_id, FELINOR_TOKEN, "{seed}: a Felinor was destroyed");
                    ever.insert(id.clone());
                }
                // One state check: the three marks are collected together after the Cry (R59).
                s.expect_events(json!(["cardPlayed", "destroyed", "destroyed", "destroyed"]));

                let mut again = wide_board(&seed, false, [SEVEN_SEVEN, VANILLA, MENACE, POINTMASTER]);
                again.play("core-043", json!({ "zone": 3 }));
                assert_eq!(destroyed_ids(&again), dead, "{seed}: the same seed picks the same three");
            }
            // Uniform over the seven: across forty seeds each of them was picked at least once.
            assert_eq!(ever.len(), 7, "{ever:?}");
        }

        #[test]
        fn r3139003_a_param_declares_how_many_and_an_upgrade_destroys_four() {
            crate::register_all();
            let declared: Vec<(String, i64, i64)> = crate::card_def(super::super::ID)
                .params
                .iter()
                .flatten()
                .map(|param| (param.key.clone(), param.base.into(), param.radiant.into()))
                .collect();
            assert_eq!(declared, vec![("destroys".to_string(), 3, 3)]);
            let mut s = wide_board("big-felinor-four", false, [SEVEN_SEVEN, VANILLA, MENACE, POINTMASTER]);
            let me = s.card("core-043").id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &me, "destroys"), 4);
            s.play("core-043", json!({ "zone": 3 }));
            assert_eq!(destroyed_ids(&s).len(), 4);
        }

        /// R46: the Rock is Indestructible and a pick of it is still one of the three, so it stays and the others
        /// lose one: two or three die, three on a seed that never picks it.
        #[test]
        fn r46_r3139003_an_indestructible_unit_survives_even_when_it_is_one_of_the_three_picks() {
            let mut seen: IndexSet<usize> = IndexSet::new();
            for n in 0..40 {
                let seed = format!("big-felinor-rock-{n}");
                let mut s = wide_board(&seed, false, [ROCK, SEVEN_SEVEN, VANILLA, MENACE]);
                let rock = s.card(ROCK).clone();
                s.play("core-043", json!({ "zone": 3 }));

                s.expect_in_zone(&rock, "field");
                assert!(!destroyed_ids(&s).contains(&rock.id), "{seed}");
                seen.insert(destroyed_ids(&s).len());
            }
            assert_eq!(seen, IndexSet::from([2, 3]));
        }

        #[test]
        fn spares_felinor_units_on_both_sides() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-012", "field");
            s.expect_in_zone("core-t-felinor", "field");
            assert_eq!(def_at(&s, PlayerId::P2, 1).as_deref(), Some("core-012"));
            assert_eq!(def_at(&s, PlayerId::P1, 4).as_deref(), Some("core-t-felinor"));
        }

        #[test]
        fn spares_itself_because_big_felinor_is_felinor_tagged_and_not_tags_already_excludes_it() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-043", "field");
            s.expect_stats("core-043", json!({ "attack": 3, "health": 10, "maxHealth": 10 }));
            assert_eq!(def_at(&s, PlayerId::P1, 3).as_deref(), Some("core-043"));
        }

        #[test]
        fn s8_conventions_an_empty_target_set_fizzles_and_the_unit_still_enters_the_field() {
            let mut s = scenario(json!({ "seed": "big-felinor", "p1": { "hand": ["core-043", "core-021"] } }));
            s.play("core-043", json!({ "zone": 1 }));

            s.expect_in_zone("core-043", "field");
            s.expect_events(json!(["cardPlayed", "summoned"]));
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
        }
    }

    mod radiant {
        use super::*;

        /// Issue #3139, R3139003: the Radiant face's three are among the four enemy non-Felinors; p1's own
        /// Units never die.
        #[test]
        fn r3139003_r60_three_of_the_four_enemy_non_felinor_units_die_and_none_of_the_casters() {
            let mut ever: IndexSet<String> = IndexSet::new();
            for n in 0..40 {
                let seed = format!("big-felinor-radiant-{n}");
                let mut s = wide_board(&seed, true, [SEVEN_SEVEN, VANILLA, MENACE, POINTMASTER]);
                let mine: Vec<String> = [1, 2, 4].map(|lane| s.unit(PlayerId::P1, lane).map(|c| c.id).unwrap_or_default()).to_vec();
                let theirs: Vec<String> = (1..=4).map(|lane| s.unit(PlayerId::P2, lane).map(|c| c.id).unwrap_or_default()).collect();

                s.play("core-043", json!({ "zone": 3 }));

                let dead = destroyed_ids(&s);
                assert_eq!(dead.len(), 3, "{seed}");
                assert!(dead.iter().all(|id| theirs.contains(id)), "{seed}: an enemy Unit each");
                assert!(mine.iter().all(|id| !dead.contains(id)), "{seed}");
                ever.extend(dead);
            }
            assert_eq!(ever.len(), 4);
        }

        #[test]
        fn s8_conventions_the_restated_clause_replaces_the_base_one_so_only_enemy_non_felinors_die() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-025", "graveyard");
            // Your own non-Felinor is untouched: that is the whole difference between the faces.
            s.expect_in_zone("core-020", "field");
            assert_eq!(def_at(&s, PlayerId::P1, 2).as_deref(), Some("core-020"));
        }

        #[test]
        fn still_spares_felinors_on_both_sides_and_itself() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-012", "field");
            s.expect_in_zone("core-t-felinor", "field");
            s.expect_in_zone("core-043", "field");
            s.expect_stats("core-043", json!({ "attack": 6, "health": 20, "maxHealth": 20 }));
        }

        #[test]
        fn r59_the_one_enemy_death_is_still_the_single_state_check_after_the_whole_cry() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_events(json!(["cardPlayed", "destroyed"]));
            assert_eq!(
                s.events().iter().filter(|event| event.event_type() == GameEventType::Destroyed).count(),
                1,
            );
        }
    }

    /// R767000: through v0.5.11o the Cry destroyed every non-Felinor Unit of its scope, the Radiant face's the
    /// enemy's.
    #[test]
    fn r767000_the_script_kept_for_v0_5_11o_destroys_every_non_felinor_unit_of_its_scope() {
        crate::under_kept("core-043", script_v0_5_11o(), || {
            let mut s = wide_board("big-felinor-kept", false, [SEVEN_SEVEN, VANILLA, MENACE, POINTMASTER]);
            s.play("core-043", json!({ "zone": 3 }));
            assert_eq!(destroyed_ids(&s).len(), 7);
            let mut s = wide_board("big-felinor-kept", true, [SEVEN_SEVEN, VANILLA, MENACE, POINTMASTER]);
            s.play("core-043", json!({ "zone": 3 }));
            assert_eq!(destroyed_ids(&s).len(), 4);
        });
    }
}
