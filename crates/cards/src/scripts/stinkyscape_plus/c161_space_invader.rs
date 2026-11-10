//! SPEC §8.12 S+ #161 Space Invader (Unit, Swarm 2/2 → 4/4, cost 1, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//! The list's second #74, renumbered by SP5.
//!   Base:    "Start of turn: Evolve.
//!            Evolve: Summon a copy of this."
//!   Radiant: "Starship Piece
//!            Start of turn: Evolve.
//!            Evolve: Summon a copy of this."
//!
//! Start of turn (R62) runs the card's own `evolve` verb on it: it evolves at the start of its
//! controller's turn, once (R3536000), so a copy evolves at its own next one. The Evolve text is
//! `summon_copy` of this card (R57): a new instance, not yet evolved, in the leftmost open unit zone
//! (R64, no Cry, R1), a Radiant one of a Radiant Space Invader; a full unit row summons none and it has
//! evolved all the same. Starship Piece is the catalog's keyword (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-161";

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
        })),
        evolve: Some(hook(|_ctx| {
            vec![summon_copy(json_as(json!({ "of": { "of": "self" } })))]
        })),
        ..Script::default()
    };
    // The Radiant face prints the same text and adds Starship Piece: catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #161 Space Invader — SPEC §8.12 row 161.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool, others: usize) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend((0..others).map(|_| json!(VANILLA)));
        scenario(json!({
            "p1": { "hand": [VANILLA], "field": field, "library": crate::filler(8) },
            "p2": { "hand": [VANILLA], "library": crate::filler(8) },
        }))
    }

    fn invaders(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|card| card.def_id == ID)
            .collect()
    }

    fn evolved_events(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    fn own_turn(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
    }

    mod base {
        use super::*;

        #[test]
        fn it_evolves_at_its_start_of_turn_and_summons_one_copy_that_is_not_evolved() {
            let mut s = board(false, 0);
            let first = s.unit_at(P1, 1);
            s.end_turn();
            assert_eq!(invaders(&s).len(), 1, "the other seat's turn holds nothing");
            s.end_turn();
            assert_eq!(s.card(&first).evolved, Some(true));
            let all = invaders(&s);
            assert_eq!(all.len(), 2);
            assert_eq!(s.unit_at(P1, 2).def_id, ID, "R64: the leftmost open zone");
            let copy = s.unit_at(P1, 2);
            assert_ne!(s.card(&copy).evolved, Some(true), "a copy is a new card (R57)");
            assert!(!copy.radiant);
            s.expect_stats(&copy, json!({ "attack": 2, "health": 2 }));
            assert_eq!(evolved_events(&s), 1);
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn each_copy_evolves_at_its_own_next_start_of_turn() {
            let mut s = board(false, 0);
            own_turn(&mut s);
            assert_eq!((invaders(&s).len(), evolved_events(&s)), (2, 1));
            own_turn(&mut s);
            assert_eq!(
                (invaders(&s).len(), evolved_events(&s)),
                (3, 2),
                "the first copy evolved and copied"
            );
            own_turn(&mut s);
            assert_eq!(
                (invaders(&s).len(), evolved_events(&s)),
                (4, 3),
                "one new evolution a turn"
            );
        }

        #[test]
        fn a_full_unit_row_summons_no_copy_but_it_still_evolves() {
            let mut s = board(false, 4);
            let invader = s.unit_at(P1, 1);
            own_turn(&mut s);
            assert_eq!(s.card(&invader).evolved, Some(true));
            assert_eq!(invaders(&s).len(), 1);
            assert_eq!(evolved_events(&s), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_copy_is_a_radiant_space_invader_with_starship_piece() {
            let mut s = board(true, 0);
            let first = s.unit_at(P1, 1);
            s.expect_stats(&first, json!({ "attack": 4, "health": 4 }));
            assert!(has_keyword(&s.stats(&first).keywords, KeywordKind::StarshipPiece));
            own_turn(&mut s);
            assert_eq!(s.card(&first).evolved, Some(true));
            let copy = s.unit_at(P1, 2);
            assert_eq!(copy.def_id, ID);
            assert!(copy.radiant);
            assert_ne!(s.card(&copy).evolved, Some(true));
            s.expect_stats(&copy, json!({ "attack": 4, "health": 4 }));
            own_turn(&mut s);
            assert_eq!((invaders(&s).len(), evolved_events(&s)), (3, 2));
        }
    }
}
