//! SPEC §8.12 S+ #121.1 Toxopid Leg, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 2/8
//! **Crater**
//! **Can’t attack**
//! **Taunt**
//! \~\~\~
//! 4/16
//! **Crater**
//! **Can’t attack**
//! **Taunt**
//!
//! Crater, Can't attack and Taunt are catalog data. Since issue #4154 both faces are also "Immune to
//! Toxic", a static flag (`immuneToToxic`, R4154001): no Toxic goes on it, so the Toxopid's own
//! end-of-turn Toxic spares a Leg on its controller's opponent's side.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-121-1";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            immune_to_toxic: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.6.6g the Leg had no script (issue #4154); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #121.1 Toxopid Leg — SPEC §8.12 row 121.1.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;

    fn leg_on(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({}),
        )
    }

    fn slot_of_card(s: &Scenario, id: &str) -> ZoneSlot {
        match &s.card(id).zone {
            Zone::Field { player, row, lane } => ZoneSlot {
                player: *player,
                row: *row,
                lane: *lane,
            },
            zone => panic!("not on the field: {zone:?}"),
        }
    }

    /// R4154001: a Scourge Mortar v3's Toxic adds nothing to a Leg, on either face, while it still Toxics a
    /// Unit beside it.
    #[test]
    fn r4154001_a_leg_takes_no_toxic() {
        const MORTAR: &str = "stinkyscapeplus-146"; // Scourge Mortar v3: the Toxic
        const TOUGH: &str = "stand-in (1) 2/30";
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s121-1-immune",
                json!({ "field": [MORTAR] }),
                json!({ "field": [{ "def": ID, "radiant": radiant }, TOUGH] }),
            );
            let leg = s.unit_at(PlayerId::P2, 1).id;
            let tough = s.unit_at(PlayerId::P2, 2).id;
            s.activate(MORTAR, json!({ "ability": "toxic", "targets": pick::id(&leg) }));
            assert_eq!(jackioh_engine::query::toxic_on(s.card(&leg)), 0, "radiant {radiant}");
            crate::next_turn_of(&mut s, PlayerId::P1);
            s.activate(MORTAR, json!({ "ability": "toxic", "targets": pick::id(&tough) }));
            assert!(jackioh_engine::query::toxic_on(s.card(&tough)) > 0, "radiant {radiant}: a Unit beside it takes it");
        }
    }

    #[test]
    fn its_keywords_are_its_catalogs() {
        for radiant in [false, true] {
            let s = leg_on(radiant, "s121-1-keywords");
            let leg = s.card(ID).clone();
            assert!(unit_has(s.state(), &leg, KeywordKind::Taunt), "radiant {radiant}");
            assert!(
                unit_has(s.state(), &leg, KeywordKind::CantAttack),
                "radiant {radiant}"
            );
            // Can't attack: no attack is offered.
            let offered: Vec<ActionBody> = legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == &leg.id))
                .collect();
            assert!(offered.is_empty(), "radiant {radiant}: Can't attack");
        }
    }

    #[test]
    fn crater_locks_its_zone_on_a_summon() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s121-1-crater",
                json!({ "hand": [{ "def": ID, "radiant": radiant }] }),
                json!({}),
            );
            s.play(ID, json!({ "zone": 3 }));
            let slot = slot_of_card(&s, ID);
            assert_eq!((slot.row, slot.lane), (Row::Units, 3), "radiant {radiant}");
            assert!(is_locked(s.state(), slot), "radiant {radiant}: Crater locks");
            assert!(
                s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::Locked {
                        player: P1,
                        row: Row::Units,
                        lane: 3
                    }
                )),
                "radiant {radiant}: Locked is announced"
            );
        }
    }
}
