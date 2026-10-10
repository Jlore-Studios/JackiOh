//! `cargo jackioh catalog check`, `cargo jackioh catalog loc <path>` and `cargo jackioh
//! catalog-version` (SURFACE §12).
//!
//! `catalog check` is the port of `packages/cards/scripts/validate-catalog.ts`: structural
//! validation of `crates/cards/catalog.json` against SPEC §5, §6.1, §7 and §8, and patch v0.2.0's
//! sets and fields (docs/classic-sets.md B2, B3.4, E36, E40).
//!
//! This checks the shape and the census of the catalog, set by set (Core, Classic, Classic+): that
//! it holds every card and every token exactly once, that every enum value is in the union the
//! engine's wire types declare, that stats sit on Units (and Animated backrow cards, B3.1) and
//! nowhere else, that the rarity distribution each set prints is the one the file carries, and that
//! the v0.2.0 fields (`printedRarity`, `params`, `loc`, a face's `type` and `xStats`) are well
//! formed. It does NOT re-read the §8 cells — `crates/cards/tests/cross/catalog.rs` does that with
//! an independent transcription. The pending-fragment proof is its own command, `patches check`
//! (TS's `pnpm validate:catalog` ran both).
//!
//! `catalog loc <path>` prints a card script's lines of code (SURFACE §7.5): `loc` is frozen
//! gameplay data in `catalog.json`, so nothing recomputes it; a card added after v0.3.0 takes its
//! number from here.
//!
//! `catalog-version` is the port of `scripts/catalog-version.mjs`: it prints the catalog version
//! this checkout ships, the newest entry of `crates/cards/patches/patches.json` (R388). The version
//! is read as data: the list's order is the order of versions, never a comparison of strings
//! (R105). It fails, printing nothing on stdout, when the list names no newest version.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use indexmap::{IndexMap, IndexSet};
use jackioh_engine::wire::{
    CardType, KEYWORD_KINDS, PrintedRarity, Rarity, SetName, Tag, param_placeholders, set_ships,
};

use crate::patches::js::{self, Json, Object};
use crate::patches::repo_root;
use crate::spec::is_ident_char;

/* ------------------------------------------------------------------------------------- unions */

// TS listed each union's literals and proved every list exhaustive with an `Exhaustive` alias that
// failed to compile when the union grew a member the list did not carry. The engine's `ALL` lists
// are the unions themselves, in their declaration order, so they are exhaustive by construction.

/// `CARD_TYPES`.
fn card_types() -> Vec<&'static str> {
    CardType::ALL.iter().map(|each| each.as_str()).collect()
}

/// `TAGS`.
fn tags() -> Vec<&'static str> {
    Tag::ALL.iter().map(|each| each.as_str()).collect()
}

/// `RARITIES`.
fn rarities() -> Vec<&'static str> {
    Rarity::ALL.iter().map(|each| each.as_str()).collect()
}

/// `SET_NAMES`.
fn set_names() -> Vec<&'static str> {
    SetName::ALL.iter().map(|each| each.as_str()).collect()
}

/// `KEYWORD_KINDS`, as the literals a catalog writes.
fn keyword_kinds() -> Vec<&'static str> {
    KEYWORD_KINDS.iter().map(|each| each.as_str()).collect()
}

/// §6.1, R385, E6: the keywords that carry a number; every other kind is bare.
const NUMBERED_KEYWORDS: &[&str] = &["Armor", "Lucky", "Brittle", "Spell Damage"];

/* ------------------------------------------------------------------------------- expectations */

/// The id segment of each set the catalog holds (B2.2: `classicplus` holds no hyphen).
struct SetExpectation {
    set: &'static str,
    segment: &'static str,
    /// Non-token cards, indexed 1..cards.
    cards: usize,
    /// Tokens a card defines, indexed N.k after card N.
    card_defined_tokens: Vec<String>,
    /// Tokens no one card defines, indexed T-name.
    shared_tokens: Vec<String>,
    rarities: &'static [(&'static str, usize)],
}

impl SetExpectation {
    /// R1420: whether the set ships. A set that does not is checked for its shape alone: every entry
    /// it holds must be one of its listed indices, once, and no rarity may pass its count, but none
    /// need be there yet, and none counts toward the totals or the tag census, which are the
    /// shipped catalog's.
    fn ships(&self) -> bool {
        SetName::ALL
            .iter()
            .find(|set| set.as_str() == self.set)
            .is_some_and(|set| set_ships(*set))
    }
}

fn range(prefix: &str, from: usize, to: usize) -> Vec<String> {
    (from..=to).map(|n| format!("{prefix}{n}")).collect()
}

fn texts(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// `SETS`.
fn sets() -> Vec<SetExpectation> {
    vec![
        // §8 + §7: the 100 Core indices, the 5 card-defined tokens, and the 6 named ones — the 4
        // tokens several cards share, The Coin, which §2.1's setup deals (R244), and the Ghoul Token,
        // a card of its own in patch v0.1.1 (R353). §8: "Distribution: 32 Common, 40 Rare, 16 Epic,
        // 7 Legendary, 5 Mythic" (patch v0.2.9, issue #44, made #27, #28 and #40 Rare).
        SetExpectation {
            set: "Core",
            segment: "core",
            cards: 100,
            card_defined_tokens: texts(&["51.1", "65.1", "90.1", "93.1", "95.1"]),
            shared_tokens: texts(&["T-rush", "T-sheep", "T-felinor", "T-bread", "T-coin", "T-ghoul"]),
            rarities: &[
                ("Common", 32),
                ("Rare", 40),
                ("Epic", 16),
                ("Legendary", 7),
                ("Mythic", 5),
            ],
        },
        // B2.1, B2.5: 90 cards and one shared token; the designer's rarities, as patch v0.2.9
        // (issue #44) left them: 35/26/18/10/1. Issue #170 adds the shared token, Glitch, which only
        // R673's roll ever makes (R674).
        SetExpectation {
            set: "Classic",
            segment: "classic",
            cards: 90,
            card_defined_tokens: Vec::new(),
            shared_tokens: texts(&["T-glitch"]),
            rarities: &[
                ("Common", 35),
                ("Rare", 26),
                ("Epic", 18),
                ("Legendary", 10),
                ("Mythic", 1),
            ],
        },
        // B2.1, B2.3, B2.5, B8: 78 cards, 28 tokens a card defines and the ten AI generated cards,
        // as patch v0.2.9 (issue #44) left them: 13/24/25/13/3.
        SetExpectation {
            set: "Classic+",
            segment: "classicplus",
            cards: 78,
            card_defined_tokens: [
                range("12.", 1, 8),
                range("19.", 1, 5),
                range("32.", 1, 3),
                texts(&["36.1", "38.1", "42.1", "46.1"]),
                range("65.", 1, 5),
                texts(&["73.1", "75.1", "76.1"]),
            ]
            .concat(),
            shared_tokens: range("T-AI-", 1, 10),
            rarities: &[
                ("Common", 13),
                ("Rare", 24),
                ("Epic", 25),
                ("Legendary", 13),
                ("Mythic", 3),
            ],
        },
        // R1420: the Meditative set (issue #496, docs/meditative-set.md M2): 102 cards and the 30 tokens they
        // define, the designer's rarities with five filled in by §8's rubric. It does not ship yet, so its
        // entries are checked for their shape alone until the patch that lists it in SHIPPED_SETS.
        SetExpectation {
            set: "Meditative",
            segment: "meditative",
            cards: 102,
            card_defined_tokens: texts(&[
                "19.1", "22.1", "28.1", "30.1", "39.1", "39.2", "39.3", "39.4", "39.5", "45.1", "49.1",
                "49.2", "49.3", "70.1", "71.1", "91.1", "93.1", "93.2", "93.3", "95.1", "96.1", "97.1",
                "97.2", "97.3", "97.4", "97.5", "97.6", "97.7", "97.8", "97.9",
            ]),
            shared_tokens: Vec::new(),
            rarities: &[
                ("Common", 29),
                ("Rare", 29),
                ("Epic", 22),
                ("Legendary", 16),
                ("Mythic", 6),
            ],
        },
    ]
}

fn expected_indices(set: &SetExpectation) -> Vec<String> {
    let mut out: Vec<String> = (1..=set.cards).map(|n| n.to_string()).collect();
    out.extend(set.card_defined_tokens.iter().cloned());
    out.extend(set.shared_tokens.iter().cloned());
    out
}

/// §5, §7, §8, B2.4: how many catalog entries carry each tag, tokens included — a census, so a tag
/// that drifts onto or off a card fails here (R278: Jlockeed is Core #13 and #14's and the three
/// Classic+ Jlockheed cards'). TS's `Record<Tag, number>`: the `match` is exhaustive over the union.
fn expected_tag_count(tag: Tag) -> usize {
    match tag {
        Tag::Human => 38,
        Tag::Felinor => 11,
        Tag::Ky => 9,
        Tag::Cn => 7,
        Tag::Fruit => 15,
        Tag::CallToChaos => 2,
        Tag::Quickdraw => 5,
        Tag::Jlockeed => 6,
        Tag::Book => 14,
        Tag::Pancake => 10,
        Tag::Ai => 10,
        Tag::Plague => 17,
        Tag::Catalyst => 2,
        Tag::Prime => 2,
        Tag::Acclaimed => 2,
        // The Meditative set's (R1420): counted once it ships.
        Tag::Wincon => 0,
        Tag::Token => 50,
    }
}

/// `EXPECTED_TAG_COUNTS`, in `TAGS` order.
fn expected_tag_counts() -> Vec<(&'static str, usize)> {
    Tag::ALL
        .iter()
        .map(|tag| (tag.as_str(), expected_tag_count(*tag)))
        .collect()
}

/// B2.5: a token's printed rarity is one a card could carry.
fn printed_rarities() -> Vec<&'static str> {
    PrintedRarity::ALL.iter().map(|each| each.as_str()).collect()
}

/// B3.1: a backrow card that becomes a Unit prints the Unit's attack and health.
const ANIMATED_KEYWORDS: &[&str] = &["Animated", "Animated on your turn"];

/* ------------------------------------------------------------------------------------ helpers */

/// TS kept the failures in a module-level list; here each check is handed the list.
fn fail(failures: &mut Vec<String>, at: &str, message: &str) {
    failures.push(format!("{at}: {message}"));
}

/// §5, B2.2: "43" → "core-043", "90.1" → "core-090-1", "T-rush" → "core-t-rush",
/// "T-AI-1" → "classicplus-t-ai-01" (a shared token's trailing number is two digits).
fn id_for_index(segment: &str, index: &str) -> String {
    if index.starts_with("T-") {
        // index.toLowerCase().replace(/-(\d+)$/, (_, n) => `-${n.padStart(2, "0")}`)
        let lower = index.to_lowercase();
        let name = match lower.rfind('-') {
            Some(cut) if js::is_digits(&lower[cut + 1..]) => {
                format!("{}-{:0>2}", &lower[..cut], &lower[cut + 1..])
            }
            _ => lower,
        };
        return format!("{segment}-{name}");
    }
    let mut parts = index.split('.');
    let main = parts.next().unwrap_or_default();
    let padded = format!("{main:0>3}");
    match parts.next() {
        None => format!("{segment}-{padded}"),
        Some(sub) => format!("{segment}-{padded}-{sub}"),
    }
}

fn is_plain_object(value: Option<&Json>) -> bool {
    matches!(value, Some(Json::Object(_)))
}

/// `JSON.stringify(value) ?? String(value)`: a missing value is `undefined`.
fn describe(value: Option<&Json>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(value) => js::stringify(value),
    }
}

/// `Number.isInteger(value)` on a value that may be missing.
fn is_int(value: Option<&Json>) -> bool {
    value.is_some_and(Json::is_integer)
}

/// `/^[a-z][A-Za-z0-9]*$/`: a camelCase word.
fn is_camel_word(key: &str) -> bool {
    let mut bytes = key.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_lowercase()) && bytes.all(|b| b.is_ascii_alphanumeric())
}

/// A text with every param placeholder removed (TS's
/// `text.replace(/\{[A-Za-z][A-Za-z0-9]*(?:\|[^|{}]*\|[^|{}]*)?\}/g, "")`, `PARAM_PLACEHOLDER`'s
/// own pattern).
fn strip_placeholders(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for placeholder in js::placeholder_spans(text) {
        out.push_str(&text[at..placeholder.start]);
        at = placeholder.end;
    }
    out.push_str(&text[at..]);
    out
}

/// `new Set(items).size !== items.length` over JSON values: strings, numbers, booleans and `null`
/// by value, objects and arrays by identity (so never equal to another).
fn repeats_a_value(items: &[Json]) -> bool {
    let same = |a: &Json, b: &Json| match (a, b) {
        (Json::Null, Json::Null) => true,
        (Json::Bool(a), Json::Bool(b)) => a == b,
        (Json::Number(a), Json::Number(b)) => a == b || (a.is_nan() && b.is_nan()),
        (Json::String(a), Json::String(b)) => a == b,
        _ => false,
    };
    items
        .iter()
        .enumerate()
        .any(|(i, a)| items[i + 1..].iter().any(|b| same(a, b)))
}

/* ---------------------------------------------------------------------------------------- faces */

fn validate_face(
    failures: &mut Vec<String>,
    at_card: &str,
    face_name: &str,
    face: Option<&Json>,
    card_type: Option<&Json>,
) {
    let at = format!("{at_card}.{face_name}");
    if !is_plain_object(face) {
        fail(
            failures,
            &at,
            &format!("face is not an object (got {})", describe(face)),
        );
        return;
    }
    let Some(Json::Object(face)) = face else {
        return;
    };

    // B2.7: a face may carry a type of its own, a CardType that differs from the card's.
    let face_type = face.get("type");
    if let Some(face_type) = face_type {
        match face_type.as_str() {
            Some(text) if card_types().contains(&text) => {
                if card_type.and_then(Json::as_str) == Some(text) {
                    fail(
                        failures,
                        &at,
                        &format!(
                            "`type` repeats the card's own type {}; leave it out",
                            describe(card_type)
                        ),
                    );
                }
            }
            _ => fail(
                failures,
                &at,
                &format!("`type` is not a CardType (got {})", describe(Some(face_type))),
            ),
        }
    }
    // `(faceType ?? cardType) === "Unit"`: a face's `null` type falls back like a missing one.
    let running_type = match face_type {
        None | Some(Json::Null) => card_type,
        some => some,
    };
    let is_unit = running_type.and_then(Json::as_str) == Some("Unit");
    let kinds_written: Vec<Option<&Json>> = match face.get("keywords") {
        Some(Json::Array(keywords)) => keywords
            .iter()
            .map(|keyword| keyword.as_object().and_then(|keyword| keyword.get("kind")))
            .collect(),
        _ => Vec::new(),
    };
    let animated = kinds_written.iter().any(|kind| {
        kind.and_then(Json::as_str)
            .is_some_and(|kind| ANIMATED_KEYWORDS.contains(&kind))
    });

    // Stats: present on both faces of every Unit and every Animated backrow card (B3.1), absent on
    // every other non-Unit (§5, §8).
    for stat in ["attack", "health"] {
        let value = face.get(stat);
        if is_unit || animated {
            if !is_int(value) {
                let what = if is_unit { "Unit" } else { "Animated" };
                fail(
                    failures,
                    &at,
                    &format!(
                        "{what} face must carry an integer `{stat}` (got {})",
                        describe(value)
                    ),
                );
            }
        } else if value.is_some() {
            fail(
                failures,
                &at,
                &format!("non-Unit face must not carry `{stat}` (got {})", describe(value)),
            );
        }
    }

    // B2.7: "[3X/3X]" stats — a Unit's positive multiples of X, its printed attack and health 0.
    if let Some(x_stats) = face.get("xStats") {
        if !is_unit {
            fail(failures, &at, "`xStats` is a Unit face's (B2.7)");
        }
        let well_formed = match x_stats {
            Json::Object(x_stats) => {
                let mut keys: Vec<&str> = x_stats.keys().map(String::as_str).collect();
                keys.sort();
                keys.join(",") == "attack,health"
                    && [x_stats.get("attack"), x_stats.get("health")]
                        .iter()
                        .all(|n| is_int(*n) && n.and_then(Json::as_f64).is_some_and(|n| n > 0.0))
            }
            _ => false,
        };
        if !well_formed {
            fail(
                failures,
                &at,
                &format!(
                    "`xStats` must be {{ attack, health }} of positive integers (got {})",
                    describe(Some(x_stats))
                ),
            );
        }
        let zero = Json::Number(0.0);
        if face.get("attack") != Some(&zero) || face.get("health") != Some(&zero) {
            fail(failures, &at, "a face with `xStats` prints 0/0 (B2.7)");
        }
    }

    if !matches!(face.get("text"), Some(Json::String(_))) {
        fail(
            failures,
            &at,
            &format!("`text` must be a string (got {})", describe(face.get("text"))),
        );
    }

    let Some(Json::Array(keywords)) = face.get("keywords") else {
        fail(
            failures,
            &at,
            &format!(
                "`keywords` must be an array (got {})",
                describe(face.get("keywords"))
            ),
        );
        return;
    };
    let kinds = keyword_kinds();
    for (i, keyword) in keywords.iter().enumerate() {
        let kw_at = format!("{at}.keywords[{i}]");
        let Json::Object(keyword) = keyword else {
            fail(
                failures,
                &kw_at,
                &format!("keyword must be an object (got {})", describe(Some(keyword))),
            );
            continue;
        };
        let kind = keyword.get("kind");
        let Some(kind_text) = kind.and_then(Json::as_str).filter(|kind| kinds.contains(kind)) else {
            fail(
                failures,
                &kw_at,
                &format!("`kind` is not a Keyword kind (got {})", describe(kind)),
            );
            continue;
        };
        let numbered = NUMBERED_KEYWORDS.contains(&kind_text);
        let n = keyword.get("n");
        if numbered && !is_int(n) {
            fail(
                failures,
                &kw_at,
                &format!("{kind_text} must carry an integer `n` (got {})", describe(n)),
            );
        }
        if !numbered && n.is_some() {
            fail(
                failures,
                &kw_at,
                &format!("{kind_text} must not carry `n` (got {})", describe(n)),
            );
        }
        let extra: Vec<&str> = keyword
            .keys()
            .map(String::as_str)
            .filter(|k| *k != "kind" && *k != "n")
            .collect();
        if !extra.is_empty() {
            fail(
                failures,
                &kw_at,
                &format!("unknown keyword field(s) {}", extra.join(", ")),
            );
        }
        // TS narrowed the object to `Keyword` here; the checks above are that shape.
    }

    // ME-GRANT (MD-D13, R1107): the Death abilities a face grants, by key, each its quoted text.
    if let Some(grants) = face.get("grants") {
        let well_formed = match grants {
            Json::Object(grants) => {
                !grants.is_empty()
                    && grants
                        .values()
                        .all(|text| text.as_str().is_some_and(|text| !text.is_empty()))
            }
            _ => false,
        };
        if !well_formed {
            fail(
                failures,
                &at,
                &format!(
                    "`grants` must be a non-empty object of non-empty texts (got {})",
                    describe(Some(grants))
                ),
            );
        }
    }

    let unknown_fields: Vec<&str> = face
        .keys()
        .map(String::as_str)
        .filter(|k| !["type", "attack", "health", "xStats", "keywords", "text", "grants"].contains(k))
        .collect();
    if !unknown_fields.is_empty() {
        fail(
            failures,
            &at,
            &format!("unknown face field(s) {}", unknown_fields.join(", ")),
        );
    }
}

/// B3.4 rule 5: `params` — distinct camelCase keys, integer values within [min, max], a direction,
/// a positive step — each written `{key}` in at least one face's text, and every `{key}` a text
/// writes declared. A number one face alone writes may be tuned on that face only (`tunedOn`, R749),
/// and one only the base face writes must be (R1431); a number that belongs to a power names it by
/// its Activate ability's id (`power`, R1430).
fn validate_params(failures: &mut Vec<String>, at: &str, value: &Object) {
    let params = value.get("params");
    let texts: Vec<String> = ["base", "radiant"]
        .iter()
        .map(|face| match value.get(*face) {
            Some(Json::Object(face)) => face
                .get("text")
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string(),
            _ => String::new(),
        })
        .collect();
    let writes = |text: &str, key: &str| {
        param_placeholders(text)
            .iter()
            .any(|placeholder| placeholder.key == key)
    };
    let mut declared: IndexSet<String> = IndexSet::new();
    if let Some(params) = params {
        let list = match params {
            Json::Array(list) if !list.is_empty() => list,
            _ => {
                fail(
                    failures,
                    at,
                    &format!(
                        "`params` must be a non-empty array when present (got {})",
                        describe(Some(params))
                    ),
                );
                return;
            }
        };
        for (i, param) in list.iter().enumerate() {
            let p_at = format!("{at}.params[{i}]");
            let Json::Object(param) = param else {
                fail(
                    failures,
                    &p_at,
                    &format!("param must be an object (got {})", describe(Some(param))),
                );
                continue;
            };
            let key_value = param.get("key");
            let Some(key) = key_value.and_then(Json::as_str).filter(|key| is_camel_word(key)) else {
                fail(
                    failures,
                    &p_at,
                    &format!("`key` must be a camelCase word (got {})", describe(key_value)),
                );
                continue;
            };
            if declared.contains(key) {
                fail(failures, &p_at, &format!("repeats the key \"{key}\""));
            }
            declared.insert(key.to_string());
            let (base, radiant) = (param.get("base"), param.get("radiant"));
            let (better, step) = (param.get("better"), param.get("step"));
            let (min, max, tuned_on) = (param.get("min"), param.get("max"), param.get("tunedOn"));
            if !is_int(base) || !is_int(radiant) {
                fail(
                    failures,
                    &p_at,
                    &format!("{key}: `base` and `radiant` must be integers"),
                );
            }
            if !matches!(better.and_then(Json::as_str), Some("up" | "down")) {
                fail(
                    failures,
                    &p_at,
                    &format!("{key}: `better` is \"up\" or \"down\" (got {})", describe(better)),
                );
            }
            if step.is_some()
                && (!is_int(step) || step.and_then(Json::as_f64).is_some_and(|step| step <= 0.0))
            {
                fail(
                    failures,
                    &p_at,
                    &format!("{key}: `step` must be a positive integer"),
                );
            }
            if min.is_some() && !is_int(min) {
                fail(failures, &p_at, &format!("{key}: `min` must be an integer"));
            }
            if max.is_some() && !is_int(max) {
                fail(failures, &p_at, &format!("{key}: `max` must be an integer"));
            }
            for n in [base, radiant] {
                let (Some(n), true) = (n.and_then(Json::as_f64), is_int(n)) else {
                    continue;
                };
                if let (Some(min), true) = (min.and_then(Json::as_f64), is_int(min))
                    && n < min
                {
                    let (n, min) = (js::number_string(n), js::number_string(min));
                    fail(failures, &p_at, &format!("{key}: {n} is below its min {min}"));
                }
                if let (Some(max), true) = (max.and_then(Json::as_f64), is_int(max))
                    && n > max
                {
                    let (n, max) = (js::number_string(n), js::number_string(max));
                    fail(failures, &p_at, &format!("{key}: {n} is above its max {max}"));
                }
            }
            // R749, R1431: a number tuned on one face only is one the other face does not print.
            let tuned_face = tuned_on.and_then(Json::as_str);
            if tuned_on.is_some() && !matches!(tuned_face, Some("radiant" | "base")) {
                fail(
                    failures,
                    &p_at,
                    &format!(
                        "{key}: `tunedOn` is \"radiant\" or \"base\" when present (got {})",
                        describe(tuned_on)
                    ),
                );
            }
            if tuned_face == Some("radiant") && writes(&texts[0], key) {
                fail(
                    failures,
                    &p_at,
                    &format!(
                        "{key}: tuned on the Radiant face only, but the base face's text writes {{{key}}}"
                    ),
                );
            }
            if tuned_face == Some("base") && writes(&texts[1], key) {
                fail(
                    failures,
                    &p_at,
                    &format!(
                        "{key}: tuned on the base face only, but the Radiant face's text writes {{{key}}}"
                    ),
                );
            }
            // R1431: a number only the base face prints is tuned on the base face only.
            if tuned_face != Some("base") && writes(&texts[0], key) && !writes(&texts[1], key) {
                fail(
                    failures,
                    &p_at,
                    &format!(
                        "{key}: only the base face's text writes {{{key}}}, so it is `tunedOn: \"base\"`"
                    ),
                );
            }
            // R1430: the power a number belongs to, named by its Activate ability's id.
            let power = param.get("power");
            if power.is_some() && !power.and_then(Json::as_str).is_some_and(is_camel_word) {
                fail(
                    failures,
                    &p_at,
                    &format!(
                        "{key}: `power` names an Activate ability by its id, a camelCase word (got {})",
                        describe(power)
                    ),
                );
            }
            let extra: Vec<&str> = param
                .keys()
                .map(String::as_str)
                .filter(|k| {
                    ![
                        "key", "base", "radiant", "better", "step", "min", "max", "tunedOn", "power",
                    ]
                    .contains(k)
                })
                .collect();
            if !extra.is_empty() {
                fail(
                    failures,
                    &p_at,
                    &format!("unknown param field(s) {}", extra.join(", ")),
                );
            }
            if !texts.iter().any(|text| writes(text, key)) {
                fail(
                    failures,
                    &p_at,
                    &format!("{key}: no face's text writes {{{key}}}"),
                );
            }
        }
    }
    for (i, text) in texts.iter().enumerate() {
        let t_at = format!("{at}.{}.text", if i == 0 { "base" } else { "radiant" });
        for placeholder in param_placeholders(text) {
            let key = &placeholder.key;
            if !declared.contains(key) {
                fail(failures, &t_at, &format!("{{{key}}} is not a declared param"));
            }
            // R482: `{key|singular|plural}` names two different, non-empty wordings.
            if let Some(one) = &placeholder.one
                && (one.is_empty() || Some(one) == placeholder.many.as_ref())
            {
                fail(
                    failures,
                    &t_at,
                    &format!("{{{key}|…}} needs a singular and a different plural wording"),
                );
            }
        }
        // A brace that is no placeholder is a typo in one.
        if strip_placeholders(text).contains(['{', '}']) {
            fail(failures, &t_at, "a brace that is not a param placeholder");
        }
    }
}

/// R349's fallback face, as `validate_fallback` holds a `radiantFallback` entry's `radiant` to it.
const RADIANT_FALLBACK_FACTOR: f64 = 2.0;

fn validate_fallback(failures: &mut Vec<String>, at: &str, value: &Object) {
    if value.get("radiantFallback") != Some(&Json::Bool(true)) {
        fail(
            failures,
            at,
            &format!(
                "`radiantFallback` is `true` when present (got {})",
                describe(value.get("radiantFallback"))
            ),
        );
        return;
    }
    if value.get("type").and_then(Json::as_str) != Some("Unit") {
        fail(failures, at, "`radiantFallback` is a Unit's (R349)");
    }
    let (Some(Json::Object(base)), Some(Json::Object(radiant))) = (value.get("base"), value.get("radiant"))
    else {
        return;
    };
    // `{ ...base, attack: doubled(base.attack), health: doubled(base.health) }`: a stat the base
    // face carries keeps its place, doubled when it is a number; one it lacks is `undefined`, which
    // `JSON.stringify` leaves out.
    let mut expected = base.clone();
    for stat in ["attack", "health"] {
        if let Some(Json::Number(n)) = expected.get_mut(stat) {
            *n *= RADIANT_FALLBACK_FACTOR;
        }
    }
    let expected = Json::Object(expected);
    if js::stringify(radiant) != js::stringify(&expected) {
        fail(
            failures,
            at,
            &format!(
                "a `radiantFallback` card's radiant face is its base face doubled (R349): expected {}",
                describe(Some(&expected))
            ),
        );
    }
}

/* ----------------------------------------------------------------------------------------- main */

/// What `catalog check` found: every failure, and the census it prints when there is none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogReport {
    pub failures: Vec<String>,
    pub entries: usize,
    pub non_token_count: usize,
    pub token_count: usize,
    /// R279: the entries that carry `refs`.
    pub refs_count: usize,
    pub param_count: usize,
    pub loc_count: usize,
    /// R1420: the entries of sets that do not ship yet, which no total counts.
    pub unshipped_count: usize,
}

/// `validate-catalog.ts`'s top level over a parsed catalog (an object keyed by card id).
pub fn check_catalog(catalog: &Object) -> CatalogReport {
    let sets = sets();
    // R1420: the totals are the shipped catalog's.
    let expected_non_token: usize = sets.iter().filter(|set| set.ships()).map(|set| set.cards).sum();
    let expected_token: usize = sets
        .iter()
        .filter(|set| set.ships())
        .map(|set| set.card_defined_tokens.len() + set.shared_tokens.len())
        .sum();
    let expected_total = expected_non_token + expected_token;
    let (type_union, set_union, tag_union, rarity_union) = (card_types(), set_names(), tags(), rarities());
    let printed_union = printed_rarities();
    let mut failures: Vec<String> = Vec::new();
    let entries = catalog.len();

    // 1. 111 entries, R1420's unshipped sets aside. (TS checks the total here and again after the
    // loop, so a wrong total fails twice; kept.)
    let unshipped_entries = catalog
        .values()
        .filter(|value| {
            let Json::Object(value) = value else {
                return false;
            };
            let set = value.get("set").and_then(Json::as_str);
            sets.iter()
                .find(|expectation| set == Some(expectation.set))
                .is_some_and(|expectation| !expectation.ships())
        })
        .count();
    let entries = entries - unshipped_entries;
    if entries != expected_total {
        fail(
            &mut failures,
            "catalog",
            &format!("expected {expected_total} entries, found {entries}"),
        );
    }

    let mut non_token_count = 0;
    let mut token_count = 0;
    // Per set: rarity counts over its non-token cards.
    let mut rarity_counts: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();
    let mut tag_counts: IndexMap<String, usize> = IndexMap::new();
    // R279: every entry's `refs`, checked against the ids once the whole file has been read.
    let mut refs_by_card: IndexMap<&str, &Json> = IndexMap::new();
    // Per set: index -> the keys that carry it (an index is unique only within its set, B2.2).
    let mut seen_indices: IndexMap<&str, IndexMap<String, Vec<String>>> = IndexMap::new();
    let mut loc_count = 0;
    let mut param_count = 0;
    // R1420: the entries of sets that do not ship yet, counted apart from the totals.
    let mut unshipped_count = 0;

    for (key, value) in catalog {
        let at = format!("catalog[\"{key}\"]");
        let Json::Object(value) = value else {
            fail(
                &mut failures,
                &at,
                &format!("entry is not an object (got {})", describe(Some(value))),
            );
            continue;
        };

        let index = value.get("index");
        let id = value.get("id");

        // The object must be keyed by the card's own id (integration decision 1).
        match id {
            Some(Json::String(id)) if id != key => {
                fail(&mut failures, &at, &format!("key does not match `id` \"{id}\""))
            }
            Some(Json::String(_)) => {}
            _ => fail(
                &mut failures,
                &at,
                &format!("`id` must be a string (got {})", describe(id)),
            ),
        }

        // 4. `id` follows the index convention of its set (§5, B2.2).
        let set_value = value.get("set");
        let set_of = sets
            .iter()
            .find(|expectation| set_value.and_then(Json::as_str) == Some(expectation.set));
        // R1420: an entry of a set that does not ship counts toward no total and no tag.
        let counted = set_of.is_none_or(SetExpectation::ships);
        if !counted {
            unshipped_count += 1;
        }
        match (index.and_then(Json::as_str), set_of) {
            (None, _) => fail(
                &mut failures,
                &at,
                &format!("`index` must be a string (got {})", describe(index)),
            ),
            (Some(_), None) => {
                let shipping: Vec<&str> = sets.iter().map(|set| set.set).collect();
                fail(
                    &mut failures,
                    &at,
                    &format!(
                        "`set` {} is not a set the catalog holds ({})",
                        describe(set_value),
                        shipping.join(", ")
                    ),
                );
            }
            (Some(index), Some(set_of)) => {
                let expected_id = id_for_index(set_of.segment, index);
                if id.and_then(Json::as_str) != Some(expected_id.as_str()) {
                    fail(
                        &mut failures,
                        &at,
                        &format!(
                            "index \"{index}\" implies id \"{expected_id}\", found {}",
                            describe(id)
                        ),
                    );
                }
                seen_indices
                    .entry(set_of.set)
                    .or_default()
                    .entry(index.to_string())
                    .or_default()
                    .push(key.clone());
            }
        }

        let name = value.get("name");
        if !name.and_then(Json::as_str).is_some_and(|name| !name.is_empty()) {
            fail(
                &mut failures,
                &at,
                &format!("`name` must be a non-empty string (got {})", describe(name)),
            );
        }

        // 5. `type`, `tags`, `set`, `rarity` are in the unions the wire's catalog types declare.
        let card_type = value.get("type");
        if !card_type
            .and_then(Json::as_str)
            .is_some_and(|t| type_union.contains(&t))
        {
            fail(
                &mut failures,
                &at,
                &format!("`type` is not a CardType (got {})", describe(card_type)),
            );
        }

        if !set_value
            .and_then(Json::as_str)
            .is_some_and(|s| set_union.contains(&s))
        {
            fail(
                &mut failures,
                &at,
                &format!("`set` is not a SetName (got {})", describe(set_value)),
            );
        }

        let mut tag_list: Vec<String> = Vec::new();
        match value.get("tags") {
            Some(Json::Array(tags)) => {
                tag_list = tags.iter().map(|tag| js::to_js_string(Some(tag))).collect();
                if counted {
                    for tag in &tag_list {
                        *tag_counts.entry(tag.clone()).or_default() += 1;
                    }
                }
                for (i, tag) in tags.iter().enumerate() {
                    if !tag.as_str().is_some_and(|tag| tag_union.contains(&tag)) {
                        fail(
                            &mut failures,
                            &at,
                            &format!("tags[{i}] is not a Tag (got {})", describe(Some(tag))),
                        );
                    }
                }
                if tag_list.iter().collect::<IndexSet<_>>().len() != tag_list.len() {
                    fail(
                        &mut failures,
                        &at,
                        &format!("`tags` repeats a tag ({})", tag_list.join(", ")),
                    );
                }
            }
            other => fail(
                &mut failures,
                &at,
                &format!("`tags` must be an array (got {})", describe(other)),
            ),
        }

        let rarity = value.get("rarity");
        if !rarity
            .and_then(Json::as_str)
            .is_some_and(|r| rarity_union.contains(&r))
        {
            fail(
                &mut failures,
                &at,
                &format!("`rarity` is not a Rarity (got {})", describe(rarity)),
            );
        }

        // 8. token === (rarity === "Token"); 2. the token/non-token split.
        match value.get("token") {
            Some(Json::Bool(token)) => {
                let token = *token;
                if token != (rarity.and_then(Json::as_str) == Some("Token")) {
                    fail(
                        &mut failures,
                        &at,
                        &format!(
                            "token={token} but rarity={} (§5: rarity is \"Token\" for exactly the tokens)",
                            describe(rarity)
                        ),
                    );
                }
                let tagged_token = tag_list.iter().any(|tag| tag == "Token");
                if token {
                    if counted {
                        token_count += 1;
                    }
                    // 9. every token carries the Token tag (§5.1: random pools filter on it).
                    if !tagged_token {
                        let listed = if tag_list.is_empty() {
                            "none".to_string()
                        } else {
                            tag_list.join(", ")
                        };
                        fail(
                            &mut failures,
                            &at,
                            &format!("token is missing the \"Token\" tag (tags: {listed})"),
                        );
                    }
                } else {
                    if counted {
                        non_token_count += 1;
                    }
                    if tagged_token {
                        fail(&mut failures, &at, "non-token carries the \"Token\" tag");
                    }
                    if let (Some(rarity), Some(set)) =
                        (rarity.and_then(Json::as_str), set_value.and_then(Json::as_str))
                    {
                        *rarity_counts
                            .entry(set.to_string())
                            .or_default()
                            .entry(rarity.to_string())
                            .or_default() += 1;
                    }
                }
                // B2.5: a token may print the rarity the designer gave it, for display only; a card
                // never does.
                if let Some(printed) = value.get("printedRarity") {
                    if !token {
                        fail(&mut failures, &at, "`printedRarity` is a token's (B2.5)");
                    }
                    if !printed
                        .as_str()
                        .is_some_and(|printed| printed_union.contains(&printed))
                    {
                        fail(
                            &mut failures,
                            &at,
                            &format!(
                                "`printedRarity` is not a printed rarity (got {})",
                                describe(Some(printed))
                            ),
                        );
                    }
                }
            }
            other => fail(
                &mut failures,
                &at,
                &format!("`token` must be a boolean (got {})", describe(other)),
            ),
        }

        // §5: cost is 0..6, 100, "X", or {base, embiggen}.
        match value.get("cost") {
            Some(Json::Number(cost)) => {
                if !(cost.is_finite() && cost.fract() == 0.0) || *cost < 0.0 {
                    fail(
                        &mut failures,
                        &at,
                        &format!(
                            "numeric `cost` must be a non-negative integer (got {})",
                            js::number_string(*cost)
                        ),
                    );
                }
            }
            Some(Json::String(x)) if x == "X" => {
                // fine
            }
            Some(Json::Object(cost)) => {
                let numeric = |field: &str| matches!(cost.get(field), Some(Json::Number(_)));
                if !numeric("base") || !numeric("embiggen") {
                    fail(
                        &mut failures,
                        &at,
                        &format!(
                            "embiggen `cost` needs numeric `base` and `embiggen` (got {})",
                            describe(value.get("cost"))
                        ),
                    );
                }
                let extra: Vec<&str> = cost
                    .keys()
                    .map(String::as_str)
                    .filter(|k| *k != "base" && *k != "embiggen")
                    .collect();
                if !extra.is_empty() {
                    fail(
                        &mut failures,
                        &at,
                        &format!("unknown cost field(s) {}", extra.join(", ")),
                    );
                }
            }
            other => fail(
                &mut failures,
                &at,
                &format!("`cost` is not a CardCost (got {})", describe(other)),
            ),
        }

        // 6, 7. Faces: valid keywords, stats on Units (and Animated backrow cards) only.
        validate_face(&mut failures, &at, "base", value.get("base"), card_type);
        validate_face(&mut failures, &at, "radiant", value.get("radiant"), card_type);

        // B3.4 rule 5: the numbers Degrade, Upgrade and KY's Constant may move.
        validate_params(&mut failures, &at, value);
        if value.contains_key("params") {
            param_count += 1;
        }

        // E36: generated lines of code (now `cargo jackioh catalog loc`, SURFACE §7.5), a
        // non-negative integer when present.
        if let Some(loc) = value.get("loc") {
            loc_count += 1;
            if !loc.is_integer() || loc.as_f64().is_some_and(|loc| loc < 0.0) {
                fail(
                    &mut failures,
                    &at,
                    &format!(
                        "`loc` must be a non-negative integer (got {})",
                        describe(Some(loc))
                    ),
                );
            }
        }

        if let Some(refs) = value.get("refs") {
            refs_by_card.insert(key.as_str(), refs);
        }

        // R349: a card that prints no Radiant form carries `radiantFallback: true`, and its `radiant`
        // face is exactly the fallback the rule gives it: the base face with its attack and health
        // doubled, the same keywords and the same text.
        if value.contains_key("radiantFallback") {
            validate_fallback(&mut failures, &at, value);
        }

        let known = [
            "id",
            "index",
            "name",
            "set",
            "type",
            "tags",
            "rarity",
            "printedRarity",
            "token",
            "cost",
            "refs",
            "params",
            "loc",
            "radiantFallback",
            "base",
            "radiant",
        ];
        let unknown_fields: Vec<&str> = value
            .keys()
            .map(String::as_str)
            .filter(|k| !known.contains(k))
            .collect();
        if !unknown_fields.is_empty() {
            fail(
                &mut failures,
                &at,
                &format!("unknown field(s) {}", unknown_fields.join(", ")),
            );
        }
    }

    // 1, 2. The totals: B2.1's 268 cards and, with Glitch (issue #170), 50 tokens, 318 entries.
    if entries != expected_total {
        fail(
            &mut failures,
            "catalog",
            &format!("expected {expected_total} entries, found {entries}"),
        );
    }
    if non_token_count != expected_non_token {
        fail(
            &mut failures,
            "catalog",
            &format!("expected {expected_non_token} non-token cards, found {non_token_count}"),
        );
    }
    if token_count != expected_token {
        fail(
            &mut failures,
            "catalog",
            &format!("expected {expected_token} tokens, found {token_count}"),
        );
    }

    for expectation in &sets {
        // 3. Each set's indices each exactly once: 1..N plus its token indices.
        let indices = expected_indices(expectation);
        let empty = IndexMap::new();
        let seen = seen_indices.get(expectation.set).unwrap_or(&empty);
        for index in &indices {
            match seen.get(index) {
                // R1420: a set that has not shipped need not hold every card yet.
                None if !expectation.ships() => {}
                None => fail(
                    &mut failures,
                    expectation.set,
                    &format!("index \"{index}\" is missing"),
                ),
                Some(keys) if keys.len() > 1 => fail(
                    &mut failures,
                    expectation.set,
                    &format!(
                        "index \"{index}\" appears {} times ({})",
                        keys.len(),
                        keys.join(", ")
                    ),
                ),
                Some(_) => {}
            }
        }
        for index in seen.keys() {
            if !indices.contains(index) {
                fail(
                    &mut failures,
                    expectation.set,
                    &format!("unexpected index \"{index}\""),
                );
            }
        }

        // 10. The set's rarity distribution across its non-token cards (§8, B2.5).
        let no_counts = IndexMap::new();
        let counted = rarity_counts.get(expectation.set).unwrap_or(&no_counts);
        for (rarity, expected) in expectation.rarities {
            let found = counted.get(*rarity).copied().unwrap_or(0);
            // R1420: a set that has not shipped may hold fewer, never more.
            let wrong = if expectation.ships() {
                found != *expected
            } else {
                found > *expected
            };
            if wrong {
                fail(
                    &mut failures,
                    expectation.set,
                    &format!("expected {expected} {rarity} non-token cards, found {found}"),
                );
            }
        }
        for (rarity, found) in counted {
            if !expectation.rarities.iter().any(|(name, _)| name == rarity) {
                fail(
                    &mut failures,
                    expectation.set,
                    &format!(
                        "{found} non-token card(s) carry rarity \"{rarity}\", which the set does not distribute"
                    ),
                );
            }
        }
    }

    // 11. The tag census (R278).
    for (tag, expected) in expected_tag_counts() {
        let found = tag_counts.get(tag).copied().unwrap_or(0);
        if found != expected {
            fail(
                &mut failures,
                "catalog",
                &format!("expected {expected} entries tagged \"{tag}\", found {found}"),
            );
        }
    }

    // 12. R279: `refs` is a non-empty list of distinct catalog ids. Whether each one is named in
    // the card's text, and every named card listed, is crates/cards/tests/cross/references.rs's to
    // prove.
    for (key, refs) in &refs_by_card {
        let at = format!("catalog[\"{key}\"].refs");
        let refs = match refs {
            Json::Array(refs) if !refs.is_empty() => refs,
            other => {
                fail(
                    &mut failures,
                    &at,
                    &format!(
                        "must be a non-empty array of card ids when present (got {})",
                        describe(Some(*other))
                    ),
                );
                continue;
            }
        };
        if repeats_a_value(refs) {
            fail(
                &mut failures,
                &at,
                &format!("repeats an id ({})", js::join(refs, ", ")),
            );
        }
        for reference in refs {
            if !reference
                .as_str()
                .is_some_and(|reference| catalog.contains_key(reference))
            {
                fail(
                    &mut failures,
                    &at,
                    &format!("{} is not a catalog id", describe(Some(reference))),
                );
            }
        }
    }

    CatalogReport {
        failures,
        entries,
        non_token_count,
        token_count,
        refs_count: refs_by_card.len(),
        param_count,
        loc_count,
        unshipped_count,
    }
}

/* ------------------------------------------------------------------------------------------ loc */

/// One string literal the scanner is inside, across lines: a plain `"…"` (with escapes) or a raw
/// `r#"…"#` closed by that many `#`s.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    Plain,
    Raw(usize),
}

/// An identifier character, which a raw string's `r` may not follow.
/// A raw string opening at `i` (`r"`, `r#"`, `br"`, `cr##"` …): its length up to and including
/// the quote, and its `#` count.
fn raw_string_open(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if i > 0 && is_ident_char(chars[i - 1]) {
        return None;
    }
    let r_at = match (chars.get(i), chars.get(i + 1)) {
        (Some('r'), _) => i,
        (Some('b' | 'c'), Some('r')) => i + 1,
        _ => return None,
    };
    let hashes = chars[r_at + 1..].iter().take_while(|c| **c == '#').count();
    (chars.get(r_at + 1 + hashes) == Some(&'"')).then_some((r_at + 2 + hashes - i, hashes))
}

/// The length of a character literal at `i` (`'x'`, `'\n'`, `'\''`, `'\u{2028}'`), or 1 for a
/// lone `'` (a lifetime or a label).
fn char_literal_length(chars: &[char], i: usize) -> usize {
    match chars.get(i + 1) {
        Some('\\') => chars
            .iter()
            .enumerate()
            .skip(i + 3)
            .find(|(_, c)| **c == '\'')
            .map_or(1, |(close, _)| close - i + 1),
        Some(_) if chars.get(i + 2) == Some(&'\'') => 3,
        _ => 1,
    }
}

/// A `use` declaration's first line, visibility included (`use`, `pub use`, `pub(crate) use`).
fn starts_use(code: &str) -> bool {
    let mut rest = code;
    if let Some(after) = rest.strip_prefix("pub") {
        let after = after.trim_start();
        rest = match after.strip_prefix('(') {
            Some(inner) => match inner.find(')') {
                Some(close) => inner[close + 1..].trim_start(),
                None => return false,
            },
            None => after,
        };
    }
    rest.strip_prefix("use")
        .is_some_and(|tail| tail.is_empty() || tail.starts_with(|c: char| c.is_whitespace() || c == '{'))
}

/// SURFACE §7.5: the lines of code in one card script — every line that holds code once comments
/// are removed, except the lines of a `use` declaration — above the file's `#[cfg(test)]`. Strings
/// (plain, raw, byte) and character literals are read as such, so a `//` inside a string is code,
/// not a comment; block comments nest, as Rust's do. `gen-loc.ts`'s `countLoc`, for Rust.
pub fn count_loc(source: &str) -> usize {
    let mut code_lines: Vec<String> = Vec::new();
    let mut block_depth = 0usize;
    let mut quote: Option<Quote> = None;
    for line in source.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        let mut code = String::new();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            let next = chars.get(i + 1).copied();
            if block_depth > 0 {
                if ch == '*' && next == Some('/') {
                    block_depth -= 1;
                    i += 2;
                } else if ch == '/' && next == Some('*') {
                    block_depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            match quote {
                Some(Quote::Plain) => {
                    code.push(ch);
                    if ch == '\\' {
                        if let Some(next) = next {
                            code.push(next);
                        }
                        i += 2;
                        continue;
                    }
                    if ch == '"' {
                        quote = None;
                    }
                    i += 1;
                    continue;
                }
                Some(Quote::Raw(hashes)) => {
                    code.push(ch);
                    let closes = ch == '"'
                        && chars.len() > i + hashes
                        && chars[i + 1..=i + hashes].iter().all(|c| *c == '#');
                    if closes {
                        code.extend(std::iter::repeat_n('#', hashes));
                        quote = None;
                        i += 1 + hashes;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                None => {}
            }
            if ch == '/' && next == Some('/') {
                break;
            }
            if ch == '/' && next == Some('*') {
                block_depth += 1;
                i += 2;
                continue;
            }
            if ch == '"' {
                quote = Some(Quote::Plain);
                code.push(ch);
                i += 1;
                continue;
            }
            if let Some((length, hashes)) = raw_string_open(&chars, i) {
                code.extend(&chars[i..i + length]);
                quote = Some(Quote::Raw(hashes));
                i += length;
                continue;
            }
            if ch == '\'' {
                let length = char_literal_length(&chars, i);
                code.extend(&chars[i..i + length]);
                i += length;
                continue;
            }
            code.push(ch);
            i += 1;
        }
        code_lines.push(code.trim().to_string());
    }

    let mut count = 0;
    let mut in_use = false;
    for code in &code_lines {
        // A card's tests sit below its script (SURFACE §7.3) and are not its lines of code.
        if code.starts_with("#[cfg(test)]") {
            break;
        }
        if code.is_empty() {
            continue;
        }
        if !in_use && starts_use(code) {
            in_use = true;
        }
        if in_use {
            if code.ends_with(';') {
                in_use = false;
            }
            continue;
        }
        count += 1;
    }
    count
}

/* -------------------------------------------------------------------------------- catalog-version */

/// The newest version `patches.json` at `path` names: its last entry's `version`. The list's
/// order is the order of versions (R105, R388).
pub fn newest_version(path: &Path) -> anyhow::Result<String> {
    let list = fs::read_to_string(path)
        .map_err(anyhow::Error::from)
        .and_then(|text| js::parse(&text).map_err(anyhow::Error::from))
        .map_err(|cause| {
            anyhow!(
                "catalog-version: the patch list could not be read from {}: {cause}",
                path.display()
            )
        })?;
    let newest = match &list {
        Json::Array(entries) => entries
            .last()
            .and_then(Json::as_object)
            .and_then(|entry| entry.get("version")),
        _ => None,
    };
    match newest.and_then(Json::as_str) {
        Some(version) if !version.is_empty() => Ok(version.to_string()),
        _ => bail!(
            "catalog-version: the patch list at {} names no newest version (R388)",
            path.display()
        ),
    }
}

/* ---------------------------------------------------------------------------------------- the CLI */

/// `cargo jackioh catalog …` (SURFACE §12).
#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: CatalogCommand,
}

#[derive(clap::Subcommand)]
pub enum CatalogCommand {
    /// The catalog's data checks (`validate-catalog.ts`): census, unions, faces, params, refs.
    Check,
    /// Print a card script's lines of code (SURFACE §7.5), for a card added after v0.3.0.
    Loc {
        /// The card's `.rs` script file.
        path: PathBuf,
    },
}

/// `cargo jackioh catalog-version [path/to/patches.json]` (SURFACE §12).
#[derive(clap::Args)]
pub struct VersionArgs {
    /// The patch list to read; `crates/cards/patches/patches.json` when left out.
    pub path: Option<PathBuf>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        CatalogCommand::Check => run_check(),
        CatalogCommand::Loc { path } => {
            let source = fs::read_to_string(&path).map_err(|error| anyhow!("{}: {error}", path.display()))?;
            println!("{}", count_loc(&source));
            Ok(())
        }
    }
}

/// `catalog check`: validate-catalog's verdict.
fn run_check() -> anyhow::Result<()> {
    let catalog_path = repo_root().join("crates/cards/catalog.json");
    let text =
        fs::read_to_string(&catalog_path).map_err(|error| anyhow!("{}: {error}", catalog_path.display()))?;
    let raw = js::parse(&text).map_err(|error| anyhow!("{}: {error}", catalog_path.display()))?;
    let Json::Object(catalog) = &raw else {
        bail!(
            "FAIL catalog.json: expected a JSON object keyed by card id, got {}",
            describe(Some(&raw))
        );
    };
    let report = check_catalog(catalog);

    if !report.failures.is_empty() {
        let mut message = format!(
            "validate-catalog: {} failure(s) in {}\n",
            report.failures.len(),
            catalog_path.display()
        );
        for failure in &report.failures {
            message.push_str(&format!("\n  FAIL {failure}"));
        }
        message.push('\n');
        bail!(message);
    }

    println!("validate-catalog: OK");
    println!(
        "  {} entries ({} cards + {} tokens)",
        report.entries, report.non_token_count, report.token_count
    );
    for expectation in sets() {
        let tokens = expectation.card_defined_tokens.len() + expectation.shared_tokens.len();
        let rarities: Vec<String> = expectation
            .rarities
            .iter()
            .map(|(r, n)| format!("{n} {r}"))
            .collect();
        let shipped = if expectation.ships() {
            ""
        } else {
            " (not shipped yet, R1420)"
        };
        println!(
            "  {}{shipped}: {} cards + {tokens} tokens; rarities {}",
            expectation.set,
            expectation.cards,
            rarities.join(", ")
        );
    }
    if report.unshipped_count > 0 {
        println!(
            "  {} entries of sets that have not shipped, outside the totals (R1420)",
            report.unshipped_count
        );
    }
    let tags: Vec<String> = expected_tag_counts()
        .iter()
        .map(|(tag, n)| format!("{n} {tag}"))
        .collect();
    println!(
        "  tags {}; refs on {} entries, params on {}, loc on {}",
        tags.join(", "),
        report.refs_count,
        report.param_count,
        report.loc_count
    );
    Ok(())
}

/// `catalog-version`: the newest patch, with no newline, as `process.stdout.write` printed it.
pub fn run_version(args: VersionArgs) -> anyhow::Result<()> {
    let path = args
        .path
        .unwrap_or_else(|| repo_root().join("crates/cards/patches/patches.json"));
    let newest = newest_version(&path)?;
    let mut stdout = std::io::stdout();
    stdout.write_all(newest.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped_catalog() -> Object {
        match js::parse(jackioh_cards::catalog_json()).expect("crates/cards/catalog.json") {
            Json::Object(catalog) => catalog,
            other => panic!("catalog.json is not an object: {}", js::stringify(&other)),
        }
    }

    fn parse_object(text: &str) -> Object {
        match js::parse(text).expect("JSON") {
            Json::Object(object) => object,
            other => panic!("not an object: {}", js::stringify(&other)),
        }
    }

    #[test]
    fn the_shipped_catalog_passes_every_check() {
        let report = check_catalog(&shipped_catalog());
        assert_eq!(report.failures, Vec::<String>::new());
        assert_eq!(
            (report.entries, report.non_token_count, report.token_count),
            (318, 268, 50)
        );
    }

    /// A Meditative entry made from Core #2's: its id, index, name and set its own.
    fn meditative_entry(index: &str, id: &str) -> Json {
        let Json::Object(mut entry) = shipped_catalog()["core-002"].clone() else {
            panic!("core-002 is an object");
        };
        entry.insert("id".to_string(), Json::from(id));
        entry.insert("index".to_string(), Json::from(index));
        entry.insert(
            "name".to_string(),
            Json::from(format!("Meditative fixture {index}")),
        );
        entry.insert("set".to_string(), Json::from("Meditative"));
        Json::Object(entry)
    }

    #[test]
    fn r1420_holds_a_set_being_built_to_its_shape_alone_outside_the_totals() {
        // The real catalog with the Meditative entries the card parts have added so far taken out,
        // so this test counts its own fixture alone.
        let mut catalog = shipped_catalog();
        catalog.retain(|_, entry| {
            entry
                .as_object()
                .and_then(|entry| entry.get("set"))
                .and_then(Json::as_str)
                != Some("Meditative")
        });
        catalog.insert(
            "meditative-002".to_string(),
            meditative_entry("2", "meditative-002"),
        );
        let report = check_catalog(&catalog);
        assert_eq!(report.failures, Vec::<String>::new());
        assert_eq!(
            (
                report.entries,
                report.non_token_count,
                report.token_count,
                report.unshipped_count
            ),
            (318, 268, 50, 1)
        );

        // An index the brief does not list, or an id that does not follow it, still fails.
        let mut wrong = shipped_catalog();
        wrong.insert(
            "meditative-150".to_string(),
            meditative_entry("150", "meditative-150"),
        );
        wrong.insert("meditative-x".to_string(), meditative_entry("3", "meditative-x"));
        let failures = check_catalog(&wrong).failures.join("\n");
        assert!(failures.contains("unexpected index \"150\""), "{failures}");
        assert!(failures.contains("implies id \"meditative-003\""), "{failures}");
    }

    #[test]
    fn id_for_index_follows_the_index_convention_of_its_set() {
        assert_eq!(id_for_index("core", "43"), "core-043");
        assert_eq!(id_for_index("core", "90.1"), "core-090-1");
        assert_eq!(id_for_index("core", "T-rush"), "core-t-rush");
        assert_eq!(id_for_index("classicplus", "T-AI-1"), "classicplus-t-ai-01");
        assert_eq!(id_for_index("classicplus", "12.8"), "classicplus-012-8");
    }

    #[test]
    fn a_broken_entry_fails_naming_the_entry_and_the_field() {
        let mut catalog = shipped_catalog();
        // #2 Bigot, a Unit that declares no numbers, so the one param below is the only one.
        let mut entry = catalog["core-002"].as_object().expect("core-002").clone();
        // A Unit face without its attack, a param no face writes, and a field nobody knows.
        if let Some(Json::Object(base)) = entry.get_mut("base") {
            base.shift_remove("attack");
        }
        entry.insert(
            "params".to_string(),
            Json::Array(vec![Json::Object(parse_object(
                r#"{"key":"amount","base":1,"radiant":2,"better":"up"}"#,
            ))]),
        );
        entry.insert("colour".to_string(), Json::from("red"));
        catalog.insert("core-002".to_string(), Json::Object(entry));
        let failures = check_catalog(&catalog).failures;
        let at = "catalog[\"core-002\"]";
        for expected in [
            format!("{at}.base: Unit face must carry an integer `attack` (got undefined)"),
            format!("{at}.params[0]: amount: no face's text writes {{amount}}"),
            format!("{at}: unknown field(s) colour"),
        ] {
            assert!(failures.contains(&expected), "{expected} in {failures:#?}");
        }
        assert_eq!(failures.len(), 3, "{failures:#?}");
    }

    #[test]
    fn a_brace_that_is_no_placeholder_and_an_undeclared_placeholder_fail() {
        let value = parse_object(
            r#"{"base":{"text":"Deal {damage} damage. {oops"},"radiant":{"text":"Draw {draw|card|card}."}}"#,
        );
        let mut failures = Vec::new();
        validate_params(&mut failures, "x", &value);
        assert_eq!(
            failures,
            [
                "x.base.text: {damage} is not a declared param",
                "x.base.text: a brace that is not a param placeholder",
                "x.radiant.text: {draw} is not a declared param",
                "x.radiant.text: {draw|…} needs a singular and a different plural wording",
            ]
        );
    }

    /// `validate_params` on one card whose faces read `base` and `radiant` and declare `params`.
    fn params_failures(params: &str, base: &str, radiant: &str) -> Vec<String> {
        let value = parse_object(&format!(
            r#"{{"params":{params},"base":{{"text":"{base}"}},"radiant":{{"text":"{radiant}"}}}}"#
        ));
        let mut failures = Vec::new();
        validate_params(&mut failures, "x", &value);
        failures
    }

    #[test]
    fn r1431_a_number_only_the_base_face_writes_is_tuned_there_alone_and_one_the_radiant_face_writes_is_not()
    {
        let marked = r#"[{"key":"cards","base":1,"radiant":1,"better":"up","tunedOn":"base"}]"#;
        let unmarked = r#"[{"key":"cards","base":1,"radiant":1,"better":"up"}]"#;
        // Mind Melt's shape: the base face alone writes the number, tuned there alone.
        assert_eq!(
            params_failures(marked, "Exile {cards}.", "Exile every card of a cost."),
            Vec::<String>::new()
        );
        // The Radiant face writes it too, so it is no base-only number.
        assert_eq!(
            params_failures(marked, "Exile {cards}.", "Exile {cards} twice."),
            ["x.params[0]: cards: tuned on the base face only, but the Radiant face's text writes {cards}"]
        );
        // A number only the base face writes and not marked is refused.
        assert_eq!(
            params_failures(unmarked, "Exile {cards}.", "Exile every card of a cost."),
            ["x.params[0]: cards: only the base face's text writes {cards}, so it is `tunedOn: \"base\"`"]
        );
        // Both faces write it: tuned on both, unmarked.
        assert_eq!(
            params_failures(unmarked, "Exile {cards}.", "Exile {cards} twice."),
            Vec::<String>::new()
        );
        // A face that is neither.
        assert_eq!(
            params_failures(
                r#"[{"key":"cards","base":1,"radiant":1,"better":"up","tunedOn":"both"}]"#,
                "Exile {cards}.",
                "Exile {cards} twice."
            ),
            ["x.params[0]: cards: `tunedOn` is \"radiant\" or \"base\" when present (got \"both\")"]
        );
    }

    #[test]
    fn r1430_a_number_names_the_power_it_belongs_to_by_its_activate_ability_s_id() {
        let with_power = |power: &str| {
            params_failures(
                &format!(r#"[{{"key":"shot","base":2,"radiant":4,"better":"up","power":{power}}}]"#),
                "Deal {shot} damage.",
                "Deal {shot} damage twice.",
            )
        };
        assert_eq!(with_power(r#""burn""#), Vec::<String>::new());
        for (power, shown) in [(r#""Steady Shot""#, "\"Steady Shot\""), ("3", "3")] {
            assert_eq!(
                with_power(power),
                [format!(
                    "x.params[0]: shot: `power` names an Activate ability by its id, a camelCase word (got {shown})"
                )]
            );
        }
    }

    #[test]
    fn count_loc_counts_code_above_the_tests_without_comments_blank_lines_or_use_declarations() {
        let source = r##"//! SPEC §8.1 #2 Bigot.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{
    destroy,
    destroy_all,
};

pub const ID: &str = "core-002"; // the id
/* a block /* nested */ comment
   still a comment */
pub fn script() -> CardScripts {
    let url = "https://example.invalid"; // a `//` in a string is code
    let raw = r#"a "quoted" // string"#;
    let quote = '"';
    CardScripts::default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn counted_never() {}
}
"##;
        assert_eq!(count_loc(source), 7);
    }

    #[test]
    fn catalog_version_is_the_newest_patch() {
        let path = repo_root().join("crates/cards/patches/patches.json");
        assert_eq!(newest_version(&path).unwrap(), jackioh_cards::catalog_version());
    }
}
