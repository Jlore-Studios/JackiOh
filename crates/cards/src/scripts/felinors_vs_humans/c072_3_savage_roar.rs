//! FvH #72.3 Savage Roar (SPEC §8.14 row 72.3; §6.1 Deft, R4587000): (1) Spell, Other-Origins, Token
//! (printed Epic), a token FvH #72 Otherworldly Naturalism adds.
//!   Base:    "Give your Units +{attack} Attack and Deft this turn." (attack 2)
//!   Radiant: "Give your Units +{attack} Attack and Deft this turn." (attack 4)
//!
//! Engine: IA #63 Nitrous Injector's `buffThisTurn` (R4587000) on each Unit its caster controls as it
//! resolves, the declared `attack` and Deft, gone at the next turn; a Unit that arrives later gets none
//! (Hearthstone's Savage Roar), and the hero nothing ("your Units"). The faces differ only in the
//! declared number. Built by S05 (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-072-3";

fn savage_roar() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "attack");
            cards_in_scope(ctx, &json_as(json!({ "side": "self" })))
                .iter()
                .map(|unit| {
                    buff_this_turn(json_as(json!({
                        "target": { "of": "instance", "instanceId": unit.id },
                        "attack": amount,
                        "keywords": [{ "kind": "Deft" }],
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = savage_roar();
    // The same script: the Radiant face's +4 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #72.3 Savage Roar — SPEC §8.14 row 72.3 (R4587000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const UNIT: &str = "stand-in (1) 2/2";
    const LATE: &str = "stand-in (1) 3/3";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 holds Savage Roar and a 3/3 to play after it, with two 2/2s on its field; p2 a 2/2.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, LATE, FILLER],
                "field": [UNIT, UNIT],
                "library": filler(2),
            },
            "p2": { "hand": [FILLER], "field": [UNIT], "library": filler(2) },
        }))
    }

    fn deft(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Deft)
    }

    /// Savage Roar played: each of p1's Units has +`attack` and Deft; p2's Unit nothing.
    fn roared(radiant: bool, attack: i32) {
        let mut s = board(radiant);
        let units = nonempty(vec![s.unit_at(P1, 1), s.unit_at(P1, 2)], "p1's units");
        let enemy = s.unit_at(P2, 1);

        s.play(ID, json!({}));

        for unit in &units {
            s.expect_stats(unit, json!({ "attack": 2 + attack, "health": 2 }));
            assert!(deft(&s, unit), "{} has Deft", unit.id);
        }
        s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
        assert!(!deft(&s, &enemy));
    }

    mod base {
        use super::*;

        #[test]
        fn r4587000_your_units_have_2_attack_and_deft_this_turn() {
            roared(false, 2);
        }

        #[test]
        fn r4587000_a_unit_played_after_it_gets_none() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.play(LATE, json!({ "zone": 3 }));
            let late = s.unit_at(P1, 3);
            s.expect_stats(&late, json!({ "attack": 3, "health": 3 }));
            assert!(!deft(&s, &late));
        }

        #[test]
        fn r4587000_it_is_gone_next_turn() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.end_turn();
            s.end_turn();
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
            assert!(!deft(&s, &unit));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4587000_your_units_have_4_attack_and_deft_this_turn() {
            roared(true, 4);
        }
    }
}
