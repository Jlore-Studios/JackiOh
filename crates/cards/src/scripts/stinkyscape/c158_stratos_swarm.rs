//! SPEC §8.10 Stinkyscape 158 Stratos Swarm, a card of the Stinkyscape set.
//! (3) Spell, Rare: "Fill your board with Stratos." / Radiant: "Fill your board with Radiant
//! Stratos." (R1860008).
//!
//! Engine: `fill_board` with Stinkyscape #68 Stratos (Core #62 Friend of Felinors' verb): every
//! open unit zone of its caster's, a Dormant card's zone not one (R1839000), then, as Stratos flies,
//! every empty backrow zone, where each stands (R2248000), each a summoned card of its own
//! definition, no Cry (R1), no token; Radiant `radiant: true`. Each is a 3/1 (Radiant 6/2) Flying,
//! Ranged Unit; since #2521 none is a Starship Piece, so one that dies fuses nothing. A Spell's
//! resolution hangs off `cry` (Core #5 Stockpile's shape).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-158";

/// Stinkyscape #68 Stratos. Read through `card_def` so a wrong id fails as the script is built,
/// not in play.
fn stratos() -> String {
    crate::card_def("stinkyscape-068").id
}

pub fn script() -> CardScripts {
    let base_token = stratos();
    let radiant_token = stratos();
    CardScripts {
        base: Script {
            cry: Some(hook(move |_ctx| vec![fill_board(json_as(json!({ "defId": base_token })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(move |_ctx| {
                vec![fill_board(json_as(json!({ "defId": radiant_token, "radiant": true })))]
            })),
            ..Script::default()
        },
    }
}

// Stinkyscape #158 Stratos Swarm — SPEC §8.10 row 158, BUILD M12 row S 158: "fills every open unit
// zone of its caster's with Stratos, 3/1 Flying Ranged Units, no Cry, then every empty backrow zone
// (R2248000); a Dormant card's zone is not filled; a full board, both rows, summons none; radiant
// Radiant Stratos, 6/2" (#2521: no Starship Pieces).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const SWARM: &str = "stinkyscape-158";
    const STRATOS: &str = "stinkyscape-068"; // (1) 3/1 Flying, Ranged.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const WELL: &str = "core-006"; // Mana Well, a Field Spell to fill a backrow zone.

    /// The def ids in p1's five unit zones, `None` where empty.
    fn row(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(PlayerId::P1, lane).map(|unit| unit.def_id))
            .collect()
    }

    /// The def ids in p1's five backrow zones, `None` where empty.
    fn backrow(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.backrow(PlayerId::P1, lane).map(|card| card.def_id))
            .collect()
    }

    #[test]
    fn r1860008_the_swarm_prints_the_name_stinkyscape_68_carries() {
        crate::register_all();
        let stratos = crate::card_def(STRATOS);
        let swarm = crate::card_def(SWARM);
        assert_eq!(stratos.name, "Stratos");
        assert_eq!(swarm.base.text, format!("Fill your board with {}.", stratos.name));
        assert_eq!(swarm.radiant.text, format!("Fill your board with Radiant {}.", stratos.name));
    }

    mod base {
        use super::*;

        #[test]
        fn r1860008_r2248000_fills_every_open_unit_zone_then_the_backrow_with_stratos_no_cry() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SWARM, ANCHOR], "mana": 10, "field": [VANILLA], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(SWARM, json!({}));

            let lanes = row(&s);
            assert_eq!(lanes[0], Some(VANILLA.to_string()));
            for lane in lanes.iter().skip(1) {
                assert_eq!(lane, &Some(STRATOS.to_string()));
            }
            assert_eq!(backrow(&s), vec![Some(STRATOS.to_string()); 5], "it flies (R2248000)");
            // Each summoned Stratos is base, a 3/1 Flying Ranged Unit and no Starship Piece.
            let token = s.unit(PlayerId::P1, 2).expect("a Stratos");
            assert!(!token.radiant);
            s.expect_stats(token.id.as_str(), json!({ "attack": 3, "health": 1 }));
            let keywords = s.stats(token.id.as_str()).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Flying));
            assert!(has_keyword(&keywords, KeywordKind::Ranged));
            assert!(!has_keyword(&keywords, KeywordKind::StarshipPiece));
        }

        #[test]
        fn a_stratos_that_dies_fuses_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SWARM, HIT_JOB, ANCHOR], "mana": 10, "field": [VANILLA], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(SWARM, json!({}));
            let token = s.unit_at(PlayerId::P1, 2);
            s.play(HIT_JOB, json!({ "targets": pick::instance(&token) }));
            s.expect_in_zone(&token, "graveyard");
            assert!(dormant_starship_of(s.state(), PlayerId::P1).is_none(), "no Dormant Starship");
        }

        #[test]
        fn r1839000_a_dormant_cards_zone_is_not_filled() {
            crate::register_all();
            // A Dormant Unit in lane 2 (a Dormant Starship stands in the backrow, R2249000).
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SWARM, ANCHOR],
                    "mana": 10,
                    "field": [VANILLA, { "def": VANILLA, "lane": 2, "slumber": { "turns": 2 } }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let dormant = s.unit_at(PlayerId::P1, 2).id;
            assert!(is_slumbering(s.card(dormant.as_str())));
            s.play(SWARM, json!({}));

            let lanes = row(&s);
            assert_eq!(lanes[0], Some(VANILLA.to_string()));
            assert_eq!(
                s.unit_at(PlayerId::P1, 2).id,
                dormant,
                "lane 2 still tops with the Dormant card"
            );
            let slot = ZoneSlot {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 2,
            };
            assert_eq!(
                pile_at(s.state(), slot).map(|pile| pile.len()),
                Some(1),
                "no Stratos stacked onto it"
            );
            for lane in lanes.iter().skip(2) {
                assert_eq!(lane, &Some(STRATOS.to_string()));
            }
        }

        #[test]
        fn r1860008_r2248000_a_full_unit_row_fills_the_backrow_and_a_full_board_summons_none() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SWARM, ANCHOR],
                    "mana": 10,
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(SWARM, json!({}));

            for lane in row(&s) {
                assert_eq!(lane, Some(VANILLA.to_string()));
            }
            assert_eq!(backrow(&s), vec![Some(STRATOS.to_string()); 5]);
            let summoned = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Summoned { .. }))
                .count();
            assert_eq!(summoned, 5);

            // Both rows full: none.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SWARM, ANCHOR],
                    "mana": 10,
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "backrow": [WELL, WELL, WELL, WELL, WELL],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(SWARM, json!({}));
            assert!(
                !s.events().iter().any(|event| matches!(event, GameEvent::Summoned { .. })),
                "nothing summoned"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860008_radiant_fills_every_open_unit_zone_with_radiant_stratos() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": SWARM, "radiant": true }, ANCHOR],
                    "mana": 10,
                    "field": [VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(SWARM, json!({}));

            for lane in 2..=5 {
                let token = s.unit(PlayerId::P1, lane).expect("a Radiant Stratos");
                assert_eq!(token.def_id, STRATOS);
                assert!(token.radiant, "lane {lane} is Radiant");
            }
            let token = s.unit(PlayerId::P1, 2).expect("a Stratos");
            s.expect_stats(token.id.as_str(), json!({ "attack": 6, "health": 2 }));
        }
    }
}
