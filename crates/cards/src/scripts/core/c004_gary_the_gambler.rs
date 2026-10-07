//! SPEC §8.1 #4 Gary the Gambler — 1/1 → 2/2 Unit, Human, cost 1.
//! Base: "Cry: flip 5 coins; +1 attack per heads, +1 max health per tails. Then flip a coin:
//! heads gains Divine Shield, tails gains Rush".
//! Radiant: "7 coins; +2 per heads, +2 per tails" plus the same rider verbatim — the cell changes
//! only the numbers of the base clause (§8 Conventions), so it is the same flip at 7 coins and 2
//! per side with an identical keyword coin.
//!
//! Engine cell: 5 or 7 seeded rolls and a permanent buff layer (§10.4 layer 4), then one seeded
//! coin granting Divine Shield on heads, Rush on tails. R32/R130: Lucky has no defined "best" for
//! a coin effect that pays on both faces, so it does not apply — the flips never consult it, which
//! is why this file asks for plain coins and no `lucky` option.
//!
//! A card file may not call `ctx.rng` itself — that is state and randomness in a card script
//! (CLAUDE.md rules 4 and 5) — so the rolls live in `flip_coins` and `flip_coin_keyword`
//! (engine/src/effects/coins.rs). The stat flip makes exactly `coins` seeded `ctx.rng.coin()`
//! calls and applies the totals as ONE layer-4 buff (one `buffed` event), so heads + tails always
//! accounts for every flip; the rider takes exactly one more seeded draw and grants the keyword
//! (§10.4 granted keywords, one `keywordGranted` event), so a replay at the same seed and cursor
//! reproduces both (§10.7). Stats first, rider second: the draw prefix is unchanged.

use jackioh_engine::effects::{flip_coin_keyword, flip_coins};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-004";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    flip_coins(json_as(json!({
                        "target": { "of": "self" },
                        "coins": 5,
                        "perHeads": { "attack": 1 },
                        "perTails": { "health": 1 }
                    }))),
                    flip_coin_keyword(json_as(json!({
                        "target": { "of": "self" },
                        "headsKeyword": { "kind": "Divine Shield" },
                        "tailsKeyword": { "kind": "Rush" }
                    }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    flip_coins(json_as(json!({
                        "target": { "of": "self" },
                        "coins": 7,
                        "perHeads": { "attack": 2 },
                        "perTails": { "health": 2 }
                    }))),
                    flip_coin_keyword(json_as(json!({
                        "target": { "of": "self" },
                        "headsKeyword": { "kind": "Divine Shield" },
                        "tailsKeyword": { "kind": "Rush" }
                    }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// SPEC §8.1 #4 Gary the Gambler. BUILD M4-T4 row 4: "Fixed seed → fixed stats; heads+tails = 5
// (radiant 7 at +2 each); second coin grants Divine Shield on heads, Rush on tails, identical on
// both faces; Lucky has no effect (R32/R130)".
//
// The flips land as ONE permanent layer-4 buff (§10.4), so `buffs.attack` is the heads total and
// `buffs.health` the tails total: the invariant the row asks for is that those two always account
// for exactly 5 flips (radiant 7) at the card's per-flip rate, whatever the seed rolled. The exact
// numbers under a fixed seed are asserted as determinism — the same seed twice gives the same
// stats. After the stat flip, one more seeded coin grants Divine Shield on heads, Rush on tails
// (§10.4 granted keywords), identical on both faces.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;
    use serde_json::json;

    /// The buff the Cry left, read back off the live instance (never off a captured one), as
    /// `(attack, health)`.
    fn gains(s: &Scenario, card_ref: &str) -> (i32, i32) {
        let gary = s.card(card_ref);
        (gary.buffs.attack, gary.buffs.health)
    }

    /// The keyword the rider coin granted, read back off the live instance.
    fn granted_kind(s: &Scenario, card_ref: &str) -> Option<String> {
        let granted = serde_json::to_value(&s.card(card_ref).granted_keywords).unwrap();
        granted.get(0).and_then(|keyword| keyword["kind"].as_str()).map(str::to_string)
    }

    fn both_faces() -> indexmap::IndexSet<String> {
        indexmap::IndexSet::from(["Divine Shield".to_string(), "Rush".to_string()])
    }

    mod c4_gary_the_gambler {
        use super::*;

        #[test]
        fn flips_5_coins_1_attack_per_heads_and_1_max_health_per_tails_so_the_two_gains_total_5() {
            let mut s = scenario(json!({ "seed": "core-004-base", "p1": { "hand": ["core-004"] } }));
            s.play("core-004", json!({}));

            let (heads, tails) = gains(&s, "core-004");
            assert_eq!(heads + tails, 5);
            assert!(heads >= 0);
            assert!(tails >= 0);
            // Base 1/1 plus the buff, read through the layers.
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
        fn radiant_flips_7_coins_at_2_a_side_so_the_two_gains_total_14() {
            let mut s = scenario(json!({
                "seed": "core-004-radiant",
                "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
            }));
            s.play("core-004", json!({}));

            let (attack, health) = gains(&s, "core-004");
            assert_eq!(attack + health, 14);
            assert_eq!(attack % 2, 0);
            assert_eq!(health % 2, 0);
            // Radiant 2/2 plus the buff.
            s.expect_stats("core-004", json!({ "attack": 2 + attack, "maxHealth": 2 + health }));
        }

        #[test]
        fn r32_r130_lucky_has_no_effect_5_coins_plus_the_rider_coin_take_exactly_6_seeded_rolls_with_no_reroll_for_a_best()
         {
            // Lucky is `rng.lucky(x, roll, better)` (rng.rs): it rolls x extra times and keeps the best, so
            // a Lucky flip would show up as extra draws on the cursor. No Core card can put Lucky on a unit
            // before its own Cry resolves, so counting the draws is what R32/R130 are actually about: the
            // flips never ask for a reroll. The control play cancels whatever a play costs in draws by itself.
            let mut control = scenario(json!({ "seed": "core-004-r32", "p1": { "hand": ["core-008"] } }));
            let control_before = control.state().rng_cursor;
            control.play("core-008", json!({}));
            let overhead = control.state().rng_cursor - control_before;

            let mut s = scenario(json!({ "seed": "core-004-r32", "p1": { "hand": ["core-004"] } }));
            let before = s.state().rng_cursor;
            s.play("core-004", json!({}));
            // 5 stat flips + 1 keyword coin.
            assert_eq!(s.state().rng_cursor - before - overhead, 6);
        }

        #[test]
        fn base_the_second_coin_grants_divine_shield_on_heads_rush_on_tails_deterministically() {
            let mut seen: indexmap::IndexSet<String> = indexmap::IndexSet::new();
            for n in 1..=200 {
                let seed = format!("gary-key-{n}");
                let mut s = scenario(json!({ "seed": seed, "p1": { "hand": ["core-004"] } }));
                s.play("core-004", json!({}));

                // The stat flip is untouched: heads + tails still accounts for all 5 coins.
                let (attack, health) = gains(&s, "core-004");
                assert_eq!(attack + health, 5);

                // Exactly one granted keyword, on one face of the coin or the other.
                let kind = granted_kind(&s, "core-004");
                assert!(matches!(kind.as_deref(), Some("Divine Shield") | Some("Rush")));
                seen.insert(kind.clone().unwrap());

                // The same seed played twice grants the same keyword.
                let mut again = scenario(json!({ "seed": seed, "p1": { "hand": ["core-004"] } }));
                again.play("core-004", json!({}));
                assert_eq!(granted_kind(&again, "core-004"), kind);
            }
            // 200 seeds land on both faces: the rider really is a coin, not a fixed grant.
            assert_eq!(seen, both_faces());
        }

        #[test]
        fn radiant_the_same_exact_rider_on_top_of_7_coins_at_2() {
            let mut seen: indexmap::IndexSet<String> = indexmap::IndexSet::new();
            for n in 1..=200 {
                let seed = format!("gary-key-radiant-{n}");
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
                }));
                s.play("core-004", json!({}));

                // The stat flip is untouched: 7 coins at +2 a side, totalling 14.
                let (attack, health) = gains(&s, "core-004");
                assert_eq!(attack + health, 14);
                assert_eq!(attack % 2, 0);
                assert_eq!(health % 2, 0);

                // Byte-identical rider: the same two keywords, one per face.
                let kind = granted_kind(&s, "core-004");
                assert!(matches!(kind.as_deref(), Some("Divine Shield") | Some("Rush")));
                seen.insert(kind.clone().unwrap());

                // The same seed played twice grants the same keyword.
                let mut again = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [{ "def": "core-004", "radiant": true }] }
                }));
                again.play("core-004", json!({}));
                assert_eq!(granted_kind(&again, "core-004"), kind);
            }
            assert_eq!(seen, both_faces());
        }
    }
}
