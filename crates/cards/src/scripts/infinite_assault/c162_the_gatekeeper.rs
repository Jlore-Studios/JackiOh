//! SPEC §8.13 IA #162 The Gatekeeper ((3) Unit, Epic, 5/9 → 10/18), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Immune to Spells. After this survives an attack, your hero gains {armor} Armor." (armor 3)
//!   Radiant: "Taunt, Immune to Spells. After this survives an attack, your hero gains {armor} Armor." (armor 6)
//!
//! Engine: both combat hooks, as Shield Amplifier's (MD-C26, R1026): `afterAttack`, after an attack it
//! made, when its facts say it survived, and `afterAttacked`, after an attack made on it, declared
//! or forced alike, when it still stands after that combat's closing check (read as the absence of
//! `SELF_KEY`, present when the defender died and runs on its snapshot, R89). Each gains the
//! controller's hero the declared {armor} Armor. One script: the Radiant face's 6 is its declared
//! param.

use jackioh_engine::effects::gain_hero_armor;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-162";

fn gatekeeper() -> Script {
    Script {
        after_attack: Some(hook(|ctx| {
            let survived = after_attack_of(ctx).is_some_and(|facts| facts.survived);
            if !survived {
                return vec![];
            }
            vec![gain_hero_armor(json_as(json!({
                "amount": param(ctx, "armor"),
            })))]
        })),
        after_attacked: Some(hook(|ctx| {
            if ctx.data.contains_key(SELF_KEY) {
                return vec![];
            }
            vec![gain_hero_armor(json_as(json!({
                "amount": param(ctx, "armor"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 6 is its declared `armor`, read off the running face.
    let base = gatekeeper();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #162 The Gatekeeper — SPEC §8.13 row 162 (MD-C26, R1026).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const WEAK: &str = "stand-in (1) 2/2";
    const STRONG: &str = "stand-in (4) 10/10";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "the-gatekeeper",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [WEAK],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "field": [WEAK], "library": crate::filler(3) },
        }))
    }

    fn armor(s: &Scenario) -> i32 {
        s.state().players[P1].hero.armor
    }

    mod base {
        use super::*;

        #[test]
        fn after_this_survives_an_attack_your_hero_gains_3_armor() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let keeper = s.unit_at(P1, 2);
            s.attack(&attacker, &keeper);
            // The 2/2 pops nothing through the 5/9; the keeper stands at 9 - 2.
            s.expect_stats(&s.unit_at(P1, 2), json!({ "attack": 5, "health": 7 }));
            assert_eq!(armor(&s), 3);
        }

        #[test]
        fn no_armor_when_it_does_not_survive() {
            let mut s = scenario(json!({
                "seed": "the-gatekeeper-dies",
                "p1": {
                    "hand": [{ "def": ID }],
                    "field": [WEAK],
                    "library": crate::filler(3),
                    "mana": 8,
                },
                "p2": { "field": [STRONG], "library": crate::filler(3) },
            }));
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let keeper = s.unit_at(P1, 2);
            s.attack(&attacker, &keeper);
            s.expect_in_zone(&keeper, "graveyard");
            assert_eq!(armor(&s), 0);
        }

        #[test]
        fn after_its_own_survived_attack_your_hero_gains_3_armor() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 2 }));
            let keeper = s.unit_at(P1, 2);
            let target = s.unit_at(P2, 1);
            s.attack(&keeper, &target);
            assert_eq!(armor(&s), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn after_this_survives_an_attack_your_hero_gains_6_armor() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 2 }));
            s.expect_stats(&s.unit_at(P1, 2), json!({ "attack": 10, "health": 18 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let keeper = s.unit_at(P1, 2);
            s.attack(&attacker, &keeper);
            assert_eq!(armor(&s), 6);
        }
    }
}
