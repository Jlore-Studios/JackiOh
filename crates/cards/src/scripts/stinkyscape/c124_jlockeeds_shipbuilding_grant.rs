//! Stinkyscape #124 Jlockeed's Shipbuilding Grant (SPEC §8.10 row 124). (2) Spell, Jlockeed, Common.
//!   Base:    "Add {cards|random Starship Piece|random Starship Pieces} to your hand." — cards 3
//!   Radiant: "Add {cards|random Radiant Starship Piece|random Radiant Starship Pieces} to your hand." — 3
//!
//! R1878000: the pool is `query::starship_pieces()`, every non-token card that prints Starship Piece on
//! its base face, so never the Dormant Starship (R1857001), from the one set that prints the keyword,
//! named so that it reaches the pieces whether or not the set ships (R1420). One draw of the match rng
//! per card (R60), repeats allowed; a full hand burns the rest (§2.4). The count is the declared `cards`
//! (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-124";

fn grant(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(crate::query::starship_pieces()),
                count: Some(param(&*ctx, "cards")),
                radiant: radiant.then_some(true),
                ..AddRandomFromCatalogArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: grant(false),
        radiant: grant(true),
    }
}

// Stinkyscape #124 Jlockeed's Shipbuilding Grant — SPEC §8.10 row 124, BUILD M12 row S 124.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const GRANT: &str = "stinkyscape-124";
    const FILLER: &str = "core-005";
    /// S #177 Cloaking Module, a piece since it was added (#2521).
    const CLOAKING_MODULE: &str = "stinkyscape-177";
    /// The fifteen Units that #2521 took off the Starship Pieces: the pool offers none of them.
    const NO_LONGER_PIECES: [&str; 15] = [
        "stinkyscape-068",
        "stinkyscape-069",
        "stinkyscape-072",
        "stinkyscape-073",
        "stinkyscape-074",
        "stinkyscape-075",
        "stinkyscape-076",
        "stinkyscape-077",
        "stinkyscape-078",
        "stinkyscape-080",
        "stinkyscape-081",
        "stinkyscape-082",
        "stinkyscape-083",
        "stinkyscape-085",
        "stinkyscape-086",
    ];

    fn holding(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": GRANT, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
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
    fn r1878000_adds_three_random_starship_pieces_never_the_dormant_starship() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..12 {
            let mut s = holding(false, &format!("grant-{i}"), 1);
            s.play(GRANT, json!({}));
            let added = added(&s);
            assert_eq!(added.len(), 3, "{added:?}");
            for (def_id, radiant) in added {
                assert!(prints_piece(&def_id), "{def_id} is a Starship Piece");
                assert!(!crate::card_def(&def_id).is_token(), "{def_id} is no token");
                assert_ne!(def_id, DORMANT_STARSHIP_DEF_ID);
                assert!(!NO_LONGER_PIECES.contains(&def_id.as_str()), "{def_id}");
                assert!(!radiant);
                seen.insert(def_id);
            }
        }
        assert!(seen.len() > 3, "{seen:?}");
    }

    #[test]
    fn r1878000_the_pool_holds_cloaking_module_and_none_of_the_fifteen_former_pieces() {
        crate::register_all();
        let pool: Vec<String> = crate::query::query(&crate::query::starship_pieces())
            .into_iter()
            .map(|def| def.id.clone())
            .collect();
        assert!(pool.iter().all(|def_id| prints_piece(def_id)), "{pool:?}");
        assert!(pool.iter().any(|def_id| def_id == CLOAKING_MODULE), "{pool:?}");
        for former in NO_LONGER_PIECES {
            assert!(!pool.iter().any(|def_id| def_id == former), "{former} in {pool:?}");
            assert!(!prints_piece(former), "{former}");
        }
    }

    #[test]
    fn r60_the_picks_replay_from_the_seed() {
        let mut first = holding(false, "grant-replay", 1);
        first.play(GRANT, json!({}));
        let mut again = holding(false, "grant-replay", 1);
        again.play(GRANT, json!({}));
        assert_eq!(added(&first), added(&again));
    }

    #[test]
    fn s2_4_a_full_hand_burns_what_does_not_fit() {
        let mut s = holding(false, "grant-full", 9);
        s.play(GRANT, json!({}));
        assert_eq!(added(&s).len(), 1);
        let burned = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if prints_piece(def_id)))
            .count();
        assert_eq!(burned, 2);
    }

    #[test]
    fn r386_cards_reads_through_param() {
        let mut s = holding(false, "grant-param", 1);
        step_param(s.card_mut(GRANT), "cards", 1);
        s.play(GRANT, json!({}));
        assert_eq!(added(&s).len(), 4);
    }

    #[test]
    fn radiant_adds_three_radiant_pieces() {
        let mut s = holding(true, "grant-radiant", 1);
        s.play(GRANT, json!({}));
        let added = added(&s);
        assert_eq!(added.len(), 3, "{added:?}");
        assert!(added.iter().all(|(def_id, radiant)| *radiant && prints_piece(def_id)));
    }
}
