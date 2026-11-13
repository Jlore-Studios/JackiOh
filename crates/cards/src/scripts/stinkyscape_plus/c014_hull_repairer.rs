//! SPEC §8.12 S+ #14 Hull Repairer (Unit 0/2 → 0/4, Starship Piece, Equip, cost 1, Common), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            End of turn: Heal this {health} health" — health 4
//!   Radiant: "Starship Piece, Equip
//!            End of turn: Heal this {health} health" — health 8
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the End-of-turn trigger and heals itself (R1852000). `heal` restores
//! the declared number, read through `piece_number` so a Radiant piece heals its 8 wherever it stands, never past
//! full health (§6.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-014";

/// "End of turn: Heal this {health} health", the number read on the face this script was built for
/// (`piece_number`, R1859001): fused into an Equip host, the piece runs its own face's script under the
/// host's, whose `param` would read the host's face.
fn repairer(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            vec![heal(json_as(json!({
                "target": { "of": "self" },
                "amount": piece_number(ctx, ID, radiant, "health"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: repairer(false),
        radiant: repairer(true),
    }
}

// S+ #14 Hull Repairer — SPEC §8.12 row 14 (§6.2, §6.3; R62, R1086, R1852000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// The printed heals, base and Radiant.
    const HEAL: i32 = 4;
    const RADIANT_HEAL: i32 = 8;

    /// An equipped host with room to be hurt, and a durable enemy.
    const HOST: &str = "stand-in (1) 4/30 R8/60";
    const ENEMY: &str = "stand-in (1) 2/30";

    fn board(radiant: bool) -> Scenario {
        weapon_board(
            "s014",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [ENEMY] }),
        )
    }

    fn equipped(radiant: bool) -> (Scenario, String) {
        let mut s = weapon_board(
            "s014-equip",
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
        fn r62_it_heals_4_at_the_end_of_its_turn_and_never_past_full() {
            let mut s = board(false);
            s.card_mut(ID).damage = 1;
            s.end_turn();
            assert_eq!(s.card(ID).damage, 0, "healed no more than the damage it had");
            s.expect_stats(ID, json!({ "health": 2 }));
        }

        #[test]
        fn r62_it_heals_exactly_4_of_a_larger_wound() {
            let (mut s, host) = equipped(false);
            s.card_mut(host.as_str()).damage = HEAL + 3;
            s.end_turn();
            assert_eq!(s.card(host.as_str()).damage, 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_it_heals_8_at_the_end_of_its_turn() {
            let mut s = board(true);
            s.card_mut(ID).damage = 3;
            s.end_turn();
            assert_eq!(s.card(ID).damage, 0);
            s.expect_stats(ID, json!({ "attack": 0, "health": 4 }));
        }

        #[test]
        fn r62_r1086_an_equipped_host_heals_exactly_8() {
            let (mut s, host) = equipped(true);
            s.card_mut(host.as_str()).damage = RADIANT_HEAL + 5;
            s.end_turn();
            assert_eq!(s.card(host.as_str()).damage, 5);
        }
    }
}
