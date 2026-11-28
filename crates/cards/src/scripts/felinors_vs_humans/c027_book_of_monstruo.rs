//! SPEC §8.14 FvH #27 Book of Monstruo. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Add a random Chaos Golem to your hand."
//!   Radiant: "Add a random Radiant Chaos Golem to your hand."
//!
//! "a random Chaos Golem" is any Call to Chaos edition's Golem token: Core #95.1, C+ #73.1, M #95.1,
//! S #150.1, TG #39.1, S+ #142.1 and #142.2, and FvH #100.1 and #100.2. No tag names them, so the
//! pool names them by id (`defId`, with `withTokens`, R60): a pool that names its cards reaches them
//! whether or not their set ships (R1420), so the set's own two Golems are in it while it is held.
//! One card, Radiant on the Radiant face; a full hand burns it (§2.4). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-027";

/// The Golem tokens of every Call to Chaos edition, in catalog order.
const CHAOS_GOLEMS: [&str; 9] = [
    "core-095-1",
    "classicplus-073-1",
    "meditative-095-1",
    "stinkyscape-150-1",
    "traininggrounds-039-1",
    "stinkyscapeplus-142-1",
    "stinkyscapeplus-142-2",
    "felinorsvshumans-100-1",
    "felinorsvshumans-100-2",
];

/// R60, R1420: the Golems by id, tokens as they are.
fn golems() -> CatalogQueryArgs {
    json_as(json!({ "defId": CHAOS_GOLEMS, "withTokens": true }))
}

fn monstruo(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(golems()),
                count: Some(1),
                radiant: radiant.then_some(true),
                ..AddRandomFromCatalogArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: monstruo(false),
        radiant: monstruo(true),
    }
}

// FvH #27 Book of Monstruo — SPEC §8.14 row 27.
#[cfg(test)]
mod tests {
    use super::{CHAOS_GOLEMS, ID, golems};
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

    #[test]
    fn r1420_the_pool_is_the_nine_call_to_chaos_golems_the_held_sets_two_among_them() {
        crate::register_all();
        let mut pool: Vec<String> = crate::query::query(&golems()).into_iter().map(|def| def.id.clone()).collect();
        pool.sort();
        let mut named: Vec<String> = CHAOS_GOLEMS.iter().map(|id| id.to_string()).collect();
        named.sort();
        assert_eq!(pool, named);
        assert!(pool.contains(&"felinorsvshumans-100-1".to_string()));
        assert!(pool.contains(&"felinorsvshumans-100-2".to_string()));
    }

    #[test]
    fn it_declares_no_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r60_adds_one_random_chaos_golem_of_any_edition() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..12 {
                let mut s = holding(false, &format!("monstruo-{n}"), 1);
                s.play(ID, json!({}));
                let added = added(&s);
                assert_eq!(added.len(), 1, "{added:?}");
                for (def_id, radiant) in nonempty(added, "the Golem added") {
                    assert!(CHAOS_GOLEMS.contains(&def_id.as_str()), "{def_id} is a Chaos Golem");
                    assert!(!radiant);
                    seen.insert(def_id);
                }
            }
            assert!(seen.len() > 1, "{seen:?}");
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_golem() {
            let mut s = holding(false, "monstruo-full", 10);
            s.play(ID, json!({}));
            assert!(added(&s).is_empty());
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if CHAOS_GOLEMS.contains(&def_id.as_str())))
                .count();
            assert_eq!(burned, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_one_radiant_chaos_golem() {
            let mut s = holding(true, "monstruo-radiant", 1);
            s.play(ID, json!({}));
            let added = added(&s);
            assert_eq!(added.len(), 1, "{added:?}");
            assert!(added.iter().all(|(def_id, radiant)| *radiant && CHAOS_GOLEMS.contains(&def_id.as_str())));
        }
    }
}
