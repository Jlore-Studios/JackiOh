//! SPEC §8.10 Stinkyscape 67 Tractor Beam, a card of the Stinkyscape set.
//!   Base:    "Starship Piece, Equip\nStart of turn: Gain a random Stinkyscape Resource." (0/2)
//!   Radiant: "Starship Piece, Equip\nStart of turn: Gain a random Radiant Stinkyscape Resource." (0/4)
//!
//! Starship Piece and Equip are printed keywords, the engine's: its death fuses a copy into its owner's
//! Dormant Starship (R1852000), and played onto one of its controller's Units it fuses into that Unit,
//! the host taking its Start of turn line and staying no piece (R1086, R1852001). Start of turn (R62):
//! `gain_resources` of one random Resource into its controller's hand (R1840000, R1840002's roll), a
//! ninth sold at once (R1840001); the first gain opens the Marketplace (R1853000). Radiant: a Radiant
//! Resource.

use jackioh_engine::effects::gain_resources;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-067";

/// "Gain a random Stinkyscape Resource": one, printed in the words.
const RESOURCES: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![gain_resources(json_as(
                json!({ "count": RESOURCES, "radiant": radiant }),
            ))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S 67 Tractor Beam — SPEC §8.10 row 67, BUILD M12 row S 67 (R1852000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // 9/9
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents

    fn beam(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "library": filler(4), "field": [{ "def": ID, "radiant": radiant }] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn gains_at_its_start_of_turn_not_the_opponents() {
        let mut s = beam("beam-turns", false);
        s.end_turn();
        assert!(
            crate::resources_in_hand(&s, P1).is_empty(),
            "the opponent's start of turn gains nothing"
        );
        assert!(crate::resources_in_hand(&s, P2).is_empty());
        s.end_turn();
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 1, "its own start of turn gains one");
        assert!(card_def(&in_hand[0].def_id).resource.is_some());
        assert!(!in_hand[0].radiant);
    }

    #[test]
    fn the_rarity_table_over_the_seeds() {
        let mut seen: Vec<String> = Vec::new();
        for seed in (0..40).map(|n| format!("beam-roll-{n}")) {
            let mut s = beam(&seed, false);
            s.start_turn();
            let gained = def_ids(&crate::resources_in_hand(&s, P1));
            assert_eq!(gained.len(), 1, "{seed}");
            if !seen.contains(&gained[0]) {
                seen.push(gained[0].clone());
            }
        }
        assert!(seen.len() >= 3, "several Resources over the seeds: {seen:?}");
    }

    #[test]
    fn r1086_equipped_the_host_gains_and_is_no_piece() {
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER], "library": filler(4), "field": [BODY] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let host = s.unit_at(P1, 1);
        s.play(ID, json!({ "zone": 1, "equip": true }));
        assert_eq!(s.unit_at(P1, 1).id, host.id, "the host is the kept card");
        assert!(
            s.stats(&host)
                .keywords
                .iter()
                .all(|keyword| keyword.kind() != KeywordKind::StarshipPiece),
            "Equip does not pass Starship Piece on (R1852001)"
        );
        s.start_turn();
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 1, "the host gains at its start of turn");
    }

    #[test]
    fn r1852000_its_death_builds_a_ship() {
        let mut s = scenario(json!({
            "p1": { "hand": [NETHER, FILLER], "library": filler(4), "field": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        assert!(!building_a_ship(s.state(), P1));
        s.play(NETHER, json!({}));
        assert!(
            building_a_ship(s.state(), P1),
            "its death fused it into a Dormant Starship"
        );
        let ship = dormant_starship_of(s.state(), P1).expect("the ship").clone();
        assert_eq!(def_ids_of_pieces(&ship), vec![ID.to_string()]);
    }

    fn def_ids_of_pieces(ship: &CardInstance) -> Vec<String> {
        starship_pieces_of(ship)
            .into_iter()
            .map(|piece| piece.def_id)
            .collect()
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = beam("beam-radiant", true);
        s.start_turn();
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand[0].radiant);
    }
}
