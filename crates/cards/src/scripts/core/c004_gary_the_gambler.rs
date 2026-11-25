//! SPEC §8.1 #4 Gary the Gambler — 1/1 → 2/2 Unit, Human, cost 1.
//! Base: "Cry: Flip a coin 6 times. Gain +1 Attack per heads and +1 Health per tails."
//! Radiant: "12 times; +1 per heads, +1 per tails" — the cell changes only the numbers (§8 Conventions).
//! Issue #4783 took the rider, "Flip a coin. If heads, gain Divine Shield; if tails, gain Rush.", off
//! both faces and gave the stat flips one more coin (base 5 → 6) and two more on the Radiant face
//! (10 → 12); `script_v0_6_13c` keeps the rider for the replays of v0.6.13c and before (R767000).
//!
//! R1440 (revising R32/R130): a coin is a luck-based roll whose better side is heads, so a Lucky X
//! given to Gary (R1438) flips each coin X more times and keeps heads; the verbs read that Lucky, so
//! this file passes no `lucky` option. A card file may not call `ctx.rng` (CLAUDE.md rules 4 and 5):
//! `flip_coins` draws the stats as ONE layer-4 buff (§10.4), so a replay at the same seed and cursor
//! reproduces it (§10.7). Its text says "Flip a coin" (R1444), so it stays in the Luck-based pool.

use jackioh_engine::effects::{flip_coin_keyword, flip_coins};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-004";

/// The stat flips: `coins` coins, `perHeads` Attack per heads and `perTails` Health per tails.
fn stat_coins(ctx: &EffectContext<'_>) -> Effect {
    flip_coins(json_as(json!({
        "target": { "of": "self" },
        "coins": param(ctx, "coins"),
        "perHeads": { "attack": param(ctx, "perHeads") },
        "perTails": { "health": param(ctx, "perTails") }
    })))
}

/// Both faces; `rider`: whether a last coin grants Divine Shield on heads or Rush on tails, as through
/// v0.6.13c.
fn scripts(rider: bool) -> CardScripts {
    let base = Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![stat_coins(&*ctx)];
            if rider {
                effects.push(flip_coin_keyword(json_as(json!({
                    "target": { "of": "self" },
                    "headsKeyword": { "kind": "Divine Shield" },
                    "tailsKeyword": { "kind": "Rush" }
                }))));
            }
            effects
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// The coins and the gain per side are the declared numbers `coins`, `perHeads` and `perTails`
/// (R386): 6, 1 and 1, and 12, 1 and 1 on the Radiant face, read off the face that is up.
pub fn script() -> CardScripts {
    scripts(false)
}

/// R767000: through v0.6.13c both faces flipped a last coin after the stat coins (5, Radiant 10, catalog
/// data): Divine Shield on heads, Rush on tails. `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_13c() -> CardScripts {
    scripts(true)
}

// SPEC §8.1 #4 Gary the Gambler. BUILD M4-T4 row 4 (R32, R130, R1440, R1438).
//
// The flips land as ONE layer-4 buff, so `buffs.attack` is the heads total and `buffs.health` the
// tails total, accounting for every flip whatever the seed; a fixed seed's exact numbers are asserted
// as determinism (the same seed twice gives the same stats).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// The Cry's buff as `(attack, health)`, read off the live instance (never off a captured one).
    fn gains(s: &Scenario, card: &str) -> (i32, i32) {
        let gary = s.card(card);
        (gary.buffs.attack, gary.buffs.health)
    }

    /// The keyword a rider coin granted, read back off the live instance.
    fn granted_kind(s: &Scenario, card: &str) -> Option<String> {
        s.card(card)
            .granted_keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .next()
    }

    mod n4_gary_the_gambler {
        use super::*;

        #[test]
        fn flips_6_coins_1_attack_per_heads_and_1_max_health_per_tails_so_the_two_gains_total_6() {
            let mut s = scenario(json!({ "seed": "core-004-base", "p1": { "hand": ["core-004"] } }));
            s.play("core-004", json!({}));

            let (heads, tails) = gains(&s, "core-004");
            assert_eq!(heads + tails, 6);
            assert!(heads >= 0);
            assert!(tails >= 0);
            s.expect_stats("core-004", json!({ "attack": 1 + heads, "maxHealth": 1 + tails }));
        }

        #[test]
        fn a_fixed_seed_gives_fixed_stats() {
            let mut first = scenario(json!({ "seed": "core-004-fixed", "p1": { "hand": ["core-004"] } }));
            first.play("core-004", json!({}));
            let mut second = scenario(json!({ "seed": "core-004-fixed", "p1": { "hand": ["core-004"] } }));
            second.play("core-004", json!({}));

            assert_eq!(gains(&second, "core-004"), gains(&first, "core-004"));
        }

        #[test]
        fn radiant_a_fixed_seed_gives_fixed_stats_12_coins_at_exactly_1_a_side() {
            let mut first = scenario(json!({
                "seed": "core-004-radiant-fixed",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            first.play("core-004", json!({}));
            let mut second = scenario(json!({
                "seed": "core-004-radiant-fixed",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            second.play("core-004", json!({}));

            let gains1 = gains(&first, "core-004");
            let gains2 = gains(&second, "core-004");
            assert_eq!(gains1, gains2);
            let (attack, health) = gains1;
            assert_eq!(attack + health, 12);
            assert!(attack >= 0 && health >= 0);
        }

        #[test]
        fn radiant_flips_12_coins_at_1_a_side_so_the_two_gains_total_12() {
            let mut s = scenario(json!({
                "seed": "core-004-radiant",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            s.play("core-004", json!({}));

            let (attack, health) = gains(&s, "core-004");
            assert_eq!(attack + health, 12);
            s.expect_stats(
                "core-004",
                json!({ "attack": 2 + attack, "maxHealth": 2 + health }),
            );
        }

        /// Issue #4783: no face flips a coin for a keyword any more, so 200 seeds grant Gary none.
        #[test]
        fn neither_face_grants_a_keyword_since_issue_4783() {
            crate::register_all();
            for n in 1..=200 {
                for radiant in [false, true] {
                    let mut s = scenario(json!({
                        "seed": format!("gary-no-rider-{n}"),
                        "p1": { "hand": [{ "def": "core-004", "radiant": radiant }] }
                    }));
                    s.play("core-004", json!({}));
                    assert_eq!(granted_kind(&s, "core-004"), None, "seed {n}, radiant {radiant}");
                    assert_eq!(s.count(GameEventType::KeywordGranted), 0);
                }
            }
        }

        /// The draws a play of Gary takes beyond what a play costs by itself (a Mr. Vanilla's on the
        /// same seed), with `lucky` given to it in the hand.
        fn coin_draws(seed: &str, radiant: bool, lucky: i32) -> i64 {
            let mut control = scenario(json!({ "seed": seed, "p1": { "hand": ["core-008"] } }));
            let control_before = i64::from(control.state().rng_cursor);
            control.play("core-008", json!({}));
            let overhead = i64::from(control.state().rng_cursor) - control_before;

            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "hand": [{ "def": "core-004", "radiant": radiant }] }
            }));
            if lucky > 0 {
                crate::give_lucky(&mut s, "core-004", lucky);
            }
            let before = i64::from(s.state().rng_cursor);
            s.play("core-004", json!({}));
            i64::from(s.state().rng_cursor) - before - overhead
        }

        #[test]
        fn r32_r130_with_no_lucky_its_6_coins_take_exactly_6_seeded_rolls() {
            crate::register_all();
            // A Lucky flip shows as extra draws on the cursor (`rng.lucky`, rng.rs); Gary prints no
            // Lucky, so each flip is one draw (R1440).
            assert_eq!(coin_draws("core-004-r32", false, 0), 6);
        }

        #[test]
        fn r32_r130_radiant_with_no_lucky_its_12_coins_take_exactly_12_seeded_rolls() {
            crate::register_all();
            assert_eq!(coin_draws("core-004-r32", true, 0), 12);
        }

        #[test]
        fn r1438_r1440_given_lucky_1_its_6_coins_take_12_draws() {
            crate::register_all();
            // Lucky 1 given in the hand rides onto the field (R1438), and each of the 6 coins flips twice.
            assert_eq!(coin_draws("core-004-r32", false, 1), 12);
        }

        #[test]
        fn r1440_given_lucky_1_it_lands_more_heads_over_200_seeds() {
            crate::register_all();
            // Heads is the better side (R1440): a coin lands heads 3 times in 4 with Lucky 1, so over
            // 1,200 coins the attack it gains (+1 per heads) is well above the plain flip's.
            let heads = |lucky: bool| -> i32 {
                (0..200)
                    .map(|n| {
                        let mut s = scenario(
                            json!({ "seed": format!("core-004-lucky-{n}"), "p1": { "hand": ["core-004"] } }),
                        );
                        if lucky {
                            crate::give_lucky(&mut s, "core-004", 1);
                        }
                        s.play("core-004", json!({}));
                        let (heads, tails) = gains(&s, "core-004");
                        assert_eq!(heads + tails, 6);
                        heads
                    })
                    .sum()
            };
            assert!(heads(true) > heads(false));
        }

        /// R1444: its text still says "Flip a coin", so it stays in the Luck-based pool.
        #[test]
        fn r1444_it_is_still_luck_based() {
            crate::register_all();
            assert!(jackioh_engine::catalog::is_luck_based(&crate::card_def("core-004")));
        }

        /// The `rolled`, `buffed` and `keywordGranted` events of the last step, in order, as
        /// `(type, coins in the batch)`.
        fn reported(s: &Scenario) -> Vec<(GameEventType, usize)> {
            s.last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Rolled { rolls, .. } => Some((GameEventType::Rolled, rolls.len())),
                    GameEvent::Buffed { .. } => Some((GameEventType::Buffed, 0)),
                    GameEvent::KeywordGranted { .. } => Some((GameEventType::KeywordGranted, 0)),
                    _ => None,
                })
                .collect()
        }

        /// The coins of the last step's `rolled` batches, in order.
        fn batches(s: &Scenario) -> Vec<Vec<RollResult>> {
            s.last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Rolled { rolls, .. } => Some(rolls.clone()),
                    _ => None,
                })
                .collect()
        }

        /// R2652001: the six stat coins are one batch shown before the buff; the heads it kept are the
        /// attack gained.
        #[test]
        fn r2652001_base_six_coins_are_one_batch_before_the_buff() {
            crate::register_all();
            for n in 1..=20 {
                let mut s = scenario(json!({ "seed": format!("gary-rolled-{n}"), "p1": { "hand": ["core-004"] } }));
                s.play("core-004", json!({}));
                assert_eq!(
                    reported(&s),
                    vec![(GameEventType::Rolled, 6), (GameEventType::Buffed, 0)]
                );
                let rolled = batches(&s);
                let heads = rolled[0].iter().filter(|coin| coin.heads()).count() as i32;
                assert_eq!(gains(&s, "core-004"), (heads, 6 - heads));
                // No Lucky: each coin is the one face it drew.
                assert!(rolled.iter().flatten().all(|coin| coin.faces.len() == 1 && coin.kept == 0));
                let first = s.last_events().iter().find(|event| event.event_type() == GameEventType::Rolled);
                assert!(matches!(first, Some(GameEvent::Rolled { kind: RollKind::Coin, sides: 2, def_id, .. }) if def_id == "core-004"));
            }
        }

        /// R2652001, R1361, R68: a report is answered by nothing and takes no number from
        /// `next_seq`, so the play's numbers are the ones every other event of it took; nor does the
        /// marker of its Cry (R2783000).
        #[test]
        fn r2652001_the_reports_take_no_number_from_the_trigger_loop() {
            let mut s = scenario(json!({ "seed": "gary-rolled-seq", "p1": { "hand": ["core-004"] } }));
            let before = s.state().next_seq;
            s.play("core-004", json!({}));
            let numbered = s
                .last_events()
                .iter()
                .filter(|event| {
                    !matches!(event.event_type(), GameEventType::Rolled | GameEventType::HookResolving)
                })
                .count() as u32;
            assert_eq!(batches(&s).len(), 1);
            assert_eq!(s.state().next_seq - before, numbered);
        }

        /// R2652001, §9.3: a state round-tripped through JSON flips the same coins and reports them the
        /// same.
        #[test]
        fn r2652001_a_round_tripped_state_rolls_the_same_coins() {
            crate::register_all();
            let setup = json!({ "seed": "gary-rolled-thaw", "p1": { "hand": ["core-004"] } });
            let mut live = scenario(setup.clone());
            let mut thawed = scenario(setup);
            *thawed.state_mut() =
                serde_json::from_value(crate::js(thawed.state())).expect("the state round-trips through JSON");
            live.play("core-004", json!({}));
            thawed.play("core-004", json!({}));
            assert_eq!(batches(&thawed), batches(&live));
            assert_eq!(batches(&live).len(), 1);
            assert_eq!(hash_state(thawed.state()), hash_state(live.state()));
        }

        #[test]
        fn r386_an_upgrade_flips_7_coins_and_a_degrade_5() {
            for (upgrade, coins) in [(true, 7), (false, 5)] {
                let mut s = scenario(json!({ "seed": "core-004-tuned", "p1": { "hand": ["core-004"] } }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-004", "coins")
                } else {
                    crate::degrade_number(&mut s, "core-004", "coins")
                };
                assert_eq!(moved, coins);
                s.play("core-004", json!({}));
                let (heads, tails) = gains(&s, "core-004");
                assert_eq!(heads + tails, coins);
            }
        }

        #[test]
        fn r386_an_upgrade_makes_heads_worth_2_attack_and_tails_2_health_and_a_radiant_degrade_flips_11() {
            let mut s = scenario(json!({ "seed": "core-004-tuned", "p1": { "hand": ["core-004"] } }));
            assert_eq!(crate::upgrade_number(&mut s, "core-004", "perHeads"), 2);
            assert_eq!(crate::upgrade_number(&mut s, "core-004", "perTails"), 2);
            s.play("core-004", json!({}));
            let (attack, health) = gains(&s, "core-004");
            assert_eq!((attack % 2, health % 2, attack / 2 + health / 2), (0, 0, 6));

            let mut radiant = scenario(json!({
                "seed": "core-004-tuned",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            assert!(!crate::can_degrade_number(&radiant, "core-004", "perHeads"));
            assert!(!crate::can_degrade_number(&radiant, "core-004", "perTails"));
            assert_eq!(crate::degrade_number(&mut radiant, "core-004", "coins"), 11);
            radiant.play("core-004", json!({}));
            let (attack, health) = gains(&radiant, "core-004");
            assert_eq!(attack + health, 11);
        }

        /// R767000: the scripts kept for v0.6.13c and before, with the 5 coins (Radiant 10) they
        /// declared then: the stat coins, then a rider coin granting Divine Shield on heads and Rush on
        /// tails, identical on both faces.
        mod kept_v0_6_13c {
            use super::*;

            fn kept<T>(run: impl FnOnce() -> T) -> T {
                crate::under_kept_as(
                    "core-004",
                    crate::scripts::core::c004_gary_the_gambler::script_v0_6_13c(),
                    |def| {
                        for declared in def.params.iter_mut().flatten() {
                            if declared.key == "coins" {
                                declared.base = 5;
                                declared.radiant = 10;
                            }
                        }
                    },
                    run,
                )
            }

            #[test]
            fn r767000_r32_r130_the_5_coins_and_the_rider_coin_take_exactly_6_seeded_rolls() {
                kept(|| {
                    assert_eq!(coin_draws("core-004-r32", false, 0), 6);
                    assert_eq!(coin_draws("core-004-r32", true, 0), 11);
                });
            }

            #[test]
            fn r767000_r2652001_five_coins_then_the_rider_are_two_batches() {
                kept(|| {
                    for n in 1..=20 {
                        let mut s =
                            scenario(json!({ "seed": format!("gary-rolled-{n}"), "p1": { "hand": ["core-004"] } }));
                        s.play("core-004", json!({}));
                        assert_eq!(
                            reported(&s),
                            vec![
                                (GameEventType::Rolled, 5),
                                (GameEventType::Buffed, 0),
                                (GameEventType::Rolled, 1),
                                (GameEventType::KeywordGranted, 0),
                            ]
                        );
                        let rolled = batches(&s);
                        let heads = rolled[0].iter().filter(|coin| coin.heads()).count() as i32;
                        assert_eq!(gains(&s, "core-004"), (heads, 5 - heads));
                        let rider = if rolled[1][0].heads() { "Divine Shield" } else { "Rush" };
                        assert_eq!(granted_kind(&s, "core-004").as_deref(), Some(rider));
                    }
                });
            }

            #[test]
            fn r767000_the_rider_lands_on_both_keywords_over_the_seeds_on_both_faces() {
                kept(|| {
                    for radiant in [false, true] {
                        let mut seen: IndexSet<String> = IndexSet::new();
                        for n in 1..=200 {
                            let mut s = scenario(json!({
                                "seed": format!("gary-key-{n}"),
                                "p1": { "hand": [{ "def": "core-004", "radiant": radiant }] }
                            }));
                            s.play("core-004", json!({}));
                            let (attack, health) = gains(&s, "core-004");
                            assert_eq!(attack + health, if radiant { 10 } else { 5 });
                            let kind = granted_kind(&s, "core-004");
                            assert!(["Divine Shield", "Rush"].contains(&kind.as_deref().unwrap_or("")));
                            seen.insert(kind.unwrap_or_default());
                        }
                        let mut kinds: Vec<String> = seen.into_iter().collect();
                        kinds.sort();
                        assert_eq!(kinds, vec!["Divine Shield".to_string(), "Rush".to_string()]);
                    }
                });
            }
        }
    }
}
