//! S #119 Unsecure System (SPEC §8.10 row S 119; §6.1, §10.4, R78, R1868001): (3) Spell, Epic.
//!   Base:    "Double the Attack of all Units. All Units gain First Strike."
//!   Radiant: "Double the Attack of all Units. Your Units gain First Strike."
//!
//! Engine: existing primitives (ST-S14-8): each Unit on the field of both sides is given a `buff` of
//! its Attack as it is shown now through the layers (§10.4), floored at 0, so a 3/3 becomes 6/3 and a
//! 0-Attack Unit stays at 0 (R1868001); then `grant_keyword` First Strike on every Unit of both
//! sides, a granted keyword it keeps for its stay (R78), not for the turn. A Unit Immune to Spells is
//! passed by.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-119";

fn unsecure_system(first_strike: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let doubled = cards_in_scope(ctx, &json_as(json!({ "side": "any" })));
            let striking = cards_in_scope(ctx, &json_as(json!({ "side": first_strike })));
            let buffs = doubled.iter().filter_map(|unit| {
                let attack = unit_view(&*ctx.state, unit).attack;
                (attack > 0).then(|| {
                    buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": unit.id },
                        "attack": attack,
                    })))
                })
            });
            let grants = striking.iter().map(|unit| {
                grant_keyword(json_as(json!({
                    "target": { "of": "instance", "instanceId": unit.id },
                    "keyword": { "kind": "First Strike" },
                })))
            });
            buffs.chain(grants).collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: unsecure_system("any"),
        radiant: unsecure_system("self"),
    }
}

// S #119 Unsecure System — SPEC §8.10 row S 119, BUILD M12 row S 119 (#118, R1868001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const UNSECURE: &str = "stinkyscape-119";
    const STATE_OF_THE_GAME: &str = "classic-041"; // (1) Unit 3/3, Indestructible.
    const BIG_D: &str = "core-001"; // (2) Unit 0/7.
    const FILLER: &str = "core-005";

    fn first_strike(s: &Scenario, card: &CardInstance) -> bool {
        s.stats(card)
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::FirstStrike)
    }

    fn unsecure_board(radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [{ "def": UNSECURE, "radiant": radiant }, FILLER],
                "field": [STATE_OF_THE_GAME, BIG_D],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": [STATE_OF_THE_GAME], "library": filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1868001_a_3_3_becomes_6_3_on_both_sides_and_0_stays_0() {
            let mut s = unsecure_board(false);
            let (mine, zero, theirs) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1));

            s.play(UNSECURE, json!({}));

            s.expect_stats(&mine, json!({ "attack": 6, "health": 3 }));
            s.expect_stats(&theirs, json!({ "attack": 6, "health": 3 }));
            s.expect_stats(&zero, json!({ "attack": 0, "health": 7 }));
            for unit in [&mine, &zero, &theirs] {
                assert!(first_strike(&s, unit));
            }
        }

        #[test]
        fn r1868001_first_strike_lasts_until_it_leaves_the_field() {
            let mut s = unsecure_board(false);
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));

            s.play(UNSECURE, json!({}));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);

            assert!(first_strike(&s, &mine) && first_strike(&s, &theirs), "not for the turn");
            s.expect_stats(&mine, json!({ "attack": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1868001_enemies_doubled_without_first_strike() {
            let mut s = unsecure_board(true);
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));

            s.play(UNSECURE, json!({}));

            s.expect_stats(&mine, json!({ "attack": 6 }));
            s.expect_stats(&theirs, json!({ "attack": 6 }));
            assert!(first_strike(&s, &mine));
            assert!(!first_strike(&s, &theirs));
        }
    }
}
