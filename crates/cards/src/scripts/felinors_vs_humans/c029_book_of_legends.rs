//! SPEC §8.14 FvH #29 Book of Legends. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Add {cards|random Radiant Acclaimed card|random Radiant Acclaimed cards} to your hand." — 1
//!   Radiant: "Add {cards|random Radiant Acclaimed card|random Radiant Acclaimed cards} to your hand." — 3
//!
//! S #124's add of the declared `cards` (R386) from the Acclaimed tag: any type, tokens left out (§5.1,
//! so M #49.1 YileGPT Prime is not in it), from the sets that ship (R380), each draw of one cast a
//! different card while `UNIQUE_RANDOM_DRAWS` holds (R3563000). Both faces add Radiant cards and differ
//! only in the number, which is catalog data, so they run one script. A full hand burns the rest (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-029";

/// The Acclaimed cards, tokens left out.
fn acclaimed() -> CatalogQueryArgs {
    json_as(json!({ "tags": ["Acclaimed"] }))
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(acclaimed()),
                count: Some(param(&*ctx, "cards")),
                radiant: Some(true),
                ..AddRandomFromCatalogArgs::default()
            })]
        })),
        ..Script::default()
    };
    // The Radiant face adds 3 to the base face's 1: the declared number, read off the catalog.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #29 Book of Legends — SPEC §8.14 row 29 (R60, R3563000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile

    fn holding(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The cards the last step added to p1's hand: `(def id, radiant)`.
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

    fn acclaimed_card(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        def.tags.contains(&Tag::Acclaimed) && !def.is_token()
    }

    mod base {
        use super::*;

        #[test]
        fn r60_adds_one_random_radiant_acclaimed_card_never_a_token() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..10 {
                let mut s = holding(false, &format!("legends-{n}"), 1);
                s.play(ID, json!({}));
                let added = added(&s);
                assert_eq!(added.len(), 1, "{added:?}");
                for (def_id, radiant) in nonempty(added, "the card added") {
                    assert!(acclaimed_card(&def_id), "{def_id} is an Acclaimed card");
                    assert_ne!(def_id, "meditative-049-1");
                    assert!(radiant, "{def_id}");
                    seen.insert(def_id);
                }
            }
            assert!(seen.len() > 1, "{seen:?}");
        }

        #[test]
        fn r386_cards_reads_through_param() {
            let mut s = holding(false, "legends-param", 1);
            step_param(s.card_mut(ID), "cards", 1);
            s.play(ID, json!({}));
            assert_eq!(added(&s).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3563000_adds_three_different_radiant_acclaimed_cards() {
            for n in 0..6 {
                let mut s = holding(true, &format!("legends-radiant-{n}"), 1);
                s.play(ID, json!({}));
                let added = added(&s);
                assert_eq!(added.len(), 3, "{added:?}");
                let different: IndexSet<&String> = added.iter().map(|(def_id, _)| def_id).collect();
                assert_eq!(different.len(), 3, "{added:?}");
                assert!(added.iter().all(|(def_id, radiant)| *radiant && acclaimed_card(def_id)), "{added:?}");
            }
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_rest() {
            let mut s = holding(true, "legends-full", 9);
            s.play(ID, json!({}));
            assert_eq!(added(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if acclaimed_card(def_id)))
                .count();
            assert_eq!(burned, 2);
        }
    }
}
