//! Stinkyscape #128 Jlockeed's Yamato Cannon (SPEC §8.10 row 128). (2, Embiggened 3) Unit, Jlockeed, Rare;
//! 4/1 → 8/2.
//!   Base:    "Ranged, Cleave / Embiggened (3): Starship Piece."
//!   Radiant: "Ranged, Cleave, Rush / Embiggened (3): Starship Piece."
//!
//! Ranged (R783000) and Cleave (§6.1, R63) are printed keywords the engine plays: it takes no strike back
//! from a defender without Ranged, and its attack splashes the Units beside its target, which never
//! strike back. R1878001: the Embiggened version is a Starship Piece, lent by its text to the card stamped
//! embiggened, as S #127's, so its death fuses it into its owner's Dormant Starship (R1852000). The
//! Radiant face's 8/2 and, since #2521, its Rush (§6.1: it may attack a Unit, never the hero, the turn it
//! lands) are catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-128";

/// R1878001: "Embiggened (3): Starship Piece.": the keyword holds only on a card stamped embiggened.
fn embiggened_piece(args: HookArgs<'_>) -> Vec<Keyword> {
    if args.self_.embiggened == Some(true) { vec![Keyword::StarshipPiece] } else { Vec::new() }
}

pub fn script() -> CardScripts {
    let base = Script {
        conditional_keywords: Some(read_hook(embiggened_piece)),
        ..Script::default()
    };
    // One script: the Radiant face's 8/2 and Rush are catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Stinkyscape #128 Jlockeed's Yamato Cannon — SPEC §8.10 row 128, BUILD M12 row S 128: "a 4/1 Ranged Cleave
// Unit: its attack hits the Units beside its target too and it takes no strike back from a defender
// without Ranged (R783000); played at (3) it is a Starship Piece, at (2) none; radiant 8/2 with Rush,
// attacking a Unit the turn it lands" (#2521).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CANNON: &str = "stinkyscape-128";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005";

    fn played(radiant: bool, embiggened: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "yamato",
            "p1": {
                "hand": [{ "def": CANNON, "radiant": radiant }, HIT_JOB, FILLER],
                "library": filler(6),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": [
                    { "def": VANILLA, "lane": 2 },
                    { "def": VANILLA, "lane": 3 },
                    { "def": VANILLA, "lane": 4 },
                ],
                "library": filler(6),
            },
        }));
        s.play(CANNON, json!({ "zone": 3, "embiggen": embiggened }));
        s
    }

    fn is_piece(s: &Scenario, id: &str) -> bool {
        has_keyword(&s.stats(id).keywords, KeywordKind::StarshipPiece)
    }

    #[test]
    fn r783000_r63_its_attack_cleaves_the_units_beside_its_target_and_takes_no_strike_back() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "seed": "yamato-attack",
                "p1": { "hand": [FILLER], "field": [{ "def": CANNON, "lane": 3, "radiant": radiant }], "library": filler(6) },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": VANILLA, "lane": 2, "damage": 2 },
                        { "def": VANILLA, "lane": 3 },
                        { "def": VANILLA, "lane": 4, "damage": 2 },
                    ],
                    "library": filler(6),
                },
            }));
            let cannon = s.unit(P1, 3).expect("the Cannon").id;
            let [left, middle, right] = [2, 3, 4].map(|lane| s.unit(P2, lane).expect("a Vanilla").id);
            s.attack(cannon.as_str(), middle.as_str());
            // The target takes the hit, its neighbors the splash, and the 4 back never lands.
            s.expect_in_zone(middle.as_str(), "graveyard");
            s.expect_in_zone(left.as_str(), "graveyard");
            s.expect_in_zone(right.as_str(), "graveyard");
            s.expect_in_zone(cannon.as_str(), "field");
            s.expect_stats(cannon.as_str(), json!({ "health": if radiant { 2 } else { 1 } }));
        }
    }

    #[test]
    fn radiant_rush_attacks_a_unit_the_turn_it_lands_never_the_hero() {
        for embiggened in [false, true] {
            let mut s = played(true, embiggened);
            let cannon = s.unit(P1, 3).expect("the Cannon").id;
            let has_rush = has_keyword(&s.stats(cannon.as_str()).keywords, KeywordKind::Rush);
            assert!(has_rush, "embiggened {embiggened}");
            s.expect_refused(|s| s.attack(cannon.as_str(), "hero"));
            let target = s.unit(P2, 3).expect("a Vanilla").id;
            s.attack(cannon.as_str(), target.as_str());
            // Its 8 kills the 4/4 it hits, and the Cleave hits the 4/4s beside it.
            s.expect_in_zone(target.as_str(), "graveyard");
            assert!(
                s.last_events()
                    .iter()
                    .any(|event| event.event_type() == GameEventType::AttackDeclared),
                "embiggened {embiggened}"
            );
        }
    }

    #[test]
    fn the_base_face_has_no_rush_and_waits_a_turn() {
        let mut s = played(false, false);
        let cannon = s.unit(P1, 3).expect("the Cannon").id;
        assert!(!has_keyword(&s.stats(cannon.as_str()).keywords, KeywordKind::Rush));
        let target = s.unit(P2, 3).expect("a Vanilla").id;
        s.expect_refused(|s| s.attack(cannon.as_str(), target.as_str()));
    }

    #[test]
    fn r1878001_r1852000_played_at_3_it_is_a_starship_piece_and_its_death_fuses_it_into_the_dormant_starship() {
        for radiant in [false, true] {
            let mut s = played(radiant, true);
            let cannon = s.unit(P1, 3).expect("the Cannon").id;
            assert!(is_piece(&s, &cannon), "radiant {radiant}");
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": cannon }] }));
            let ship = dormant_starship_of(s.state(), P1).expect("a Dormant Starship").clone();
            let pieces: Vec<String> = starship_pieces_of(&ship).into_iter().map(|piece| piece.def_id).collect();
            assert_eq!(pieces, vec![CANNON.to_string()], "radiant {radiant}");
        }
    }

    #[test]
    fn r1878001_played_at_2_it_is_no_piece() {
        for radiant in [false, true] {
            let mut s = played(radiant, false);
            let cannon = s.unit(P1, 3).expect("the Cannon").id;
            assert!(!is_piece(&s, &cannon), "radiant {radiant}");
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": cannon }] }));
            assert!(dormant_starship_of(s.state(), P1).is_none(), "radiant {radiant}");
        }
    }
}
