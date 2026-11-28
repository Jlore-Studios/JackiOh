//! SPEC §8.14 FvH #23 Book of Excavation. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Add {resources|random Stinkyscape Resource|random Stinkyscape Resources} to your hand." — 3
//!   Radiant: "Add {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources} to
//!            your hand." — 3
//!
//! S #62 Miner v1's gain, the declared `resources` times (R386): each Resource rolled by the rarity table
//! as it is gained (R1840002) into its caster's hand, a Resource being a hand card (R2250000); a full
//! hand burns the rest, each still rolled. The Radiant face's are Radiant.

use jackioh_engine::effects::{GainResourcesArgs, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-023";

fn excavation(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![gain_resources(GainResourcesArgs {
                count: param(&*ctx, "resources"),
                radiant: radiant.then_some(true),
                ..GainResourcesArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: excavation(false),
        radiant: excavation(true),
    }
}

// FvH #23 Book of Excavation — SPEC §8.14 row 23 (R1840002, R2250000).
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

    /// p1's Resources in hand: `(def id, radiant)`.
    fn resources(s: &Scenario) -> Vec<(String, bool)> {
        crate::resources_in_hand(s, P1)
            .into_iter()
            .map(|card| (card.def_id, card.radiant))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1840002_r2250000_adds_three_random_resources_to_your_hand() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..8 {
                let mut s = holding(false, &format!("excavation-{n}"), 1);
                assert!(resources(&s).is_empty());
                s.play(ID, json!({}));
                let gained = resources(&s);
                assert_eq!(gained.len(), 3, "{gained:?}");
                for (def_id, radiant) in nonempty(gained, "the Resources gained") {
                    assert!(!radiant, "{def_id}");
                    seen.insert(def_id);
                }
            }
            assert!(seen.len() > 1, "{seen:?}");
        }

        #[test]
        fn r2250000_a_full_hand_burns_the_rest() {
            // Nine cards left once the Book is played: one Resource fits and the other two burn.
            let mut s = holding(false, "excavation-full", 9);
            s.play(ID, json!({}));
            assert_eq!(resources(&s).len(), 1);
            assert_eq!(s.last_count(GameEventType::Burned), 2);
        }

        #[test]
        fn r386_resources_reads_through_param() {
            let mut s = holding(false, "excavation-param", 1);
            step_param(s.card_mut(ID), "resources", 1);
            s.play(ID, json!({}));
            assert_eq!(resources(&s).len(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_three_radiant_resources() {
            let mut s = holding(true, "excavation-radiant", 1);
            s.play(ID, json!({}));
            let gained = resources(&s);
            assert_eq!(gained.len(), 3, "{gained:?}");
            assert!(gained.iter().all(|(_, radiant)| *radiant), "{gained:?}");
        }
    }
}
