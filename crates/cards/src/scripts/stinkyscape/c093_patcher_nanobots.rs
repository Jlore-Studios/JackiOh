//! SPEC §8.10 Stinkyscape 93 Patcher Nanobots, a card of the Stinkyscape set.
//! (2) Unit, Common, 3/4 → 6/8: Starship Piece, Taunt.
//! "Death: Heal your hero {heal}." — heal 6, Radiant 12.
//!
//! Engine: Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned
//! first if they have none (R1852000). Death (§6.2): `heal` on its controller's hero as it died
//! (R89), no cap on a hero (§6.3 Heal). The Radiant face runs the same script: its heal is the
//! declared `heal` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-093";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![heal(json_as(json!({
                "target": { "of": "selfHero" },
                "amount": param(ctx, "heal"),
            })))]
        })),
        ..Script::default()
    };
    // One script: the Radiant heal 12 is the declared `heal`, which `param` reads off the running
    // face; its stats are the catalog's.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #93 Patcher Nanobots — SPEC §8.10 row 93, BUILD M12 row S 93 (R1852000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const NANOBOTS: &str = "stinkyscape-093";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    mod base {
        use super::*;

        #[test]
        fn death_heals_your_hero_6_past_its_starting_health_then_fuses() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "health": 29, "field": [NANOBOTS], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(NANOBOTS).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(NANOBOTS, "graveyard");
            s.expect_health(PlayerId::P1, 35);
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
            assert_eq!(
                starship_pieces_of(&ship.expect("the ship"))
                    .iter()
                    .map(|piece| piece.def_id.as_str())
                    .collect::<Vec<_>>(),
                vec![NANOBOTS]
            );
        }

        #[test]
        fn heal_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "health": 20, "field": [NANOBOTS], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let heal = crate::upgrade_number(&mut s, NANOBOTS, "heal");
            let me = s.card(NANOBOTS).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_health(PlayerId::P1, 20 + heal);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_death_heals_your_hero_12() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "health": 29,
                    "field": [{ "def": NANOBOTS, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(NANOBOTS).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(NANOBOTS, "graveyard");
            s.expect_health(PlayerId::P1, 41);
        }
    }
}
