//! B3.4 rule 5: the numbers Degrade, Upgrade and KY's Constant may move are declared in the catalog
//! as `params`, and each face's text writes them as `{key}`. `fill_params` (the engine's wire, TS
//! packages/shared) is the one function that turns a face's text into what a player reads, from the
//! printed values or from an instance's current ones; the client, R277's diff and the text tests all
//! read a face through it.
//!
//! R482 fixes how: every item of a Classic or Classic+ card's Numbers line (docs/classic-sets.md
//! B6–B7; the AI generated cards of B8 have none) is one param, written `{key}` in each face's text
//! that shows that number and nowhere as a literal; a count and the words that agree with it are
//! written `{key|singular|plural}`, so the text reads right at every value a Degrade or an Upgrade can
//! move it to; a numbered keyword (Armor, Lucky, Brittle, Spell Damage) and an Echo, Tribute or
//! Activate count is no param, since B3.4's X change tunes those.
//!
//! Port of `packages/cards/test/params.test.ts`.

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, card_def};
use jackioh_engine::{CardDef, FaceKind, ParamBetter, Tag, fill_params, param_placeholders};
use serde_json::{Value, json};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

/// `fillParams(card, face, { [key]: value })`.
fn filled_with(card: &CardDef, face: FaceKind, key: &str, value: i32) -> String {
    let values: IndexMap<String, i32> = [(key.to_string(), value)].into_iter().collect();
    fill_params(card, face, Some(&values))
}

/// `expect(actual).toMatchObject(expected)` on JSON: every key `expected` names holds the same value.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual.as_object(), expected.as_object()) {
        (Some(actual), Some(expected)) => expected.iter().all(|(key, value)| actual.get(key) == Some(value)),
        _ => false,
    }
}

/// The first param a card declares, as JSON (`cardDef(id).params?.[0]`).
fn first_param(id: &str) -> Value {
    let def = card_def(id);
    let first = def.params.as_ref().and_then(|params| params.first());
    serde_json::to_value(first).expect("a Param serialises")
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `\b<number> <noun>\b` for a number `accept` takes and a noun of `nouns`: a run of digits that
/// starts at a word boundary, one space, the noun, and a word boundary after it (the hand-written
/// form of the two regular expressions R482's last check uses; there is no regex crate).
fn number_then_noun(text: &str, accept: impl Fn(&str) -> bool, nouns: &[&str]) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if !bytes[at].is_ascii_digit() || (at > 0 && is_word(bytes[at - 1])) {
            at += 1;
            continue;
        }
        let mut end = at;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if accept(&text[at..end]) && bytes.get(end) == Some(&b' ') {
            let rest = &text[end + 1..];
            for noun in nouns {
                if let Some(after) = rest.strip_prefix(noun)
                    && !after.bytes().next().is_some_and(is_word)
                {
                    return true;
                }
            }
        }
        at = end;
    }
    false
}

/// `/\b1 (cards|times|Plague Counters|Units|Spells)\b/`: a lone 1 before a plural noun.
fn one_with_a_plural(text: &str) -> bool {
    number_then_noun(text, |n| n == "1", &["cards", "times", "Plague Counters", "Units", "Spells"])
}

/// `/\b([2-9]|\d{2,}) (card|time|Plague Counter|Unit|Spell)\b(?!s)/`: a number above 1 before a
/// singular noun.
fn many_with_a_singular(text: &str) -> bool {
    number_then_noun(
        text,
        |n| (n.len() == 1 && ('2'..='9').contains(&n.chars().next().unwrap_or('0'))) || n.len() >= 2,
        &["card", "time", "Plague Counter", "Unit", "Spell"],
    )
}

mod b3_4_params_the_numbers_a_card_declares {
    use super::*;

    #[test]
    fn fills_each_face_s_text_with_its_own_printed_values() {
        let heal = card_def("classic-003");
        assert_eq!(heal.base.text, "Heal a target {heal}.");
        assert_eq!(fill_params(&heal, FaceKind::Base, None), "Heal a target 9.");
        assert_eq!(fill_params(&heal, FaceKind::Radiant, None), "Heal a target 18.");
    }

    #[test]
    fn fills_an_instance_s_current_values_when_given_and_leaves_a_text_without_params_alone() {
        let flame = card_def("classic-016");
        assert_eq!(filled_with(&flame, FaceKind::Base, "damage", 5), "Deal 5 damage.");
        let none: IndexMap<String, i32> = IndexMap::new();
        assert_eq!(fill_params(&flame, FaceKind::Radiant, Some(&none)), "Deal 8 damage.");
        let vanilla = card_def("core-008");
        assert!(vanilla.params.is_none());
        assert_eq!(fill_params(&vanilla, FaceKind::Base, None), vanilla.base.text);
    }

    #[test]
    fn r482_declares_every_placeholder_a_text_writes_and_writes_every_param_it_declares() {
        let mut wrong: Vec<String> = Vec::new();
        for card in entries() {
            let declared = card.params.clone().unwrap_or_default();
            let keys: IndexSet<String> = declared.iter().map(|param| param.key.clone()).collect();
            let written: IndexSet<String> = [&card.base.text, &card.radiant.text]
                .into_iter()
                .flat_map(|text| param_placeholders(text).into_iter().map(|placeholder| placeholder.key))
                .collect();
            for key in &written {
                if !keys.contains(key) {
                    wrong.push(format!("{}: {{{key}}} is not declared", card.id));
                }
            }
            for key in &keys {
                if !written.contains(key) {
                    wrong.push(format!("{}: param {key} is written in neither face", card.id));
                }
            }
            if keys.len() != declared.len() {
                wrong.push(format!("{}: repeats a key", card.id));
            }
        }
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn keeps_every_value_inside_its_bounds_with_a_direction_and_a_positive_step() {
        let mut wrong: Vec<String> = Vec::new();
        for card in entries() {
            for param in card.params.iter().flatten() {
                let at = format!("{} {}", card.id, param.key);
                if param.better != ParamBetter::Up && param.better != ParamBetter::Down {
                    wrong.push(format!("{at}: better"));
                }
                if let Some(step) = param.step
                    && step <= 0
                {
                    wrong.push(format!("{at}: step"));
                }
                for value in [param.base, param.radiant] {
                    if let Some(min) = param.min
                        && value < min
                    {
                        wrong.push(format!("{at}: {value} < min {min}"));
                    }
                    if let Some(max) = param.max
                        && value > max
                    {
                        wrong.push(format!("{at}: {value} > max {max}"));
                    }
                }
            }
        }
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r482_reads_the_numbers_lines_as_the_brief_writes_them_on_cards_that_show_one_of_each_kind() {
        // A Radiant-only number: Cloaked Toe Cracker's "gain 1 mana" exists on its Radiant face only.
        assert_eq!(
            serde_json::to_value(&card_def("classic-006").params).expect("params serialise"),
            json!([{ "key": "mana", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }])
        );
        assert!(!card_def("classic-006").base.text.contains("{mana}"));
        // A stated step and a lower-is-better threshold with a stated floor (Flame Lance deals 10 [20],
        // balance patch 1).
        assert!(matches_object(
            &first_param("classic-083"),
            &json!({ "key": "damage", "base": 10, "radiant": 20, "step": 2 })
        ));
        assert!(matches_object(
            &first_param("classic-038"),
            &json!({ "key": "plays", "base": 3, "radiant": 2, "better": "down", "min": 2 })
        ));
        // A percentage: step 10, never above 100.
        assert!(matches_object(
            &first_param("classicplus-019-2"),
            &json!({ "key": "chance", "base": 25, "radiant": 50, "step": 10, "max": 100 })
        ));
    }

    #[test]
    fn r482_never_makes_a_numbered_keyword_an_echo_or_an_activate_count_a_param_the_x_change_tunes_those() {
        let keys = |id: &str| -> Vec<String> {
            card_def(id).params.unwrap_or_default().into_iter().map(|param| param.key).collect()
        };
        // Solarius prints Spell Damage +2; its Cry (which drew 1) is gone on both faces (balance
        // patch 1), so no number is a param.
        assert_eq!(
            serde_json::to_value(&card_def("classicplus-038").base.keywords).expect("keywords serialise"),
            json!([{ "kind": "Spell Damage", "n": 2 }])
        );
        assert_eq!(keys("classicplus-038"), Vec::<String>::new());
        // Top Loser's Armor, Twice Forward's printed Brittle, Brother Ping's Activate, Echo's Echo 1.
        assert_eq!(keys("classicplus-019-1"), Vec::<String>::new());
        assert_eq!(keys("classicplus-074"), vec!["plays".to_string(), "brittleGain".to_string()]);
        assert_eq!(keys("classicplus-076-1"), vec!["damage".to_string()]);
        assert!(card_def("classic-057").params.is_none());
        // No param shares its name with a keyword kind.
        for card in entries() {
            for param in card.params.iter().flatten() {
                let named_like_a_keyword = ["armor", "lucky", "brittle", "spellDamage", "echo", "activate"]
                    .contains(&param.key.as_str())
                    && card
                        .base
                        .keywords
                        .iter()
                        .any(|keyword| keyword.kind().as_str().to_lowercase() == param.key);
                assert!(!named_like_a_keyword, "{} {}", card.id, param.key);
            }
        }
    }

    #[test]
    fn r482_writes_a_count_with_the_words_that_agree_with_it_so_the_text_reads_right_at_every_value() {
        let pickle = card_def("classic-008"); // Pickle: "they discard {discard|card|cards}"
        assert!(pickle.base.text.contains("{discard|card|cards}"));
        assert!(fill_params(&pickle, FaceKind::Base, None).contains("they discard 1 card,"));
        assert!(fill_params(&pickle, FaceKind::Radiant, None).contains("they discard 2 cards,"));
        assert!(filled_with(&pickle, FaceKind::Base, "discard", 3).contains("they discard 3 cards,"));
        assert_eq!(
            serde_json::to_value(param_placeholders("Draw {draw|card|cards}, then {damage}."))
                .expect("placeholders serialise"),
            json!([{ "key": "draw", "one": "card", "many": "cards" }, { "key": "damage" }])
        );
        // No filled face leaves a count disagreeing with its noun ("1 cards", "2 card").
        let mut wrong: Vec<String> = Vec::new();
        for card in entries() {
            for param in card.params.iter().flatten() {
                let low = param.min.unwrap_or(1);
                for value in [low, low + param.step.unwrap_or(1), param.base, param.radiant] {
                    for face in [FaceKind::Base, FaceKind::Radiant] {
                        let text = filled_with(card, face, &param.key, value);
                        if one_with_a_plural(&text) {
                            wrong.push(format!("{} {face} at {value}: {text}", card.id));
                        }
                        if many_with_a_singular(&text) {
                            wrong.push(format!("{} {face} at {value}: {text}", card.id));
                        }
                    }
                }
            }
        }
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r482_declares_no_params_on_the_ten_ai_generated_cards_b8_gives_them_no_numbers_line() {
        let with_params: Vec<String> = entries()
            .into_iter()
            .filter(|card| card.tags.contains(&Tag::Ai) && card.params.is_some())
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(with_params, Vec::<String>::new());
    }

    #[test]
    fn the_hand_written_count_checks_read_as_their_regular_expressions() {
        assert!(one_with_a_plural("Draw 1 cards."));
        assert!(!one_with_a_plural("Draw 11 cards."));
        assert!(!one_with_a_plural("Draw 1 card."));
        assert!(many_with_a_singular("Draw 2 card."));
        assert!(many_with_a_singular("Deal 12 time, then"));
        assert!(!many_with_a_singular("Draw 2 cards."));
        assert!(!many_with_a_singular("Draw 1 card."));
        assert!(!many_with_a_singular("Summon 2 Units."));
    }
}
