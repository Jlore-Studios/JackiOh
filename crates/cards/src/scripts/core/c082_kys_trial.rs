//! #82 KY's Trial (SPEC §8.4 row 82): Spell, KY, cost 1, Rare.
//!   Base:    "Discover among 3 distinct random numbers 1–100; add the Radiant Core card with that
//!             index to your hand"
//!   Radiant: "It costs 0"
//!
//! "It" is the card the Discover adds, not this spell (§8 Conventions; JackiOh_Core_Cards.md #82).
//! The pool is the engine's (§5.1, R54): `query` drops tokens and, by R387, never offers #82, and
//! `set: Core` is 1–100. Three options are drawn without replacement (§6.3, R60). The options are
//! the numbers (R247): `offer: "index"` labels each with its §5 index and names no card. The pick
//! returns as a mode (§10.6, R81); an index is unique only within its set (B2.2). R74/§5.2: Radiant
//! is a flag on a fresh instance. R65: `costOverride` 0 costs 0 in every zone (R78). R4: a full hand
//! burns the card that arrives.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-082";

/// §6.3 Discover offers three, and R54's roll is "1–100", which is the non-token Core set.
const OPTIONS: i32 = 3;

/// B2.6: KY's Trial names the Core set's numbers, so its pool stays Core whatever else ships.
const TRIAL_SET: SetName = SetName::Core;

/// The `resume` step the Discover answer re-enters (§10.6).
const PICKED: &str = "picked";

/// The faces differ only in whether the card that arrives carries a `costOverride` of 0 (R65).
fn trial(costs_zero: bool) -> Script {
    Script {
        // A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9).
        cry: Some(hook(|_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "count": OPTIONS,
                // §5.1's one pool source. Tokens and #82 itself are excluded for us — see the header.
                "query": { "set": TRIAL_SET },
                // R247: the options are the numbers, not the cards they index.
                "offer": "index",
                "prompt": "KY's Trial: Discover a number from 1 to 100",
            })))]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let index = chosen_options(ctx).into_iter().next();
                // §8 Conventions: an empty pick fizzles and the spell still counts as played.
                let def_id = index.and_then(|index| def_by_index(TRIAL_SET, &index).map(|def| def.id.clone()));
                let Some(def_id) = def_id else {
                    return vec![];
                };
                let mut args = json!({ "defId": def_id, "radiant": true });
                // "It costs (0)": the declared number `setCost` (R386).
                if costs_zero {
                    args["costOverride"] = json!(param(&*ctx, "setCost"));
                }
                vec![add_to_hand(json_as(args))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = trial(false);
    // "It costs 0": the card the Discover adds, per the source note and BUILD M4-T4.
    let radiant = trial(true);
    CardScripts { base, radiant }
}

// #82 KY's Trial (SPEC §8.4 row 82; R54, R60, R65, R247). BUILD M4-T4's row: "Three distinct numbers
// 1–100 never 82 or a token index (R54); chosen card is radiant; radiant costs 0". The options are
// the numbers (R247), so each case reads them off `state.pending.options` and looks the card up in
// the catalog, as a player does in the collection.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRIAL: &str = "core-082";

    use crate::js;

    /// The ten token indices are the ten non-integer ones; R54 forbids every one of them.
    fn is_numbered_one_to_hundred(index: &str) -> bool {
        !index.is_empty()
            && index.chars().all(|c| c.is_ascii_digit())
            && index.parse::<u64>().map(|n| (1..=100).contains(&n)).unwrap_or(false)
    }

    /// The catalog card a number names (§5: its index), as the collection shows it.
    fn card_numbered(index: &str) -> Option<String> {
        crate::CATALOG
            .values()
            .find(|def| def.index == index)
            .map(|def| def.id.clone())
    }

    fn mode_option(selection: &Selection) -> String {
        match selection {
            Selection::Mode { option } => option.clone(),
            _ => String::new(),
        }
    }

    /// The numbers a Discover offered, read out of the prompt the play opened (§10.6, R247).
    fn numbers(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.as_ref();
        assert!(pending.is_some());
        assert_eq!(pending.map(|p| p.kind), Some(PromptKind::Discover));
        pending
            .map(|p| {
                p.options
                    .iter()
                    .map(|option| match &option.selection {
                        Selection::Mode { option } => option.clone(),
                        _ => format!("not-a-mode:{}", option.key),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The catalog ids those numbers name.
    fn offered(s: &Scenario) -> Vec<String> {
        numbers(s)
            .into_iter()
            .map(|index| card_numbered(&index).unwrap_or_else(|| format!("no card numbered {index}")))
            .collect()
    }

    fn trial_scenario(radiant_face: bool) -> Scenario {
        let mut s = scenario(json!({ "seed": "kys-trial", "p1": { "hand": [TRIAL] } }));
        // Stands in for a missing `{ def, radiant }` form on SideSetup.hand (harness request).
        if radiant_face {
            let id = s.card(TRIAL).id.clone();
            find_instance_mut(s.state_mut(), &id).expect("the trial in hand").radiant = true;
        }
        s.play(TRIAL, json!({}));
        s
    }

    mod n82_ky_s_trial_base {
        use super::*;

        #[test]
        fn r54_discovers_among_3_numbers_1_100_three_distinct_core_cards_no_token_index() {
            crate::register_all();
            let ids = offered(&trial_scenario(false));

            assert_eq!(ids.len(), 3);
            // R60: "Discover options are always different".
            assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 3);
            for id in &ids {
                let def = crate::CATALOG.get(id);
                assert!(def.is_some(), "{id} is not a catalog card");
                let def = def.unwrap();
                assert_eq!(def.set, SetName::Core);
                // R54 "never a token index", which §5.1 gives for free: no Token def, tag or rarity.
                assert!(!def.token);
                assert!(!def.tags.contains(&Tag::Token));
                assert_ne!(def.rarity, Rarity::Token);
                // R54 "rolls 1–100 only".
                assert!(is_numbered_one_to_hundred(&def.index));
            }
        }

        #[test]
        fn r54_rerolls_its_own_index_n82_is_never_one_of_the_three() {
            crate::register_all();
            let s = trial_scenario(false);

            assert!(!offered(&s).contains(&TRIAL.to_string()));
            assert!(!numbers(&s).contains(&"82".to_string()));
        }

        #[test]
        fn r74_the_chosen_card_arrives_in_your_hand_radiant() {
            crate::register_all();
            let mut s = trial_scenario(false);
            let first = s.state().pending.as_ref().and_then(|p| p.options.first().cloned());
            let chosen = card_numbered(&first.as_ref().map(|o| mode_option(&o.selection)).unwrap_or_default())
                .unwrap_or_default();

            s.answer(json!(first.map(|o| o.key).unwrap_or_default()));

            let added = s.hand(Some(P1)).into_iter().find(|card| card.def_id == chosen);
            assert!(added.is_some(), "no {chosen} in hand");
            assert!(added.unwrap().radiant);
            s.expect_events(json!(["promptOpened", "promptAnswered", "addedToHand"]));
        }

        #[test]
        fn base_the_card_arrives_at_its_printed_cost_only_the_radiant_face_makes_it_free_r65() {
            crate::register_all();
            let mut s = trial_scenario(false);
            let first = s.state().pending.as_ref().and_then(|p| p.options.first().cloned());
            let chosen = card_numbered(&first.as_ref().map(|o| mode_option(&o.selection)).unwrap_or_default())
                .unwrap_or_default();

            s.answer(json!(first.map(|o| o.key).unwrap_or_default()));

            assert!(s
                .hand(Some(P1))
                .into_iter()
                .find(|card| card.def_id == chosen)
                .and_then(|card| card.cost_override)
                .is_none());
        }
    }

    mod n82_ky_s_trial_radiant {
        use super::*;

        #[test]
        fn r54_the_radiant_face_discovers_from_the_same_pool_3_distinct_never_n82_never_a_token() {
            crate::register_all();
            let ids = offered(&trial_scenario(true));

            assert_eq!(ids.len(), 3);
            assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 3);
            assert!(!ids.contains(&TRIAL.to_string()));
            for id in &ids {
                let index = crate::CATALOG.get(id).map(|def| def.index.clone()).unwrap_or_default();
                assert!(is_numbered_one_to_hundred(&index));
            }
        }

        #[test]
        fn it_costs_0_r65_the_card_the_discover_adds_is_radiant_and_carries_a_0_cost_override() {
            crate::register_all();
            let mut s = trial_scenario(true);
            let first = s.state().pending.as_ref().and_then(|p| p.options.first().cloned());
            let chosen = card_numbered(&first.as_ref().map(|o| mode_option(&o.selection)).unwrap_or_default())
                .unwrap_or_default();

            s.answer(json!(first.map(|o| o.key).unwrap_or_default()));

            let added = s.hand(Some(P1)).into_iter().find(|card| card.def_id == chosen);
            assert_eq!(added.as_ref().map(|card| card.radiant), Some(true));
            // R65 starts the calculation from `costOverride`, so 0 here is a card that costs 0 in hand and
            // keeps costing 0 in every zone (R78).
            assert_eq!(added.and_then(|card| card.cost_override), Some(0));
        }

        #[test]
        fn r386_a_degrade_makes_the_card_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "kys-trial", "p1": { "hand": [{ "def": TRIAL, "radiant": true }] } }));
            assert!(!crate::can_upgrade_number(&s, TRIAL, "setCost"));
            assert_eq!(crate::degrade_number(&mut s, TRIAL, "setCost"), 1);
            s.play(TRIAL, json!({}));
            let first = s.state().pending.as_ref().and_then(|p| p.options.first().cloned());
            let chosen = card_numbered(&first.as_ref().map(|o| mode_option(&o.selection)).unwrap_or_default())
                .unwrap_or_default();

            s.answer(json!(first.map(|o| o.key).unwrap_or_default()));

            let added = s.hand(Some(P1)).into_iter().find(|card| card.def_id == chosen);
            assert_eq!(added.and_then(|card| card.cost_override), Some(1));
        }
    }

    mod n82_ky_s_trial_r247_the_options_are_the_numbers {
        use super::*;

        #[test]
        fn r247_offers_three_numbers_each_option_is_a_card_s_index_labelled_with_it_and_keyed_by_it() {
            crate::register_all();
            let s = trial_scenario(false);
            let options = s.state().pending.as_ref().map(|p| p.options.clone()).unwrap_or_default();

            assert_eq!(options.len(), 3);
            for option in &options {
                let index = mode_option(&option.selection);
                assert!(is_numbered_one_to_hundred(&index), "{index} is a number from 1 to 100");
                assert_eq!(option.label, index);
                assert_eq!(option.key, format!("mode:{index}"));
                // Nothing in the option names a card: neither its key, its label nor its selection.
                assert!(!serde_json::to_string(option).expect("serialises").contains("core-"));
            }
        }

        #[test]
        fn r247_the_chooser_s_view_shows_the_numbers_and_names_no_card_the_other_seat_sees_only_that_a_prompt_is_open(
        ) {
            crate::register_all();
            let s = trial_scenario(false);
            let chooser = s.view(Some(P1)).pending;
            let watcher = s.view(Some(P2)).pending;

            assert!(matches!(chooser, Some(PendingView::ForYou(_))));
            let Some(PendingView::ForYou(chooser)) = chooser else {
                panic!("the chooser has no prompt");
            };
            assert!(chooser.for_you);
            assert_eq!(chooser.kind, PromptKind::Discover);
            assert_eq!(
                chooser.options.iter().map(|option| option.label.clone()).collect::<Vec<_>>(),
                numbers(&s)
            );
            for option in &chooser.options {
                assert!(option.def_id.is_none(), "option {} names no definition", option.key);
                assert!(option.instance_id.is_none());
            }
            // §10.6: the other seat learns that a choice is open, and whose.
            assert_eq!(js(&watcher), json!({ "forYou": false, "pendingFor": "p1" }));
        }

        #[test]
        fn r247_legalactions_offers_one_answer_per_number_and_the_answer_adds_the_radiant_card_with_that_index() {
            crate::register_all();
            let mut s = trial_scenario(true);
            let choice_id = s.state().pending.as_ref().map(|p| p.id.clone()).unwrap_or_default();
            let answers: Vec<Vec<Selection>> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|body| match body {
                    ActionBody::Answer { choice_id: id, selection } if id == choice_id => Some(selection),
                    _ => None,
                })
                .collect();

            let expected: Vec<Vec<Selection>> = numbers(&s)
                .into_iter()
                .map(|index| vec![Selection::Mode { option: index }])
                .collect();
            assert_eq!(answers, expected);

            let picked = numbers(&s).get(2).cloned().unwrap_or_default();
            s.answer(json!([{ "pick": "mode", "option": picked }]));

            let wanted = card_numbered(&picked);
            let added = s
                .hand(Some(P1))
                .into_iter()
                .find(|card| Some(&card.def_id) == wanted.as_ref());
            assert!(added.is_some(), "no card numbered {picked} in hand");
            let added = added.unwrap();
            assert!(added.radiant);
            assert_eq!(added.cost_override, Some(0));
            assert!(view_for(s.state(), P1).pending.is_none());
        }
    }
}
