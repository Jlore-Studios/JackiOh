//! S #156 Catalyst Comet (SPEC §8.10 row S 156; §6.3, R80, R316, R1867010): (1) Spell, Epic.
//!   Base:    "Add a random Catalyst card to your hand. Shuffle this into your deck."
//!   Radiant: "Add a random Radiant Catalyst card to your hand. Shuffle this into your deck."
//!
//! Engine: one pick from the match rng (R60) among the Catalyst cards of the sets that ship
//! (`add_random_from_catalog` with the tag, non-token cards, R380: C+ #38, C+ #46, M #45, M #49,
//! M #91 and M+ #16, S #147 once the set ships), a full hand burning it (§2.4). Then the Comet
//! itself, once it has resolved, into its owner's deck at a random position instead of staying in
//! the graveyard (`shuffle_card_into` on the hook's own card, C #30 Recycle's verb, R80, R316: a
//! full deck leaves it in the graveyard), the moved instance keeping its Radiant flag (R78).

use jackioh_engine::effects::{add_random_from_catalog, shuffle_card_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-156";

pub fn script() -> CardScripts {
    CardScripts {
        base: comet(false),
        radiant: comet(true),
    }
}

fn comet(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut add = json!({ "query": { "tags": ["Catalyst"] } });
            if radiant {
                add["radiant"] = json!(true);
            }
            let mut effects = vec![add_random_from_catalog(json_as(add))];
            // The Comet itself, as it stands resolving (R174 has no stay to lose: a Spell resolving
            // was never on the field).
            if let Some(me) = ctx.self_.as_ref() {
                let id = me.id.clone();
                effects.push(shuffle_card_into(json_as(json!({ "instanceId": id }))));
            }
            effects
        })),
        ..Script::default()
    }
}

// S #156 Catalyst Comet — SPEC §8.10 row S 156, BUILD M12 row S 156 (R1867010).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const COMET: &str = "stinkyscape-156";
    const FILLER: &str = "core-005";

    /// The shipped Catalyst cards: the Comet's pool, the set's own #147 IN33DH3 among them since its
    /// release (#1884).
    const POOL: [&str; 7] = [
        "classicplus-038",
        "classicplus-046",
        "meditative-045",
        "meditative-049",
        "meditative-091",
        "meditativeplus-016",
        "stinkyscape-147",
    ];

    fn added_card(seed: &str, radiant: bool) -> CardInstance {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": COMET, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(COMET, json!({}));
        s.hand(P1).into_iter().find(|card| card.def_id != FILLER).expect("a card added")
    }

    mod base {
        use super::*;

        #[test]
        fn r1867010_adds_a_catalyst_card_each_of_the_seven_seen_over_the_seeds() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 0..120 {
                let added = added_card(&format!("comet-{seed}"), false);
                assert!(POOL.contains(&added.def_id.as_str()), "a Catalyst card: {}", added.def_id);
                assert!(!added.radiant);
                seen.insert(added.def_id);
            }
            let expected: IndexSet<String> = POOL.into_iter().map(str::to_string).collect();
            assert_eq!(seen, expected, "every pool card comes up");
        }

        #[test]
        fn r1867010_the_comet_is_in_its_casters_deck_afterwards_not_the_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "comet-deck",
                "p1": { "hand": [COMET, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let id = s.card(COMET).id.clone();

            s.play(COMET, json!({}));

            s.expect_in_zone(&id, "library");
            assert_eq!(s.card(&id).def_id, COMET);
            assert!(!s.card(&id).radiant);
            assert_eq!(s.hand(P1).len(), 2);
        }

        #[test]
        fn r1867010_a_full_deck_leaves_the_comet_in_the_graveyard() {
            crate::register_all();
            let library = vec![json!(FILLER); 60];
            let mut s = scenario(json!({
                "p1": { "hand": [COMET, FILLER], "library": library },
                "p2": { "hand": [FILLER] },
            }));
            let id = s.card(COMET).id.clone();

            s.play(COMET, json!({}));

            s.expect_in_zone(&id, "graveyard");
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::LibraryOverflow { .. }
            )));
        }

        #[test]
        fn r1867010_s_147_joins_the_pool() {
            let mut seen_s147 = 0;
            for seed in 0..200 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("comet-preview-{seed}"),
                    "p1": { "hand": [COMET, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(COMET, json!({}));
                let added = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id != FILLER)
                    .expect("a card added");
                assert!(
                    POOL.contains(&added.def_id.as_str()) || added.def_id == "stinkyscape-147",
                    "a Catalyst card: {}",
                    added.def_id
                );
                if added.def_id == "stinkyscape-147" {
                    seen_s147 += 1;
                }
            }
            assert!(seen_s147 > 0, "S #147 comes up over the seeds");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867010_the_card_is_radiant_and_the_comet_goes_back_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "comet-radiant",
                "p1": { "hand": [{ "def": COMET, "radiant": true }, FILLER], "library": [FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let id = s.card(COMET).id.clone();

            s.play(COMET, json!({}));

            let added = s.hand(P1).into_iter().find(|card| card.def_id != FILLER).expect("a card added");
            assert!(added.radiant, "{} is Radiant", added.def_id);
            s.expect_in_zone(&id, "library");
            assert!(s.card(&id).radiant, "the Comet goes back Radiant");
        }
    }
}
