//! #51.1 KY's Empty Notebook (SPEC §8.3, §7, §5.1, §6.3 Shuffle into; R11, R50, R60, R80, R1828002).
//! Spell token, cost 1, tags KY + Token. The only generator is #51 KY's Private Tutor's
//! "No match at all → add a KY's Empty Notebook".
//!   Base:    "Shuffle {cards|random KY card|random KY cards} into your deck." — cards 3
//!   Radiant: "Shuffle {cards|random Radiant KY card|random Radiant KY cards} into your deck." — cards 3
//!
//! The whole card is one `shuffleRandomFromCatalog` (R1828002): `cards` independent picks (repeats
//! allowed, R60) from the KY cards, Radiant on the Radiant face, R80's cap turning the rest away.
//! Being a token is data (§7) and the text names no token, so §5.1 keeps the KY tokens out of the pool.
//! R11: a SPELL token goes to the GY like any spell (§3.2), so Discover can find it (R50, #72 Reminisce).
//!
//! Through v0.4.9 it was "Draw {draw}."; `script_v0_4_9` keeps that for those versions' replays (R767000).

use jackioh_engine::effects::shuffle_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-051-1";

/// A spell's script is its `cry` hook (§10.9). R1828002: `cards` (R386) random KY cards into the deck.
fn notebook() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![shuffle_random_from_catalog(json_as(json!({
                "query": { "tags": ["KY"] },
                "count": param(&*ctx, "cards"),
                "radiant": ctx.radiant,
            })))]
        })),
        ..Script::default()
    }
}

/// Through v0.4.9: draws `draw`, 1 on the base face and 2 on the Radiant.
fn notebook_v0_4_9() -> Script {
    Script {
        cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = notebook();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.4.9 the card drew; `KEPT_SCRIPTS` keeps it.
pub fn script_v0_4_9() -> CardScripts {
    let base = notebook_v0_4_9();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #51.1 KY's Empty Notebook (SPEC §8.3, §7, §5.1, §6.3 Shuffle into; R11, R50, R60, R1828002).
// BUILD M4-T4 row 51.1 (R1828002, R60, R80).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;
    use crate::def_ids;

    const P1: PlayerId = PlayerId::P1;

    /// Three ordinary library cards, top first, so a draw is visible as an id moving to the hand.
    const LIBRARY: [&str; 3] = ["core-005", "core-016", "core-010"];

    /// §2.5/R82 TURN ANCHOR. #10 Rapid Replenish, a 0-cost Spell, is always an affordable play, so one in
    /// hand keeps a side's turn from auto-ending; it adds no body and nothing to a count but its own.
    const ANCHOR: &str = "core-010";

    /// The ids a query answers, in its order.
    fn ids<T: std::borrow::Borrow<CardDef>>(defs: Vec<T>) -> Vec<String> {
        defs.iter().map(|def| <T as std::borrow::Borrow<CardDef>>::borrow(def).id.clone()).collect()
    }

    /// The non-token KY cards, the pool the Notebook shuffles from (§5.1).
    fn ky_pool() -> Vec<String> {
        ids(crate::query::query(&json_as(json!({ "tags": ["KY"] }))))
    }

    /// p1 plays the Notebook on `radiant`'s face, its `cards` stepped `step` times, with `seed`, and
    /// returns the scenario and the cards its play put in the library.
    fn shuffled(seed: &str, radiant: bool, step: i32) -> (Scenario, Vec<CardInstance>) {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": "core-051-1", "radiant": radiant }, ANCHOR], "library": LIBRARY },
        }));
        let before: Vec<String> = s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();
        for _ in 0..step.abs() {
            step_param(s.card_mut("core-051-1"), "cards", step.signum());
        }
        s.play("core-051-1", json!({}));
        let added = s.pile(P1, "library").into_iter().filter(|card| !before.contains(&card.id)).collect();
        (s, added)
    }

    #[test]
    fn r386_an_upgrade_shuffles_4_and_a_degrade_2() {
        for (step, count) in [(1, 4), (-1, 2)] {
            let (s, added) = shuffled("notebook-tuned", false, step);
            assert_eq!(added.len(), count);
            assert_eq!(s.pile(P1, "library").len(), LIBRARY.len() + count);
        }
    }

    mod n51_1_ky_s_empty_notebook_base {
        use super::*;

        #[test]
        fn r1828002_shuffles_3_random_non_token_ky_cards_into_your_deck() {
            let (mut s, added) = shuffled("notebook-base", false, 0);
            assert_eq!(added.len(), 3);
            for card in &added {
                assert!(ky_pool().contains(&card.def_id), "{} is no non-token KY card", card.def_id);
                assert!(!card.radiant);
            }
            assert_eq!(def_ids(&s.pile(P1, "hand")), vec![ANCHOR.to_string()]);
            s.expect_in_zone("core-051-1", "graveyard");
        }

        /// R80: a card shuffled into a full library is not created, so a deck one short of its cap takes one.
        #[test]
        fn r1828002_a_deck_one_short_of_its_cap_takes_one_and_turns_the_rest_away() {
            crate::register_all();
            let cap = usize::try_from(jackioh_engine::config::LIBRARY_CAP).unwrap_or(usize::MAX);
            let mut s = scenario(json!({
                "seed": "notebook-cap",
                "p1": { "hand": ["core-051-1", ANCHOR], "library": vec![LIBRARY[0]; cap - 1] },
            }));
            s.play("core-051-1", json!({}));
            assert_eq!(s.pile(P1, "library").len(), cap);
            let shuffled = s
                .events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::ShuffledIn)
                .count();
            assert_eq!(shuffled, 1);
        }

        #[test]
        fn r1828002_across_30_seeds_no_ky_token_is_shuffled_in() {
            let tokens = ids(crate::query::query(&json_as(json!({ "tags": ["KY", "Token"] }))));
            assert_eq!(
                tokens,
                [
                    "core-051-1",
                    "classicplus-042-1",
                    "meditative-045-1",
                    "meditativeplus-020-1",
                    "stinkyscape-004-2",
                ]
                .map(String::from)
            );
            let mut seen: Vec<String> = Vec::new();
            for n in 0..30 {
                let (_, added) = shuffled(&format!("notebook-pool-{n}"), false, 0);
                for card in added {
                    assert!(!tokens.contains(&card.def_id), "{} is a KY token", card.def_id);
                    assert!(ky_pool().contains(&card.def_id));
                    if !seen.contains(&card.def_id) {
                        seen.push(card.def_id);
                    }
                }
            }
            // R60: the picks reach across the pool.
            assert!(seen.len() > 3);
        }

        #[test]
        fn r11_a_spell_token_goes_to_the_graveyard_like_any_spell() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-051-1"], "library": LIBRARY } }));

            s.play("core-051-1", json!({})).expect_in_zone("core-051-1", "graveyard");
            // R50: which is why #72 Reminisce can then Discover it back out of the graveyard.
            let graveyard = def_ids(&s.pile(P1, "graveyard"));
            assert!(graveyard.contains(&"core-051-1".to_string()));
        }
    }

    mod n51_1_ky_s_empty_notebook_radiant {
        use super::*;

        #[test]
        fn r1828002_shuffles_3_random_radiant_ky_cards_into_your_deck() {
            let (s, added) = shuffled("notebook-radiant", true, 0);
            assert_eq!(added.len(), 3);
            for card in &added {
                assert!(ky_pool().contains(&card.def_id), "{} is no non-token KY card", card.def_id);
                assert!(card.radiant);
            }
            assert_eq!(def_ids(&s.pile(P1, "hand")), vec![ANCHOR.to_string()]);
        }
    }

    /// R767000: the script kept for the replays of v0.4.9 and before.
    mod v0_4_9 {
        use super::*;

        #[test]
        fn r767000_r1828002_a_game_on_v0_4_9_folds_under_the_kept_script_it_draws_1_and_2_radiant() {
            crate::register_all();
            jackioh_engine::with_shipped(crate::catalog_at("v0.4.9"), || {
                for (radiant, draws) in [(false, 1), (true, 2)] {
                    let (s, added) = shuffled("notebook-kept", radiant, 0);
                    assert!(added.is_empty());
                    assert_eq!(s.pile(P1, "library").len(), LIBRARY.len() - draws);
                    assert_eq!(s.pile(P1, "hand").len(), 1 + draws);
                }
            });
        }
    }

    mod n51_1_ky_s_empty_notebook_s5_1_absent_from_every_random_pool {
        use super::*;

        #[test]
        fn s5_1_the_whole_non_token_catalog_leaves_it_out_and_holds_n51_that_generates_it() {
            crate::register_all();
            let all = ids(crate::query::query(&json_as(json!({}))));

            assert!(!all.contains(&"core-051-1".to_string()));
            assert!(all.contains(&"core-051".to_string()));
        }

        #[test]
        fn s5_1_the_ky_pool_is_core_n31_n51_n82_classic_n41_n42_n62_meditative_n34_n38_n45_n79_n87_meditative_plus_n20_n25_and_stinkyscape_n25_the_token_tagged_ky_cards_are_never_offered()
        {
            crate::register_all();
            // The pool #57 Conjure KY generates from: the KY tag of every set (R380), minus tokens, minus
            // the generator: §8.8 #34, #38, #45, #79, #87; §8.9 (#1187) #20 and #25 but not its token #20.1;
            // §8.10 (#1884) #25 but not its token #4.2.
            assert_eq!(
                ids(crate::query::pool("core-057", &json_as(json!({ "tags": ["KY"] })))),
                vec![
                    "core-031".to_string(),
                    "core-051".to_string(),
                    "core-082".to_string(),
                    "classicplus-041".to_string(),
                    "classicplus-042".to_string(),
                    "classicplus-062".to_string(),
                    "meditative-034".to_string(),
                    "meditative-038".to_string(),
                    "meditative-045".to_string(),
                    "meditative-079".to_string(),
                    "meditative-087".to_string(),
                    "meditativeplus-020".to_string(),
                    "meditativeplus-025".to_string(),
                    "stinkyscape-025".to_string(),
                ]
            );
            assert!(!ids(crate::query::query(&json_as(json!({ "tags": ["KY"] })))).contains(&"core-051-1".to_string()));
        }

        #[test]
        fn r60_it_is_reachable_only_by_a_query_that_names_the_token_pool_itself_s5_1() {
            crate::register_all();
            assert!(ids(crate::query::query(&json_as(json!({ "tags": ["Token"] })))).contains(&"core-051-1".to_string()));
            assert_eq!(
                ids(crate::query::query(&json_as(json!({ "defId": "core-051-1" })))),
                vec!["core-051-1".to_string()]
            );
            // A cost or type filter that does not ask for tokens still cannot reach it.
            assert!(
                !ids(crate::query::query(&json_as(json!({ "type": "Spell", "cost": 1 })))).contains(&"core-051-1".to_string())
            );
        }
    }
}
