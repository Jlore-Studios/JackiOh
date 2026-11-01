//! S #41 Drone Sympathizer (SPEC §8.10 row 41): (1) Spell, Common, a card of the Stinkyscape set.
//!   Base:    "Add {cards|random Drone Core card|random Drone Core cards} to your hand." — cards 2
//!   Radiant: "Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your hand."
//!            — cards 2
//! Engine: `add_random_from_catalog` over the Drone Core tag (S #27–#36, non-token, R380), one draw of the
//!   match rng per card, repeats allowed, every tier as likely (R60), each on its base face at its printed
//!   cost; a full hand burns the overflow (§2.4). The count is the declared `cards` (R386). The pool names
//!   no set, so it reaches the set's Drone Cores because the set ships (R1420). Radiant: added Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-041";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Drone Core"] },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
            })))]
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

// S #41 Drone Sympathizer — SPEC §8.10 row 41, BUILD M12 row S 41 (R1420).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const SYMPATHIZER: &str = "stinkyscape-041";
    const FILLER: &str = "core-005";

    fn tiers() -> Vec<String> {
        (27..=36).map(|n| format!("stinkyscape-{n:03}")).collect()
    }

    fn played(seed: &str, radiant: bool, hand: usize) -> Scenario {
        let mut cards = vec![json!({ "def": SYMPATHIZER, "radiant": radiant })];
        cards.extend((0..hand).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(SYMPATHIZER, json!({}));
        s
    }

    fn cores(s: &Scenario) -> Vec<(String, bool, i32)> {
        s.hand(P1)
            .into_iter()
            .filter(|card| tiers().contains(&card.def_id))
            .map(|card| (card.def_id, card.radiant, card.cost_mod))
            .collect()
    }

    #[test]
    fn r60_adds_two_drone_cores() {
        let s = played("sympathizer", false, 1);
        let added = cores(&s);
        assert_eq!(added.len(), 2, "{added:?}");
        for (_, radiant, cost_mod) in &added {
            assert!(!radiant, "on its base face");
            assert_eq!(*cost_mod, 0, "at its printed cost");
        }
    }

    #[test]
    fn r60_every_tier_is_seen_over_the_seeds() {
        let mut seen: Vec<String> = (0..60)
            .flat_map(|n| cores(&played(&format!("sympathizer-{n}"), false, 1)))
            .map(|(def_id, _, _)| def_id)
            .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen, tiers());
    }

    /// Held, its pool of the set's own Drone Cores was empty; since the release (#1884) the set ships,
    /// so they arrive with no preview.
    #[test]
    fn r1857000_since_the_release_the_drone_cores_arrive_without_a_preview() {
        let s = played("sympathizer-held", false, 1);
        assert!(!cores(&s).is_empty());
    }

    #[test]
    fn a_full_hand_burns_the_second() {
        let s = played("sympathizer-full", false, HAND_CAP as usize - 1);
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        assert_eq!(cores(&s).len(), 1);
        let burned: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(burned.len(), 1);
        assert!(tiers().contains(&burned[0]), "the burned card is a Drone Core");
    }

    #[test]
    fn cards_reads_through_param() {
        let mut s = scenario(json!({
            "p1": { "hand": [SYMPATHIZER, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        set_param(s.card_mut(SYMPATHIZER), "cards", 4);
        s.play(SYMPATHIZER, json!({}));
        assert_eq!(cores(&s).len(), 4);
    }

    #[test]
    fn radiant_both_radiant() {
        let s = played("sympathizer-radiant", true, 1);
        let added = cores(&s);
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|(_, radiant, _)| *radiant));
    }
}
