//! SPEC §8.10 Stinkyscape 132 Defensive Crystal, a card of the Stinkyscape set.
//! (2) Unit, Common, 3/6 → 6/12: Starship Piece.
//! "Death: Your hero gains {heroArmor} Armor." — heroArmor 1, Radiant 2 (R1860006).
//!
//! Engine: Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned
//! first if they have none (R1852000). Death (§6.2): `gain_hero_armor`, §6.1's Armor on the hero,
//! kept for the rest of the game and taken off each hit on the hero, as Meditative #95's "your
//! hero gains 6 Armor" (R124). The Radiant face runs the same script: its Armor is the declared
//! `heroArmor` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-132";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![gain_hero_armor(json_as(json!({
                "amount": param(ctx, "heroArmor"),
            })))]
        })),
        ..Script::default()
    };
    // One script: the Radiant 2 is the declared `heroArmor`, which `param` reads off the running
    // face; its stats are the catalog's.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #132 Defensive Crystal — SPEC §8.10 row 132, BUILD M12 row S 132 (R124, R1852000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const CRYSTAL: &str = "stinkyscape-132";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn armor_of(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.armor
    }

    mod base {
        use super::*;

        #[test]
        fn r1860006_death_gives_your_hero_1_armor_then_fuses() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [CRYSTAL], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            assert_eq!(armor_of(&s, PlayerId::P1), 0);
            let me = s.card(CRYSTAL).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(CRYSTAL, "graveyard");
            assert_eq!(armor_of(&s, PlayerId::P1), 1);
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
        }

        #[test]
        fn hero_armor_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [CRYSTAL], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let armor = crate::upgrade_number(&mut s, CRYSTAL, "heroArmor");
            let me = s.card(CRYSTAL).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            assert_eq!(armor_of(&s, PlayerId::P1), armor);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860006_radiant_death_gives_your_hero_2_armor() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [{ "def": CRYSTAL, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CRYSTAL).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(CRYSTAL, "graveyard");
            assert_eq!(armor_of(&s, PlayerId::P1), 2);
        }
    }
}
