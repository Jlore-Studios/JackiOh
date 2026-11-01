//! S #21 Moonberry (SPEC §8.10 row S 21; §2.4, §5.1, §6.1, R1837000, R1867007): (1) Spell,
//! Fruit, Common.
//!   Base:    "Flashback / Add a random Other-Origins card to your hand."
//!   Radiant: "Flashback / Add a random Radiant Other-Origins card to your hand. It costs
//!            ({discount}) less." — discount 1, the Radiant face alone.
//!
//! Engine: Flashback is the catalog keyword ([§6.1]], R783012): the script only adds. One pick from
//! the match rng (R60) among the Other-Origins tokens of the sets that ship
//! (`add_random_from_catalog` with the tag's tokens let in, C+ #32.1–#32.3 now and S #10.1–#12.3
//! once the set ships, R1837000, R1420), added to the hand through §2.4's pipeline, a full hand
//! burning it. Radiant: the card Radiant with `costMod` −discount, never below (0) (R65).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-021";

fn moonberry(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "tags": ["Other-Origins"], "token": true } });
            if radiant {
                let cheaper = -param(&*ctx, "discount");
                args["radiant"] = json!(true);
                args["costMod"] = json!(cheaper);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: moonberry(false),
        radiant: moonberry(true),
    }
}

// S #21 Moonberry — SPEC §8.10 row S 21, BUILD M12 row S 21 (R1867007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const MOONBERRY: &str = "stinkyscape-021";
    const FILLER: &str = "core-005";

    const SHIPPED_TOKENS: [&str; 3] = ["classicplus-032-1", "classicplus-032-2", "classicplus-032-3"];

    const STINKYSCAPE_TOKENS: [&str; 9] = [
        "stinkyscape-010-1",
        "stinkyscape-010-2",
        "stinkyscape-010-3",
        "stinkyscape-011-1",
        "stinkyscape-011-2",
        "stinkyscape-011-3",
        "stinkyscape-012-1",
        "stinkyscape-012-2",
        "stinkyscape-012-3",
    ];

    fn added_def(seed: &str, radiant: bool) -> String {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": MOONBERRY, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(MOONBERRY, json!({}));
        s.hand(P1).into_iter().find(|card| card.def_id != FILLER).map(|card| card.def_id).unwrap_or_default()
    }

    fn added_card(seed: &str, radiant: bool) -> CardInstance {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": MOONBERRY, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(MOONBERRY, json!({}));
        s.hand(P1).into_iter().find(|card| card.def_id != FILLER).expect("a card added")
    }

    fn priced(state: &GameState, card: &CardInstance) -> i32 {
        effective_cost(state, card, Default::default())
    }

    mod base {
        use super::*;

        #[test]
        fn r1867007_adds_an_other_origins_token_each_of_c_32_1_to_32_3_and_the_sets_nine_seen_over_the_seeds() {
            // Since the release (#1884) the set's own nine join C+ #32.1 to #32.3.
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 0..200 {
                let def = added_def(&format!("moonberry-{seed}"), false);
                assert!(
                    SHIPPED_TOKENS.contains(&def.as_str()) || STINKYSCAPE_TOKENS.contains(&def.as_str()),
                    "only an Other-Origins token of a set that ships: {def}"
                );
                seen.insert(def);
            }
            let expected: IndexSet<String> = SHIPPED_TOKENS
                .into_iter()
                .chain(STINKYSCAPE_TOKENS)
                .map(str::to_string)
                .collect();
            assert_eq!(seen, expected);
        }

        #[test]
        fn r1867007_never_a_non_token_card() {
            for seed in 0..40 {
                let def = added_def(&format!("moonberry-token-{seed}"), false);
                assert!(crate::card_def(&def).token, "{def} is a token");
            }
            assert_ne!(added_def("moonberry-berry", false), MOONBERRY);
        }

        #[test]
        fn r1867007_the_nine_s_tokens_join_the_pool() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 0..200 {
                let added = added_card(&format!("moonberry-preview-{seed}"), false);
                assert!(crate::card_def(&added.def_id).token, "{} is a token", added.def_id);
                assert!(
                    SHIPPED_TOKENS.contains(&added.def_id.as_str())
                        || STINKYSCAPE_TOKENS.contains(&added.def_id.as_str()),
                    "an Other-Origins token: {}",
                    added.def_id
                );
                seen.insert(added.def_id);
            }
            for token in STINKYSCAPE_TOKENS {
                assert!(seen.contains(token), "{token} is added over the seeds");
            }
        }
    }

    /// A play of a graveyard card (B5 E11), sent to `reduce` as `legalActions` offers it: the
    /// harness's `play` takes a card from a hand, so a graveyard play is reduced here (C #28 Second
    /// Wind's shape).
    fn graveyard_play(state: &GameState, id: &str, attempt: u32) -> (Option<String>, GameState) {
        let action = json_as::<Action>(json!({
            "type": "play",
            "playerId": "p1",
            "instanceId": id,
            "nonce": format!("moonberry-flashback-{id}-{attempt}"),
        }));
        let result = reduce(state, &action);
        (result.error, result.state)
    }

    #[test]
    fn r1867007_played_again_from_the_graveyard_once_by_flashback_then_not_again() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [MOONBERRY, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(MOONBERRY, json!({}));
        let grave =
            s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == MOONBERRY).expect("played");

        let (error, state) = graveyard_play(s.state(), &grave.id, 1);
        assert!(error.is_none(), "{error:?}");
        let added: Vec<&CardInstance> =
            state.players.p1.hand.iter().filter(|card| card.def_id != FILLER).collect();
        assert_eq!(added.len(), 2);

        let spent = state
            .players
            .p1
            .graveyard
            .iter()
            .find(|card| card.id == grave.id)
            .expect("back in the graveyard");
        assert!(
            legal_actions(&state, P1)
                .iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, .. } => Some(instance_id),
                    _ => None,
                })
                .all(|id| id != &spent.id)
        );
        let (error, _) = graveyard_play(&state, &spent.id, 2);
        assert!(error.is_some());
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_card_is_radiant_and_costs_1_less_a_0_one_staying_0() {
            crate::register_all();
            let probe = scenario(json!({ "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
            let mut zeros = 0;
            for seed in 0..60 {
                let added = added_card(&format!("moonberry-radiant-{seed}"), true);
                assert!(added.radiant, "{} is Radiant", added.def_id);
                assert_eq!(added.cost_mod, -1, "{} costs 1 less", added.def_id);
                let printed = query_cost(&crate::card_def(&added.def_id));
                assert_eq!(
                    priced(probe.state(), &added),
                    (printed - 1).max(0),
                    "{} costs 1 less, never below (0)",
                    added.def_id
                );
                if printed == 0 {
                    zeros += 1;
                }
            }
            assert!(zeros > 0, "a (0) card stays (0)");
        }

        #[test]
        fn discount_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "moonberry-param",
                "p1": { "hand": [{ "def": MOONBERRY, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let discount = crate::upgrade_number(&mut s, MOONBERRY, "discount");
            s.play(MOONBERRY, json!({}));
            let added = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id != FILLER)
                .expect("a card added");
            assert_eq!(added.cost_mod, -discount);
        }
    }
}
