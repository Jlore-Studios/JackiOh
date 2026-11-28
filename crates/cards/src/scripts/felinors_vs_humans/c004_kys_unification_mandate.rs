//! SPEC §8.14 FvH #4 KY's Unification Mandate ((3) Spell, KY, Legendary), a card of the Felinors vs Humans
//! set (R5235000).
//!   Base and Radiant: "Set the Attack and Health of your Units to {stats}/{stats}. For the rest of the
//!   game, whenever you summon a Unit, set its Attack and Health to {stats}/{stats}." (5; Radiant 8)
//!
//! `set_stats_all` over its controller's field Units, Hearthstone's set (R1185002): an overwrite of the
//! stat line, damage healed, an Immutable Unit passed by (R23) and, the Spell's effect, one Immune to
//! Spells too (R4453000) (FV2 #4). Then `watch_summons` (R5237000, `unitsOnly`): for the rest of the game
//! its `summoned` step sets each Unit its controller summons later, played or summoned, read from the
//! watch's `instanceId`. The step runs with no `self` (R127), so the Cry reads the declared `stats` once,
//! its tuning included, and hands it on in the watch's data. Several Mandates each set, in the order
//! they were cast, so the last one made has the last word.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-004";

/// The step the summon watch re-enters.
const SUMMONED_STEP: &str = "summoned";
/// The watch's data key for the stats the Cry read.
const STATS_KEY: &str = "stats";

/// The Spell; the faces differ only in their declared `stats`.
fn mandate() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let stats = param(&*ctx, "stats");
            vec![
                set_stats_all(json_as(json!({ "side": "self", "attack": stats, "health": stats }))),
                watch_summons(WatchSummonsArgs {
                    step: SUMMONED_STEP.to_string(),
                    label: format!("Whenever you summon a Unit, set its Attack and Health to {stats}/{stats}"),
                    hook: None,
                    data: Some(IndexMap::from([(STATS_KEY.to_string(), json!(stats))])),
                    units_only: Some(true),
                }),
            ]
        })),
        resume: IndexMap::from([(
            SUMMONED_STEP,
            hook(|ctx| {
                let Some(id) = ctx.data.get("instanceId").and_then(Value::as_str) else {
                    return Vec::new();
                };
                let Some(stats) = ctx.data.get(STATS_KEY).and_then(Value::as_i64) else {
                    return Vec::new();
                };
                vec![set_stats(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "attack": stats,
                    "health": stats,
                })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = mandate();
    // The same script: the Radiant face's 8 is its declared `stats`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #4 KY's Unification Mandate — SPEC §8.14 row 4 (R5237000, R1185002, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{next_turn_of, weapon_board};

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    const VANILLA: &str = "stand-in (1) 4/4";
    const SMALL: &str = "stand-in (0) 1/1";

    /// p1 holds the Mandate (`radiant` its face) and two SMALLs, two VANILLAs on the field (one
    /// damaged); p2 a VANILLA on the field and a SMALL in hand.
    fn mandate_board(seed: &str, radiant: bool) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, SMALL, SMALL],
                "field": [VANILLA, { "def": VANILLA, "damage": 2 }],
            }),
            json!({ "field": [VANILLA], "hand": [SMALL, FILLER] }),
        )
    }

    /// The Unit `player` has in `lane`, by id.
    fn at(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit_at(player, lane).id
    }

    fn expect_set(s: &mut Scenario, id: &str, stats: i32) {
        s.expect_stats(id, json!({ "attack": stats, "health": stats, "maxHealth": stats }));
    }

    /// The Cry sets p1's Units, never p2's, and each SMALL p1 plays later, this turn and the next.
    fn expect_mandate(seed: &str, radiant: bool, stats: i32) {
        let mut s = mandate_board(seed, radiant);
        let (first, damaged, theirs) = (at(&s, P1, 1), at(&s, P1, 2), at(&s, P2, 1));
        s.play(ID, json!({}));
        expect_set(&mut s, &first, stats);
        expect_set(&mut s, &damaged, stats);
        s.expect_stats(theirs.as_str(), json!({ "attack": 4, "health": 4 }));
        s.play(SMALL, json!({}));
        let later = at(&s, P1, 3);
        expect_set(&mut s, &later, stats);
        // The opponent's summons take nothing.
        s.end_turn();
        s.play(SMALL, json!({}));
        let their_small = at(&s, P2, 2);
        s.expect_stats(their_small.as_str(), json!({ "attack": 1, "health": 1 }));
        // For the rest of the game: a turn later it still sets.
        next_turn_of(&mut s, P1);
        s.play(SMALL, json!({}));
        let next = at(&s, P1, 4);
        expect_set(&mut s, &next, stats);
        // Each set is once: the earlier Units took nothing more.
        expect_set(&mut s, &later, stats);
    }

    mod base {
        use super::*;

        #[test]
        fn r5237000_sets_your_units_to_5_5_now_and_every_unit_you_summon_later() {
            expect_mandate("fvh004-base", false, 5);
        }

        #[test]
        fn r1748000_a_buffed_mandate_sets_6_6_now_and_later() {
            let mut s = mandate_board("fvh004-buff", false);
            assert_eq!(crate::upgrade_number(&mut s, ID, "stats"), 6);
            let first = at(&s, P1, 1);
            s.play(ID, json!({}));
            expect_set(&mut s, &first, 6);
            s.play(SMALL, json!({}));
            let later = at(&s, P1, 3);
            expect_set(&mut s, &later, 6);
        }

        #[test]
        fn r5237000_several_mandates_each_set_in_the_order_cast() {
            let mut s = weapon_board(
                "fvh004-two",
                json!({ "hand": [ID, { "def": ID, "radiant": true }, SMALL], "mana": 10 }),
                json!({ "hand": [FILLER] }),
            );
            s.play(ID, json!({}));
            let radiant = must(
                s.hand(P1).into_iter().find(|card| card.def_id == ID),
                "the Radiant Mandate in hand",
            );
            s.play(&radiant, json!({}));
            s.play(SMALL, json!({}));
            let small = at(&s, P1, 1);
            // 5/5, then 8/8: the last made sets last.
            expect_set(&mut s, &small, 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237000_sets_your_units_to_8_8_now_and_every_unit_you_summon_later() {
            expect_mandate("fvh004-radiant", true, 8);
        }
    }
}
