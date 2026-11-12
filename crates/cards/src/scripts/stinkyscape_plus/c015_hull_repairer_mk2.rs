//! SPEC §8.12 S+ #15 Hull Repairer mk2 (Unit 0/4 → 0/8, Starship Piece, Equip, cost 2, Rare), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            End of turn: Heal this to full health"
//!   Radiant: "Starship Piece, Equip
//!            End of turn: Heal this to full health, and gain +{health} max health" — health 2
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the End-of-turn trigger (R1852000). The heal is `heal` with `toFull`
//! (as C #19 Midrange Menace's); the Radiant face then `buff`s its health alone, which raises current and
//! maximum health together (as S+ #47 Plastanium's), in the text's order. The +2 is the declared `health`.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-015";

/// "Heal this to full health".
fn heal_to_full() -> Effect {
    heal(json_as(json!({ "target": { "of": "self" }, "toFull": true })))
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            end_of_turn: Some(hook(|_ctx| vec![heal_to_full()])),
            ..Script::default()
        },
        radiant: Script {
            end_of_turn: Some(hook(|ctx| {
                vec![
                    heal_to_full(),
                    buff(json_as(json!({ "target": { "of": "self" }, "health": param(&*ctx, "health") }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// S+ #15 Hull Repairer mk2 — SPEC §8.12 row 15 (§6.2, §6.3; R62, R1086, R1852000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// The Radiant face's gain, and an equipped host with room to be hurt.
    const MAX_HEALTH_GAIN: i32 = 2;
    const HOST: &str = "stand-in (1) 4/30 R8/60";
    const ENEMY: &str = "stand-in (1) 2/30";

    fn board(radiant: bool) -> Scenario {
        weapon_board(
            "s015",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [ENEMY] }),
        )
    }

    fn equipped(radiant: bool) -> (Scenario, String) {
        let mut s = weapon_board(
            "s015-equip",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [{ "def": HOST, "radiant": radiant }], "mana": 10 }),
            json!({ "field": [ENEMY] }),
        );
        let host = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 1, "equip": true }));
        (s, host)
    }

    mod base {
        use super::*;

        #[test]
        fn r62_it_heals_to_full_at_the_end_of_its_turn_and_gains_no_max_health() {
            let mut s = board(false);
            s.card_mut(ID).damage = 3;
            s.end_turn();
            assert_eq!(s.card(ID).damage, 0);
            s.expect_stats(ID, json!({ "attack": 0, "health": 4, "maxHealth": 4 }));
        }

        #[test]
        fn r62_r1086_an_equipped_host_heals_to_full() {
            let (mut s, host) = equipped(false);
            let max = s.stats(host.as_str()).max_health;
            s.card_mut(host.as_str()).damage = 25;
            s.end_turn();
            s.expect_stats(host.as_str(), json!({ "health": max, "maxHealth": max }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_it_heals_to_full_and_gains_2_max_health_each_end_of_turn() {
            let mut s = board(true);
            s.card_mut(ID).damage = 5;
            s.end_turn();
            let max = 8 + MAX_HEALTH_GAIN;
            s.expect_stats(ID, json!({ "health": max, "maxHealth": max }));
            s.end_turn();
            s.end_turn();
            let max = max + MAX_HEALTH_GAIN;
            s.expect_stats(ID, json!({ "health": max, "maxHealth": max }));
        }

        #[test]
        fn r62_r1086_an_equipped_host_heals_to_full_and_gains_2_max_health() {
            let (mut s, host) = equipped(true);
            let max = s.stats(host.as_str()).max_health + MAX_HEALTH_GAIN;
            s.card_mut(host.as_str()).damage = 25;
            s.end_turn();
            s.expect_stats(host.as_str(), json!({ "health": max, "maxHealth": max }));
        }
    }
}
