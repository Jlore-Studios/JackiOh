//! SPEC §8.13 IA #156 Mirror Firewall ((3) Unit, Rare, 2/8 → 4/16), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt. After this is attacked: Deal damage to the attacker equal to the damage this took."
//!   Radiant: "Taunt. After this is attacked: Deal damage to the attacker equal to twice the damage this took."
//!
//! Engine: `afterAttacked` (MD-C26, R1026) deals the damage this took in that combat (R4591003) to
//! the attacker — once per combat it was attacked in, when the attacker still stands after that
//! combat's closing check and something landed — multiplied (1, 2 on the Radiant face). It runs
//! even when this Unit died, on the snapshot it fought with (R89).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-156";

/// The Radiant face returns twice the damage this took (R275: doubled stats alone are no Radiant
/// line for a Unit with text).
const BASE_MULTIPLE: i32 = 1;
const RADIANT_MULTIPLE: i32 = 2;

fn mirror(multiple: i32) -> Script {
    Script {
        after_attacked: Some(hook(move |ctx| {
            let Some(facts) = after_attacked_of(&*ctx) else {
                return vec![];
            };
            if !facts.attacker_survived || facts.damage_taken <= 0 {
                return vec![];
            }
            vec![damage(json_as(json!({
                "to": { "of": "instance", "instanceId": facts.attacker_id },
                "amount": facts.damage_taken * multiple,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mirror(BASE_MULTIPLE),
        radiant: mirror(RADIANT_MULTIPLE),
    }
}

// IA #156 Mirror Firewall — SPEC §8.13 row 156 (MD-C26, R89, R1026, R4591003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ATTACKER: &str = "stand-in (3) 3/10";
    const KILLER: &str = "stand-in (3) 10/3";

    fn board(radiant: bool, attacker: &str) -> Scenario {
        scenario(json!({
            "seed": "mirror-firewall",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(3), "mana": 8 },
            "p2": { "field": [attacker], "library": crate::filler(3) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4591003_it_returns_the_damage_it_took() {
            let mut s = board(false, ATTACKER);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 2, "health": 8 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let mirror = s.unit_at(P1, 1);
            s.attack(&attacker, &mirror);
            // 3 lands on the 2/8; the strike-back 2 and the returned 3: 10 - 2 - 3.
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 3, "health": 5 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 2, "health": 5 }));
        }

        #[test]
        fn r4591003_it_strikes_back_even_as_it_dies() {
            let mut s = board(false, KILLER);
            s.play(ID, json!({ "zone": 1 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let mirror = s.unit_at(P1, 1);
            s.attack(&attacker, &mirror);
            // 10 lands, killing the 2/8; the strike-back 2 and the returned 10 kill the 10/3 too.
            s.expect_in_zone(&mirror, "graveyard");
            s.expect_in_zone(&attacker, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_returns_twice_the_damage_it_took() {
            let mut s = board(true, ATTACKER);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 4, "health": 16 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1);
            let mirror = s.unit_at(P1, 1);
            s.attack(&attacker, &mirror);
            // 3 lands on the 4/16; the strike-back 4 and twice 3: 10 - 4 - 6.
            s.expect_in_zone(&attacker, "graveyard");
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 4, "health": 13 }));
        }
    }
}
