//! Stinkyscape #125 Jlockeed's Spacerace Propaganda (SPEC §8.10 row 125). (2) Field Spell, Jlockeed, Rare.
//!   Base:    "Activate: Add a random Starship Piece to your hand."
//!   Radiant: "Activate: Add a random Radiant Starship Piece to your hand."
//!
//! A Field Spell (ST8, R1857001): its Activate runs while it stands face-up in its backrow zone, once a
//! turn (R384), and adds one card from S #124's Starship Piece pool (`query::starship_pieces()`, R1878000);
//! a full hand burns it (§2.4). It declares no numbers: one card a use, and the Activate count is no
//! param (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-125";

fn propaganda(radiant: bool) -> Script {
    let label = if radiant {
        "Add a random Radiant Starship Piece to your hand"
    } else {
        "Add a random Starship Piece to your hand"
    };
    let campaign = ActivationDecl {
        id: "propaganda".to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(crate::query::starship_pieces()),
                radiant: radiant.then_some(true),
                ..AddRandomFromCatalogArgs::default()
            })]
        }),
    };
    Script {
        activations: vec![campaign],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: propaganda(false),
        radiant: propaganda(true),
    }
}

// Stinkyscape #125 Jlockeed's Spacerace Propaganda — SPEC §8.10 row 125, BUILD M12 row S 125
// (R384).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const PROPAGANDA: &str = "stinkyscape-125";
    const ABILITY: &str = "propaganda";
    const FILLER: &str = "core-005";

    fn standing(radiant: bool, fillers: usize) -> Scenario {
        scenario(json!({
            "seed": "propaganda",
            "p1": {
                "hand": (0..fillers).map(|_| json!(FILLER)).collect::<Vec<_>>(),
                "backrow": [{ "def": PROPAGANDA, "lane": 1, "radiant": radiant }],
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn added(s: &Scenario) -> Vec<(String, bool)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => {
                    let card = s.card(instance_id);
                    Some((card.def_id.clone(), card.radiant))
                }
                _ => None,
            })
            .collect()
    }

    fn prints_piece(def_id: &str) -> bool {
        has_keyword(&crate::card_def(def_id).base.keywords, KeywordKind::StarshipPiece)
    }

    #[test]
    fn r384_its_activate_adds_one_random_starship_piece_once_a_turn() {
        let mut s = standing(false, 1);
        s.activate(PROPAGANDA, json!({ "ability": ABILITY }));
        let first = added(&s);
        assert_eq!(first.len(), 1, "{first:?}");
        assert!(prints_piece(&first[0].0), "{first:?}");
        assert!(!first[0].1);
        s.expect_refused(|s| s.activate(PROPAGANDA, json!({ "ability": ABILITY })));
        s.end_turn().end_turn();
        s.activate(PROPAGANDA, json!({ "ability": ABILITY }));
        assert_eq!(added(&s).len(), 1);
    }

    #[test]
    fn r1878000_the_piece_is_drawn_from_the_starship_piece_pool() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..10 {
            let mut s = scenario(json!({
                "seed": format!("propaganda-{i}"),
                "p1": { "hand": [FILLER], "backrow": [{ "def": PROPAGANDA, "lane": 1 }], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.activate(PROPAGANDA, json!({ "ability": ABILITY }));
            for (def_id, _) in added(&s) {
                assert!(prints_piece(&def_id), "{def_id}");
                assert_ne!(def_id, DORMANT_STARSHIP_DEF_ID);
                seen.insert(def_id);
            }
        }
        assert!(seen.len() > 1, "{seen:?}");
    }

    #[test]
    fn s2_4_a_full_hand_burns_the_piece() {
        let mut s = standing(false, 10);
        s.activate(PROPAGANDA, json!({ "ability": ABILITY }));
        assert!(added(&s).is_empty());
        let burned: Vec<&GameEvent> = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if prints_piece(def_id)))
            .collect();
        assert_eq!(burned.len(), 1, "{burned:?}");
    }

    #[test]
    fn radiant_adds_a_radiant_piece() {
        let mut s = standing(true, 1);
        s.activate(PROPAGANDA, json!({ "ability": ABILITY }));
        let added = added(&s);
        assert_eq!(added.len(), 1, "{added:?}");
        assert!(added[0].1, "Radiant");
    }
}
