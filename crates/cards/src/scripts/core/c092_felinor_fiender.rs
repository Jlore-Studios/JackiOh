//! #92 Felinor Fiender (SPEC §8.4, §3.2, §10.4 layer 2, R13, R39, R362, BUILD M4-T4 row 92).
//!
//! Base: "Stack. Also has the stats of all your Felinors, including those under Stack." Radiant:
//! "Stack. Also has twice the stats of all your Felinors, including those under Stack." (patch v0.1.1: the Radiant
//! face traded Charge for twice the count). Both faces print Stack alone (§8 Conventions), so §10.4
//! layer 1 already grants it and this file grants nothing. The printed 5/7 → 10/14 stays: "has the
//! stats of" is R39's printed-plus-the-sum, and R362 doubles only the sum on the Radiant face.
//!
//! The whole card is §10.4 LAYER 2, the set-stat layer that exists for this card alone, expressed
//! through `Script.setStat` ("a card that sets its own stats from the board (#92 Felinor Fiender,
//! R39)"). §10.4 writes layer 2 as "Felinor Fiender adds the sum of your Felinors' LAYER-4 stats",
//! so each Felinor is measured with `statsWithBuffs` — layers 1 to 4, printed face plus permanent
//! buffs, BEFORE auras. `unitView` would be layer 5 and is the wrong number here; it would also make
//! the layers recurse, since `unitView` is what calls this hook.
//!
//! R13 is the reason both zone readers appear: "cards under a Stack are not on the field except for
//! Felinor Fiender's count" — the one dormancy exception in the game (§3.2). `activeUnitsOf` gives
//! the top card of every pile and `dormantUnitsOf` the cards beneath them.
//!
//! R39 (decide, `FIENDER_STATS_MODE = "printed-plus-sum"`): "printed plus the combined Felinor
//! stats, never below printed", so each sum floors at 0 — a Felinor carrying a negative health buff
//! can pull the total down toward printed but never past it.
//!
//! Fiender itself is tagged Human, not Felinor (§8, catalog), so "all your Felinors" cannot include
//! it. It is excluded by id anyway, because Fuse unions its ingredients' tags (R77) and a fused
//! Felinor Fiender would otherwise count its own layer-4 stats into its own layer 2. R131 rules on
//! exactly that: it never counts itself even once a Fuse has given it the tag, matched by instance
//! rather than by tag, and a second Fiender contributes only printed and buffed stats (R116), so the
//! layer cannot recurse.
//!
//! Layer 2 is now live: `unitView` in `engine/src/layers.ts` calls `scriptOf(instance).setStat` after
//! layer 1's printed face and BEFORE the layer-4 buffs and layer-5 auras, and it ADDS what the hook
//! returns, flooring each component at 0 (R116). So this hook returns the sum alone — adding the
//! printed face here would count it twice — and R115 skips the hook entirely on a Vanilla'd card.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-092";

/// §5, §8: the tag that makes a unit one of "your Felinors".
const FELINOR: Tag = Tag::Felinor;

/// "All your Felinors, including ones under a Stack": every Felinor this card's controller has on
/// the field, dormant ones included (R13, §3.2). Reading state to find them is not mutation; nothing
/// here writes (CLAUDE.md rule 5).
fn felinors_of(state: &GameState, self_: &CardInstance) -> Vec<CardInstance> {
    let mut mine: Vec<CardInstance> = active_units_of(state, self_.controller).into_iter().cloned().collect();
    mine.extend(dormant_units_of(state, self_.controller).into_iter().cloned());
    mine.into_iter()
        .filter(|unit| unit.id != self_.id && def_of(Some(state), &unit.def_id).tags.contains(&FELINOR))
        .collect()
}

/// §10.4 layer 2, which ADDS this result to layer 1's printed face — so this returns only the sum,
/// never printed plus the sum (R116). Each Felinor contributes its layer-4 stats (`statsWithBuffs`):
/// printed plus permanent buffs, before auras, which is also what stops the layers recursing.
fn set_stat_times(multiple: i32) -> SetStatHook {
    read_hook(move |args| {
        let mut attack = 0;
        let mut max_health = 0;
        for felinor in felinors_of(args.state, args.self_) {
            let stats = stats_with_buffs(args.state, &felinor);
            attack += stats.attack;
            max_health += stats.max_health;
        }

        // R116: the hook returns the DELTA layer 2 adds to the printed face, not an absolute total, so
        // the printed stats must not be added here — layer 2 does that. R39's "never below printed" is
        // the floor at 0: a Felinor carrying a negative buff can pull the sum toward 0 but not past it.
        SetStat {
            attack: Some(attack.max(0) * multiple),
            max_health: Some(max_health.max(0) * multiple),
        }
    })
}

/// "Has the stats of all your Felinors": the sum once (R39).
const BASE_MULTIPLE: i32 = 1;
/// "Has twice the stats of all your Felinors": the sum twice (R362).
const RADIANT_MULTIPLE: i32 = 2;

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            set_stat: Some(set_stat_times(BASE_MULTIPLE)),
            ..Script::default()
        },
        radiant: Script {
            set_stat: Some(set_stat_times(RADIANT_MULTIPLE)),
            ..Script::default()
        },
    }
}

// #92 Felinor Fiender (SPEC §8 row 92, §3.2, §10.4 layer 2; R13, R39, R92, R362).
//
// BUILD M4-T4 row 92: "Plays onto an occupied zone; card beneath is dormant; stats = printed + all
// your Felinors including dormant ones (R13, R39); radiant printed + twice that sum (R362)".
//
// §8: printed 5/7 → 10/14, keywords Stack on both faces, text "Stack. Has the stats of all your
// Felinors, including those under Stack." and radiant "Stack. Has twice the stats of all your
// Felinors, including those under Stack." (patch v0.1.1 traded the radiant Charge for the doubled
// sum) — which is what the two `describe` blocks below check separately (CLAUDE.md rule 6).
//
// R39 is the decided reading: "printed plus the combined Felinor stats, NEVER BELOW PRINTED". The
// floor is per sum, so "no Felinors at all" is the reachable boundary of the never clause and the
// first test below is it. A NEGATIVE total is unreachable with the Core set as it stands: only an
// aura (§10.4 layer 5) subtracts stats (#46 Suppressive Aura, #65.1 Spikey Pillow) and layer 2
// reads layer 4, so no card can hand a Felinor a negative permanent buff. Reported, not asserted.
//
// §10.4's wording — "Felinor Fiender adds the sum of your Felinors' LAYER-4 stats" — is the sharp
// part, and the Spikey Pillow test is what pins it: a Felinor whose aura-reduced attack differs
// from its buffed attack must contribute the buffed one.
//
// SPEC §8 TAGS #92 `Human`, NOT `Felinor` (flagged to the designer as possibly wrong for a card
// called Felinor Fiender). These tests assert what §8 actually says: a second Fiender contributes
// nothing, because neither is one of "your Felinors".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FIENDER: &str = "core-092";
    /// #43 Big Felinor, 3/10, tagged Felinor — a Cry that never fires from a `field` setup (R1).
    const BIG_FELINOR: &str = "core-043";
    /// The shared Felinor Token, 1/1 (§7).
    const FELINOR_TOKEN: &str = "core-t-felinor";
    /// #65.1 Spikey Pillow: "your units have −2 attack" as an aura, i.e. §10.4 layer 5.
    const SPIKEY_PILLOW: &str = "core-065-1";
    /// #44 True Strike: 4 damage to a target, for killing a Felinor mid-turn. (#35 Lunar Eclipse would
    /// read more naturally at 3 damage, but its script throws — see the report.)
    const TRUE_STRIKE: &str = "core-044";

    fn keyword_kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card).keywords.iter().map(|keyword| keyword.kind().to_string()).collect()
    }

    /// `s.state.players.p1.units[0]` (the pile in lane 1, top first).
    fn pile_in_lane_1(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1].units.first().cloned().flatten().unwrap_or_default()
    }

    // =========================================================================================
    // base
    // =========================================================================================

    mod n92_felinor_fiender_base {
        use super::*;

        #[test]
        fn r39_with_no_felinors_it_is_exactly_its_printed_5_7_the_never_below_printed_floor() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-alone",
                "p1": { "field": [FIENDER], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.expect_stats(FIENDER, json!({ "attack": 5, "maxHealth": 7, "health": 7 }));
            assert_eq!(crate::card_def(FIENDER).base.attack, Some(5));
        }

        #[test]
        fn r39_printed_plus_one_big_felinor_s_3_10_is_8_17() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-one-felinor",
                "p1": { "field": [FIENDER, BIG_FELINOR], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.expect_stats(FIENDER, json!({ "attack": 8, "maxHealth": 17, "health": 17 }));
            // The Felinor itself is untouched: the sum is one-way.
            s.expect_stats(BIG_FELINOR, json!({ "attack": 3, "maxHealth": 10 }));
        }

        #[test]
        fn r39_the_sum_is_over_all_your_felinors_3_10_plus_1_1_makes_it_9_18() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-two-felinors",
                "p1": { "field": [FIENDER, BIG_FELINOR, FELINOR_TOKEN], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.expect_stats(FIENDER, json!({ "attack": 9, "maxHealth": 18 }));
        }

        #[test]
        fn your_felinors_the_enemy_s_felinors_are_not_counted() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-enemy-felinors",
                "p1": { "field": [FIENDER], "hand": ["core-005"] },
                "p2": { "field": [BIG_FELINOR, FELINOR_TOKEN], "hand": ["core-005"] },
            }));

            s.expect_stats(FIENDER, json!({ "attack": 5, "maxHealth": 7 }));
        }

        #[test]
        fn s8_tags_it_human_so_a_second_fiender_adds_nothing_to_either_of_them() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-two-fienders",
                "p1": { "field": [FIENDER, FIENDER], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let first = s.unit(P1, 1);
            let second = s.unit(P1, 2);

            assert!(crate::card_def(FIENDER).tags.contains(&Tag::Human));
            assert!(!crate::card_def(FIENDER).tags.contains(&Tag::Felinor));
            assert!(first.is_some());
            assert!(second.is_some());
            if let Some(first) = &first {
                s.expect_stats(first, json!({ "attack": 5, "maxHealth": 7 }));
            }
            if let Some(second) = &second {
                s.expect_stats(second, json!({ "attack": 5, "maxHealth": 7 }));
            }
        }

        #[test]
        fn s10_4_layer_4_is_max_health_so_a_damaged_felinor_contributes_its_full_10() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-damaged-felinor",
                "p1": { "field": [FIENDER, { "def": BIG_FELINOR, "damage": 7 }], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            // The Felinor is at 3 health of 10; the Fiender still gains 10.
            s.expect_stats(BIG_FELINOR, json!({ "health": 3, "maxHealth": 10 }));
            s.expect_stats(FIENDER, json!({ "attack": 8, "maxHealth": 17, "health": 17 }));
        }

        #[test]
        fn s10_4_layer_2_reads_each_felinor_s_layer_4_stats_before_auras() {
            crate::register_all();
            // A Spikey Pillow's aura is layer 5: it takes the Big Felinor's attack from 3 to 1 and the
            // Fiender's own total by 2. Layer 2 must still add the Felinor's 3, giving 5 + 3 − 2 = 6.
            // An implementation that summed `unitView` (layer 5) would read 5 + 1 − 2 = 4.
            let mut s = scenario(json!({
                "seed": "core-092-aura-layers",
                "p1": { "field": [FIENDER, BIG_FELINOR, SPIKEY_PILLOW], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.expect_stats(BIG_FELINOR, json!({ "attack": 1, "maxHealth": 10 }));
            s.expect_stats(FIENDER, json!({ "attack": 6, "maxHealth": 17 }));
        }

        #[test]
        fn the_set_stat_layer_is_recomputed_continuously_a_felinor_leaving_drops_the_bonus_at_once() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-recompute",
                "p1": { "field": [FIENDER, FELINOR_TOKEN], "hand": [TRUE_STRIKE, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let token = s.unit(P1, 2);
            assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some(FELINOR_TOKEN));
            s.expect_stats(FIENDER, json!({ "attack": 6, "maxHealth": 8 }));

            // 4 damage kills the 1/1 token, which — being a unit token — ceases to exist (R11).
            let token_id = token.as_ref().map(|card| card.id.clone()).unwrap_or_default();
            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "instance", "instanceId": token_id }] }));

            if let Some(token) = &token {
                s.expect_in_zone(token, "gone");
            }
            s.expect_stats(FIENDER, json!({ "attack": 5, "maxHealth": 7 }));
        }

        #[test]
        fn the_bonus_follows_a_felinor_arriving_mid_turn_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-arrival",
                "p1": { "field": [FIENDER], "hand": [FELINOR_TOKEN, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            s.expect_stats(FIENDER, json!({ "attack": 5, "maxHealth": 7 }));

            s.play(FELINOR_TOKEN, json!({}));

            s.expect_stats(FIENDER, json!({ "attack": 6, "maxHealth": 8 }));
        }

        #[test]
        fn s8_the_base_face_prints_stack_and_no_charge() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "core-092-keywords-base",
                "p1": { "field": [FIENDER], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            assert!(keyword_kinds(&s, FIENDER).contains(&"Stack".to_string()));
            assert!(!keyword_kinds(&s, FIENDER).contains(&"Charge".to_string()));
        }
    }

    // =========================================================================================
    // §3.2 Stack — the dormancy rule this card exists for (R13, R92)
    // =========================================================================================

    mod n92_felinor_fiender_s3_2_stack {
        use super::*;

        #[test]
        fn s3_2_stack_it_plays_onto_an_occupied_unit_zone_and_becomes_the_top_of_the_pile() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-stack-play",
                "p1": { "field": [BIG_FELINOR], "hand": [FIENDER, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(FIENDER, json!({ "zone": 1 }));

            // The pile is ordered top-first; the card beneath keeps its place and stops acting.
            let pile: Vec<String> = pile_in_lane_1(&s).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(pile, vec![FIENDER.to_string(), BIG_FELINOR.to_string()]);
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(FIENDER.to_string()));
        }

        #[test]
        fn r13_a_felinor_dormant_under_the_stack_still_counts_the_one_dormancy_exception() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-stack-count",
                "p1": { "field": [BIG_FELINOR], "hand": [FIENDER, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(FIENDER, json!({ "zone": 1 }));

            // 5 + 3 / 7 + 10, read off the Felinor nobody else on the board can see or target.
            s.expect_stats(FIENDER, json!({ "attack": 8, "maxHealth": 17 }));
        }

        #[test]
        fn r92_the_dormant_card_under_the_stack_has_no_position_and_cannot_be_switched() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-stack-position",
                "p1": { "field": [BIG_FELINOR], "hand": [FIENDER, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(FIENDER, json!({ "zone": 1 }));
            let dormant = pile_in_lane_1(&s).get(1).cloned();
            assert_eq!(dormant.as_ref().map(|card| card.def_id.as_str()), Some(BIG_FELINOR));

            let dormant_id = dormant.map(|card| card.id).unwrap_or_default();
            s.expect_refused_with(|s| s.switch_position(dormant_id.as_str()), "not on the field");
            // The top of the pile switches normally.
            s.switch_position(FIENDER);
            assert_eq!(s.stats(FIENDER).position, Position::Def);
        }

        #[test]
        fn r13_the_card_beneath_is_dormant_still_on_the_field_keeping_its_damage_and_not_acting() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-stack-dormant",
                "p1": { "field": [{ "def": BIG_FELINOR, "damage": 4 }], "hand": [FIENDER, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let felinor = s.unit(P1, 1);

            s.play(FIENDER, json!({ "zone": 1 }));

            // The Fiender's own health is its own: the dormant card's 4 damage stays on the dormant card.
            s.expect_stats(FIENDER, json!({ "attack": 8, "maxHealth": 17, "health": 17 }));
            let dormant = pile_in_lane_1(&s).get(1).cloned();
            assert_eq!(dormant.as_ref().map(|card| card.id.clone()), felinor.map(|card| card.id));
            assert_eq!(dormant.as_ref().map(|card| card.damage), Some(4));
            // §3.2: only the top of the pile is the acting card, and the pile is still one field zone.
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(FIENDER.to_string()));
            if let Some(dormant) = &dormant {
                s.expect_in_zone(dormant, "field");
            }
        }
    }

    // =========================================================================================
    // radiant
    // =========================================================================================

    mod n92_felinor_fiender_radiant {
        use super::*;

        #[test]
        fn s8_radiant_prints_10_14_and_with_no_felinors_is_exactly_that_r39() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-radiant-alone",
                "p1": { "field": [{ "def": FIENDER, "radiant": true }], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.expect_stats(FIENDER, json!({ "attack": 10, "maxHealth": 14, "health": 14 }));
        }

        #[test]
        fn r362_the_radiant_face_adds_twice_the_sum_10_14_2_3_10_1_1_is_18_36() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-radiant-sum",
                "p1": {
                    "field": [{ "def": FIENDER, "radiant": true }, BIG_FELINOR, FELINOR_TOKEN],
                    "hand": ["core-005"],
                },
                "p2": { "hand": ["core-005"] },
            }));

            // 10 + 2 × (3 + 1) / 14 + 2 × (10 + 1).
            s.expect_stats(FIENDER, json!({ "attack": 18, "maxHealth": 36 }));
        }

        #[test]
        fn patch_v0_1_1_the_radiant_keyword_list_is_stack_alone_so_a_played_radiant_fiender_cannot_attack_that_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-radiant-keywords",
                "p1": { "field": [BIG_FELINOR], "hand": [{ "def": FIENDER, "radiant": true }, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(FIENDER, json!({ "zone": 2 }));
            assert_eq!(keyword_kinds(&s, FIENDER), vec!["Stack".to_string()]);
            // 10 + 2 × 3 attack while the Big Felinor stands beside it.
            s.expect_stats(FIENDER, json!({ "attack": 16, "maxHealth": 34 }));
            s.expect_refused(|s| s.attack(FIENDER, "hero"));
        }

        #[test]
        fn radiant_keeps_the_enemy_felinor_exclusion_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-radiant-enemy",
                "p1": { "field": [{ "def": FIENDER, "radiant": true }], "hand": ["core-005"] },
                "p2": { "field": [BIG_FELINOR], "hand": ["core-005"] },
            }));

            s.expect_stats(FIENDER, json!({ "attack": 10, "maxHealth": 14 }));
        }

        #[test]
        fn r13_r362_radiant_counts_dormant_felinors_twice_as_well() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-092-radiant-stack",
                "p1": { "field": [BIG_FELINOR], "hand": [{ "def": FIENDER, "radiant": true }, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(FIENDER, json!({ "zone": 1 }));

            s.expect_stats(FIENDER, json!({ "attack": 16, "maxHealth": 34 }));
        }
    }
}
