//! SPEC §8.13 IA #83 Swarm Logic ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your Units have +{attack} Attack and Rush this turn." (attack 2)
//!   Radiant: "Your Units have +{attack} Attack and Rush this turn." (attack 4)
//!
//! Two `thisTurn` lends over its controller's Units (R4588000): the declared `attack` beside the
//! auras' grants, and Rush beside their keywords, each reaching the Units there now and the ones
//! that arrive later, and both ending at cleanup.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-083";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                give_units_attack_this_turn(json_as(json!({ "amount": param(&*ctx, "attack") }))),
                give_units_keyword_this_turn(json_as(json!({ "keyword": { "kind": "Rush" } }))),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +4 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #83 Swarm Logic — SPEC §8.13 row 83 (R4588000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [UNIT] },
            "p2": { "field": [UNIT] },
        }))
    }

    fn rushed(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Rush)
    }

    mod base {
        use super::*;

        #[test]
        fn r4588000_your_units_have_the_attack_and_rush_this_turn() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let ours = s.unit_at(P1, 1);
            s.expect_stats(&ours, json!({ "attack": 4 }));
            assert!(rushed(&s, &ours));
            let theirs = s.unit_at(P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 2 }));
            assert!(!rushed(&s, &theirs));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588000_your_units_have_the_attack_and_rush_this_turn() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let ours = s.unit_at(P1, 1);
            s.expect_stats(&ours, json!({ "attack": 6 }));
            assert!(rushed(&s, &ours));
        }
    }
}
