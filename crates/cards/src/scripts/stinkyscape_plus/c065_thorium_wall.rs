//! SPEC §8.12 S+ #65 Thorium Wall, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! 0/8 embiggen 0/32  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Embiggen: GIGA**  
//! \~\~\~  
//! 0/16 embiggen 0/64  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Armor 2**  
//! **Embiggen: GIGA**
//!
//! An embiggened Wall is GIGA at four times its face stats (R3555001): `conditional_keywords` lends
//! GIGA to the card stamped embiggened (R1878001, R3546002), and `set_stat` adds three times the face
//! (§10.4 layer 2, read off `face_of`, never `unit_view`, or the layers recurse). A Vanilla takes both
//! back to the printed stats (R115) while the size stays (R3546002). Armor 2 on the Radiant face is
//! catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-065";

/// "Embiggened (4): GIGA, 0/32": the embiggened stats are this many times the face.
pub const EMBIGGENED_SCALE: i32 = 4;

/// R3555001: the Walls' shared text. Embiggened, the card is GIGA at `EMBIGGENED_SCALE` times its face
/// stats; played at its base price it is neither.
pub fn embiggened_wall() -> Script {
    Script {
        conditional_keywords: Some(read_hook(|a| {
            if a.self_.embiggened == Some(true) {
                vec![Keyword::Giga]
            } else {
                vec![]
            }
        })),
        set_stat: Some(read_hook(|a| {
            if a.self_.embiggened != Some(true) {
                return SetStat::default();
            }
            let face = face_of(a.state, a.self_);
            SetStat {
                attack: Some((EMBIGGENED_SCALE - 1) * face.attack),
                max_health: Some((EMBIGGENED_SCALE - 1) * face.health),
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Armor 2 on the Radiant face is catalog data.
    let base = embiggened_wall();
    let radiant = embiggened_wall();
    CardScripts { base, radiant }
}

// S+ #65 Thorium Wall — SPEC §8.12 row S+ 65 (R115, R1878001, R3546002, R3555001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::{ZoneSlot, is_empty};

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// The wall in p1's hand with mana to play it either way.
    fn holding(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            json!({}),
        )
    }

    /// Every unit lane a play of the wall is offered, sorted, embiggened or not.
    fn offered_lanes(s: &Scenario, embiggen: bool) -> Vec<i32> {
        let id = s.card(ID).id.clone();
        let mut lanes: Vec<i32> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id,
                    zone,
                    embiggen: offered,
                    ..
                } if instance_id == id && offered == Some(embiggen) => zone,
                _ => None,
            })
            .filter(|zone| zone.row == Row::Units)
            .map(|zone| zone.lane)
            .collect();
        lanes.sort();
        lanes
    }

    #[test]
    fn r3555001_normal_it_fills_one_zone_at_0_8() {
        for radiant in [false, true] {
            let mut s = holding(radiant, &format!("s65-normal-{radiant}"));
            assert_eq!(offered_lanes(&s, false), vec![1, 2, 3, 4, 5], "radiant {radiant}");
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.card(ID).giga, None, "radiant {radiant}: no GIGA stamp");
            let stats = if radiant { (0, 16) } else { (0, 8) };
            s.expect_stats(ID, json!({ "attack": stats.0, "maxHealth": stats.1 }));
            assert!(
                is_empty(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 2 }),
                "radiant {radiant}: lane 2 stays open"
            );
        }
    }

    #[test]
    fn r3555001_embiggened_it_is_giga_at_0_32() {
        for radiant in [false, true] {
            let mut s = holding(radiant, &format!("s65-embiggened-{radiant}"));
            assert_eq!(offered_lanes(&s, true), vec![1, 2, 3, 4], "radiant {radiant}");
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            assert_eq!(s.card(ID).giga, Some(true), "radiant {radiant}");
            for (row, lane) in [(Row::Units, 1), (Row::Units, 2), (Row::Backrow, 1), (Row::Backrow, 2)] {
                assert!(
                    !is_empty(s.state(), ZoneSlot { player: P1, row, lane }),
                    "radiant {radiant}: {row} {lane} is filled"
                );
            }
            let health = if radiant { 64 } else { 32 };
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": health }));
            if radiant {
                assert_eq!(s.stats(ID).armor, 2);
            }
        }
    }

    #[test]
    fn r3555001_a_vanilla_returns_it_to_0_8_and_keeps_its_size() {
        let mut s = holding(false, "s65-vanilla");
        s.play(ID, json!({ "zone": 1, "embiggen": true }));
        assert_eq!(s.card(ID).giga, Some(true));
        let id = s.card(ID).id.clone();
        crate::run_as(
            &mut s,
            jackioh_engine::effects::transform::vanilla(json_as(json!({ "instanceId": id }))),
            P1,
        );
        // R115: back to the printed stats; R3546002: the stamped size stays.
        s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 8 }));
        assert_eq!(s.card(ID).giga, Some(true));
        for (row, lane) in [(Row::Units, 1), (Row::Units, 2), (Row::Backrow, 1), (Row::Backrow, 2)] {
            assert!(
                !is_empty(s.state(), ZoneSlot { player: P1, row, lane }),
                "{row} {lane} is still filled"
            );
        }
    }

    #[test]
    fn cant_attack() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                &format!("s65-cant-{radiant}"),
                json!({ "field": [{ "def": ID, "radiant": radiant }] }),
                json!({ "field": [VANILLA] }),
            );
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }
}
