//! S #90 After Burner (SPEC §8.10 row 90; R21, R1852000, R1852003, R1859000, R1859001): (2) Unit,
//! Common, 4/8 → 8/16, Starship Piece, a card of the Stinkyscape set.
//!
//! Base and Radiant: "Starship Piece\nWhen launched, your Starship gains {keywords|random combat
//! keyword|random combat keywords}." Keywords 2, Radiant 4.
//! Engine: Starship Piece is catalog data: its death fuses a copy into its owner's Dormant Starship
//! (R1852000). When launched (`when_launched`, R1852003) runs only as part of that Starship, after its
//! fused Cries; on the field it does nothing. `grant_random_keywords` onto the Starship ("self" there),
//! R21's draws from the combat keyword pool, each one it lacks, fewer when fewer are left (R1859000).
//! The count is read on the face the piece fused on (`piece_number`, R1859001). Tunes: keywords 2 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-090";

fn after_burner(radiant: bool) -> Script {
    Script {
        triggers: vec![when_launched("after-burner", move |ctx, _event| {
            vec![grant_random_keywords(json_as(json!({
                "target": { "of": "self" },
                "count": piece_number(ctx, ID, radiant, "keywords"),
            })))]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: after_burner(false),
        radiant: after_burner(true),
    }
}

// S #90 After Burner — SPEC §8.10 row 90, BUILD M12 row S 90: "on the field it does nothing; its death
// fuses a copy into its owner's Dormant Starship (R1852000); when that Starship launches, after its fused
// Cries, the Starship gains 2 random combat keywords it lacks (R21, R1852003); keywords read on the face
// it fused on (R1859001); radiant 8/16 and 4".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::prelude::{dormant_starship_of, random_pool_keywords};
    use jackioh_engine::testkit::*;

    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.

    fn kinds(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        s.stats(id).keywords.iter().map(Keyword::kind).collect()
    }

    /// The keywords the launch gave the Starship: those it has after it and lacked before.
    fn gained_on_launch(seed: &str, radiant: bool) -> Vec<KeywordKind> {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": ID, "lane": 1, "radiant": radiant }]),
            json!({}),
            json!({}),
        );
        let before = kinds(&s, &ship);
        launch_starship(&mut s, &ship);
        let pool: Vec<KeywordKind> = random_pool_keywords().iter().map(Keyword::kind).collect();
        let gained: Vec<KeywordKind> = kinds(&s, &ship)
            .into_iter()
            .filter(|kind| !before.contains(kind) && *kind != KeywordKind::Starship)
            .collect();
        assert!(gained.iter().all(|kind| pool.contains(kind)), "{gained:?}");
        gained
    }

    #[test]
    fn r1859000_when_launched_the_starship_gains_2_random_combat_keywords_it_lacks() {
        assert_eq!(gained_on_launch("r1859000-base", false).len(), 2);
    }

    #[test]
    fn r1859001_a_radiant_piece_gives_4_read_on_the_face_it_fused_on() {
        assert_eq!(gained_on_launch("r1859000-radiant", true).len(), 4);
    }

    #[test]
    fn r1859000_on_the_field_it_does_nothing_when_a_starship_launches() {
        let (mut s, ship) = built_starship(
            "r1859000-field",
            json!([{ "def": AP_ROUNDS, "lane": 1 }]),
            json!({ "field": [{ "def": ID, "lane": 3 }] }),
            json!({}),
        );
        let burner = s.unit_at(PlayerId::P1, 3).id;
        let before = kinds(&s, &burner);
        let ship_before = kinds(&s, &ship);
        launch_starship(&mut s, &ship);
        assert_eq!(kinds(&s, &burner), before);
        let ship_after: Vec<KeywordKind> = kinds(&s, &ship)
            .into_iter()
            .filter(|kind| *kind != KeywordKind::Starship)
            .collect();
        assert_eq!(ship_after, ship_before, "the AP Rounds' Starship gains nothing");
        assert!(dormant_starship_of(s.state(), PlayerId::P1).is_none());
    }
}
