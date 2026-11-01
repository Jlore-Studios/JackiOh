//! SPEC §8.1 #4 Gary the Gambler — 1/1 → 2/2 Unit, Human, cost 1.
//! Base: "Cry: flip 5 coins; +1 attack per heads, +1 max health per tails. Then flip a coin:
//! heads gains Divine Shield, tails gains Rush".
//! Radiant: "10 coins; +1 per heads, +1 per tails" plus the same rider verbatim — the cell changes
//! only the numbers of the base clause (§8 Conventions).
//!
//! R1440 (revising R32/R130): a coin is a luck-based roll whose better side is heads, so a Lucky X
//! given to Gary (R1438) flips each coin X more times and keeps heads; the verbs read that Lucky, so
//! this file passes no `lucky` option. A card file may not call `ctx.rng` (CLAUDE.md rules 4 and 5):
//! `flip_coins` draws the stats as ONE layer-4 buff (§10.4), then `flip_coin_keyword` one draw for the
//! rider, so a replay at the same seed and cursor reproduces both (§10.7).

use jackioh_engine::effects::{flip_coin_keyword, flip_coins};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-004";

/// The coins and the gain per side are the declared numbers `coins`, `perHeads` and `perTails`
/// (R386): 5, 1 and 1, and 10, 1 and 1 on the Radiant face, read off the face that is up.
pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                flip_coins(json_as(json!({
                    "target": { "of": "self" },
                    "coins": param(&*ctx, "coins"),
                    "perHeads": { "attack": param(&*ctx, "perHeads") },
                    "perTails": { "health": param(&*ctx, "perTails") }
                }))),
                flip_coin_keyword(json_as(json!({
                    "target": { "of": "self" },
                    "headsKeyword": { "kind": "Divine Shield" },
                    "tailsKeyword": { "kind": "Rush" }
                }))),
            ]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// SPEC §8.1 #4 Gary the Gambler. BUILD M4-T4 row 4 (R32, R130, R1440, R1438).
//
// The flips land as ONE layer-4 buff, so `buffs.attack` is the heads total and `buffs.health` the
// tails total, accounting for every flip whatever the seed; a fixed seed's exact numbers are asserted
// as determinism (the same seed twice gives the same stats).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The Cry's buff as `(attack, health)`, read off the live instance (never off a captured one).
    fn gains(s: &Scenario, card: &str) -> (i32, i32) {
        let gary = s.card(card);
        (gary.buffs.attack, gary.buffs.health)
    }

    /// The keyword the rider coin granted, read back off the live instance.
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
        fn flips_5_coins_1_attack_per_heads_and_1_max_health_per_tails_so_the_two_gains_total_5() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "core-004-base", "p1": { "hand": ["core-004"] } }));
            s.play("core-004", json!({}));

            let (heads, tails) = gains(&s, "core-004");
            assert_eq!(heads + tails, 5);
            assert!(heads >= 0);
            assert!(tails >= 0);
            s.expect_stats("core-004", json!({ "attack": 1 + heads, "maxHealth": 1 + tails }));
        }

        #[test]
        fn a_fixed_seed_gives_fixed_stats() {
            crate::register_all();
            let mut first = scenario(json!({ "seed": "core-004-fixed", "p1": { "hand": ["core-004"] } }));
            first.play("core-004", json!({}));
            let mut second = scenario(json!({ "seed": "core-004-fixed", "p1": { "hand": ["core-004"] } }));
            second.play("core-004", json!({}));

            assert_eq!(gains(&second, "core-004"), gains(&first, "core-004"));
        }

        #[test]
        fn radiant_a_fixed_seed_gives_fixed_stats_10_coins_at_exactly_1_a_side() {
            crate::register_all();
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
            assert_eq!(attack + health, 10);
            assert!(attack >= 1 && health >= 1);
        }

        #[test]
        fn radiant_flips_10_coins_at_1_a_side_so_the_two_gains_total_10() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-004-radiant",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            s.play("core-004", json!({}));

            let (attack, health) = gains(&s, "core-004");
            assert_eq!(attack + health, 10);
            s.expect_stats(
                "core-004",
                json!({ "attack": 2 + attack, "maxHealth": 2 + health }),
            );
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
        fn r32_r130_with_no_lucky_5_coins_plus_the_rider_coin_take_exactly_6_seeded_rolls() {
            crate::register_all();
            // A Lucky flip shows as extra draws on the cursor (`rng.lucky`, rng.rs); Gary prints no
            // Lucky, so each flip is one draw (R1440): 5 stat flips + 1 keyword coin.
            assert_eq!(coin_draws("core-004-r32", false, 0), 6);
        }

        #[test]
        fn r32_r130_radiant_with_no_lucky_10_coins_plus_the_rider_coin_take_exactly_11_seeded_rolls() {
            crate::register_all();
            assert_eq!(coin_draws("core-004-r32", true, 0), 11);
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
            // 1,000 coins the attack it gains (+1 per heads) is well above the plain flip's.
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
                        assert_eq!(heads + tails, 5);
                        heads
                    })
                    .sum()
            };
            assert!(heads(true) > heads(false));
        }

        #[test]
        fn base_the_second_coin_grants_divine_shield_on_heads_rush_on_tails_deterministically() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 1..=200 {
                let seed = format!("gary-key-{n}");
                let mut s = scenario(json!({ "seed": seed, "p1": { "hand": ["core-004"] } }));
                s.play("core-004", json!({}));

                let (attack, health) = gains(&s, "core-004");
                assert_eq!(attack + health, 5);

                let kind = granted_kind(&s, "core-004");
                assert!(["Divine Shield", "Rush"].contains(&kind.as_deref().unwrap_or("")));
                seen.insert(kind.clone().unwrap_or_default());

                let mut again = scenario(json!({ "seed": seed, "p1": { "hand": ["core-004"] } }));
                again.play("core-004", json!({}));
                assert_eq!(granted_kind(&again, "core-004"), kind);
            }
            // 200 seeds land on both faces: the rider really is a coin, not a fixed grant.
            let mut kinds: Vec<String> = seen.into_iter().collect();
            kinds.sort();
            assert_eq!(kinds, vec!["Divine Shield".to_string(), "Rush".to_string()]);
        }

        #[test]
        fn radiant_the_same_exact_rider_on_top_of_10_coins_at_1() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 1..=200 {
                let seed = format!("gary-key-radiant-{n}");
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
                }));
                s.play("core-004", json!({}));

                let (attack, health) = gains(&s, "core-004");
                assert_eq!(attack + health, 10);

                let kind = granted_kind(&s, "core-004");
                assert!(["Divine Shield", "Rush"].contains(&kind.as_deref().unwrap_or("")));
                seen.insert(kind.clone().unwrap_or_default());

                let mut again = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
                }));
                again.play("core-004", json!({}));
                assert_eq!(granted_kind(&again, "core-004"), kind);
            }
            let mut kinds: Vec<String> = seen.into_iter().collect();
            kinds.sort();
            assert_eq!(kinds, vec!["Divine Shield".to_string(), "Rush".to_string()]);
        }

        #[test]
        fn r386_an_upgrade_flips_6_coins_and_a_degrade_4() {
            for (upgrade, coins) in [(true, 6), (false, 4)] {
                crate::register_all();
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
        fn r386_an_upgrade_makes_heads_worth_2_attack_and_tails_2_health_and_a_radiant_degrade_flips_9() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "core-004-tuned", "p1": { "hand": ["core-004"] } }));
            assert_eq!(crate::upgrade_number(&mut s, "core-004", "perHeads"), 2);
            assert_eq!(crate::upgrade_number(&mut s, "core-004", "perTails"), 2);
            s.play("core-004", json!({}));
            let (attack, health) = gains(&s, "core-004");
            assert_eq!((attack % 2, health % 2, attack / 2 + health / 2), (0, 0, 5));

            let mut radiant = scenario(json!({
                "seed": "core-004-tuned",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            assert!(!crate::can_degrade_number(&radiant, "core-004", "perHeads"));
            assert!(!crate::can_degrade_number(&radiant, "core-004", "perTails"));
            assert_eq!(crate::degrade_number(&mut radiant, "core-004", "coins"), 9);
            radiant.play("core-004", json!({}));
            let (attack, health) = gains(&radiant, "core-004");
            assert_eq!(attack + health, 9);
        }
    }
}
