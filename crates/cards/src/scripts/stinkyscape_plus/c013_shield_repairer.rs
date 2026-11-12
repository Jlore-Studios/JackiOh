//! SPEC §8.12 S+ #13 Shield Repairer (Unit 2/2 → 4/4, Starship Piece, Equip, cost 2, Rare), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            End of turn: gain Divine Shield"
//!   Radiant: "Starship Piece, Equip, Divine Shield
//!            End of turn: gain Divine Shield"
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the End-of-turn trigger and gains the shield (R1852000).
//! "Gain Divine Shield" is `grant_keyword`: it gives a spent or absent shield and does nothing to a
//! standing one (§6.1). The two faces share the hook; the Radiant face's printed Divine Shield is catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-013";

/// "End of turn: gain Divine Shield".
fn repairer() -> Script {
    Script {
        end_of_turn: Some(hook(|_ctx| {
            vec![grant_keyword(json_as(json!({
                "target": { "of": "self" },
                "keyword": { "kind": "Divine Shield" },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: repairer(),
        radiant: repairer(),
    }
}

// S+ #13 Shield Repairer — SPEC §8.12 row 13 (§6.1, §6.2; R62, R1086, R1852000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";
    /// A durable enemy to spend a shield on.
    const TOUGH: &str = "stand-in (1) 2/30";

    fn shielded(s: &Scenario, id: &str) -> bool {
        s.stats(id).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield)
    }

    fn board(radiant: bool) -> Scenario {
        weapon_board(
            "s013",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [TOUGH] }),
        )
    }

    /// Ends p1's turn, p2's Unit hits `id`, and p1's turn comes back round: its shield
    /// is spent between the two end-of-turns.
    fn hit_by_enemy(s: &mut Scenario, id: &str) {
        s.end_turn();
        assert_eq!(s.state().active, P2);
        let attacker = s.unit_at(P2, 1).id;
        s.attack(attacker.as_str(), id);
    }

    mod base {
        use super::*;

        #[test]
        fn r62_it_gains_a_divine_shield_at_the_end_of_its_turn() {
            let mut s = board(false);
            assert!(!shielded(&s, ID));
            s.end_turn();
            assert!(shielded(&s, ID));
        }

        #[test]
        fn r62_it_regains_a_spent_shield() {
            let mut s = board(false);
            hit_by_enemy(&mut s, ID);
            assert!(!shielded(&s, ID), "the first hit spent the shield");
            next_turn_of(&mut s, P1);
            s.end_turn();
            assert!(shielded(&s, ID), "the end of turn gave it back");
        }

        #[test]
        fn r62_r1086_an_equipped_host_gains_it_at_its_end_of_turn() {
            let mut s = weapon_board(
                "s013-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TOUGH] }),
            );
            let host = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            assert!(!shielded(&s, &host));
            s.end_turn();
            assert!(shielded(&s, &host));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_it_enters_shielded_and_stays_shielded_through_its_end_of_turn() {
            let mut s = board(true);
            assert!(shielded(&s, ID));
            s.end_turn();
            assert!(shielded(&s, ID));
            s.expect_stats(ID, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn r62_it_regains_a_spent_shield() {
            let mut s = board(true);
            hit_by_enemy(&mut s, ID);
            assert!(!shielded(&s, ID));
            next_turn_of(&mut s, P1);
            s.end_turn();
            assert!(shielded(&s, ID));
        }
    }
}
