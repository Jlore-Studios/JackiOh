//! SPEC §8.13 IA #52 Thermal Paste ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Give a friendly Unit +{attack} Attack and Rush. Give it Brittle 1." (attack 3)
//!   Radiant: "Give a friendly Unit +{attack} Attack and Rush. Give it Brittle 1." (attack 6)
//!
//! A declared friendly Unit, aimed to help (R81, as IA #6 Hotfix's): a permanent Attack `buff`, the
//! Rush keyword granted, then Brittle 1 given (R385's "Give Brittle N", as S+ #48 Thorium's). The
//! Brittle's 1 is the script's literal, never a declared number (R1748003's numbered keyword).

use jackioh_engine::effects::{buff, give_brittle, grant_keyword};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-052";

/// "Brittle 1": the text's number, a numbered keyword's, not a declared param (R1748003).
const BRITTLE: i32 = 1;

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "attack": param(&*ctx, "attack") }))),
                grant_keyword(json_as(
                    json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Rush" } }),
                )),
                give_brittle(json_as(json!({ "target": { "of": "chosen" }, "n": BRITTLE }))),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +6 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #52 Thermal Paste — SPEC §8.13 row 52.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT],
            },
            "p2": { "field": [UNIT] },
        }))
    }

    fn pasted(s: &mut Scenario, lane: i32) -> CardInstance {
        let unit = s.unit_at(P1, lane);
        s.play(ID, pick::targets(pick::instance(&unit)));
        unit
    }

    fn brittle_of(s: &Scenario, card: &CardInstance) -> Option<i32> {
        s.stats(card).keywords.iter().find_map(|keyword| match keyword {
            Keyword::Brittle { n } => Some(*n),
            _ => None,
        })
    }

    mod base {
        use super::*;

        #[test]
        fn a_friendly_unit_gets_3_attack_rush_and_brittle_1() {
            let mut s = board(false);
            let unit = pasted(&mut s, 1);
            s.expect_stats(&unit, json!({ "attack": 5, "health": 2 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
            assert_eq!(brittle_of(&s, &unit), Some(1));
        }

        #[test]
        fn r81_an_enemy_unit_is_no_legal_pick() {
            let s = board(false);
            let enemy = s.unit_at(P2, 1);
            assert!(!pick::instance_ids(&s.play_targets(ID)).contains(&enemy.id));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_friendly_unit_gets_6_attack_rush_and_brittle_1() {
            let mut s = board(true);
            let unit = pasted(&mut s, 1);
            s.expect_stats(&unit, json!({ "attack": 8, "health": 2 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
            assert_eq!(brittle_of(&s, &unit), Some(1));
        }
    }
}
