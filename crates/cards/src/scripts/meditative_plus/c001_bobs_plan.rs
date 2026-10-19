//! Meditative+ #1 Bob's Plan (SPEC §8.9 row 1, R1185000, R640, R380, R387, R673). (1) Spell,
//! Quickdraw, Mythic.
//!   Base:    "Add a random Wincon card to your hand, and its plan: a Base Camp for Reach the Summit, a
//!            Spa Day for Aestheticize the Game."
//!   Radiant: "Add a random Radiant Wincon card to your hand, and its Radiant plan: a Radiant Base Camp
//!            for Reach the Summit, a Radiant Spa Day for Aestheticize the Game."
//!
//! THE PICK is one draw of the match rng over the Wincon pool (`{ tags: [Wincon] }`, the non-token
//! cards of the sets that ship, R380, in catalog order, so it replays, R60; never this card, R387),
//! through `pick_generated`, so R673's Glitch odds apply to it as to any card generated into a hand.
//! THE PLAN is the AI generated card `WINCON_PLANS` names for the card picked (R1185000, the brief's
//! MP5 D3): M+ #1.1 Base Camp for M #8 Reach the Summit, M+ #1.2 Spa Day for M #20 Aestheticize the
//! Game, added after it; a card with no plan (a later set's Wincon card, or a Glitch) is added alone.
//! The Radiant face adds both Radiant. A full hand burns what does not fit, in that order (§2.4).
//! Quickdraw (R640) is a static flag beside the tag.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001";

/// R1185000: each Wincon card's plan — the AI generated card that helps it win (MP5 D3).
pub const WINCON_PLANS: &[(&str, &str)] = &[
    // M #8 Reach the Summit: levels and mana, M+ #1.1 Base Camp.
    ("meditative-008", "meditativeplus-001-1"),
    // M #20 Aestheticize the Game: the hero's health, M+ #1.2 Spa Day.
    ("meditative-020", "meditativeplus-001-2"),
];

/// The plan `WINCON_PLANS` names for a card, if it names one.
pub fn plan_of(def_id: &str) -> Option<&'static str> {
    WINCON_PLANS
        .iter()
        .find(|(wincon, _)| *wincon == def_id)
        .map(|(_, plan)| *plan)
}

fn bobs_plan(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let pool = crate::query::pool(ID, &json_as(json!({ "tags": ["Wincon"] })));
            let Some(picked) = pick_generated(&mut *ctx.sink.rng, &pool, Some(&*ctx.sink.state)) else {
                return vec![];
            };
            let picked = picked.id.clone();
            let mut out = vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))];
            if let Some(plan) = plan_of(&picked) {
                out.push(add_to_hand(json_as(json!({ "defId": plan, "radiant": radiant }))));
            }
            out
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bobs_plan(false),
        radiant: bobs_plan(true),
    }
}

// Meditative+ #1 Bob's Plan — SPEC §8.9 row 1, BUILD M11 row M+ 1: "Quickdraw (R640); adds one
// Wincon card from the shipped pool, M 8 or M 20, each seen over the seeds, by one rng pick that
// replays; after M 8 a base Base Camp (M+ 1.1), after M 20 a base Spa Day (M+ 1.2); a full hand burns
// the helper, then the card; a Wincon card the plan table does not name (a fixture) is added alone;
// radiant both are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const SUMMIT: &str = "meditative-008";
    const AESTHETICIZE: &str = "meditative-020";
    const BASE_CAMP: &str = "meditativeplus-001-1";
    const SPA_DAY: &str = "meditativeplus-001-2";

    fn casting(seed: &str, radiant: bool, hand_extra: usize) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(std::iter::repeat_n(json!(FILLER), hand_extra));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    /// The cards the play added to p1's hand, in hand order.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    fn ids(cards: &[CardInstance]) -> Vec<&str> {
        cards.iter().map(|card| card.def_id.as_str()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r640_it_is_quickdraw() {
            assert_eq!(script().base.flags().quickdraw, Some(true));
            assert_eq!(script().radiant.flags().quickdraw, Some(true));
        }

        #[test]
        fn r1185000_each_wincon_card_comes_with_its_plan_and_both_are_seen_over_the_seeds() {
            let mut seen: Vec<String> = Vec::new();
            for n in 0..SEED_SEARCH_BOUND {
                let mut s = casting(&format!("bobs-plan-{n}"), false, 1);
                s.play(ID, json!({}));
                let cards = added(&s);
                let expected = match cards.first().map(|card| card.def_id.as_str()) {
                    Some(SUMMIT) => vec![SUMMIT, BASE_CAMP],
                    Some(AESTHETICIZE) => vec![AESTHETICIZE, SPA_DAY],
                    other => panic!("seed {n} added {other:?}"),
                };
                assert_eq!(ids(&cards), expected, "seed {n}");
                assert!(cards.iter().all(|card| !card.radiant), "seed {n}: the base face adds them base");
                if !seen.contains(&expected[0].to_string()) {
                    seen.push(expected[0].to_string());
                }
                if seen.len() == 2 {
                    return;
                }
            }
            panic!("only {seen:?} were picked in {SEED_SEARCH_BOUND} seeds");
        }

        #[test]
        fn r60_the_pick_replays_from_the_seed() {
            let pick = |seed: &str| {
                let mut s = casting(seed, false, 1);
                s.play(ID, json!({}));
                ids(&added(&s)).iter().map(|id| id.to_string()).collect::<Vec<_>>()
            };
            for n in 0..8 {
                let seed = format!("bobs-plan-replay-{n}");
                assert_eq!(pick(&seed), pick(&seed));
            }
        }

        #[test]
        fn r1185000_a_full_hand_burns_the_plan_then_the_card() {
            // Nine fillers beside it: after the play, nine in hand, one free slot.
            let mut s = casting("bobs-plan-full", false, 9);
            s.play(ID, json!({}));
            let kept = added(&s);
            assert_eq!(kept.len(), 1, "the Wincon card fits, its plan burns");
            let plan = plan_of(&kept[0].def_id).expect("a planned card");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == plan));
            // With no slot at all, both burn, the card first.
            let mut s = casting("bobs-plan-full", false, 10);
            s.play(ID, json!({}));
            assert!(added(&s).is_empty());
            let burned: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(burned.len(), 3, "the card, its plan and Bob's Plan itself: {burned:?}");
        }

        #[test]
        fn r1185000_a_wincon_card_the_plan_table_does_not_name_is_added_alone() {
            crate::register_all();
            // A fixture Wincon card of the Core set, and a catalog whose only Wincon card it is.
            let face = json!({ "keywords": [], "text": "fx-wincon" });
            let wincon: CardDef = json_as(json!({
                "id": "fx-wincon", "index": "fx-wincon", "name": "fx-wincon", "set": "Core",
                "type": "Spell", "tags": ["Wincon"], "rarity": "Common", "token": false, "cost": 0,
                "base": face, "radiant": face,
            }));
            let catalog: CardDefs = registered_catalog()
                .iter()
                .filter(|(id, _)| id.as_str() != SUMMIT && id.as_str() != AESTHETICIZE)
                .map(|(id, def)| (id.clone(), def.clone()))
                .chain([(wincon.id.clone(), wincon)])
                .collect();
            register_catalog(catalog);
            let mut scripts = registered_scripts();
            scripts.insert(
                "fx-wincon".to_string(),
                CardScripts {
                    base: Script::default(),
                    radiant: Script::default(),
                },
            );
            register_scripts(scripts);
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.play(ID, json!({}));
            assert_eq!(ids(&added(&s)), vec!["fx-wincon"]);
            clear_overrides();
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1185000_the_card_and_its_plan_are_both_radiant() {
            for n in 0..6 {
                let mut s = casting(&format!("bobs-plan-radiant-{n}"), true, 1);
                s.play(ID, json!({}));
                let cards = added(&s);
                assert_eq!(cards.len(), 2, "seed {n}");
                assert_eq!(plan_of(&cards[0].def_id), Some(cards[1].def_id.as_str()));
                assert!(cards.iter().all(|card| card.radiant), "seed {n}");
            }
        }
    }
}
