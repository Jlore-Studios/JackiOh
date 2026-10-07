//! The loadout rules L1–L6 (SPEC §9.4), one test per rule with its exact message (BUILD M6-T3).
//!
//! Two things beyond "the rule fires" are under test here. First the message, because §9.4 says a
//! failure "names the deck and the card" and `apps/web` shows that same sentence in the
//! deckbuilder (BUILD e2e `09-deckbuilder.cy.ts`), so the string is part of the contract. Second
//! that one violation reports one rule: each injection in the fixtures breaks a single rule while
//! leaving the other five satisfiable, and a validator that let a duplicate card also read as
//! "you do not own this" would bury the real reason in the builder's error list.
//!
//! `LoadoutError.deck` is the 1-based deck number the message shows; it is absent on the
//! loadout-wide rules (L1, L4, L5), whose messages name no single deck.
//!
//! Port of `packages/validator/test/validator.test.ts` (part 5). The property tests drew their
//! loadouts from fast-check; here they come from the engine's own seeded `Rng`, with the same fixed
//! seed and run count, so they fail the same way twice.

use indexmap::IndexSet;
use jackioh_engine::config::{DECK_SIZE, MAX_COPIES};
use jackioh_engine::rng::Rng;
use jackioh_engine::validator::{
    CardId, LOADOUT_DECKS, LoadoutDeck, LoadoutError, LoadoutInput, LoadoutResult, LoadoutRule, validate_loadout,
};

use super::fixtures::validator_loadouts::{
    ARCHIVIST, CATALOG_VERSION, CEASELESS_VOID, HIT_JOB, INJECTIONS, JELLY_BEAN, NOT_IN_CATALOG, POOL_IDS, add_deck,
    add_spare_card, banned_card, catalog, collection, cross_deck, drop_card, drop_deck, duplicate_in_deck,
    insert_token, legal_loadout, unknown_card, unown_card,
};

/// A fixed seed and run count: the property tests must fail the same way twice.
const SEED: &str = "20260917";
const NUM_RUNS: usize = 200;

fn errors_of(result: &LoadoutResult) -> Vec<LoadoutError> {
    if result.ok {
        panic!("expected validate_loadout to reject this loadout");
    }
    result.errors.clone()
}

/// The distinct rules a result reports, sorted, for the no-cascade assertions.
fn rules_of(result: &LoadoutResult) -> Vec<LoadoutRule> {
    let mut rules: Vec<LoadoutRule> = errors_of(result)
        .into_iter()
        .map(|error| error.rule)
        .collect::<IndexSet<LoadoutRule>>()
        .into_iter()
        .collect();
    rules.sort();
    rules
}

/// The one error a single-rule injection is expected to produce.
fn sole_error(result: &LoadoutResult) -> LoadoutError {
    let errors = errors_of(result);
    if errors.len() != 1 {
        let messages: Vec<&str> = errors.iter().map(|error| error.message.as_str()).collect();
        panic!("expected exactly one error, got {}: {}", errors.len(), messages.join(" | "));
    }
    errors[0].clone()
}

/// `{ ok: true }`.
fn accepted() -> LoadoutResult {
    LoadoutResult {
        ok: true,
        errors: Vec::new(),
    }
}

mod loadout_rules_l1_l6_s9_4_m6_t3 {
    use super::*;

    #[test]
    fn accepts_a_legal_loadout() {
        assert_eq!(validate_loadout(&legal_loadout()), accepted());
    }

    #[test]
    fn l1_rejects_a_loadout_that_does_not_hold_exactly_three_decks() {
        let error = sole_error(&validate_loadout(&drop_deck(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L1);
        assert_eq!(
            error.message,
            format!(
                "A trio needs exactly {LOADOUT_DECKS} decks; this one has {}.",
                LOADOUT_DECKS - 1
            )
        );
        assert_eq!(error.deck, None);
        assert_eq!(error.card_id, None);
    }

    #[test]
    fn l1_rejects_a_loadout_with_a_fourth_deck_however_legal_that_deck_is() {
        let error = sole_error(&validate_loadout(&add_deck(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L1);
        assert_eq!(
            error.message,
            format!(
                "A trio needs exactly {LOADOUT_DECKS} decks; this one has {}.",
                LOADOUT_DECKS + 1
            )
        );
        assert_eq!(error.deck, None);
    }

    #[test]
    fn l2_rejects_a_deck_that_is_one_card_short() {
        let error = sole_error(&validate_loadout(&drop_card(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L2);
        assert_eq!(
            error.message,
            format!("Deck 1 has {} cards; every deck needs exactly {DECK_SIZE}.", DECK_SIZE - 1)
        );
        assert_eq!(error.deck, Some(1));
        assert_eq!(error.card_id, None);
    }

    #[test]
    fn l2_rejects_a_deck_that_is_one_card_over() {
        let error = sole_error(&validate_loadout(&add_spare_card(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L2);
        assert_eq!(
            error.message,
            format!("Deck 1 has {} cards; every deck needs exactly {DECK_SIZE}.", DECK_SIZE + 1)
        );
        assert_eq!(error.deck, Some(1));
    }

    #[test]
    fn l2_counts_one_card_as_1_card_the_builder_reaches_that_by_deleting() {
        let mut trimmed = legal_loadout();
        let first = trimmed
            .decks
            .first_mut()
            .expect("fixture error: the legal loadout has no decks");
        first.cards.truncate(1);

        let error = sole_error(&validate_loadout(&trimmed));
        assert_eq!(error.rule, LoadoutRule::L2);
        assert_eq!(
            error.message,
            format!("Deck 1 has 1 card; every deck needs exactly {DECK_SIZE}.")
        );
    }

    #[test]
    fn l2_names_a_deck_by_its_builder_label_when_it_has_one() {
        let mut named = legal_loadout();
        let first = named
            .decks
            .first_mut()
            .expect("fixture error: the legal loadout has no decks");
        first.name = Some("Aggro".to_string());

        let error = sole_error(&validate_loadout(&drop_card(&named)));
        assert_eq!(
            error.message,
            format!("Aggro has {} cards; every deck needs exactly {DECK_SIZE}.", DECK_SIZE - 1)
        );
    }

    #[test]
    fn l3_rejects_a_second_copy_of_a_card_in_one_deck() {
        let error = sole_error(&validate_loadout(&duplicate_in_deck(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L3);
        assert_eq!(
            error.message,
            format!(
                "Deck 2 has {} copies of \"Glowy Jelly Bean\" (core-051); at most {MAX_COPIES} copy of a card is allowed per deck.",
                MAX_COPIES + 1
            )
        );
        assert_eq!(error.deck, Some(2));
        assert_eq!(error.card_id.as_deref(), Some(JELLY_BEAN));
    }

    #[test]
    fn l3_rejects_a_token_card_in_a_deck() {
        let error = sole_error(&validate_loadout(&insert_token(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L3);
        assert_eq!(
            error.message,
            format!(
                "Deck {LOADOUT_DECKS} cannot contain \"Sheep Token\" (core-051.1): Token cards are never deckable."
            )
        );
        assert_eq!(error.deck, Some(LOADOUT_DECKS));
        assert_eq!(error.card_id.as_deref(), Some("core-051.1"));
    }

    #[test]
    fn l4_rejects_a_card_that_is_in_two_decks_of_the_loadout() {
        let error = sole_error(&validate_loadout(&cross_deck(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L4);
        assert_eq!(
            error.message,
            "\"Hit Job\" (core-012) appears in Deck 1 and Deck 2; a card may be in only one deck of a trio."
        );
        assert_eq!(error.deck, None); // the message names both decks; the field names one
        assert_eq!(error.card_id.as_deref(), Some(HIT_JOB));
    }

    #[test]
    fn l5_rejects_a_card_the_profile_does_not_own() {
        let error = sole_error(&validate_loadout(&unown_card(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L5);
        assert_eq!(
            error.message,
            "Your trio uses 1 copy of \"Archivist\" (core-030) but you own 0."
        );
        assert_eq!(error.deck, None); // L5 counts across the loadout, not per deck
        assert_eq!(error.card_id.as_deref(), Some(ARCHIVIST));
    }

    #[test]
    fn l6_rejects_a_card_that_is_not_in_the_catalog_snapshot() {
        let error = sole_error(&validate_loadout(&unknown_card(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L6);
        assert_eq!(
            error.message,
            format!("Deck 1 cannot contain \"core-999\": no such card in catalog version {CATALOG_VERSION}.")
        );
        assert_eq!(error.deck, Some(1));
        assert_eq!(error.card_id.as_deref(), Some(NOT_IN_CATALOG));
    }

    #[test]
    fn l6_rejects_a_banned_card() {
        let error = sole_error(&validate_loadout(&banned_card(&legal_loadout())));
        assert_eq!(error.rule, LoadoutRule::L6);
        assert_eq!(
            error.message,
            "Deck 1 cannot contain \"Ceaseless Void\" (core-100): that card is banned."
        );
        assert_eq!(error.deck, Some(1));
        assert_eq!(error.card_id.as_deref(), Some(CEASELESS_VOID));
    }

    #[test]
    fn reports_every_broken_rule_at_once_so_the_builder_can_show_them_together() {
        // Two independent faults in Deck 1: it is a card short, and the card at its head is not in
        // the snapshot. §9.4's validator is the deckbuilder's, so it collects rather than short-circuits.
        assert_eq!(
            rules_of(&validate_loadout(&unknown_card(&drop_card(&legal_loadout())))),
            vec![LoadoutRule::L2, LoadoutRule::L6]
        );
    }

    #[test]
    fn l5_counts_copies_across_the_loadout_which_with_max_copies_1_means_l3_or_l4_fires_with_it() {
        // The plural L5 message needs two copies of one card, and with MAX_COPIES = 1 a second copy
        // is either in the same deck (L3) or in another deck (L4). So this is the one message that
        // cannot be produced on its own; the copies still have to be counted and named correctly.
        let legal = legal_loadout();
        let decks: Vec<LoadoutDeck> = legal
            .decks
            .iter()
            .map(|deck| {
                if deck.cards.iter().any(|id| id == ARCHIVIST) {
                    let mut deck = deck.clone();
                    if let Some(first) = deck.cards.first_mut() {
                        *first = ARCHIVIST.to_string();
                    }
                    deck
                } else {
                    deck.clone()
                }
            })
            .collect();
        let result = validate_loadout(&LoadoutInput { decks, ..legal });

        assert_eq!(rules_of(&result), vec![LoadoutRule::L3, LoadoutRule::L5]);
        let l5: Vec<String> = errors_of(&result)
            .into_iter()
            .filter(|error| error.rule == LoadoutRule::L5)
            .map(|error| error.message)
            .collect();
        assert_eq!(
            l5,
            vec![format!(
                "Your trio uses {} copies of \"Archivist\" (core-030) but you own 1.",
                MAX_COPIES + 1
            )]
        );
    }
}

mod one_violation_reports_one_rule_s9_4 {
    use super::*;

    /// TS `it.each(INJECTIONS)("$rule and nothing else for $label", …)`.
    #[test]
    fn each_injection_reports_its_rule_and_nothing_else() {
        for injection in INJECTIONS.iter() {
            assert_eq!(
                rules_of(&validate_loadout(&(injection.apply)(&legal_loadout()))),
                vec![injection.rule],
                "{} and nothing else for {}",
                injection.rule,
                injection.label
            );
        }
    }
}

// --- properties over random loadouts (BUILD §"Property: random loadouts") --------------------

fn required() -> Vec<CardId> {
    vec![HIT_JOB.to_string(), JELLY_BEAN.to_string(), ARCHIVIST.to_string()]
}

fn optional() -> Vec<CardId> {
    let required = required();
    POOL_IDS
        .iter()
        .map(|id| id.to_string())
        .filter(|id| !required.contains(id))
        .collect()
}

fn deck_cards() -> usize {
    LOADOUT_DECKS * DECK_SIZE as usize
}

/// fast-check's `shuffledSubarray(items, { minLength: n, maxLength: n })`: `n` of them, in a random order.
fn shuffled_subarray(rng: &mut Rng, items: &[CardId], n: usize) -> Vec<CardId> {
    rng.shuffle(items).into_iter().take(n).collect()
}

/// fast-check's `option(string({ minLength: 1, maxLength: 12 }), { nil: undefined })`: printable ASCII.
fn random_name(rng: &mut Rng) -> Option<String> {
    if rng.int(6) == 0 {
        return None;
    }
    let length = 1 + rng.int(12) as usize;
    Some((0..length).map(|_| char::from(b' ' + rng.int(95) as u8)).collect())
}

/// A legal loadout with a random split: which owned ids land in which deck, in which order, under
/// which names. The three cards the injections work on are always dealt somewhere, so every
/// injection applies to every generated loadout.
fn legal_loadout_arb(rng: &mut Rng) -> LoadoutInput {
    let rest = shuffled_subarray(rng, &optional(), deck_cards() - required().len());
    let mut dealt = required();
    dealt.extend(rest);
    let ids = rng.shuffle(&dealt);
    let names: Vec<Option<String>> = (0..LOADOUT_DECKS).map(|_| random_name(rng)).collect();
    let size = DECK_SIZE as usize;
    LoadoutInput {
        decks: (0..LOADOUT_DECKS)
            .map(|i| LoadoutDeck {
                name: names[i].clone(),
                cards: ids[i * size..(i + 1) * size].to_vec(),
            })
            .collect(),
        catalog: catalog(),
        collection: collection(),
    }
}

mod properties_over_random_loadouts {
    use super::*;

    #[test]
    fn accepts_every_legal_loadout_whatever_the_split_order_or_names() {
        let mut rng = Rng::new(SEED, 0);
        for run in 0..NUM_RUNS {
            let input = legal_loadout_arb(&mut rng);
            assert_eq!(validate_loadout(&input), accepted(), "run {run}");
        }
    }

    #[test]
    fn reports_exactly_the_one_rule_a_single_injection_breaks() {
        let mut rng = Rng::new(SEED, 0);
        for run in 0..NUM_RUNS {
            let input = legal_loadout_arb(&mut rng);
            let injection = &INJECTIONS[rng.int(INJECTIONS.len() as i32) as usize];
            assert_eq!(
                rules_of(&validate_loadout(&(injection.apply)(&input))),
                vec![injection.rule],
                "run {run}: {}",
                injection.label
            );
        }
    }
}
