//! Writes `crates/cards/flavour.json`, `crates/cards/chinese.json` and
//! `apps/web/src/audio/voice-text.json` from `crates/cards/card-text.yaml` (#2498), the one file a
//! person edits for a card's words other than its rules: its flavour line and artist (R660), its voice
//! lines (R655, R1088, R1448), its Chinese (R1303) and the portraits' emote lines (R644). No pure crate
//! parses YAML (CLAUDE.md rule 4), so `jackioh-cards` compiles in the two JSON files beside it and the
//! client bundles all three. The export fails when a file differed from what the YAML makes, having
//! rewritten it: commit the result. Each file's `"//"` key says where it comes from; its readers skip it.

use indexmap::IndexMap;
use jackioh_cards::CATALOG;
use serde::{Deserialize, Serialize};

/// The file a person edits, relative to this crate.
const YAML: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/card-text.yaml");

/// R660: the flavour sidecar `jackioh-cards` compiles in and the client's `flavour.ts` reads.
const FLAVOUR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/flavour.json");

/// R1303: the Chinese sidecar, read the same way by the client's `chinese.ts`.
const CHINESE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/chinese.json");

/// R655, R644: the voice lines, which the client's `voiceData.ts` and `gen-voice.mjs` join to the
/// voices and effects of `card-audio.json5` beside it.
const VOICE_TEXT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/web/src/audio/voice-text.json"
);

const HEADER: &str = "Generated from crates/cards/card-text.yaml by crates/tools/tests/export_card_text.rs. Do not edit: change the YAML, then run cargo test -p jackioh-tools --test export_card_text.";

/// The YAML key of the portraits' section; every other top-level key is a catalog id.
const EMOTES: &str = "emotes";

/// One card's entry. A field the struct does not name is refused, so a misspelled one is never dropped.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardText {
    flavour: Option<String>,
    artist: Option<String>,
    voice: Option<Voice>,
    zh: Option<Chinese>,
}

/// A card's voice lines by hook, a trigger's a list (R1088).
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Voice {
    #[serde(skip_serializing_if = "Option::is_none")]
    play: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    death: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cast: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<Vec<String>>,
}

/// A card's name and faces in Chinese, and its preview labels' templates (R280).
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Chinese {
    name: String,
    base: String,
    radiant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    previews: Option<IndexMap<String, String>>,
}

/// A portrait's five emote lines (R644).
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Emote {
    greetings: String,
    well_played: String,
    oops: String,
    thanks: String,
    threaten: String,
}

/// One card's entry in `flavour.json`.
#[derive(Serialize)]
struct Flavour<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    flavour: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artist: Option<&'a String>,
}

/// `flavour.json` and `chinese.json`: the header, then one entry per card in the YAML's order.
#[derive(Serialize)]
struct Sidecar<'a, T> {
    #[serde(rename = "//")]
    header: &'a str,
    #[serde(flatten)]
    entries: IndexMap<&'a str, T>,
}

/// `voice-text.json`.
#[derive(Serialize)]
struct VoiceText<'a> {
    #[serde(rename = "//")]
    header: &'a str,
    cards: IndexMap<&'a str, &'a Voice>,
    emotes: &'a IndexMap<String, Emote>,
}

type Cards = IndexMap<String, CardText>;
type Emotes = IndexMap<String, Emote>;

/// The YAML's cards, by id in its order, and its portraits. A key written twice, a key that is not a
/// string or an entry of the wrong shape is an error naming the file and the key.
fn parse(text: &str) -> Result<(Cards, Emotes), String> {
    let root: yaml_serde::Mapping =
        yaml_serde::from_str(text).map_err(|error| format!("card-text.yaml: {error}"))?;
    let mut cards = Cards::new();
    let mut emotes = None;
    for (key, value) in root {
        let Some(id) = key.as_str() else {
            return Err(format!(
                "card-text.yaml: {key:?}: a key must be a card's id or {EMOTES}"
            ));
        };
        if id == EMOTES {
            emotes = Some(
                yaml_serde::from_value(value).map_err(|error| format!("card-text.yaml: {id}: {error}"))?,
            );
        } else {
            let entry =
                yaml_serde::from_value(value).map_err(|error| format!("card-text.yaml: {id}: {error}"))?;
            cards.insert(id.to_string(), entry);
        }
    }
    let emotes = emotes.ok_or_else(|| format!("card-text.yaml: no {EMOTES} section"))?;
    Ok((cards, emotes))
}

fn shipped() -> (Cards, Emotes) {
    let text = std::fs::read_to_string(YAML).unwrap_or_else(|error| panic!("{YAML}: {error}"));
    parse(&text).unwrap_or_else(|error| panic!("{error}"))
}

/// `value` as the written files hold it: pretty JSON with a final newline.
fn pretty(value: &impl Serialize) -> String {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|error| panic!("card-text.yaml: {error}"));
    format!("{text}\n")
}

fn render() -> [(&'static str, String); 3] {
    let (cards, emotes) = shipped();
    let flavour = Sidecar {
        header: HEADER,
        entries: cards
            .iter()
            .filter(|(_, card)| card.flavour.is_some() || card.artist.is_some())
            .map(|(id, card)| {
                let entry = Flavour {
                    flavour: card.flavour.as_ref(),
                    artist: card.artist.as_ref(),
                };
                (id.as_str(), entry)
            })
            .collect(),
    };
    let chinese = Sidecar {
        header: HEADER,
        entries: cards
            .iter()
            .filter_map(|(id, card)| Some((id.as_str(), card.zh.as_ref()?)))
            .collect(),
    };
    let voice_text = VoiceText {
        header: HEADER,
        cards: cards
            .iter()
            .filter_map(|(id, card)| Some((id.as_str(), card.voice.as_ref()?)))
            .collect(),
        emotes: &emotes,
    };
    [
        (FLAVOUR, pretty(&flavour)),
        (CHINESE, pretty(&chinese)),
        (VOICE_TEXT, pretty(&voice_text)),
    ]
}

#[test]
fn export_card_text() {
    let mut stale: Vec<&str> = Vec::new();
    for (path, text) in render() {
        let current = std::fs::read_to_string(path).unwrap_or_default();
        if current != text {
            std::fs::write(path, &text).unwrap_or_else(|error| panic!("{path}: {error}"));
            stale.push(path);
        }
    }
    assert!(
        stale.is_empty(),
        "{stale:?} differed from what crates/cards/card-text.yaml generates; this run rewrote them: commit the result"
    );
}

#[test]
fn r660_r1303_every_catalog_card_and_token_has_an_entry_in_catalog_order_with_its_name_in_a_comment() {
    let text = std::fs::read_to_string(YAML).unwrap_or_else(|error| panic!("{YAML}: {error}"));
    let tops: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with([' ', '#']))
        .collect();
    let emotes_line = format!("{EMOTES}:");
    assert_eq!(
        tops.last(),
        Some(&emotes_line.as_str()),
        "the {EMOTES} section comes last"
    );
    let mut wrong: Vec<String> = Vec::new();
    let mut ids: Vec<&str> = Vec::new();
    for line in &tops[..tops.len() - 1] {
        let Some((id, name)) = line.split_once(": # ") else {
            wrong.push(format!("{line:?}: not `<id>: # <the card's name>`"));
            continue;
        };
        ids.push(id);
        match CATALOG.get(id) {
            Some(card) if card.name == name => {}
            Some(card) => wrong.push(format!("{id}: comment {name:?}, catalog name {:?}", card.name)),
            None => wrong.push(format!("{id}: not a catalog card or token")),
        }
    }
    assert_eq!(wrong, Vec::<String>::new());
    let catalog: Vec<&str> = CATALOG.keys().map(String::as_str).collect();
    assert_eq!(
        ids, catalog,
        "one entry per catalog card and token, in catalog order"
    );
    let (cards, _) = shipped();
    assert_eq!(
        cards.keys().map(String::as_str).collect::<Vec<_>>(),
        ids,
        "the parsed ids are the commented ones"
    );
}

#[test]
fn a_misspelled_field_fails_naming_its_card() {
    let error = parse("core-001: # Big D-fender\n  flavor: x\nemotes: {}\n")
        .err()
        .unwrap_or_default();
    assert!(
        error.contains("core-001") && error.contains("unknown field `flavor`"),
        "{error}"
    );
    let twice = parse("core-001:\n  flavour: x\ncore-001:\n  flavour: y\nemotes: {}\n")
        .err()
        .unwrap_or_default();
    assert!(
        twice.contains("core-001"),
        "a key written twice is refused: {twice}"
    );
}
