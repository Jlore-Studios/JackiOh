//! SPEC §8.14 FvH #26 Book of Entropy. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Add {cards|random Call to Chaos card|random Call to Chaos cards} to your hand." — 2
//!   Radiant: "Add {cards|random Radiant Call to Chaos card|random Radiant Call to Chaos cards} to your
//!            hand." — 2
//!
//! R5253002 (FV2 #26): S #124's add of the declared `cards` (R386) from the Call to Chaos editions
//! (`query::call_to_chaos_editions()`, R3869001): any card tagged Call to Chaos, from the sets that ship
//! (R380), so FvH #100 joins the pool as its set ships (R1420). The draws of one cast differ while
//! `UNIQUE_RANDOM_DRAWS` holds (R3563000), and the cards are hidden from the opponent as every add is
//! (R440). The Radiant face's are Radiant; a full hand burns the rest (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-026";

fn entropy(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(crate::query::call_to_chaos_editions()),
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
        base: entropy(false),
        radiant: entropy(true),
    }
}

// FvH #26 Book of Entropy — SPEC §8.14 row 26 (R5253002, R3869001, R3563000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile
    /// FvH #100, the set's own Call to Chaos.
    const FVH_CHAOS: &str = "felinorsvshumans-100";

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

    fn is_call_to_chaos(def_id: &str) -> bool {
        crate::card_def(def_id).tags.contains(&Tag::CallToChaos)
    }

    fn pool() -> Vec<String> {
        crate::query::query(&crate::query::call_to_chaos_editions())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    #[test]
    fn r5253002_the_pool_is_every_shipped_edition_and_the_sets_own_once_it_ships() {
        crate::register_all();
        let held = pool();
        assert!(held.len() > 2, "{held:?}");
        assert!(held.iter().all(|def_id| is_call_to_chaos(def_id)), "{held:?}");
        assert!(!held.contains(&FVH_CHAOS.to_string()), "{held:?}");
        let _preview = preview_sets(&[SetName::FelinorsVsHumans]);
        let shipped = pool();
        assert!(shipped.contains(&FVH_CHAOS.to_string()), "{shipped:?}");
        for def_id in nonempty(held, "the shipped editions") {
            assert!(shipped.contains(&def_id), "{def_id}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r5253002_r3563000_adds_two_different_random_call_to_chaos_cards() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..10 {
                let mut s = holding(false, &format!("entropy-{n}"), 1);
                s.play(ID, json!({}));
                let added = added(&s);
                assert_eq!(added.len(), 2, "{added:?}");
                assert_ne!(added[0].0, added[1].0, "{added:?}");
                for (def_id, radiant) in nonempty(added, "the cards added") {
                    assert!(is_call_to_chaos(&def_id), "{def_id} is a Call to Chaos");
                    assert!(!radiant, "{def_id}");
                    seen.insert(def_id);
                }
            }
            assert!(seen.len() > 2, "{seen:?}");
        }

        #[test]
        fn r386_cards_reads_through_param() {
            let mut s = holding(false, "entropy-param", 1);
            step_param(s.card_mut(ID), "cards", 1);
            s.play(ID, json!({}));
            assert_eq!(added(&s).len(), 3);
        }

        #[test]
        fn s2_4_a_full_hand_burns_what_does_not_fit() {
            let mut s = holding(false, "entropy-full", 9);
            s.play(ID, json!({}));
            assert_eq!(added(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if is_call_to_chaos(def_id)))
                .count();
            assert_eq!(burned, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_two_radiant_call_to_chaos_cards() {
            let mut s = holding(true, "entropy-radiant", 1);
            s.play(ID, json!({}));
            let added = added(&s);
            assert_eq!(added.len(), 2, "{added:?}");
            assert!(added.iter().all(|(def_id, radiant)| *radiant && is_call_to_chaos(def_id)), "{added:?}");
        }
    }
}
