//! SPEC §8.12 S+ #71 Mindustrial Contract (§5.1, §6.3 Add to hand; R65, R387, R1420, R3535000, R3549003):
//! (1) Spell, Rare, a card of the held Stinkyscape+ set.
//!   Base:    "Add {card|random Mindustrial card|random Mindustrial cards} to your hand." — card 3.
//!   Radiant: "Add {card|random Radiant Mindustrial card|random Radiant Mindustrial cards} to your hand."
//!
//! Engine: one `add_random_from_catalog` (R57, R60) of `card` picks from the Stinkyscape+ cards tagged
//! Mindustrial, tokens out (§5.1); repeats are allowed and a full hand burns what does not fit (§2.4).
//! The pool names its set, so it reaches the cards while the set is held (R1420), and it never offers
//! this card (R387; it is no Mindustrial card, so the exclusion is the verb's own). The Radiant face adds
//! Radiant cards at their printed price (R3549003: the text prints no discount).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-071";

/// R3549003: "Mindustrial cards", the held set's.
fn mindustrial() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mindustrial] }))
}

fn contract(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": mindustrial(),
                "count": param(&*ctx, "card"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: contract(false),
        radiant: contract(true),
    }
}

// S+ #71 Mindustrial Contract — SPEC §8.12 row S+ 71 (R3549003; R57, R60, R387, R1420, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn signed(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// The `addedToHand` cards of p1 in the last step, in order.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .collect()
    }

    /// The pool a Mindustrial Contract draws from, by definition id, sorted.
    fn pool() -> Vec<String> {
        let mut pool: Vec<String> = crate::query::pool(ID, &mindustrial()).iter().map(|def| def.id.clone()).collect();
        pool.sort();
        pool
    }

    mod base {
        use super::*;

        #[test]
        fn adds_3_random_mindustrial_cards() {
            let mut s = signed(false, "s71-add", 0);
            s.play(ID, json!({}));
            let cards = added(&s);
            assert_eq!(cards.len(), 3);
            for card in &cards {
                assert_eq!(card.zone.z(), ZoneName::Hand);
                assert!(!card.radiant);
                assert_eq!(card.cost_mod, 0);
                assert!(card.cost_override.is_none());
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r3549003_the_pool_is_the_held_sets_mindustrial_cards_never_this_one() {
            // The scenario registers the catalog, which the pool reads.
            let _registered = signed(false, "s71-registered", 0);
            let expected = pool();
            assert!(expected.len() > 3, "the held set's Mindustrial cards");
            assert!(!expected.iter().any(|id| id == ID));
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..60 {
                let mut s = signed(false, &format!("s71-pool-{i}"), 0);
                s.play(ID, json!({}));
                for card in added(&s) {
                    let def = crate::card_def(&card.def_id);
                    assert_eq!(def.set, SetName::StinkyscapePlus, "{}", def.id);
                    assert!(def.tags.contains(&Tag::Mindustrial), "{}", def.id);
                    assert!(!def.is_token(), "{}", def.id);
                    assert!(expected.contains(&def.id), "{}", def.id);
                    seen.insert(def.id.clone());
                }
            }
            assert!(!seen.contains(ID));
            assert!(seen.len() > 20, "180 draws cover most of a pool of {}: {}", expected.len(), seen.len());
        }

        #[test]
        fn r60_repeats_are_allowed_and_the_same_seed_adds_the_same_cards() {
            let run = |seed: &str| {
                let mut s = signed(false, seed, 0);
                s.play(ID, json!({}));
                added(&s).into_iter().map(|card| card.def_id).collect::<Vec<_>>()
            };
            assert_eq!(run("s71-same"), run("s71-same"));
            let repeated = (0..400).any(|i| {
                let cards = run(&format!("s71-rep-{i}"));
                cards.iter().collect::<IndexSet<_>>().len() < cards.len()
            });
            assert!(repeated);
        }

        #[test]
        fn s2_4_a_full_hand_burns_what_does_not_fit() {
            let mut s = signed(false, "s71-full", 8);
            s.play(ID, json!({}));
            assert_eq!(added(&s).len(), 2);
            let burned = s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, 1);
        }

        #[test]
        fn r386_an_upgrade_adds_4_a_degrade_2() {
            let mut up = signed(false, "s71-up", 0);
            step_param(up.card_mut(ID), "card", 1);
            up.play(ID, json!({}));
            assert_eq!(added(&up).len(), 4);

            let mut down = signed(false, "s71-down", 0);
            step_param(down.card_mut(ID), "card", -1);
            down.play(ID, json!({}));
            assert_eq!(added(&down).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3549003_the_cards_are_radiant_at_their_printed_price() {
            let mut s = signed(true, "s71-radiant", 0);
            s.play(ID, json!({}));
            let cards = added(&s);
            assert_eq!(cards.len(), 3);
            for card in &cards {
                assert!(card.radiant, "{}", card.def_id);
                assert_eq!(card.cost_mod, 0, "the text prints no discount");
                assert_ne!(card.def_id, ID);
                assert!(pool().contains(&card.def_id));
            }
        }
    }
}
