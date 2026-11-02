//! S T-AI-10 Salvage Protocol (SPEC §7, §8.10 row T-AI-10; §2.4, §6.2, R482, R1852000, R1878000,
//! R1881001): (1) Unit, AI, Token (no printed rarity), 1/2 → 2/4, Starship Piece, an AI generated token of
//! the Stinkyscape set, built by S21.
//!   Base:    "Starship Piece\nDeath: Add a random Starship Piece to your hand."
//!   Radiant: "Starship Piece\nDeath: Add a random Radiant Starship Piece to your hand."
//! Engine: Starship Piece is catalog data: its death also fuses a copy into its owner's Dormant Starship
//!   (R1852000). Death (§6.2): one random card of `query::starship_pieces()` (R1878000, R1881001, T-AI-3's pool)
//!   added to its controller's hand, a full hand burning it (§2.4). Radiant: added Radiant. Declares no
//!   numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-10";

/// "a random Starship Piece": one.
const SALVAGED: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(crate::query::starship_pieces()),
                count: Some(SALVAGED),
                radiant: radiant.then_some(true),
                ..AddRandomFromCatalogArgs::default()
            })]
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

// S T-AI-10 Salvage Protocol — SPEC §8.10 row T-AI-10, BUILD M12 row S T-AI-10.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const SALVAGE: &str = "stinkyscape-t-ai-10";
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const FILLER: &str = "core-005";

    fn killed(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "field": [{ "def": SALVAGE, "radiant": radiant }], "hand": [HIT_JOB, FILLER], "mana": 10 },
        }));
        let id = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&id) }));
        s
    }

    fn salvaged(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    fn prints_piece(def_id: &str) -> bool {
        has_keyword(&crate::card_def(def_id).base.keywords, KeywordKind::StarshipPiece)
    }

    #[test]
    fn r1881001_r1852000_its_death_fuses_it_and_adds_a_random_starship_piece() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..8 {
            let s = killed(false, &format!("salvage-{i}"));
            let ship = dormant_starship_of(s.state(), P1).expect("a Dormant Starship");
            let pieces = starship_pieces_of(ship);
            assert_eq!(pieces.len(), 1);
            assert_eq!(pieces[0].def_id, SALVAGE);
            let added = salvaged(&s);
            assert_eq!(added.len(), 1, "{added:?}");
            assert!(prints_piece(&added[0].def_id), "{}", added[0].def_id);
            assert!(!crate::card_def(&added[0].def_id).is_token());
            assert!(!added[0].radiant);
            seen.insert(added[0].def_id.clone());
        }
        assert!(seen.len() > 1, "{seen:?}");
    }

    #[test]
    fn radiant_2_4_and_a_radiant_piece() {
        let mut s = scenario(json!({ "p1": { "field": [{ "def": SALVAGE, "radiant": true }] } }));
        s.expect_stats(SALVAGE, json!({ "attack": 2, "health": 4 }));
        let s = killed(true, "salvage-radiant");
        let added = salvaged(&s);
        assert_eq!(added.len(), 1);
        assert!(added[0].radiant);
    }
}
