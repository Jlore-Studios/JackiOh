//! ME-CN, R1303: the Chinese tables against the catalog. `chinese.json` holds every catalog card's and
//! token's name and both faces in Simplified Chinese, and for a card whose script declares `preview`
//! (R280) its preview labels, as `crates/tools/tests/export_card_text.rs` writes them from each card's
//! `zh` in `crates/cards/card-text.yaml`; `chinese-terms.json` holds the frame's words. The client
//! draws a Chinese card from them (`apps/web/src/cards/chinese.ts`, R1301). Like `flavour.json` (R660)
//! they are presentation, not card data: no rule reads them, and an edit to them is no patch.
//!
//! R1302: every face of the table meets the Chinese house style (`card_text::cjk_failures`), read with
//! its params filled in, as a player reads it.

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, chinese_json, scripts_of};
use jackioh_engine::testkit::*;
use jackioh_engine::{FaceKind, Rarity, Tag, fill_params, param_placeholders};
use serde_json::{Map, Value};

use super::card_text::{LABELS, chinese_keyword_label, chinese_terms, cjk_failures};
use super::sidecar;

/// The fields an entry may carry.
const FIELDS: &[&str] = &["name", "base", "radiant", "previews"];

const FACES: [FaceKind; 2] = [FaceKind::Base, FaceKind::Radiant];

const HEARTHSTONE_WORDS: &[(&str, &str)] = &[
    ("Cry", "战吼"),
    ("Death", "亡语"),
    ("Taunt", "嘲讽"),
    ("Rush", "突袭"),
    ("Charge", "冲锋"),
    ("Divine Shield", "圣盾"),
    ("Lifesteal", "吸血"),
    ("Reborn", "复生"),
    ("Poisonous", "剧毒"),
    ("Windfury", "风怒"),
    ("Discover", "发现"),
    ("Cast on draw", "抽到时施放"),
    ("Spell Damage", "法术伤害"),
    ("Echo", "回响"),
    ("The Coin", "幸运币"),
];

/// The set's own words (the same list): the tuning verbs print as Buff and Nerf since patch v0.3.4
/// (R1320; Upgrade and Degrade before it, R386), and the library is the deck (R366).
const SET_WORDS: &[(&str, &str)] = &[
    ("Radiant", "光辉"),
    ("Tribute", "献祭"),
    ("deck", "牌库"),
    ("Buff", "强化"),
    ("Nerf", "削弱"),
    ("Lock", "锁定"),
    ("Plague Counter", "瘟疫指示物"),
    ("Brittle", "易碎"),
    ("Animated", "活化"),
    ("Token", "衍生物"),
    ("CN", "中国"),
    ("Felinor", "猫族"),
    ("Human", "人类"),
];

fn table() -> IndexMap<String, Map<String, Value>> {
    sidecar(chinese_json(), "crates/cards/chinese.json")
}

fn text<'a>(entry: &'a Map<String, Value>, field: &str) -> &'a str {
    entry.get(field).and_then(Value::as_str).unwrap_or("")
}

fn face_field(face: FaceKind) -> &'static str {
    match face {
        FaceKind::Base => "base",
        FaceKind::Radiant => "radiant",
    }
}

fn with_texts(card: &CardDef, base: &str, radiant: &str) -> CardDef {
    let mut def = card.clone();
    def.base.text = base.to_string();
    def.radiant.text = radiant.to_string();
    def
}

fn chinese_face(card: &CardDef, entry: &Map<String, Value>, face: FaceKind) -> String {
    fill_params(
        &with_texts(card, text(entry, "base"), text(entry, "radiant")),
        face,
        None,
    )
}

fn filled(card: &CardDef, template: &str, face: FaceKind) -> String {
    fill_params(&with_texts(card, template, template), face, None)
}

fn keys_of(text: &str) -> (IndexSet<String>, bool) {
    let placeholders = param_placeholders(text);
    let agreeing = placeholders.iter().any(|p| p.one.is_some() || p.many.is_some());
    let mut keys: IndexSet<String> = placeholders.into_iter().map(|p| p.key).collect();
    keys.sort();
    (keys, agreeing)
}

/// `phrase` stands in `text` as whole words: the characters either side are not ASCII letters, digits,
/// an apostrophe or a hyphen (R279's rule for a name).
fn names_word(text: &str, phrase: &str) -> bool {
    let edge = |c: Option<char>| c.is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '\'' || c == '-'));
    text.match_indices(phrase)
        .any(|(at, _)| edge(text[..at].chars().next_back()) && edge(text[at + phrase.len()..].chars().next()))
}

/// R1303, R279: `text` with the names of the cards its card names (`refs`) blanked, each as R279 links
/// it (the name, or the name before a closing parenthesis, as whole words, a plural "s" with it), so a
/// term inside a name ("Shadow Word Death", the Stinkyscape set's S #12 token, holds "Death") asks
/// its face for no Chinese term: such a name is held to its Chinese name instead
/// (`r1303_each_card_a_text_names_appears_by_its_chinese_name`).
fn without_named_cards(card: &CardDef, text: &str) -> String {
    let mut names: Vec<&str> = card
        .refs
        .iter()
        .flatten()
        .filter_map(|id| CATALOG.get(id.as_str()))
        .flat_map(|named| [named.name.as_str(), named.name.split(" (").next().unwrap_or("")])
        .filter(|name| !name.is_empty())
        .collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let edge = |c: Option<char>| c.is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '\'' || c == '-'));
    let mut out = text.to_string();
    for name in names {
        let mut from = 0;
        while let Some(offset) = out[from..].find(name) {
            let at = from + offset;
            let mut end = at + name.len();
            if out[end..].starts_with('s') && edge(out[end + 1..].chars().next()) {
                end += 1;
            }
            if edge(out[..at].chars().next_back()) && edge(out[end..].chars().next()) {
                out.replace_range(at..end, &" ".repeat(end - at));
            }
            from = end;
        }
    }
    out
}

/// R1303: the Hearthstone and set terms `english` says as whole words that `chinese` lacks, the names
/// of the cards `card` names read as names (`without_named_cards`).
fn missing_terms(card: &CardDef, english: &str, chinese: &str) -> Vec<(&'static str, &'static str)> {
    let english = without_named_cards(card, english);
    HEARTHSTONE_WORDS
        .iter()
        .chain(SET_WORDS)
        .filter(|(term, word)| names_word(&english, term) && !chinese.contains(word))
        .copied()
        .collect()
}

/// A name with a closing edition cut off: "混沌召唤（核心版）" is named as "混沌召唤", as R279 names
/// "Call to Chaos (Core Edition)" as "Call to Chaos".
fn without_edition(name: &str) -> &str {
    for (open, close) in [('（', '）'), ('(', ')')] {
        if name.ends_with(close)
            && let Some(at) = name.rfind(open)
        {
            return name[..at].trim_end();
        }
    }
    name
}

/// Every preview label a previewed card's view carries, by face: in its owner's hand and, for a
/// permanent, face up in play (Combo-Index's grade shows on the field only, R372).
fn preview_labels(id: &str) -> Vec<(FaceKind, String)> {
    let card = &CATALOG[id];
    let mut out: Vec<(FaceKind, String)> = Vec::new();
    for face in FACES {
        let radiant = face == FaceKind::Radiant;
        let libraries = json!(["core-019", "core-019", "core-019"]);
        let mut views: Vec<Value> = Vec::new();
        let s = super::scenario(json!({
            "p1": { "hand": [{ "def": id, "radiant": radiant }, "core-010"], "library": libraries },
            "p2": { "library": libraries, "field": ["core-019"] },
        }));
        let view = serde_json::to_value(s.view(PlayerId::P1)).expect("a view serializes");
        views.extend(view["you"]["hand"].as_array().cloned().unwrap_or_default());
        if card.type_ != CardType::Spell {
            let row = if card.type_ == CardType::Unit {
                "field"
            } else {
                "backrow"
            };
            let s = super::scenario(json!({
                "p1": { row: [{ "def": id, "radiant": radiant, "faceUp": true }], "library": libraries },
                "p2": { "library": libraries, "field": ["core-019"] },
            }));
            let view = serde_json::to_value(s.view(PlayerId::P1)).expect("a view serializes");
            for pile in ["units", "backrow"] {
                views.extend(view["you"][pile].as_array().cloned().unwrap_or_default());
            }
        }
        for shown in views {
            for entry in shown["preview"].as_array().cloned().unwrap_or_default() {
                let label = entry["label"].as_str().unwrap_or("").to_string();
                if !out.contains(&(face, label.clone())) {
                    out.push((face, label));
                }
            }
        }
    }
    out
}

mod r1303_the_chinese_tables {
    use super::*;

    #[test]
    fn r1303_every_key_is_a_catalog_entry_and_every_entry_has_a_key() {
        let sidecar = table();
        let strangers: Vec<&String> = sidecar
            .keys()
            .filter(|id| !CATALOG.contains_key(id.as_str()))
            .collect();
        assert_eq!(strangers, Vec::<&String>::new());
        let without: Vec<&String> = CATALOG
            .keys()
            .filter(|id| !sidecar.contains_key(id.as_str()))
            .collect();
        assert_eq!(without, Vec::<&String>::new());
        assert!(sidecar.keys().eq(CATALOG.keys()));
    }

    #[test]
    fn r1303_an_entry_is_a_name_and_two_faces_with_previews_exactly_where_the_script_declares_preview() {
        jackioh_cards::register_all();
        let previewed: Vec<String> = scripts_of()
            .into_iter()
            .filter(|(_, scripts)| scripts.base.preview.is_some() || scripts.radiant.preview.is_some())
            .map(|(id, _)| id)
            .collect();
        let mut problems: Vec<String> = Vec::new();
        for (id, entry) in table() {
            let Some(card) = CATALOG.get(id.as_str()) else {
                continue;
            };
            for field in entry.keys() {
                if !FIELDS.contains(&field.as_str()) {
                    problems.push(format!("{id}: unknown field \"{field}\""));
                }
            }
            for field in ["name", "base", "radiant"] {
                match entry.get(field).and_then(Value::as_str) {
                    None => problems.push(format!("{id}: {field} is not a string")),
                    Some(value) if value.trim() != value => {
                        problems.push(format!("{id}: {field} has space at an end"))
                    }
                    Some(_) => {}
                }
            }
            if text(&entry, "name").is_empty() {
                problems.push(format!("{id}: the name is empty"));
            }
            for face in FACES {
                let english = &card.face(face).text;
                let chinese = text(&entry, face_field(face));
                if english.is_empty() != chinese.is_empty() {
                    problems.push(format!("{id} {face}: empty on one side only"));
                }
                if english.split('\n').count() != chinese.split('\n').count() {
                    problems.push(format!("{id} {face}: not one Chinese line per English line"));
                }
            }
            let previews = entry.get("previews");
            if previews.is_some() != previewed.contains(&id) {
                problems.push(format!(
                    "{id}: previews {:?}, preview declared {}",
                    previews.is_some(),
                    previewed.contains(&id)
                ));
            }
            if let Some(previews) = previews {
                match previews.as_object() {
                    None => problems.push(format!("{id}: previews is not an object")),
                    Some(previews) => {
                        for (label, chinese) in previews {
                            if !chinese
                                .as_str()
                                .is_some_and(|value| !value.is_empty() && value.trim() == value)
                            {
                                problems
                                    .push(format!("{id}: the preview \"{label}\" is not a trimmed string"));
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }

    /// R280's labels in Chinese. A key is the English label written with the face's `{key}`s where it
    /// prints a number; its value is the Chinese label, written the same way. The client fills both with
    /// the numbers the face is filled with, so a tuned number (R386) moves in the Chinese label too.
    #[test]
    fn r1303_every_preview_label_a_view_carries_has_a_chinese_label_in_the_chinese_face() {
        jackioh_cards::register_all();
        let sidecar = table();
        let mut problems: Vec<String> = Vec::new();
        for (id, entry) in &sidecar {
            let Some(previews) = entry.get("previews").and_then(Value::as_object) else {
                continue;
            };
            let card = &CATALOG[id.as_str()];
            let mut unused: IndexSet<&String> = previews.keys().collect();
            for (face, label) in preview_labels(id) {
                let found = previews.iter().find(|(key, _)| filled(card, key, face) == label);
                let Some((key, chinese)) = found else {
                    problems.push(format!("{id} {face}: no Chinese for the label \"{label}\""));
                    continue;
                };
                unused.shift_remove(key);
                let chinese = filled(card, chinese.as_str().unwrap_or(""), face);
                if !chinese_face(card, entry, face).contains(&chinese) {
                    problems.push(format!("{id} {face}: \"{chinese}\" is not in the Chinese face"));
                }
            }
            // R2476000: a label shown only in a state the plain views above never reach (Classic+ #34's
            // chosen line, an Asteroid's pool rider) is held to the same two texts on some face: its
            // English in the English face, its Chinese in that face's Chinese.
            for key in unused {
                let chinese = previews[key.as_str()].as_str().unwrap_or("");
                let faithful = FACES.iter().any(|&face| {
                    card.face(face).text.contains(&filled(card, key, face))
                        && chinese_face(card, entry, face).contains(&filled(card, chinese, face))
                });
                if !faithful {
                    problems.push(format!("{id}: no view carries the label \"{key}\""));
                }
            }
            // A Chinese label may print a number its English leaves out (C+ #74's "每打出{plays}张牌" for
            // "your opponent plays"), but only one the card declares, so the client can fill it.
            let declared: Vec<&str> = card
                .params
                .iter()
                .flatten()
                .map(|param| param.key.as_str())
                .collect();
            for (key, chinese) in previews {
                let written = keys_of(key)
                    .0
                    .into_iter()
                    .chain(keys_of(chinese.as_str().unwrap_or("")).0);
                for written in written.filter(|written| !declared.contains(&written.as_str())) {
                    problems.push(format!(
                        "{id}: \"{key}\" writes {{{written}}}, which the card does not declare"
                    ));
                }
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn r1303_each_face_writes_exactly_its_english_faces_keys_and_no_agreeing_form() {
        let mut problems: Vec<String> = Vec::new();
        for (id, entry) in table() {
            let Some(card) = CATALOG.get(id.as_str()) else {
                continue;
            };
            for face in FACES {
                let (english, _) = keys_of(&card.face(face).text);
                let (chinese, agreeing) = keys_of(text(&entry, face_field(face)));
                if english != chinese {
                    problems.push(format!(
                        "{id} {face}: writes {chinese:?}, its English {english:?}"
                    ));
                }
                if agreeing {
                    problems.push(format!("{id} {face}: writes an agreeing form"));
                }
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }

    /// R1303, R2511000: no two cards share a Chinese name; a Legacy card (Training Grounds #41–#60)
    /// is its Core card's name after the Legacy tag's word and a full-width colon.
    #[test]
    fn r1303_r2511000_names_are_unique() {
        let mut seen: IndexMap<String, IndexSet<String>> = IndexMap::new();
        for (id, entry) in table() {
            seen.entry(text(&entry, "name").to_string())
                .or_default()
                .insert(id);
        }
        let shared: Vec<(String, IndexSet<String>)> =
            seen.into_iter().filter(|(_, ids)| ids.len() > 1).collect();
        assert_eq!(shared, Vec::<(String, IndexSet<String>)>::new());
        assert_eq!(
            text(&table()["traininggrounds-041"], "name"),
            format!(
                "{}：{}",
                chinese_terms()["tags"]["Legacy"].as_str().unwrap(),
                text(&table()["core-002"], "name")
            )
        );
    }

    #[test]
    fn r1303_hearthstone_s_terms_are_used_where_it_has_them() {
        let terms = chinese_terms();
        for (path, word) in [
            (["keywords", "Taunt"], "嘲讽"),
            (["keywords", "Rush"], "突袭"),
            (["keywords", "Charge"], "冲锋"),
            (["keywords", "Divine Shield"], "圣盾"),
            (["keywords", "Lifesteal"], "吸血"),
            (["keywords", "Reborn"], "复生"),
            (["keywords", "Poisonous"], "剧毒"),
            (["keywords", "Windfury"], "风怒"),
            (["keywords", "Spell Damage"], "法术伤害"),
            (["labels", "Cry:"], "战吼："),
            (["labels", "Death:"], "亡语："),
            (["labels", "Cast on draw:"], "抽到时施放："),
        ] {
            assert_eq!(terms[path[0]][path[1]], word, "{path:?}");
        }
        for (term, word) in [("Discover", "发现"), ("Echo", "回响")] {
            assert_eq!(terms["glossary"][term]["label"], word, "{term}");
        }
        let sidecar = table();
        let coin = &sidecar["core-t-coin"];
        assert_eq!(CATALOG["core-t-coin"].name, "The Coin");
        assert_eq!(text(coin, "name"), "幸运币");
        let mut problems: Vec<String> = Vec::new();
        for (id, entry) in &sidecar {
            let Some(card) = CATALOG.get(id.as_str()) else {
                continue;
            };
            for face in FACES {
                let english = &card.face(face).text;
                let chinese = text(entry, face_field(face));
                for (term, word) in missing_terms(card, english, chinese) {
                    problems.push(format!("{id} {face}: says {term} but not {word}"));
                }
                for keyword in &card.face(face).keywords {
                    let word = chinese_keyword_label(keyword, &terms);
                    if !chinese.contains(&word) {
                        problems.push(format!("{id} {face}: prints {keyword:?} but not {word}"));
                    }
                }
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn r1303_the_terms_name_every_type_tag_rarity_keyword_and_label() {
        let terms = chinese_terms();
        let named = |group: &str| -> Vec<String> {
            let mut names: Vec<String> = terms[group]
                .as_object()
                .map(|words| words.keys().cloned().collect())
                .unwrap_or_default();
            names.sort();
            names
        };
        let all = |names: Vec<&str>| -> Vec<String> {
            let mut names: Vec<String> = names.into_iter().map(str::to_string).collect();
            names.sort();
            names
        };
        assert_eq!(
            named("types"),
            all(CardType::ALL.iter().map(|t| t.as_str()).collect())
        );
        assert_eq!(named("tags"), all(Tag::ALL.iter().map(|t| t.as_str()).collect()));
        assert_eq!(
            named("rarities"),
            all(Rarity::ALL.iter().map(|r| r.as_str()).collect())
        );
        let live: Vec<KeywordKind> = KeywordKind::ALL
            .iter()
            .copied()
            .filter(|kind| !KeywordKind::RETIRED.contains(kind))
            .collect();
        assert_eq!(named("keywords"), all(live.iter().map(|k| k.as_str()).collect()));
        assert_eq!(named("labels"), all(LABELS.to_vec()));
        let mut problems: Vec<String> = Vec::new();
        let plain = |value: &Value| {
            value
                .as_str()
                .is_some_and(|word| !word.is_empty() && word.trim() == word)
        };
        for group in ["types", "tags", "rarities", "keywords", "labels"] {
            for (english, word) in terms[group].as_object().into_iter().flatten() {
                if !plain(word) {
                    problems.push(format!("{group} {english}"));
                }
            }
        }
        for word in ["radiant", "created", "allTribes"] {
            if !plain(&terms[word]) {
                problems.push(word.to_string());
            }
        }
        for (term, entry) in terms["glossary"].as_object().into_iter().flatten() {
            if !plain(&entry["label"])
                || !plain(&entry["rule"])
                || entry.as_object().is_none_or(|e| e.len() != 2)
            {
                problems.push(format!("glossary {term}"));
            }
        }
        for (english, label) in terms["labels"].as_object().into_iter().flatten() {
            if !label.as_str().is_some_and(|label| label.ends_with('：')) || !english.ends_with(':') {
                problems.push(format!("the label {english} does not end with \"：\""));
            }
        }
        let unique: IndexSet<&str> = terms["keywords"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, w)| w.as_str())
            .collect();
        assert_eq!(unique.len(), live.len(), "two keywords share a word");
        assert_eq!(problems, Vec::<String>::new());
    }

    /// R1303, R279: a term inside the name of a card the face names asks for no Chinese term (the name is
    /// held to its Chinese name), and the same term outside a name still does.
    #[test]
    fn r1303_a_term_inside_a_name_the_face_names_asks_for_no_chinese_term_and_outside_one_it_does() {
        assert_eq!(CATALOG["core-t-rush"].name, "Rush Token");
        let mut naming = CATALOG["core-001"].clone();
        naming.refs = Some(vec!["core-t-rush".to_string()]);
        let mut plain = naming.clone();
        plain.refs = None;
        let chinese = "召唤一个冲锋兵。";
        let none: Vec<(&str, &str)> = Vec::new();
        assert_eq!(missing_terms(&naming, "Summon a Rush Token.", chinese), none);
        assert_eq!(
            missing_terms(&plain, "Summon a Rush Token.", chinese),
            [("Rush", "突袭"), ("Token", "衍生物")]
        );
        assert_eq!(
            missing_terms(&naming, "Summon a Rush Token. Give it Rush.", chinese),
            [("Rush", "突袭")]
        );
        assert!(!without_named_cards(&naming, "Summon two Rush Tokens.").contains("Rush"));
        assert_eq!(
            without_named_cards(&naming, "A Rush Tokenizer has Rush."),
            "A Rush Tokenizer has Rush."
        );
    }

    /// R279 in Chinese: a card the text names (`refs`) is named by its Chinese name, as the client marks
    /// it, in the faces whose English names it, and in one face at least.
    #[test]
    fn r1303_each_card_a_text_names_appears_by_its_chinese_name() {
        let sidecar = table();
        let mut problems: Vec<String> = Vec::new();
        for (id, card) in CATALOG.iter() {
            let entry = &sidecar[id.as_str()];
            for named in card.refs.iter().flatten() {
                let chinese = without_edition(text(&sidecar[named.as_str()], "name"));
                let english = CATALOG[named.as_str()].name.split(" (").next().unwrap_or("");
                let faces: Vec<FaceKind> = FACES
                    .into_iter()
                    .filter(|face| text(entry, face_field(*face)).contains(chinese))
                    .collect();
                if faces.is_empty() {
                    problems.push(format!("{id}: names {named} but no face says {chinese}"));
                }
                for face in FACES {
                    if card.face(face).text.contains(english) && !faces.contains(&face) {
                        problems.push(format!("{id} {face}: names {english} but not {chinese}"));
                    }
                }
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn r1302_every_chinese_face_meets_the_chinese_house_style() {
        let mut problems: Vec<String> = Vec::new();
        for (id, entry) in table() {
            let Some(card) = CATALOG.get(id.as_str()) else {
                continue;
            };
            for face in FACES {
                let chinese = chinese_face(card, &entry, face);
                for why in cjk_failures(&chinese, &card.face(face).keywords) {
                    problems.push(format!("{id} {face} {why}: {chinese}"));
                }
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }
}
