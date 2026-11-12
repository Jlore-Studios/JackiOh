//! SPEC §8.12 S+ #66 Plastanium Wall, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! 0/4 embiggen 0/16  
//! **Can’t attack**  
//! **Can’t defend**  
//! **End of turn:** Gain **Divine Shield**  
//! **Embiggen: GIGA**  
//! \~\~\~  
//! 0/8 embiggen 0/32  
//! **Can’t attack**  
//! **Can’t defend**  
//! **End of turn:** Gain **Divine Shield**  
//! **Armor 2**  
//! **Embiggen: GIGA**
//!
//! The Walls' shared embiggened text (R3555001) plus S+ #13's end of turn (R62): `grant_keyword`
//! Divine Shield on itself, which brings back a spent shield and does nothing to a standing one
//! (§6.1). Armor 2 on the Radiant face is catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-066";

fn plastanium_wall() -> Script {
    Script {
        end_of_turn: Some(hook(|_ctx| {
            vec![grant_keyword(json_as(json!({
                "target": { "of": "self" },
                "keyword": { "kind": "Divine Shield" },
            })))]
        })),
        ..super::c065_thorium_wall::embiggened_wall()
    }
}

pub fn script() -> CardScripts {
    // Armor 2 on the Radiant face is catalog data.
    CardScripts {
        base: plastanium_wall(),
        radiant: plastanium_wall(),
    }
}

// S+ #66 Plastanium Wall — SPEC §8.12 row S+ 66 (R62, R3555001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOUGH: &str = "stand-in (1) 2/30";

    fn shielded(s: &Scenario, id: &str) -> bool {
        s.stats(id).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield)
    }

    fn standing(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [TOUGH] }),
        )
    }

    mod base {
        use super::*;

        #[test]
        fn r62_it_gains_divine_shield_at_its_end_of_turn() {
            let mut s = standing(false, "s66-base");
            assert!(!shielded(&s, ID));
            s.end_turn();
            assert!(shielded(&s, ID));
        }

        #[test]
        fn r62_it_regains_a_spent_shield() {
            let mut s = standing(false, "s66-regain");
            s.end_turn();
            assert!(shielded(&s, ID));
            // P2's turn: their unit hits the wall and spends the shield.
            let attacker = s.unit_at(P2, 1).id;
            s.attack(attacker.as_str(), ID);
            assert!(!shielded(&s, ID), "the hit spent the shield");
            next_turn_of(&mut s, P1);
            s.end_turn();
            assert!(shielded(&s, ID), "the end of turn gave it back");
        }

        #[test]
        fn r3555001_embiggened_it_is_giga_at_0_16() {
            let mut s = weapon_board(
                "s66-embiggened",
                json!({ "hand": [ID], "mana": 10 }),
                json!({}),
            );
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            assert_eq!(s.card(ID).giga, Some(true));
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 16 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_it_gains_divine_shield_at_its_end_of_turn() {
            let mut s = standing(true, "s66-radiant");
            assert!(!shielded(&s, ID));
            s.end_turn();
            assert!(shielded(&s, ID));
        }

        #[test]
        fn r3555001_embiggened_it_is_giga_at_0_32() {
            let mut s = weapon_board(
                "s66-radiant-embiggened",
                json!({ "hand": [{ "def": ID, "radiant": true }], "mana": 10 }),
                json!({}),
            );
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            assert_eq!(s.card(ID).giga, Some(true));
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 32 }));
            assert_eq!(s.stats(ID).armor, 2);
        }
    }
}
