//! SPEC §8.12 S+ #12 Shield Booster (Unit 1/1 → 2/2, Starship Piece, Equip, Divine Shield, cost 1, Common), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base and Radiant: "Starship Piece, Equip, Divine Shield"
//!
//! Every word is a catalog keyword the engine plays, so there is nothing to run. Played onto one of your
//! Units it fuses into it (R1086), which gains the Divine Shield with it (R1852000, §6.1); the Radiant
//! face differs by its stats alone.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-012";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face only doubles the stats: catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #12 Shield Booster — SPEC §8.12 row 12 (§6.1; R1086).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An equipped host, and a small enemy.
    const HOST: &str = "stand-in (1) 4/4 R12/12";
    const POKER: &str = "stand-in (1) 2/30";

    fn shielded(s: &Scenario, id: &str) -> bool {
        s.stats(id).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield)
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "s012",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn it_enters_with_a_divine_shield_that_absorbs_the_first_hit() {
            let mut s = played(false);
            assert!(shielded(&s, ID));
            s.expect_stats(ID, json!({ "attack": 1, "health": 1 }));
            let mut s = weapon_board(
                "s012-hit",
                json!({ "field": [ID] }),
                json!({ "field": [POKER] }),
            );
            next_hit(&mut s);
            assert!(!shielded(&s, ID), "the shield is spent");
            s.expect_stats(ID, json!({ "health": 1 }));
        }

        #[test]
        fn r1086_an_equipped_host_gains_the_divine_shield() {
            let mut s = weapon_board(
                "s012-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({}),
            );
            let host = s.unit_at(P1, 1).id;
            assert!(!shielded(&s, &host));
            s.play(ID, json!({ "zone": 1, "equip": true }));
            assert!(shielded(&s, &host));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_enters_with_a_divine_shield_and_doubled_stats() {
            let s = played(true);
            assert!(shielded(&s, ID));
            assert_eq!((s.stats(ID).attack, s.stats(ID).health), (2, 2));
        }

        #[test]
        fn r1086_an_equipped_host_gains_the_divine_shield() {
            let mut s = weapon_board(
                "s012-equip-radiant",
                json!({ "hand": [{ "def": ID, "radiant": true }], "field": [HOST], "mana": 10 }),
                json!({}),
            );
            let host = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            assert!(shielded(&s, &host));
        }
    }

    /// The enemy's Unit hits the Booster once.
    fn next_hit(s: &mut Scenario) {
        s.end_turn();
        assert_eq!(s.state().active, P2);
        let (attacker, target) = (s.unit_at(P2, 1).id, s.unit_at(P1, 1).id);
        s.attack(attacker.as_str(), target.as_str());
    }
}
