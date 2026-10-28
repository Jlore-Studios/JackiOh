//! SPEC §8.10 Stinkyscape 158 Stratos Swarm, a card of the held Stinkyscape set (R1420).
//! (3) Spell, Rare: "Fill your board with Startos." / Radiant: "Fill your board with Radiant
//! Startos." (R1860008).
//!
//! Engine: `fill_board` with Stinkyscape #68 Startos (Core #62 Friend of Felinors' verb): every
//! open unit zone of its caster's, a Dormant card's zone not one (R1839000), each a summoned card
//! of its own definition, no Cry (R1), no token; Radiant `radiant: true`. Each is a Starship
//! Piece, so each that dies fuses (R1852000). A Spell's resolution hangs off `cry` (Core #5
//! Stockpile's shape).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-158";

/// Stinkyscape #68 Startos. Read through `card_def` so a wrong id fails as the script is built,
/// not in play.
fn startos() -> String {
    crate::card_def("stinkyscape-068").id
}

pub fn script() -> CardScripts {
    let base_token = startos();
    let radiant_token = startos();
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
// zone of its caster's with Startos, Flying Starship Pieces, no Cry; a Dormant card's zone is not
// filled; a full board summons none; radiant Radiant Startos".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const SWARM: &str = "stinkyscape-158";
    const STARTOS: &str = "stinkyscape-068"; // 3/3 Flying Starship Piece.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn open() -> PreviewGuard {
        preview_sets(&[SetName::Stinkyscape])
    }

    /// The def ids in p1's five unit zones, `None` where empty.
    fn row(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(PlayerId::P1, lane).map(|unit| unit.def_id))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1860008_fills_every_open_unit_zone_with_startos_no_cry() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "p1": { "hand": [SWARM, ANCHOR], "mana": 10, "field": [VANILLA], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(SWARM, json!({}));

            let lanes = row(&s);
            assert_eq!(lanes[0], Some(VANILLA.to_string()));
            for lane in lanes.iter().skip(1) {
                assert_eq!(lane, &Some(STARTOS.to_string()));
            }
            // Each summoned Startos is base and a Starship Piece.
            let token = s.unit(PlayerId::P1, 2).expect("a Startos");
            assert!(!token.radiant);
            s.expect_stats(token.id.as_str(), json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r1860008_a_full_board_summons_none() {
            crate::register_all();
            let _preview = open();
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
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860008_radiant_fills_every_open_unit_zone_with_radiant_startos() {
            crate::register_all();
            let _preview = open();
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
                let token = s.unit(PlayerId::P1, lane).expect("a Radiant Startos");
                assert_eq!(token.def_id, STARTOS);
                assert!(token.radiant, "lane {lane} is Radiant");
            }
            let token = s.unit(PlayerId::P1, 2).expect("a Startos");
            s.expect_stats(token.id.as_str(), json!({ "attack": 6, "health": 6 }));
        }
    }
}
