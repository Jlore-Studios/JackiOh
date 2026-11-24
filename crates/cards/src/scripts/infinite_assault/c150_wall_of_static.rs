//! SPEC §8.13 IA #150 Wall of Static ((3) Unit, Common, 3/10 → 6/20), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Can't attack"
//!   Radiant: "Taunt, Can't attack. After this is attacked: Deal {damage} damage to the attacker." (damage 2)
//!
//! Engine: the base face is keywords only (the catalog's data). The Radiant face's `afterAttacked`
//! (MD-C26, R1026) deals the declared {damage} to the attacker — declared or forced alike — once
//! per combat it was attacked in, when the attacker still stands after that combat's closing check.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-150";

fn wall() -> Script {
    Script {
        after_attacked: Some(hook(|ctx| {
            let Some(facts) = after_attacked_of(&*ctx) else {
                return vec![];
            };
            if !facts.attacker_survived {
                return vec![];
            }
            vec![damage(json_as(json!({
                "to": { "of": "instance", "instanceId": facts.attacker_id },
                "amount": param(&*ctx, "damage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script::default(),
        radiant: wall(),
    }
}

// IA #150 Wall of Static — SPEC §8.13 row 150.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ATTACKER: &str = "stand-in (2) 4/10";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "wall-of-static",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "field": [ATTACKER], "library": crate::filler(3) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn a_3_10_taunt_that_cannot_attack_and_strikes_back_only() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 3, "health": 10 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let wall = s.unit_at(P1, 1);
            s.attack(&attacker, &wall);
            // Only the wall's own 3 strike-back: 10 - 3.
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 4, "health": 7 }));
            let wall = s.unit_at(P1, 1);
            s.expect_refused(|s| s.attack(&wall, &attacker));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn after_this_is_attacked_deal_2_damage_to_the_attacker() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 6, "health": 20 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let wall = s.unit_at(P1, 1);
            s.attack(&attacker, &wall);
            // The wall's 6 strike-back and the hook's 2: 10 - 8.
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 4, "health": 2 }));
        }
    }
}
