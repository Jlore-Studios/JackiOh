//! Saved decks, saved trios and the Best-of-1 deck check (SPEC §9.4, R250–R253, R641).
//!
//! A saved deck or trio is a draft: the save checks only its structure (D1–D5, T1–T3), and the
//! legality rules run when it is queued — L2, L3, L5 and L6 for a Best-of-1 deck, L1–L6 for a trio.
//! The limits a caller passes (`nameMaxLength`) come from its own config, so the tests pass a value
//! of their own. The portrait roster D5 checks against is the caller's too: the wire's `is_portrait_id`.

use std::sync::Arc;

use jackioh_engine::testkit::*;
use jackioh_engine::validator::{
    DeckDraftInput, TRIO_DECKS, check_deck_draft, check_import_room, check_trio_draft, normalize_name,
    trio_conflicts, validate_deck, validate_loadout, validate_trio,
};

use super::fixtures::validator_loadouts::{
    ARCHIVIST, CEASELESS_VOID, HIT_JOB, NOT_IN_CATALOG, POOL_IDS, SHEEP_TOKEN, catalog, collection,
    legal_decks,
};

const NAME_MAX: usize = 12;

/// The catalog as the JSON the validator is handed.
fn snapshot() -> Value {
    serde_json::to_value(catalog()).expect("the fixture catalog serialises")
}

/// The collection as JSON, so a test can set one id's count.
fn owned() -> Value {
    serde_json::to_value(collection()).expect("the fixture collection serialises")
}

/// In the snapshot and not a Token, by the flag or the tag.
fn is_deckable() -> Arc<dyn Fn(&str) -> bool + Send + Sync> {
    let snapshot = snapshot();
    Arc::new(move |card_id: &str| match snapshot["cards"].get(card_id) {
        Some(def) => {
            def["token"] != json!(true)
                && !def["tags"]
                    .as_array()
                    .is_some_and(|tags| tags.iter().any(|tag| tag == "Token"))
        }
        None => false,
    })
}

/// The roster predicate a draft check is handed.
type PortraitCheck = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// The roster predicate a caller passes for D5: the wire's `is_portrait_id`, which reads any JSON value.
fn is_portrait() -> PortraitCheck {
    Arc::new(|portrait: &str| is_portrait_id(&json!(portrait)))
}

/// The issues for a deck draft with `isDeckable` and `NAME_MAX`. The input borrows its predicates
/// (`DeckDraftInput<'a>`), so it is built and checked here, where they live.
fn draft_issues(
    name: &str,
    cards: &[String],
    portrait: Option<&str>,
    is_portrait: Option<PortraitCheck>,
) -> Value {
    let is_deckable = is_deckable();
    let input = DeckDraftInput {
        name: name.to_string(),
        cards: cards.to_vec(),
        is_deckable: &*is_deckable,
        name_max_length: NAME_MAX,
        portrait: portrait.map(str::to_string),
        is_portrait: is_portrait
            .as_deref()
            .map(|known| -> &dyn Fn(&str) -> bool { known }),
    };
    issues_of(&input)
}

/// The deck draft's issues, as JSON.
fn issues_of(input: &DeckDraftInput) -> Value {
    serde_json::to_value(check_deck_draft(input)).expect("draft issues serialise")
}

fn draft(cards: &[String], name: &str) -> Value {
    draft_issues(name, cards, None, None)
}

/// The rule of each issue.
fn rule_list(issues: &Value) -> Vec<String> {
    issues
        .as_array()
        .map(|list| {
            list.iter()
                .map(|issue| issue["rule"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// The rule and card of each issue.
fn rules_and_cards(issues: &Value) -> Value {
    Value::Array(
        issues
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|issue| json!([issue["rule"], issue["cardId"]]))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// The distinct rules a result failed, sorted; none when it is ok.
fn rules(result: &Value) -> Vec<String> {
    if result["ok"] == json!(true) {
        return Vec::new();
    }
    let mut seen: IndexSet<String> = IndexSet::new();
    for error in result["errors"].as_array().into_iter().flatten() {
        seen.insert(error["rule"].as_str().unwrap_or_default().to_string());
    }
    let mut out: Vec<String> = seen.into_iter().collect();
    out.sort();
    out
}

fn deck_result(input: Value) -> Value {
    serde_json::to_value(validate_deck(&json_as(input))).expect("a loadout result serialises")
}

fn trio_result(input: Value) -> Value {
    serde_json::to_value(validate_trio(&json_as(input))).expect("a loadout result serialises")
}

fn conflicts(decks: Value) -> Value {
    serde_json::to_value(trio_conflicts(json_as::<Vec<_>>(decks).as_slice()))
        .expect("trio conflicts serialise")
}

fn import_room(input: Value) -> Value {
    serde_json::to_value(check_import_room(&json_as(input))).expect("an import room serialises")
}

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

mod r250_a_saved_deck_is_a_draft_d1_d4_are_all_a_save_checks {
    use super::*;

    #[test]
    fn r250_saves_an_empty_a_partial_and_a_full_deck_alike() {
        assert_eq!(draft(&[], "Aggro"), json!([]));
        assert_eq!(draft(&POOL_IDS[..7], "Aggro"), json!([]));
        assert_eq!(draft(&POOL_IDS[..DECK_SIZE as usize], "Aggro"), json!([]));
    }

    #[test]
    fn r250_saves_cards_the_player_does_not_own_and_banned_cards_those_are_judged_at_queue() {
        assert_eq!(draft(&strings(&[ARCHIVIST, CEASELESS_VOID]), "Aggro"), json!([]));
    }

    #[test]
    fn r250_d1_wants_a_name_of_1_to_the_limits_characters_trimmed_with_no_control_characters() {
        assert_eq!(rule_list(&draft(&[], "   ")), vec!["D1"]);
        assert_eq!(rule_list(&draft(&[], &"x".repeat(NAME_MAX + 1))), vec!["D1"]);
        assert_eq!(draft(&[], &format!("  {}  ", "x".repeat(NAME_MAX))), json!([]));
        assert_eq!(rule_list(&draft(&[], "bad\u{0007}name")), vec!["D1"]);
        // Code points, not UTF-16 units: an emoji is one character to a player.
        assert_eq!(draft(&[], &"🃏".repeat(NAME_MAX)), json!([]));
    }

    #[test]
    fn r250_d1_counts_the_invisible_controls_as_control_characters_and_a_name_nobody_can_see_as_no_name() {
        let refused_as = |name: &str| -> Option<String> {
            draft(&[], name)
                .as_array()
                .and_then(|issues| issues.iter().find(|issue| issue["rule"] == "D1"))
                .and_then(|issue| issue["message"].as_str().map(str::to_string))
        };
        let control = Some("A deck name cannot contain control characters.".to_string());
        // The bidirectional controls make a name display as text it does not hold ("Aggro" + RLO + "orez"
        // reads "Aggrozero"), and the zero-width ones hide characters inside it.
        assert_eq!(refused_as("Aggro\u{202e}orez"), control);
        assert_eq!(refused_as("Ag\u{2066}gro\u{2069}"), control);
        assert_eq!(refused_as("Aggro\u{200f}"), control);
        assert_eq!(refused_as("Ag\u{200b}gro"), control);
        assert_eq!(refused_as("Ag\u{2060}gro"), control);
        assert_eq!(refused_as("Ag\u{00ad}gro"), control);
        // A name made only of characters that draw nothing is no name at all.
        assert_eq!(refused_as("\u{200d}"), Some("A deck needs a name.".to_string()));
        assert_eq!(
            refused_as("\u{fe0f}\u{200d}\u{fe0f}"),
            Some("A deck needs a name.".to_string())
        );
        // The joiners that emoji and several scripts are written with stay legal, as does a flag.
        assert_eq!(
            draft(&[], "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} family"),
            json!([])
        );
        assert_eq!(
            draft(
                &[],
                "\u{0645}\u{06cc}\u{200c}\u{062e}\u{0648}\u{0627}\u{0647}\u{0645}"
            ),
            json!([])
        );
        assert_eq!(
            draft(
                &[],
                "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f} Scot"
            ),
            json!([])
        );
        assert_eq!(draft(&[], "\u{2764}\u{fe0f} Aggro"), json!([]));
    }

    #[test]
    fn r250_d2_refuses_more_than_deck_size_cards() {
        let issues = draft(&POOL_IDS[..DECK_SIZE as usize + 1], "Aggro");
        assert_eq!(rule_list(&issues), vec!["D2"]);
        assert_eq!(
            issues[0]["message"],
            json!(format!(
                "A deck holds at most {DECK_SIZE} cards; this one has {}.",
                DECK_SIZE + 1
            ))
        );
    }

    #[test]
    fn r250_d3_refuses_an_id_outside_the_catalog_and_a_token() {
        assert_eq!(
            rules_and_cards(&draft(&strings(&[NOT_IN_CATALOG]), "Aggro")),
            json!([["D3", NOT_IN_CATALOG]])
        );
        assert_eq!(
            rules_and_cards(&draft(&strings(&[SHEEP_TOKEN]), "Aggro")),
            json!([["D3", SHEEP_TOKEN]])
        );
    }

    #[test]
    fn r250_d4_refuses_more_than_max_copies_of_a_card() {
        let cards: Vec<String> = (0..MAX_COPIES + 1).map(|_| HIT_JOB.to_string()).collect();
        assert_eq!(rules_and_cards(&draft(&cards, "Aggro")), json!([["D4", HIT_JOB]]));
    }

    #[test]
    fn r250_normalizes_a_name_the_way_it_is_stored_trimmed_inner_whitespace_one_space() {
        assert_eq!(normalize_name("  My   first\tdeck "), "My first deck");
    }
}

mod r641_d5_a_saved_decks_portrait_is_null_or_a_known_portrait_id {
    use super::*;

    /// A draft carrying `portrait`, D5-checked against the shared roster as the caller passes it.
    fn portrait_draft(portrait: Option<&str>, is_portrait: Option<PortraitCheck>) -> Value {
        draft_issues("Aggro", &[], portrait, is_portrait)
    }

    #[test]
    fn r641_saves_a_deck_whose_portrait_is_null_and_one_carrying_every_roster_id() {
        assert_eq!(portrait_draft(None, Some(is_portrait())), json!([]));
        for id in PORTRAIT_IDS {
            let id = id.to_string();
            assert_eq!(portrait_draft(Some(&id), Some(is_portrait())), json!([]), "{id}");
        }
    }

    #[test]
    fn r641_fails_an_unknown_portrait_id_naming_the_field_beside_the_other_draft_issues() {
        assert_eq!(
            portrait_draft(Some("not-a-portrait"), Some(is_portrait())),
            json!([{ "rule": "D5", "message": "\"portrait\" is not a known portrait id." }])
        );
        // D5 is collected with the other rules' failures, not instead of them.
        assert_eq!(
            rule_list(&draft_issues(
                "",
                &[],
                Some("not-a-portrait"),
                Some(is_portrait())
            )),
            vec!["D1", "D5"]
        );
    }

    #[test]
    fn r641_checks_nothing_when_no_portrait_is_given_or_the_caller_knows_no_roster() {
        // `portrait` absent or undefined: nothing for D5 to read.
        assert_eq!(draft(&[], "Aggro"), json!([]));
        assert_eq!(portrait_draft(None, Some(is_portrait())), json!([]));
        // `isPortrait` absent — a caller that predates portraits: the field is taken as given.
        assert_eq!(portrait_draft(Some("not-a-portrait"), None), json!([]));
    }
}

mod r251_a_card_is_its_catalog_id_across_a_trio {
    use super::*;

    #[test]
    fn r251_finds_every_card_two_decks_hold_with_the_decks_that_hold_it_in_trio_order() {
        let legal = legal_decks();
        let (a, b, c) = (&legal[0], &legal[1], &legal[2]);
        let shared = a[0].to_string();
        let mut b_shared: Vec<String> = b[1..].iter().map(|id| id.to_string()).collect();
        b_shared.push(shared.clone());
        let mut c_shared: Vec<String> = c[1..].iter().map(|id| id.to_string()).collect();
        c_shared.push(shared.clone());
        let decks = json!([{ "cards": a }, { "cards": b_shared }, { "cards": c_shared }]);
        assert_eq!(
            conflicts(decks),
            json!([{ "cardId": shared, "decks": [0, 1, 2] }])
        );
        let all: Vec<Value> = legal_decks()
            .iter()
            .map(|cards| json!({ "cards": cards }))
            .collect();
        assert_eq!(conflicts(Value::Array(all)), json!([]));
    }

    #[test]
    fn r251_reads_the_conflict_l4_reports_the_same_fact_worded_once_by_l4() {
        let legal = legal_decks();
        let (a, b, c) = (&legal[0], &legal[1], &legal[2]);
        let shared = a[0].to_string();
        let mut b_shared: Vec<String> = b[1..].iter().map(|id| id.to_string()).collect();
        b_shared.push(shared.clone());
        let decks = json!([{ "cards": a }, { "cards": b_shared }, { "cards": c }]);
        let mut coll = owned();
        coll[shared.as_str()] = json!(2);
        let result = trio_result(json!({ "decks": decks, "catalog": snapshot(), "collection": coll }));
        assert_eq!(rules(&result), vec!["L4"]);
        let cards: Vec<Value> = conflicts(decks)
            .as_array()
            .into_iter()
            .flatten()
            .map(|conflict| conflict["cardId"].clone())
            .collect();
        assert_eq!(cards, vec![json!(shared)]);
    }
}

mod r252_a_saved_trio_is_three_slots_naming_three_different_decks_any_of_them_empty {
    use super::*;

    fn trio(deck_ids: Value, name: &str) -> Vec<String> {
        let issues = check_trio_draft(&json_as(json!({
            "name": name,
            "deckIds": deck_ids,
            "nameMaxLength": NAME_MAX,
        })));
        rule_list(&serde_json::to_value(issues).expect("trio issues serialise"))
    }

    #[test]
    fn r252_saves_a_trio_with_empty_slots() {
        assert!(trio(json!([null, null, null]), "Ladder").is_empty());
        assert!(trio(json!(["a", null, "c"]), "Ladder").is_empty());
        assert_eq!(TRIO_DECKS, 3);
    }

    #[test]
    fn r252_t1_names_t2_counts_the_slots_t3_refuses_one_deck_twice() {
        assert_eq!(trio(json!(["a", "b", "c"]), ""), vec!["T1"]);
        // T1 is "a name as D1": the invisible controls are refused in a trio's name too.
        assert_eq!(trio(json!(["a", "b", "c"]), "Ladder\u{202e}"), vec!["T1"]);
        assert_eq!(trio(json!(["a", "b"]), "Ladder"), vec!["T2"]);
        assert_eq!(trio(json!(["a", "a", null]), "Ladder"), vec!["T3"]);
    }
}

mod r253_what_may_be_queued_a_best_of_1_deck_passes_l2_l3_l5_and_l6 {
    use super::*;

    fn deck() -> Vec<String> {
        legal_decks()[0].iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn r253_passes_a_legal_deck_and_names_it_in_every_refusal() {
        let deck = deck();
        assert_eq!(
            deck_result(json!({
                "deck": { "name": "Aggro", "cards": deck },
                "catalog": snapshot(),
                "collection": owned(),
            })),
            json!({ "ok": true })
        );
        let short = deck_result(json!({
            "deck": { "name": "Aggro", "cards": deck[1..] },
            "catalog": snapshot(),
            "collection": owned(),
        }));
        let errors = if short["ok"] == json!(true) {
            json!([])
        } else {
            short["errors"].clone()
        };
        assert_eq!(
            errors,
            json!([{
                "rule": "L2",
                "message": format!(
                    "Aggro has {} cards; every deck needs exactly {DECK_SIZE}.",
                    DECK_SIZE - 1
                ),
                "deck": 1,
            }])
        );
    }

    #[test]
    fn r253_words_l5_for_the_deck_not_a_trio_and_never_raises_l1_or_l4() {
        let deck = deck();
        let mut coll = owned();
        coll[deck[0].as_str()] = json!(0);
        let unowned = deck_result(json!({
            "deck": { "name": "Aggro", "cards": deck },
            "catalog": snapshot(),
            "collection": coll,
        }));
        let triples: Vec<Value> = if unowned["ok"] == json!(true) {
            Vec::new()
        } else {
            unowned["errors"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|error| json!([error["rule"], error["message"], error["deck"]]))
                .collect()
        };
        assert_eq!(
            triples,
            vec![json!([
                "L5",
                "Aggro uses 1 copy of \"Fixture Card 001\" (core-001) but you own 0.",
                1
            ])]
        );
        let mut with_token: Vec<String> = deck[1..].to_vec();
        with_token.push(SHEEP_TOKEN.to_string());
        let with_token = deck_result(json!({
            "deck": { "name": "Aggro", "cards": with_token },
            "catalog": snapshot(),
            "collection": owned(),
        }));
        assert_eq!(rules(&with_token), vec!["L3"]);
    }

    #[test]
    fn r253_checks_a_trio_with_l1_l6_a_trio_is_s9_4s_loadout() {
        // Compared by what the two functions answer, on a legal trio and on a short one.
        let all: Vec<Value> = legal_decks()
            .iter()
            .map(|cards| json!({ "cards": cards }))
            .collect();
        let legal = json!({ "decks": all, "catalog": snapshot(), "collection": owned() });
        assert_eq!(
            trio_result(legal.clone()),
            serde_json::to_value(validate_loadout(&json_as(legal))).expect("a loadout result serialises")
        );
        let two: Vec<Value> = legal_decks()
            .iter()
            .take(2)
            .map(|cards| json!({ "cards": cards }))
            .collect();
        let short = json!({ "decks": two, "catalog": snapshot(), "collection": owned() });
        assert_eq!(
            trio_result(short.clone()),
            serde_json::to_value(validate_loadout(&json_as(short.clone())))
                .expect("a loadout result serialises")
        );
        assert_eq!(rules(&trio_result(short)), vec!["L1"]);
    }
}

mod r340_room_for_an_imported_trio {
    use super::*;

    fn limits() -> Value {
        json!({ "decks": 10, "trios": 5 })
    }

    fn room(saved: Value, adding: Value) -> Value {
        import_room(json!({ "saved": saved, "limits": limits(), "adding": adding }))
    }

    /// Every key of `expected` is in `actual` with the same value.
    fn matches_object(actual: &Value, expected: &Value) -> bool {
        expected
            .as_object()
            .is_some_and(|keys| keys.iter().all(|(key, value)| actual.get(key) == Some(value)))
    }

    #[test]
    fn r340_an_import_that_fits_under_both_caps_is_ok() {
        assert_eq!(
            room(
                json!({ "decks": 7, "trios": 4 }),
                json!({ "decks": 3, "trios": 1 })
            ),
            json!({ "ok": true })
        );
        assert_eq!(
            room(
                json!({ "decks": 0, "trios": 0 }),
                json!({ "decks": 3, "trios": 1 })
            ),
            json!({ "ok": true })
        );
    }

    #[test]
    fn r340_says_exactly_how_many_deck_slots_are_missing() {
        let room = room(
            json!({ "decks": 9, "trios": 1 }),
            json!({ "decks": 3, "trios": 1 }),
        );
        assert_eq!(
            room,
            json!({
                "ok": false,
                "decksShort": 2,
                "triosShort": 0,
                "message": "Importing this trio needs 3 free deck slots, and you have 1 free deck slot. Delete 2 decks, then import it again.",
            })
        );
    }

    #[test]
    fn r340_says_exactly_how_many_trio_slots_are_missing_and_both_at_once() {
        let trios = room(
            json!({ "decks": 0, "trios": 5 }),
            json!({ "decks": 3, "trios": 1 }),
        );
        assert!(matches_object(
            &trios,
            &json!({ "ok": false, "decksShort": 0, "triosShort": 1 })
        ));
        assert_eq!(
            trios["message"],
            json!(
                "Importing this trio needs 1 free trio slot, and you have no free trio slot. Delete 1 trio, then import it again."
            )
        );
        let both = room(
            json!({ "decks": 10, "trios": 5 }),
            json!({ "decks": 3, "trios": 1 }),
        );
        assert!(matches_object(
            &both,
            &json!({ "ok": false, "decksShort": 3, "triosShort": 1 })
        ));
        assert_eq!(
            both["message"],
            json!(
                "Importing this trio needs 3 free deck slots and 1 free trio slot, and you have no free deck slot and no free trio slot. Delete 3 decks and 1 trio, then import it again."
            )
        );
    }

    #[test]
    fn r340_counts_a_profile_already_past_a_cap_as_having_no_room_at_all() {
        let room = room(
            json!({ "decks": 12, "trios": 0 }),
            json!({ "decks": 1, "trios": 1 }),
        );
        assert!(matches_object(
            &room,
            &json!({ "ok": false, "decksShort": 1, "triosShort": 0 })
        ));
    }
}
