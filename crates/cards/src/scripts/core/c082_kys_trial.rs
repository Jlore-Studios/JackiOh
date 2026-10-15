//! #82 KY's Trial (SPEC §8.4 row 82): Spell, KY, cost 1, Rare.
//!   Base:    "Discover one of 3 random sets, then one of 3 random numbers in it. Add the Radiant card
//!             with that number to your hand."
//!   Radiant: "… It costs ({setCost})."
//!
//! "It" is the card the Discover adds, not this spell (§8 Conventions; JackiOh_Core_Cards.md #82).
//! Patch v0.3.9 (#831, R831004): the first Discover is a `mode` prompt of three of the four sets that
//! ship, drawn from the match rng; the second offers three numbers of the chosen set (the Core set alone
//! before). The pool is the engine's (§5.1, R54): `query` drops tokens and, by R387, never offers #82.
//! Three options are drawn without replacement (§6.3, R60). The options are the numbers (R247):
//! `offer: "index"` labels each with its §5 index and names no card. Each pick returns as a mode (§10.6,
//! R81), and the set rides the second prompt's resume data, since an index is unique only within its
//! set (B2.2). R74/§5.2: Radiant is a flag on a fresh instance. R65: `costOverride` 0 costs 0 in every
//! zone (R78). R4: a full hand burns the card that arrives.

pub const ID: &str = "core-082";

// #82 KY's Trial (SPEC §8.4 row 82; R54, R60, R65, R247). BUILD M4-T4's row: "Three distinct numbers
// 1–100 never 82 or a token index (R54); chosen card is radiant; radiant costs 0". The options are
// the numbers (R247), so each case reads them off `state.pending.options` and looks the card up in
// the catalog, as a player does in the collection.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRIAL: &str = "core-082";
    /// The four sets that ship, which the first Discover draws from (R831004).
    const SETS: [&str; 4] = ["Core", "Classic", "Classic+", "Meditative"];

    use crate::js;

    /// The catalog card a number names in a set (§5: its index), as the collection shows it.
    fn card_numbered(set: &str, index: &str) -> Option<String> {
        crate::CATALOG
            .values()
            .find(|def| def.set.as_str() == set && def.index == index)
            .map(|def| def.id.clone())
    }

    fn mode_option(selection: &Selection) -> String {
        match selection {
            Selection::Mode { option } => option.clone(),
            _ => String::new(),
        }
    }

    /// The options of the open prompt, as their `mode` strings, after checking its kind.
    fn options_of(s: &Scenario, kind: PromptKind) -> Vec<String> {
        let pending = s.state().pending.as_ref();
        assert_eq!(pending.map(|p| p.kind), Some(kind));
        pending
            .map(|p| p.options.iter().map(|option| mode_option(&option.selection)).collect())
            .unwrap_or_default()
    }

    /// The numbers the second Discover offered (§10.6, R247).
    fn numbers(s: &Scenario) -> Vec<String> {
        options_of(s, PromptKind::Discover)
    }

    /// The catalog ids those numbers name in `set`.
    fn offered(s: &Scenario, set: &str) -> Vec<String> {
        numbers(s)
            .into_iter()
            .map(|index| card_numbered(set, &index).unwrap_or_else(|| format!("no {set} card numbered {index}")))
            .collect()
    }

    /// KY's Trial played from p1's hand on `seed`, its first Discover open.
    fn played(seed: &str, radiant_face: bool) -> Scenario {
        let mut s = scenario(json!({ "seed": seed, "p1": { "hand": [{ "def": TRIAL, "radiant": radiant_face }] } }));
        s.play(TRIAL, json!({}));
        s
    }

    /// R831004: answer the set Discover with `set`. Its offer is the match rng's, so a test that means
    /// a set puts it in the offer first, in place of the first set offered.
    fn choose_set(s: &mut Scenario, set: &str) {
        let key = format!("mode:{set}");
        let Some(pending) = s.state_mut().pending.as_mut() else {
            panic!("the set Discover is open");
        };
        if !pending.options.iter().any(|option| option.key == key) {
            pending.options[0] = PromptOption {
                key: key.clone(),
                label: set.to_string(),
                selection: Selection::Mode { option: set.to_string() },
                cost: None,
                radiant: None,
            };
        }
        s.answer(json!(key));
    }

    /// Played, the set `set` chosen: the numbers Discover open.
    fn trial_scenario(radiant_face: bool, set: &str) -> Scenario {
        let mut s = played("kys-trial", radiant_face);
        choose_set(&mut s, set);
        s
    }

    /// Answers the numbers Discover with its first option; the card it names.
    fn take_first(s: &mut Scenario, set: &str) -> String {
        let first = numbers(s).first().cloned().unwrap_or_default();
        let chosen = card_numbered(set, &first).unwrap_or_default();
        s.answer(json!(format!("mode:{first}")));
        chosen
    }

    mod n82_ky_s_trial_base {
        use super::*;

        #[test]
        fn r831004_the_first_discover_offers_3_different_sets_of_the_four_that_ship_and_seeds_reach_each() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..20 {
                let sets = options_of(&played(&format!("kys-trial-sets-{n}"), false), PromptKind::Mode);
                assert_eq!(sets.len(), 3);
                assert_eq!(sets.iter().collect::<IndexSet<_>>().len(), 3);
                for set in &sets {
                    assert!(SETS.contains(&set.as_str()), "{set} is a set that ships");
                }
                seen.extend(sets);
            }
            assert_eq!(seen.len(), SETS.len());
        }

        #[test]
        fn r831004_r54_then_discovers_among_3_numbers_of_the_chosen_set_three_distinct_cards_no_token() {
            crate::register_all();
            for set in SETS {
                let s = trial_scenario(false, set);
                let ids = offered(&s, set);
                assert_eq!(ids.len(), 3, "{set}");
                // R60: "Discover options are always different".
                assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 3, "{set}");
                for id in &ids {
                    let Some(def) = crate::CATALOG.get(id) else {
                        panic!("{id} is not a catalog card");
                    };
                    assert_eq!(def.set.as_str(), set);
                    // R54 "never a token index", which §5.1 gives for free: no Token def, tag or rarity.
                    assert!(!def.token);
                    assert!(!def.tags.contains(&Tag::Token));
                    assert_ne!(def.rarity, Rarity::Token);
                }
            }
        }

        #[test]
        fn r54_r831004_it_never_offers_its_own_card_n82_of_core() {
            crate::register_all();
            for n in 0..10 {
                let mut s = played(&format!("kys-trial-own-{n}"), false);
                choose_set(&mut s, "Core");
                assert!(!offered(&s, "Core").contains(&TRIAL.to_string()));
                assert!(!numbers(&s).contains(&"82".to_string()));
            }
        }

        #[test]
        fn r74_r831004_the_chosen_card_of_the_chosen_set_arrives_in_your_hand_radiant() {
            crate::register_all();
            let mut s = trial_scenario(false, "Classic");
            let chosen = take_first(&mut s, "Classic");

            let added = s.hand(Some(P1)).into_iter().find(|card| card.def_id == chosen);
            assert!(added.is_some(), "no {chosen} in hand");
            assert!(added.is_some_and(|card| card.radiant));
            assert_eq!(crate::CATALOG.get(&chosen).map(|def| def.set.as_str()), Some("Classic"));
            s.expect_events(json!(["promptOpened", "promptAnswered", "addedToHand"]));
        }

        #[test]
        fn base_the_card_arrives_at_its_printed_cost_only_the_radiant_face_makes_it_free_r65() {
            crate::register_all();
            let mut s = trial_scenario(false, "Meditative");
            let chosen = take_first(&mut s, "Meditative");

            assert!(s
                .hand(Some(P1))
                .into_iter()
                .find(|card| card.def_id == chosen)
                .and_then(|card| card.cost_override)
                .is_none());
        }

        #[test]
        fn r113_r831004_paused_at_the_numbers_the_set_survives_json_and_the_copy_answers_to_the_same_hash() {
            crate::register_all();
            let s = trial_scenario(false, "Classic+");
            let copy: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips");
            assert_eq!(&copy, s.state());
            let pending = s.state().pending.clone().expect("the numbers Discover");
            let body = ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![pending.options[0].selection.clone()],
            };
            let live = reduce(s.state(), &Action::new(body.clone(), P1, "trial-rt"));
            let revived = reduce(&copy, &Action::new(body, P1, "trial-rt"));
            assert!(live.error.is_none());
            assert_eq!(hash_state(&live.state), hash_state(&revived.state));
        }
    }

    mod n82_ky_s_trial_radiant {
        use super::*;

        #[test]
        fn r54_r831004_the_radiant_face_discovers_the_same_way_a_set_then_3_distinct_numbers_never_n82() {
            crate::register_all();
            let s = played("kys-trial", true);
            assert_eq!(options_of(&s, PromptKind::Mode).len(), 3);
            let s = trial_scenario(true, "Core");
            let ids = offered(&s, "Core");
            assert_eq!(ids.len(), 3);
            assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 3);
            assert!(!ids.contains(&TRIAL.to_string()));
        }

        #[test]
        fn it_costs_0_r65_the_card_the_discover_adds_is_radiant_and_carries_a_0_cost_override() {
            crate::register_all();
            let mut s = trial_scenario(true, "Classic");
            let chosen = take_first(&mut s, "Classic");

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
            choose_set(&mut s, "Core");
            let chosen = take_first(&mut s, "Core");

            let added = s.hand(Some(P1)).into_iter().find(|card| card.def_id == chosen);
            assert_eq!(added.and_then(|card| card.cost_override), Some(1));
        }
    }

    mod n82_ky_s_trial_r247_the_options_are_the_numbers {
        use super::*;

        #[test]
        fn r247_offers_three_numbers_each_option_is_a_card_s_index_labelled_with_it_and_keyed_by_it() {
            crate::register_all();
            let s = trial_scenario(false, "Core");
            let options = s.state().pending.as_ref().map(|p| p.options.clone()).unwrap_or_default();

            assert_eq!(options.len(), 3);
            for option in &options {
                let index = mode_option(&option.selection);
                assert!(card_numbered("Core", &index).is_some(), "{index} numbers a Core card");
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
            let s = trial_scenario(false, "Classic");
            let chooser = s.view(Some(P1)).pending;
            let watcher = s.view(Some(P2)).pending;

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
            let mut s = trial_scenario(true, "Meditative");
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

            let wanted = card_numbered("Meditative", &picked);
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
