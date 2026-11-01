//! B3.4 rule 5, R277, R482: numbers Degrade/Upgrade move are declared as `params` (docs/classic-sets.md
//! B6–B7, B8). `fill_params` turns `{key}` into player-facing text.

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, card_def};
use jackioh_engine::{CardDef, FaceKind, ParamBetter, Tag, fill_params, param_placeholders};
use serde_json::{Value, json};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

fn filled_with(card: &CardDef, face: FaceKind, key: &str, value: i32) -> String {
    let values: IndexMap<String, i32> = [(key.to_string(), value)].into_iter().collect();
    fill_params(card, face, Some(&values))
}

fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual.as_object(), expected.as_object()) {
        (Some(actual), Some(expected)) => expected.iter().all(|(key, value)| actual.get(key) == Some(value)),
        _ => false,
    }
}

fn first_param(id: &str) -> Value {
    let def = card_def(id);
    let first = def.params.as_ref().and_then(|params| params.first());
    serde_json::to_value(first).expect("a Param serializes")
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

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

fn one_with_a_plural(text: &str) -> bool {
    number_then_noun(
        text,
        |n| n == "1",
        &["cards", "times", "Plague Counters", "Units", "Spells"],
    )
}

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
        assert_eq!(
            filled_with(&flame, FaceKind::Base, "damage", 5),
            "Trample\nDeal 5 damage."
        );
        let none: IndexMap<String, i32> = IndexMap::new();
        assert_eq!(
            fill_params(&flame, FaceKind::Radiant, Some(&none)),
            "Trample\nDeal 8 damage."
        );
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
                .flat_map(|text| {
                    param_placeholders(text)
                        .into_iter()
                        .map(|placeholder| placeholder.key)
                })
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
        // is tuned there alone (R824000).
        assert_eq!(
            serde_json::to_value(&card_def("classic-006").params).expect("params serialize"),
            json!([{ "key": "mana", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }])
        );
        assert!(!card_def("classic-006").base.text.contains("{mana}"));
        assert!(matches_object(
            &first_param("classic-083"),
            &json!({ "key": "damage", "base": 10, "radiant": 20, "step": 2 })
        ));
        assert!(matches_object(
            &first_param("classic-038"),
            &json!({ "key": "plays", "base": 3, "radiant": 2, "better": "down", "min": 2 })
        ));
        // A percentage: step 10, never above 100, at its bound on both faces since issue #1828.
        assert!(matches_object(
            &first_param("classicplus-019-2"),
            &json!({ "key": "chance", "base": 100, "radiant": 100, "step": 10, "max": 100 })
        ));
    }

    #[test]
    fn r482_never_makes_a_numbered_keyword_an_echo_or_an_activate_count_a_param_the_x_change_tunes_those() {
        let keys = |id: &str| -> Vec<String> {
            card_def(id)
                .params
                .unwrap_or_default()
                .into_iter()
                .map(|param| param.key)
                .collect()
        };
        assert_eq!(
            serde_json::to_value(&card_def("classicplus-038").base.keywords).expect("keywords serialize"),
            json!([{ "kind": "Spell Damage", "n": 2 }])
        );
        assert_eq!(keys("classicplus-038"), Vec::<String>::new());
        assert_eq!(keys("classicplus-019-1"), Vec::<String>::new());
        assert_eq!(
            keys("classicplus-074"),
            vec!["plays".to_string(), "brittleGain".to_string()]
        );
        assert_eq!(keys("classicplus-076-1"), vec!["damage".to_string()]);
        assert!(card_def("classic-057").params.is_none());
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
        // Pickle: "they discard {discard|random card|random cards}" (#911 says the discard is random).
        let pickle = card_def("classic-008");
        assert!(pickle.base.text.contains("{discard|random card|random cards}"));
        assert!(fill_params(&pickle, FaceKind::Base, None).contains("they discard 1 random card,"));
        assert!(fill_params(&pickle, FaceKind::Radiant, None).contains("they discard 2 random cards,"));
        assert!(filled_with(&pickle, FaceKind::Base, "discard", 3).contains("they discard 3 random cards,"));
        assert_eq!(
            serde_json::to_value(param_placeholders("Draw {draw|card|cards}, then {damage}."))
                .expect("placeholders serialize"),
            json!([{ "key": "draw", "one": "card", "many": "cards" }, { "key": "damage" }])
        );
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
        // Meditative+ brief defines for Bob's Plan, tune their numbers as their §8.9 rows say (R1185000).
        let with_params: Vec<String> = entries()
            .into_iter()
            .filter(|card| {
                card.tags.contains(&Tag::Ai) && card.index.starts_with("T-AI-") && card.params.is_some()
            })
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

/// only one face prints is tuned on that face alone (R749, R1431, R824000), and a number that belongs to a
/// power is tuned only while the card has that power (R1430).
mod r1430_r1431_the_numbers_a_change_reaches {
    use super::*;
    use jackioh_engine::effects::TuneDirection;
    use jackioh_engine::testkit::{numbers_on, param_value, step_param, steppable_params};
    use jackioh_engine::{ParamTunedOn, PlayerId};

    fn writes(text: &str, key: &str) -> bool {
        param_placeholders(text)
            .iter()
            .any(|placeholder| placeholder.key == key)
    }

    #[test]
    fn r1431_every_number_only_the_base_face_prints_is_tuned_there_alone_and_reads_printed_on_the_radiant_face()
     {
        let mut base_only: Vec<String> = Vec::new();
        for card in entries() {
            for param in card.params.iter().flatten() {
                let alone = writes(&card.base.text, &param.key) && !writes(&card.radiant.text, &param.key);
                assert_eq!(
                    alone,
                    param.tuned_on == Some(ParamTunedOn::Base),
                    "{} {}: tuned on the base face only exactly when the base face alone prints it",
                    card.id,
                    param.key
                );
                if !alone {
                    continue;
                }
                base_only.push(format!("{} {}", card.id, param.key));
                // #98's Life Tap damage is Life Tap's (R1430): its own test rolls the power first.
                if param.power.is_some() {
                    continue;
                }
                let printed = param.radiant;
                let mut s = super::super::scenario(json!({
                    "p1": { "hand": [{ "def": card.id, "radiant": true }, { "def": card.id }] }
                }));
                let [shining, plain] = [0, 1].map(|at| s.hand(PlayerId::P1)[at].id.clone());
                for direction in [TuneDirection::Upgrade, TuneDirection::Degrade] {
                    let reached = steppable_params(s.state(), s.card(shining.as_str()), direction);
                    assert!(
                        !reached.iter().any(|entry| entry.param.key == param.key),
                        "{} {}: drawn on the Radiant face",
                        card.id,
                        param.key
                    );
                }
                let id = format!("param:{}", param.key);
                assert!(
                    !numbers_on(s.state(), s.card(shining.as_str()))
                        .iter()
                        .any(|entry| entry.id == id)
                );
                assert!(
                    numbers_on(s.state(), s.card(plain.as_str()))
                        .iter()
                        .any(|entry| entry.id == id)
                );
                for card_id in [&shining, &plain] {
                    step_param(s.card_mut(card_id.as_str()), &param.key, 1);
                }
                let read = |s: &jackioh_engine::testkit::Scenario, at: &str| {
                    param_value(s.state(), Some(s.card(at)), &param.key, Default::default())
                };
                assert_eq!(read(&s, &shining), printed, "{} {}", card.id, param.key);
                assert_ne!(read(&s, &plain), param.base, "{} {}", card.id, param.key);
            }
        }
        // The thirteen numbers patch v0.3.3 marked (issue #556) and issue #1748's two (#21's discards and
        // #42's chance, R1748003), and the Meditative set's own, which §8.8 tunes "on the base face
        // only": M #14's draws, M #37's surcharge (R1748003), M #49.1's picks (#529) and M #76's marks.
        // The Meditative+ set, held until it ships (R1178000), joins batch by batch (M+ #4's
        // the shipped sets'. So is the held Stinkyscape set's (R1857000), until its release re-pins them.
        // The held Training Grounds set's are too (R2194000), its snapshots' among them (R2195003).
        base_only.retain(|entry| {
            !entry.starts_with("meditativeplus-")
                && !entry.starts_with("stinkyscape-")
                && !entry.starts_with("traininggrounds-")
        });
        let (meditative, shipped): (Vec<String>, Vec<String>) = base_only
            .into_iter()
            .partition(|entry| entry.starts_with("meditative-"));
        // #2657's Bear Honeypot `tokens`, the base face's "Summon 2" (its Radiant face fills the board).
        assert_eq!(shipped.len(), 16, "{shipped:#?}");
        assert_eq!(
            meditative,
            vec![
                "meditative-014 draws".to_string(),
                "meditative-037 surcharge".to_string(),
                "meditative-049-1 picks".to_string(),
                "meditative-076 marks".to_string()
            ]
        );
    }

    #[test]
    fn r824000_every_number_only_the_radiant_face_prints_is_tuned_there_alone_and_reads_printed_on_the_base_face()
     {
        let mut radiant_only: Vec<String> = Vec::new();
        for card in entries() {
            for param in card.params.iter().flatten() {
                let alone = writes(&card.radiant.text, &param.key) && !writes(&card.base.text, &param.key);
                assert_eq!(
                    alone,
                    param.tuned_on == Some(ParamTunedOn::Radiant),
                    "{} {}: tuned on the Radiant face only exactly when the Radiant face alone prints it",
                    card.id,
                    param.key
                );
                if !alone {
                    continue;
                }
                radiant_only.push(format!("{} {}", card.id, param.key));
                let mut s = super::super::scenario(json!({
                    "p1": { "hand": [{ "def": card.id, "radiant": true }, { "def": card.id }] }
                }));
                let [shining, plain] = [0, 1].map(|at| s.hand(PlayerId::P1)[at].id.clone());
                for direction in [TuneDirection::Upgrade, TuneDirection::Degrade] {
                    let reached = steppable_params(s.state(), s.card(plain.as_str()), direction);
                    assert!(
                        !reached.iter().any(|entry| entry.param.key == param.key),
                        "{} {}: drawn on the base face",
                        card.id,
                        param.key
                    );
                }
                let id = format!("param:{}", param.key);
                assert!(
                    !numbers_on(s.state(), s.card(plain.as_str()))
                        .iter()
                        .any(|entry| entry.id == id)
                );
                assert!(
                    numbers_on(s.state(), s.card(shining.as_str()))
                        .iter()
                        .any(|entry| entry.id == id)
                );
                for card_id in [&shining, &plain] {
                    step_param(s.card_mut(card_id.as_str()), &param.key, 1);
                }
                let read = |s: &jackioh_engine::testkit::Scenario, at: &str| {
                    param_value(s.state(), Some(s.card(at)), &param.key, Default::default())
                };
                assert_eq!(read(&s, &plain), param.base, "{} {}", card.id, param.key);
                assert_ne!(read(&s, &shining), param.radiant, "{} {}", card.id, param.key);
            }
        }
        // Core, Classic and Classic+: the thirty-one numbers issue #824 marked and the thirteen marked
        // before (C #46's and C #54's among them, R749), less C #59's tokens, which both faces print
        // since patch v0.3.10, and C #75's draw, which both faces print since #2020; and the
        // with M #37's and M #38's `setCost` (R1748003).
        let meditative = radiant_only
            .iter()
            .filter(|entry| entry.starts_with("meditative-"))
            .count();
        // The Meditative+ set, held until it ships (R1178000), joins the count batch by batch; each
        // does the Stinkyscape set (R1857000), and the held Training Grounds set (R2194000).
        let held = radiant_only
            .iter()
            .filter(|entry| {
                entry.starts_with("meditativeplus-")
                    || entry.starts_with("stinkyscape-")
                    || entry.starts_with("traininggrounds-")
            })
            .count();
        // #2657's Anti-oneshot Armor `armor`, its Radiant face's "Your hero has Armor 1", and less
        // M #49.1's `radiants`, gone with its start-of-turn text.
        assert_eq!(
            (radiant_only.len() - meditative - held, meditative),
            (43, 23),
            "{radiant_only:#?}"
        );
    }

    #[test]
    fn r1430_every_number_that_belongs_to_a_power_names_an_activate_ability_its_card_declares_on_both_faces()
    {
        jackioh_cards::register_all();
        let scripts = jackioh_cards::scripts_of();
        let mut owned: Vec<String> = Vec::new();
        for card in entries() {
            for param in card.params.iter().flatten() {
                let Some(power) = param.power.as_deref() else {
                    continue;
                };
                owned.push(format!("{} {}", card.id, param.key));
                let script = scripts
                    .get(&card.id)
                    .unwrap_or_else(|| panic!("{} has a script", card.id));
                for face in [&script.base, &script.radiant] {
                    assert!(
                        face.activations.iter().any(|decl| decl.id == power),
                        "{} {}: no Activate ability \"{power}\"",
                        card.id,
                        param.key
                    );
                }
            }
        }
        // Only #98 Heroic Power's numbers belong to a power, all nine of them.
        assert_eq!(owned.len(), 9, "{owned:#?}");
        assert!(
            owned.iter().all(|entry| entry.starts_with("core-098 ")),
            "{owned:#?}"
        );
    }
}

/// R1748003 (issue #1748): every number a Spell or a Field Spell prints is a declared number, written
mod r1748003_the_numbers_a_spell_or_field_spell_prints {
    use super::*;
    use jackioh_engine::CardType;

    /// A numbered keyword's count (Lucky, Spell Damage, Echo, …), which the X row tunes (R482).
    const KEYWORD: &str = "a numbered keyword's count, which the X row tunes";
    /// cap on a cost, which the cost row and R65 own.
    const PRICE: &str = "a price a filter, a threshold, a dearer play or a cap names";
    const ENGINE: &str = "a table the engine holds, not one number the card declares";
    const FORMULA: &str = "a term of a formula, which no step moves as one number";
    const STRUCTURE: &str = "what the card's text is built on, not an amount";
    const TARGETS: &str = "how many cards the play declares";
    /// (Buffs on your Unit, a destroy on an enemy's, R902).
    const EITHER_WAY: &str = "a number whose better way hangs on the target";
    /// An AI generated card's printed number: B8 gives them no Numbers line (R482).
    const AI_PRINTED: &str = "an AI generated card's printed number (R482)";
    /// An Immutable card's number: neither a Buff nor a Nerf reaches it (R386).
    const IMMUTABLE: &str = "an Immutable card's number, which nothing tunes";
    const CARD_NAME: &str = "a digit in a card name the text names";

    const FIXED: &[(&str, &str)] = &[
        ("core-023", KEYWORD),
        // R2195003: the snapshots of Core #23 and #26, TG #49 and #50.
        ("traininggrounds-049", KEYWORD),
        ("traininggrounds-050", TARGETS),
        ("classic-023", KEYWORD),
        ("classic-057", KEYWORD),
        ("classicplus-025", KEYWORD),
        ("classicplus-036-1", KEYWORD),
        ("classicplus-047", KEYWORD),
        ("classicplus-061", KEYWORD),
        ("classicplus-065", KEYWORD),
        ("classicplus-066", KEYWORD),
        ("meditative-036", KEYWORD),
        ("meditative-036", EITHER_WAY),
        ("meditativeplus-006", KEYWORD),
        ("stinkyscape-004-2", KEYWORD),
        ("stinkyscape-004-3", KEYWORD),
        ("stinkyscape-009", KEYWORD),
        ("stinkyscape-011-2", KEYWORD),
        ("stinkyscape-013", KEYWORD),
        ("stinkyscape-023", KEYWORD),
        ("stinkyscape-024", KEYWORD),
        ("stinkyscape-039", KEYWORD),
        ("stinkyscape-047", KEYWORD),
        ("stinkyscape-145", KEYWORD),
        ("stinkyscape-155", KEYWORD),
        ("stinkyscape-157", KEYWORD),
        ("stinkyscape-t-korrelite", KEYWORD),
        ("stinkyscape-t-axnit", KEYWORD),
        ("traininggrounds-010", KEYWORD),
        ("core-042", PRICE),
        ("core-046", PRICE),
        ("core-051", PRICE),
        ("core-059", PRICE),
        ("core-083", PRICE),
        ("core-084", PRICE),
        ("core-087", PRICE),
        ("core-094", PRICE),
        ("classic-004", PRICE),
        ("classic-032", PRICE),
        ("classic-035", PRICE),
        ("classicplus-036", PRICE),
        ("classicplus-042-1", PRICE),
        ("classicplus-043", PRICE),
        ("meditative-004", PRICE),
        ("meditative-021", PRICE),
        ("meditative-037", PRICE),
        ("meditative-046", PRICE),
        ("meditative-081", PRICE),
        ("meditativeplus-003", PRICE),
        ("meditativeplus-004", PRICE),
        ("meditativeplus-005", PRICE),
        ("meditativeplus-009", PRICE),
        ("meditativeplus-019", PRICE),
        ("meditativeplus-023", PRICE),
        ("stinkyscape-004", PRICE),
        ("stinkyscape-012-3", PRICE),
        ("stinkyscape-149", PRICE),
        ("core-038", ENGINE),
        ("core-093", ENGINE),
        ("core-095", ENGINE),
        ("core-098", ENGINE),
        ("core-099", ENGINE),
        ("classicplus-043", ENGINE),
        ("classicplus-062", ENGINE),
        ("classicplus-073", ENGINE),
        ("traininggrounds-039", ENGINE),
        ("meditative-032", ENGINE),
        ("meditative-039-1", ENGINE),
        ("meditative-095", ENGINE),
        ("stinkyscape-148", ENGINE),
        ("stinkyscape-150", ENGINE),
        ("stinkyscape-151", ENGINE),
        ("stinkyscape-153", ENGINE),
        ("stinkyscape-166", ENGINE),
        ("stinkyscape-167", ENGINE),
        ("core-024", FORMULA),
        ("core-031", FORMULA),
        ("core-070", FORMULA),
        ("classicplus-053", FORMULA),
        ("core-082", STRUCTURE),
        ("classic-074", STRUCTURE),
        ("classicplus-019", STRUCTURE),
        ("classicplus-024", STRUCTURE),
        ("classicplus-035", STRUCTURE),
        ("classicplus-041", STRUCTURE),
        ("meditative-008", STRUCTURE),
        ("meditativeplus-020", STRUCTURE),
        ("meditativeplus-025", STRUCTURE),
        ("stinkyscape-007", STRUCTURE),
        ("stinkyscape-016", STRUCTURE),
        ("stinkyscape-046", STRUCTURE),
        ("stinkyscape-102", STRUCTURE),
        ("stinkyscape-112", STRUCTURE),
        ("stinkyscape-112-1", STRUCTURE),
        ("stinkyscape-118", STRUCTURE),
        ("stinkyscape-154", STRUCTURE),
        ("core-026", TARGETS),
        ("classic-029", EITHER_WAY),
        ("classic-064", EITHER_WAY),
        ("traininggrounds-034", EITHER_WAY),
        ("classicplus-t-ai-03", AI_PRINTED),
        ("classicplus-t-ai-04", AI_PRINTED),
        ("classicplus-t-ai-05", AI_PRINTED),
        ("classicplus-t-ai-06", AI_PRINTED),
        ("classicplus-t-ai-07", AI_PRINTED),
        ("classicplus-t-ai-10", AI_PRINTED),
        ("stinkyscape-t-ai-03", AI_PRINTED),
        ("stinkyscape-t-ai-05", AI_PRINTED),
        ("stinkyscape-t-ai-06", AI_PRINTED),
        ("meditative-080", IMMUTABLE),
        ("stinkyscape-095", CARD_NAME),
        ("stinkyscape-159", CARD_NAME),
        ("stinkyscape-160", CARD_NAME),
    ];

    fn literal(text: &str) -> String {
        let mut out = String::new();
        let mut depth = 0usize;
        for ch in text.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ if depth == 0 => out.push(ch),
                _ => {}
            }
        }
        out
    }

    #[test]
    fn r1748003_every_number_a_spell_or_field_spell_prints_is_declared_or_fixed() {
        jackioh_cards::register_all();
        let fixed: IndexSet<&str> = FIXED.iter().map(|(id, _)| *id).collect();
        let mut printing: IndexSet<String> = IndexSet::new();
        let mut unexplained: Vec<String> = Vec::new();
        for card in entries() {
            if !matches!(card.type_, CardType::Spell | CardType::FieldSpell) {
                continue;
            }
            let prints = [&card.base.text, &card.radiant.text]
                .iter()
                .any(|text| literal(text).bytes().any(|byte| byte.is_ascii_digit()));
            if !prints {
                continue;
            }
            printing.insert(card.id.clone());
            if !fixed.contains(card.id.as_str()) {
                unexplained.push(format!("{} {}", card.id, card.name));
            }
        }
        assert_eq!(
            unexplained,
            Vec::<String>::new(),
            "declare it as a param, or name why it is fixed"
        );
        let stale: Vec<&str> = fixed
            .iter()
            .copied()
            .filter(|id| !printing.contains(*id))
            .collect();
        assert_eq!(
            stale,
            Vec::<&str>::new(),
            "a card here that prints no number any more"
        );
        assert!(FIXED.iter().all(|(_, reason)| !reason.is_empty()));
        let reasons = [
            KEYWORD, PRICE, ENGINE, FORMULA, STRUCTURE, TARGETS, EITHER_WAY, AI_PRINTED, IMMUTABLE, CARD_NAME,
        ];
        let distinct: IndexSet<&str> = reasons.iter().copied().collect();
        assert_eq!(distinct.len(), reasons.len());
        assert!(FIXED.iter().all(|(_, reason)| distinct.contains(reason)));
    }
}
