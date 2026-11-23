//! SPEC §8.10 Stinkyscape 158 Stratos Swarm, a card of the Stinkyscape set.
//! (4) Spell, Rare: "Summon {stratos} Stratos." / Radiant: "Summon {stratos} Radiant Stratos." — stratos 7
//! (R1860008). Through v0.6.9n: "Fill your board with Stratos." / "Fill your board with Radiant Stratos."
//! (`script_v0_6_9n`, kept, R767000; issue #4440).
//!
//! Engine (R4440051): one summon of Stinkyscape #68 Stratos per `stratos` (R386), each into its caster's
//! leftmost open unit zone in turn (R64), a Dormant card's zone not one (R1839000), and once the unit row
//! is full, as Stratos flies, into the leftmost open backrow zone, where it stands (R2248000), as Drone
//! Core III's summons go; a summon that finds no zone left is lost, as a Hearthstone summon past a full
//! board is. Each is a summoned card of its own definition, no Cry (R1), no token; Radiant `radiant: true`.
//! Each is a 3/1 (Radiant 6/2) Flying, Ranged Unit; since #2521 none is a Starship Piece, so one that dies
//! fuses nothing. A Spell's resolution hangs off `cry` (Core #5 Stockpile's shape). Through v0.6.9n it
//! was `fill_board` (Core #62 Friend of Felinors' verb): every open unit zone, then every empty backrow
//! zone, up to ten.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-158";

/// Stinkyscape #68 Stratos. Read through `card_def` so a wrong id fails as the script is built,
/// not in play.
fn stratos() -> String {
    crate::card_def("stinkyscape-068").id
}

/// R4440051: "Summon {stratos} Stratos", Radiant on `radiant`: one summon per Stratos, left to right.
fn swarm(radiant: bool) -> Script {
    let token = stratos();
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(&*ctx, "stratos"))
                .map(|_| summon(json_as(json!({ "defId": token, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: swarm(false),
        radiant: swarm(true),
    }
}

/// R767000: through v0.6.9n it filled its caster's board (issue #4440); `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_6_9n() -> CardScripts {
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

// Stinkyscape #158 Stratos Swarm — SPEC §8.10 row 158, BUILD M12 row S 158 (R2248000, R4440051). (#2521: no
// Starship Pieces).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const SWARM: &str = "stinkyscape-158";
    const STRATOS: &str = "stinkyscape-068"; // Stratos: Flying, Ranged
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // Rapid Replenish: Spell (§2.5).
    const WELL: &str = "core-006"; // Mana Well: a Field Spell to fill a backrow zone.
    /// The declared `stratos`, on both faces (issue #4440).
    const SUMMONS: usize = 7;

    fn row(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(PlayerId::P1, lane).map(|unit| unit.def_id))
            .collect()
    }

    fn backrow(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.backrow(PlayerId::P1, lane).map(|card| card.def_id))
            .collect()
    }

    fn summoned(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { .. }))
            .count()
    }

    /// p1 holds the Swarm (`radiant` its face) beside `field` and `back`, with mana to cast it.
    fn holding(radiant: bool, field: Value, back: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": SWARM, "radiant": radiant }, ANCHOR],
                "mana": 10,
                "field": field,
                "backrow": back,
                "library": filler(4),
            },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }))
    }

    #[test]
    fn r1860008_the_swarm_prints_the_name_stinkyscape_68_carries() {
        crate::register_all();
        let stratos = crate::card_def(STRATOS);
        let swarm = crate::card_def(SWARM);
        assert_eq!(stratos.name, "Stratos");
        assert_eq!(swarm.base.text, format!("Summon {{stratos}} {}.", stratos.name));
        assert_eq!(swarm.radiant.text, format!("Summon {{stratos}} Radiant {}.", stratos.name));
    }

    mod base {
        use super::*;

        #[test]
        fn r4440051_r2248000_summons_seven_the_open_unit_zones_first_then_the_backrow_no_cry() {
            let mut s = holding(false, json!([VANILLA]), json!([]));
            s.play(SWARM, json!({}));

            let lanes = row(&s);
            assert_eq!(lanes[0], Some(VANILLA.to_string()));
            for lane in lanes.iter().skip(1) {
                assert_eq!(lane, &Some(STRATOS.to_string()));
            }
            // Four into the unit row, the other three into the leftmost backrow zones (R2248000).
            let stratos = Some(STRATOS.to_string());
            assert_eq!(
                backrow(&s),
                vec![stratos.clone(), stratos.clone(), stratos, None, None],
                "it flies"
            );
            assert_eq!(summoned(&s), SUMMONS);
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
        fn r4440051_a_board_with_room_for_fewer_takes_what_fits_and_the_rest_are_lost() {
            let full = json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]);
            let mut s = holding(false, full.clone(), json!([WELL, WELL, WELL]));
            s.play(SWARM, json!({}));
            for lane in row(&s) {
                assert_eq!(lane, Some(VANILLA.to_string()));
            }
            let stratos = Some(STRATOS.to_string());
            let well = Some(WELL.to_string());
            assert_eq!(
                backrow(&s),
                vec![well.clone(), well.clone(), well, stratos.clone(), stratos]
            );
            assert_eq!(summoned(&s), 2, "two zones were open: the other five are lost");

            let mut s = holding(false, full, json!([WELL, WELL, WELL, WELL, WELL]));
            s.play(SWARM, json!({}));
            assert_eq!(summoned(&s), 0, "nothing summoned");
            s.expect_in_zone(SWARM, "graveyard");
        }

        #[test]
        fn r386_stratos_reads_through_param() {
            let mut s = holding(false, json!([]), json!([]));
            set_param(s.card_mut(SWARM), "stratos", 3);
            s.play(SWARM, json!({}));
            let stratos = Some(STRATOS.to_string());
            assert_eq!(row(&s), vec![stratos.clone(), stratos.clone(), stratos, None, None]);
            assert_eq!(summoned(&s), 3);
        }

        #[test]
        fn a_stratos_that_dies_fuses_nothing() {
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
        fn r1839000_a_dormant_cards_zone_is_not_summoned_into() {
            crate::register_all();
            // A Dormant Unit in lane 2 (a Dormant Starship stands in the backrow, R2249000).
            let mut s = holding(
                false,
                json!([VANILLA, { "def": VANILLA, "lane": 2, "slumber": { "turns": 2 } }]),
                json!([]),
            );
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
            assert_eq!(summoned(&s), SUMMONS, "three in the units, four in the backrow");
        }

        /// R767000: through v0.6.9n it filled the board: every open unit zone, then every backrow zone.
        #[test]
        fn r767000_the_script_kept_for_v0_6_9n_fills_the_board() {
            crate::under_kept(SWARM, script_v0_6_9n(), || {
                let mut s = holding(false, json!([VANILLA]), json!([]));
                s.play(SWARM, json!({}));
                assert_eq!(backrow(&s), vec![Some(STRATOS.to_string()); 5]);
                assert_eq!(summoned(&s), 9, "four units and five in the backrow");
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4440051_radiant_summons_seven_radiant_stratos() {
            let mut s = holding(true, json!([VANILLA]), json!([]));
            s.play(SWARM, json!({}));

            for lane in 2..=5 {
                let token = s.unit(PlayerId::P1, lane).expect("a Radiant Stratos");
                assert_eq!(token.def_id, STRATOS);
                assert!(token.radiant, "lane {lane} is Radiant");
            }
            for lane in 1..=3 {
                let token = s.backrow(PlayerId::P1, lane).expect("a Radiant Stratos in the backrow");
                assert!(token.radiant, "backrow {lane} is Radiant");
            }
            assert_eq!(summoned(&s), SUMMONS);
            let token = s.unit(PlayerId::P1, 2).expect("a Stratos");
            s.expect_stats(token.id.as_str(), json!({ "attack": 6, "health": 2 }));
        }
    }
}
