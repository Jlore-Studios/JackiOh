//! SPEC §8.12 S+ #65 Thorium Wall, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! 0/10 embiggen 0/40 (0/8 embiggen 0/32 until issue #4353)  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Embiggen: GIGA**  
//! \~\~\~  
//! 0/20 embiggen 0/80 (0/16 embiggen 0/64 until issue #4353)  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Armor 2**  
//! **Embiggen: GIGA**
//!
//! An embiggened Wall is GIGA at four times its printed face stats (R3555001): `conditional_keywords`
//! lends GIGA to the card stamped embiggened (R1878001, R3546002), and `set_stat` adds three times the
//! printed face (§10.4 layer 2, read off the definition, never `face_of` or `unit_view`, or the layers
//! recurse), none once a Set stats has replaced the stat line (R3935000), so a set stands at exactly its
//! stats and a Buff or Nerf counts once, through layer 1. A Vanilla takes both back to the printed stats
//! (R115) while the size stays (R3546002). Armor 2 on the Radiant face is catalog data. Through v0.6.5c
//! the bonus was three times `face_of`, which a set or a Buff multiplied (issue #3935); `script_v0_6_5c`
//! keeps that for those replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-065";

/// "Embiggened (4): GIGA, 0/40": the embiggened stats are this many times the face.
pub const EMBIGGENED_SCALE: i32 = 4;

/// R3555001: the Walls' shared text. Embiggened, the card is GIGA at `EMBIGGENED_SCALE` times its printed
/// face stats; played at its base price it is neither. R3935000: no bonus once a Set stats has replaced
/// its stat line, and a Buff or Nerf counts once, at layer 1.
pub fn embiggened_wall() -> Script {
    Script {
        conditional_keywords: embiggened_giga(),
        set_stat: Some(read_hook(|a| {
            if a.self_.embiggened != Some(true) || a.self_.stats_override.is_some() {
                return SetStat::default();
            }
            let def = def_of(Some(a.state), &a.self_.def_id);
            let face = if a.self_.radiant { &def.radiant } else { &def.base };
            SetStat {
                attack: Some((EMBIGGENED_SCALE - 1) * face.attack.unwrap_or(0)),
                max_health: Some((EMBIGGENED_SCALE - 1) * face.health.unwrap_or(0)),
            }
        })),
        ..Script::default()
    }
}

/// R767000: the Walls' shared text through v0.6.5c: three times `face_of`, which a set or a Buff
/// multiplied (issue #3935).
pub fn embiggened_wall_v0_6_5c() -> Script {
    Script {
        conditional_keywords: embiggened_giga(),
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

/// R1878001, R3546002: GIGA for the card stamped embiggened.
fn embiggened_giga() -> Option<ConditionalKeywordsHook> {
    Some(read_hook(|a| {
        if a.self_.embiggened == Some(true) {
            vec![Keyword::Giga]
        } else {
            vec![]
        }
    }))
}

pub fn script() -> CardScripts {
    // Armor 2 on the Radiant face is catalog data.
    let base = embiggened_wall();
    let radiant = embiggened_wall();
    CardScripts { base, radiant }
}

/// R767000: the scripts through v0.6.5c, whose bonus a set or a Buff multiplied (issue #3935).
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5c() -> CardScripts {
    CardScripts {
        base: embiggened_wall_v0_6_5c(),
        radiant: embiggened_wall_v0_6_5c(),
    }
}

/// The Walls' tests share these: the wall `id` played embiggened from p1's hand, a Set stats on every
/// Unit, and a tuning (a Buff or Nerf's stat change) laid on a card.
#[cfg(test)]
pub(super) mod fixture {
    use jackioh_engine::effects::set_stats_all;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    pub fn embiggened(id: &str, radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({ "hand": [{ "def": id, "radiant": radiant }], "mana": 10 }),
            json!({}),
        );
        s.play(id, json!({ "zone": 1, "embiggen": true }));
        s
    }

    pub fn set_all(s: &mut Scenario, attack: i32, health: i32) {
        crate::run_as(
            s,
            set_stats_all(json_as(json!({ "side": "any", "attack": attack, "health": health }))),
            PlayerId::P1,
        );
    }

    pub fn tune(s: &mut Scenario, id: &str, tuning: Value) {
        s.card_mut(id).tuning = Some(json_as(tuning));
    }

    /// R3935000: a set stands at the stats it sets, whatever the card's size.
    pub fn assert_a_set_stands_at_its_stats(id: &str) {
        for radiant in [false, true] {
            let mut s = embiggened(id, radiant, &format!("{id}-set-{radiant}"));
            for stats in [3, 10] {
                set_all(&mut s, stats, stats);
                s.expect_stats(id, json!({ "attack": stats, "maxHealth": stats }));
                assert_eq!(s.stats(id).health, stats, "radiant {radiant}");
                assert_eq!(s.card(id).giga, Some(true), "radiant {radiant}");
            }
        }
    }

    /// R3935000: a Buff counts once, on the field or taken in hand first.
    pub fn assert_a_buff_counts_once(id: &str, faces: [i32; 2]) {
        for (radiant, face) in [(false, faces[0]), (true, faces[1])] {
            let mut s = embiggened(id, radiant, &format!("{id}-buff-{radiant}"));
            tune(&mut s, id, json!({ "attack": 2, "health": 5 }));
            let health = 4 * face + 5;
            s.expect_stats(id, json!({ "attack": 2, "maxHealth": health }));
            set_all(&mut s, 3, 3);
            tune(&mut s, id, json!({ "attack": 2, "health": 1 }));
            s.expect_stats(id, json!({ "attack": 5, "maxHealth": 4 }));
        }
    }
}

// S+ #65 Thorium Wall — SPEC §8.12 row S+ 65 (R115, R1878001, R3546002, R3555001, R3935000).
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
        let mut lanes: Vec<i32> = s
            .plays_of(ID)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    zone,
                    embiggen: offered,
                    ..
                } if offered == Some(embiggen) => zone,
                _ => None,
            })
            .filter(|zone| zone.row == Row::Units)
            .map(|zone| zone.lane)
            .collect();
        lanes.sort();
        lanes
    }

    #[test]
    fn r3555001_normal_it_fills_one_zone_at_0_10() {
        for radiant in [false, true] {
            let mut s = holding(radiant, &format!("s65-normal-{radiant}"));
            assert_eq!(offered_lanes(&s, false), vec![1, 2, 3, 4, 5], "radiant {radiant}");
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.card(ID).giga, None, "radiant {radiant}: no GIGA stamp");
            let stats = if radiant { (0, 20) } else { (0, 10) };
            s.expect_stats(ID, json!({ "attack": stats.0, "maxHealth": stats.1 }));
            assert!(
                is_empty(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 2 }),
                "radiant {radiant}: lane 2 stays open"
            );
        }
    }

    #[test]
    fn r3555001_embiggened_it_is_giga_at_0_40() {
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
            let health = if radiant { 80 } else { 40 };
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": health }));
            if radiant {
                assert_eq!(s.stats(ID).armor, 2);
            }
        }
    }

    #[test]
    fn r3555001_a_vanilla_returns_it_to_0_10_and_keeps_its_size() {
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
        s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 10 }));
        assert_eq!(s.card(ID).giga, Some(true));
        for (row, lane) in [(Row::Units, 1), (Row::Units, 2), (Row::Backrow, 1), (Row::Backrow, 2)] {
            assert!(
                !is_empty(s.state(), ZoneSlot { player: P1, row, lane }),
                "{row} {lane} is still filled"
            );
        }
    }

    #[test]
    fn r3935000_a_set_stands_at_the_stats_it_sets() {
        fixture::assert_a_set_stands_at_its_stats(ID);
    }

    #[test]
    fn r3935000_a_buff_counts_once() {
        fixture::assert_a_buff_counts_once(ID, [10, 20]);
    }

    #[test]
    fn r3935000_a_nerf_counts_once() {
        for radiant in [false, true] {
            let mut s = fixture::embiggened(ID, radiant, &format!("s65-nerf-{radiant}"));
            fixture::tune(&mut s, ID, json!({ "health": -4 }));
            let health = if radiant { 76 } else { 36 };
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": health }));
        }
    }

    #[test]
    fn r3935000_a_buff_taken_in_hand_counts_once() {
        for radiant in [false, true] {
            let mut s = holding(radiant, &format!("s65-hand-{radiant}"));
            fixture::tune(&mut s, ID, json!({ "attack": 1, "health": 2 }));
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            let health = if radiant { 82 } else { 42 };
            s.expect_stats(ID, json!({ "attack": 1, "maxHealth": health }));
        }
    }

    /// R767000: through v0.6.5c a set was multiplied by the size (issue #3935).
    #[test]
    fn r767000_the_script_kept_for_v0_6_5c_multiplies_a_set() {
        crate::under_kept(ID, script_v0_6_5c(), || {
            let mut s = fixture::embiggened(ID, false, "s65-kept");
            fixture::set_all(&mut s, 3, 3);
            s.expect_stats(ID, json!({ "attack": 12, "maxHealth": 12 }));
        });
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
