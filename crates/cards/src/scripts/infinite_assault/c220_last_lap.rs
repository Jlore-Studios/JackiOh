//! IA #220 Last Lap (SPEC §8.13 row 220), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your Units have +{attack} Attack and Charge this turn. Deal {damage} damage to your
//!            hero." (3, 5)
//!   Radiant: "Your Units have +{attack} Attack and Charge this turn. Deal {damage} damage to your
//!            hero." (6, 5)
//!
//! The Attack and the Charge are "this turn" lends: player-level modifiers with `ThisTurn` expiry,
//! so Units played afterwards read them as they land, and §2.2's cleanup takes them back
//! (R4594002). A Spell's lend passes a Unit Immune to Spells by (R4453000). Then its controller's
//! own hero takes the hit.
//!
//! Engine: `giveUnitsAttackThisTurn`, `giveUnitsKeywordThisTurn`, `damage` (R4594002). Built by
//! C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-220";

fn last_lap() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![
                give_units_attack_this_turn(json_as(json!({ "amount": param(&*ctx, "attack") }))),
                give_units_keyword_this_turn(json_as(json!({ "keyword": { "kind": "Charge" } }))),
                damage(json_as(json!({
                    "to": { "of": "selfHero" },
                    "amount": param(&*ctx, "damage"),
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's +6 is its declared `attack`.
    let base = last_lap();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #220 Last Lap — SPEC §8.13 row 220 (R4594002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const ATTACKER: &str = "core-008"; // Mr. Vanilla: a 4/4.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, ATTACKER, STOCKPILE], "library": crate::filler(4) },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn attack_of(s: &Scenario) -> i32 {
        unit_view(s.state(), &s.unit_at(P1, 1)).attack
    }

    mod base {
        use super::*;

        #[test]
        fn r4594002_a_unit_played_afterwards_attacks_at_once_with_plus_3() {
            let mut s = played(false, "lap-base");
            assert_eq!(s.state().players[P1].hero.health, 30 - 5, "5 damage to its own hero");
            s.play(ATTACKER, json!({ "zone": 1 }));
            assert_eq!(attack_of(&s), 4 + 3, "the lend reaches the new arrival");
            let hero = s.state().players[P2].hero.health;
            s.attack(ATTACKER, "hero");
            assert_eq!(s.state().players[P2].hero.health, hero - 7, "Charge attacks at once");
        }

        #[test]
        fn r4594002_the_lend_expires_next_turn() {
            let mut s = played(false, "lap-expiry");
            s.play(ATTACKER, json!({ "zone": 1 }));
            s.end_turn();
            s.end_turn();
            assert_eq!(attack_of(&s), 4, "gone at the cleanup");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594002_a_unit_played_afterwards_attacks_at_once_with_plus_6() {
            let mut s = played(true, "lap-radiant");
            s.play(ATTACKER, json!({ "zone": 1 }));
            assert_eq!(attack_of(&s), 4 + 6, "the Radiant lend reaches the new arrival");
            let hero = s.state().players[P2].hero.health;
            s.attack(ATTACKER, "hero");
            assert_eq!(s.state().players[P2].hero.health, hero - 10, "Charge attacks at once");
        }
    }
}
