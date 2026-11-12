//! SPEC §8.12 S+ #T-AI-4 Jlockeed Large Language Model, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "GIGA, Taunt"
//!   Radiant: "GIGA, Taunt, Divine Shield"
//! Engine: a (2) Unit 2/5 → 4/10, Jlockeed, AI, Token (no printed rarity, R482). Every word is a catalog keyword
//!   (§6.1): GIGA fills four zones (R3546000), Taunt and, on the Radiant face, Divine Shield, so there is no
//!   script to run. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-04";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Divine Shield and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #T-AI-4 Jlockeed Large Language Model — SPEC §8.12 row T-AI-4 (R3546000).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::{ZoneSlot, is_empty};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// What attacks into it.
    const ATTACKER: &str = "stand-in (1) 3/3";

    /// The model in p1's hand with mana to play it, and p2's attacker on the field.
    fn holding(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            json!({ "field": [ATTACKER] }),
        )
    }

    /// Every unit lane a play of the model is offered, sorted.
    fn offered_lanes(s: &Scenario) -> Vec<i32> {
        let id = s.card(ID).id.clone();
        let mut lanes: Vec<i32> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id, zone, ..
                } if instance_id == id => zone,
                _ => None,
            })
            .filter(|zone| zone.row == Row::Units)
            .map(|zone| zone.lane)
            .collect();
        lanes.sort();
        lanes
    }

    #[test]
    fn giga_fills_four_zones_with_taunt() {
        for radiant in [false, true] {
            let mut s = holding(radiant, "s-t-ai-04");
            // A GIGA anchors where it and the lane beside it fit: lanes 1 to 4.
            assert_eq!(offered_lanes(&s), vec![1, 2, 3, 4], "radiant {radiant}");
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.card(ID).giga, Some(true), "radiant {radiant}");
            for (row, lane) in [
                (Row::Units, 1),
                (Row::Units, 2),
                (Row::Backrow, 1),
                (Row::Backrow, 2),
            ] {
                assert!(
                    !is_empty(
                        s.state(),
                        ZoneSlot {
                            player: P1,
                            row,
                            lane
                        }
                    ),
                    "radiant {radiant}: {row} {lane} is filled"
                );
            }
            // Taunt: every attack on p2's turn must target it.
            let model = s.card(ID).id.clone();
            s.end_turn();
            let targets: Vec<String> = legal_actions(s.state(), P2)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Attack { target_id, .. } => Some(target_id),
                    _ => None,
                })
                .collect();
            assert!(!targets.is_empty(), "radiant {radiant}");
            assert!(
                targets.iter().all(|target| *target == model),
                "radiant {radiant}: {targets:?}"
            );
        }
    }

    #[test]
    fn radiant_divine_shield_takes_the_first_hit() {
        for radiant in [false, true] {
            let mut s = holding(radiant, "s-t-ai-04-shield");
            s.play(ID, json!({ "zone": 1 }));
            s.end_turn();
            let attacker = s.unit_at(P2, 1).id;
            s.attack(attacker.as_str(), ID);
            let model = s.card(ID);
            if radiant {
                assert_eq!(model.damage, 0, "the shield negates the 3");
                assert_eq!(model.divine_shield_spent, Some(true));
            } else {
                assert_eq!(model.damage, 3, "the base face has no shield");
                assert_ne!(model.divine_shield_spent, Some(true));
            }
        }
    }
}
