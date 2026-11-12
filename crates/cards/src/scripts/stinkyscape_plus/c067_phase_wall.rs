//! SPEC §8.12 S+ #67 Phase Wall, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! 0/6 embiggen 0/24  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Immutable, Immune to Spells**  
//! **Embiggen: GIGA**  
//! \~\~\~  
//! 0/12 embiggen 0/48  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Immutable, Immune to Spells**  
//! **Armor 2**  
//! **Embiggen: GIGA**
//!
//! The Walls' shared embiggened text alone (R3555001): Immutable and Immune to Spells are catalog
//! data, and Immutable never stops its own text. Armor 2 on the Radiant face is catalog data too.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-067";

pub fn script() -> CardScripts {
    // Immutable, Immune to Spells and the Radiant face's Armor 2 are catalog data.
    let base = super::c065_thorium_wall::embiggened_wall();
    let radiant = super::c065_thorium_wall::embiggened_wall();
    CardScripts { base, radiant }
}

// S+ #67 Phase Wall — SPEC §8.12 row S+ 67 (R23, R781001, R3555001).
#[cfg(test)]
mod tests {
    use super::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    #[test]
    fn r3555001_embiggened_it_is_giga_at_0_24() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                &format!("s67-embiggened-{radiant}"),
                json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
                json!({}),
            );
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            assert_eq!(s.card(ID).giga, Some(true), "radiant {radiant}");
            let health = if radiant { 48 } else { 24 };
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": health }));
        }
    }

    #[test]
    fn r3555001_normal_it_is_0_6() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                &format!("s67-normal-{radiant}"),
                json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
                json!({}),
            );
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.card(ID).giga, None, "radiant {radiant}");
            let health = if radiant { 12 } else { 6 };
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": health }));
        }
    }

    #[test]
    fn immune_to_spells_no_spell_reaches_it() {
        for radiant in [false, true] {
            let s = weapon_board(
                "s67",
                json!({ "field": [{ "def": ID, "radiant": radiant }], "hand": ["stand-in (0) Spell"] }),
                json!({}),
            );
            let card = s.unit_at(P1, 1);
            let spell = s.card("stand-in (0) Spell").clone();
            assert!(immune_to_spells(s.state(), &card), "radiant {radiant}");
            assert!(spell_cannot_reach(s.state(), Some(&spell), &card), "radiant {radiant}");
        }
    }

    #[test]
    fn cant_attack() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                &format!("s67-cant-{radiant}"),
                json!({ "field": [{ "def": ID, "radiant": radiant }] }),
                json!({ "field": [VANILLA] }),
            );
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }
}
