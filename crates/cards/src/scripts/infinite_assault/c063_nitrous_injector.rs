//! SPEC §8.13 IA #63 Nitrous Injector ((2) Unit, Rare, 3/3 → 6/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Give your other Units +{attack} Attack and Rush this turn." (attack 2)
//!   Radiant: "Cry: Give your other Units +{attack} Attack and Rush this turn." (attack 4)
//!
//! IA #9 Heavy Lifter Mk.II's scope of its own side's Units, this one left out, narrowed to no
//! Created check: every other Unit its controller has gets the declared `attack` and Rush for this
//! turn (`buffThisTurn`, R4587000), so a Unit played first can attack at once and the bonus is gone
//! next turn.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-063";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "attack");
            cards_in_scope(ctx, &json_as(json!({ "side": "self", "excludeSelf": true })))
                .iter()
                .map(|unit| {
                    buff_this_turn(json_as(json!({
                        "target": { "of": "instance", "instanceId": unit.id },
                        "attack": amount,
                        "keywords": [{ "kind": "Rush" }],
                    })))
                })
                .collect()
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +4 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #63 Nitrous Injector — SPEC §8.13 row 63 (R4587000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [UNIT, { "def": ID, "radiant": radiant }] },
            "p2": { "field": [UNIT], "hand": [FILLER], "library": crate::filler(2) },
        }))
    }

    /// The sick Unit played first, then the Injector beside it.
    fn injected(s: &mut Scenario) {
        s.play(UNIT, json!({ "zone": 1 }));
        s.play(ID, json!({ "zone": 2 }));
    }

    mod base {
        use super::*;

        #[test]
        fn r4587000_your_other_units_have_2_attack_and_rush_this_turn() {
            let mut s = board(false);
            injected(&mut s);
            let (unit, injector, enemy) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1));
            s.expect_stats(&unit, json!({ "attack": 4, "health": 2 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
            // The Injector is unchanged: no bonus on itself.
            s.expect_stats(&injector, json!({ "attack": 3, "health": 3 }));
            assert!(!has_keyword(&s.stats(&injector).keywords, KeywordKind::Rush));
            // The sick Unit played first can attack at once.
            s.attack(&unit, "hero");
            s.expect_health(P2, HERO_HEALTH - 4);
            // The enemy's Units get nothing.
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            assert!(!has_keyword(&s.stats(&enemy).keywords, KeywordKind::Rush));
        }

        #[test]
        fn r4587000_it_is_gone_next_turn() {
            let mut s = board(false);
            injected(&mut s);
            s.end_turn();
            s.end_turn();
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
            assert!(!has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
        }

        #[test]
        fn enemy_units_get_nothing() {
            let mut s = board(false);
            injected(&mut s);
            let enemy = s.unit_at(P2, 1);
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4587000_your_other_units_have_4_attack_and_rush_this_turn() {
            let mut s = board(true);
            injected(&mut s);
            let (unit, injector) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.expect_stats(&unit, json!({ "attack": 6, "health": 2 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush));
            s.expect_stats(&injector, json!({ "attack": 6, "health": 6 }));
            s.attack(&unit, "hero");
            s.expect_health(P2, HERO_HEALTH - 6);
        }
    }
}
