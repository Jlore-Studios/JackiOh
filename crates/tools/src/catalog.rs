//! `cargo jackioh catalog check`, `cargo jackioh catalog loc <path>` and `cargo jackioh
//! catalog-version` (SURFACE §12).
//!
//! `catalog check` is the port of `packages/cards/scripts/validate-catalog.ts`: structural
//! validation of `crates/cards/catalog.json` against SPEC §5, §6.1, §7 and §8, and patch v0.2.0's
//! sets and fields (docs/classic-sets.md B2, B3.4, E36, E40).
//!
//! This checks the shape and the census of the catalog, set by set (Core, Classic, Classic+,
//! Meditative, Meditative+, Stinkyscape, R1857000, and the held Training Grounds set, R2194000): that it holds every card and every token exactly once, that every enum value is in
//! the union the engine's wire types declare, that stats sit on Units (and Animated backrow cards,
//! B3.1, which print a whole line or none, R2280000) and nowhere else, that the rarity distribution
//! each set prints is the one the census carries (`jackioh_cards::census`), and that the v0.2.0 fields (`printedRarity`, `params`,
//! `loc`, a face's `type` and `xStats`) are well formed. It does NOT re-read the §8 cells — `crates/cards/tests/cross/catalog.rs` does that with
//! an independent transcription. The pending-fragment proof is its own command, `patches check`
//! (TS's `pnpm validate:catalog` ran both).
//!
//! `catalog loc <path>` prints a card script's lines of code (SURFACE §7.5): `loc` is frozen
//! gameplay data in `catalog.json`, so nothing recomputes it; a card added after v0.3.0 takes its
//! number from here. A test below holds every entry to carrying a `loc`, and each one written since
//! the newest shipped snapshot (a new entry, or a `loc` other than the snapshot's) to this count.
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
use jackioh_cards::census::{CENSUS, tag_count};
use jackioh_engine::wire::{
    CardType, Flavor, KEYWORD_KINDS, PrintedRarity, Rarity, SetName, Tag, param_placeholders, set_ships,
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
    /// Non-token cards outside the set's numbers, indexed T-name (R1857001: Stinkyscape's Dormant
    /// Starship). Each counts as a card, beside `cards`.
    shared_cards: Vec<String>,
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

fn texts(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// `SETS`: the census's sets (`jackioh_cards::census`, where each set's counts are written once, #2581).
fn sets() -> Vec<SetExpectation> {
    CENSUS
        .iter()
        .map(|census| SetExpectation {
            set: census.set,
            segment: census.segment,
            cards: census.cards,
            shared_cards: texts(census.shared_cards),
            card_defined_tokens: texts(census.card_defined_tokens),
            shared_tokens: texts(census.shared_tokens),
            rarities: census.rarities,
        })
        .collect()
}

fn expected_indices(set: &SetExpectation) -> Vec<String> {
    let mut out: Vec<String> = (1..=set.cards).map(|n| n.to_string()).collect();
    out.extend(set.shared_cards.iter().cloned());
    out.extend(set.card_defined_tokens.iter().cloned());
    out.extend(set.shared_tokens.iter().cloned());
    out
}

/// `EXPECTED_TAG_COUNTS`, in `TAGS` order.
fn expected_tag_counts() -> Vec<(&'static str, usize)> {
    Tag::ALL
        .iter()
        .map(|tag| (tag.as_str(), tag_count(*tag)))
        .collect()
}

/// B2.5: a token's printed rarity is one a card could carry.
fn printed_rarities() -> Vec<&'static str> {
    PrintedRarity::ALL.iter().map(|each| each.as_str()).collect()
}

/// R1437000: the names a flavor may take.
fn flavors() -> Vec<&'static str> {
    Flavor::ALL.iter().map(|each| each.as_str()).collect()
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

    // Stats: present on both faces of every Unit, on an Animated backrow card's as a whole line or not
    // at all (B3.1, R2280000: it becomes a Unit only with stats), absent on every other non-Unit (§5, §8).
    if !is_unit && animated && face.get("attack").is_some() != face.get("health").is_some() {
        fail(
            failures,
            &at,
            "an Animated face prints both `attack` and `health` or neither (R2280000)",
        );
    }
    for stat in ["attack", "health"] {
        let value = face.get(stat);
        if is_unit || (animated && value.is_some()) {
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
        } else if !animated && value.is_some() {
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
/// writes declared. A number one face alone writes is tuned on that face only (`tunedOn`, R749),
/// and says so (R1431, R824000); a number that belongs to a power names it by
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
            // R824000: a number only the Radiant face prints is tuned on the Radiant face only.
            if tuned_face != Some("radiant") && writes(&texts[1], key) && !writes(&texts[0], key) {
                fail(
                    failures,
                    &p_at,
                    &format!(
                        "{key}: only the Radiant face's text writes {{{key}}}, so it is `tunedOn: \"radiant\"`"
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
            // R1748000: a number only the card's Cry reads is `cry: true`, and only such a number is;
            // R2261001: one only one face's Cry reads, while the other face's lasting text reads it
            // too, is marked with that face.
            let cry = param.get("cry");
            let marked = match cry {
                None => Some(None),
                Some(Json::Bool(true)) => Some(Some("true")),
                Some(Json::String(face)) if matches!(face.as_str(), "base" | "radiant") => {
                    Some(Some(face.as_str()))
                }
                Some(_) => None,
            };
            // Per face: None when it never writes the number, else whether every line writing it is a Cry.
            let cry_only = |text: &str| -> Option<bool> {
                let writing: Vec<&str> = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| writes(line, key))
                    .collect();
                (!writing.is_empty()).then(|| writing.iter().all(|line| line.starts_with("Cry:")))
            };
            let wanted = match (cry_only(&texts[0]), cry_only(&texts[1])) {
                (Some(true), Some(false)) => Some("base"),
                (Some(false), Some(true)) => Some("radiant"),
                (Some(true), _) | (_, Some(true)) => Some("true"),
                _ => None,
            };
            let face_name = |face: &str| if face == "radiant" { "Radiant" } else { "base" };
            let message = match (wanted, marked) {
                (_, None) => Some(format!(
                    "`cry` is `true`, \"base\" or \"radiant\" when present (got {})",
                    describe(cry)
                )),
                (wanted, Some(marked)) if wanted == marked => None,
                (Some("true"), _) => Some(format!(
                    "every line that writes {{{key}}} is a Cry, so it is `cry: true`"
                )),
                (Some(face), _) => Some(format!(
                    "only the {} face's Cry reads {{{key}}}, so it is `cry: \"{face}\"`",
                    face_name(face)
                )),
                (None, Some(Some("true"))) => {
                    Some(format!("`cry: true`, but a line that writes {{{key}}} is no Cry"))
                }
                (None, Some(Some(face))) => Some(format!(
                    "`cry: \"{face}\"`, but the {} face's Cry alone does not read {{{key}}}",
                    face_name(face)
                )),
                (None, Some(None)) => None,
            };
            if let Some(message) = message {
                fail(failures, &p_at, &format!("{key}: {message}"));
            }
            let extra: Vec<&str> = param
                .keys()
                .map(String::as_str)
                .filter(|k| {
                    ![
                        "key", "base", "radiant", "better", "step", "min", "max", "tunedOn", "power", "cry",
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

/// R1748001: `dealsDamage` marks a Spell or a Field Spell whose text deals damage, so a Buff may give it
/// Pierce or Lifesteal. R1748002: `usesPlague` marks a Field Spell whose text names Plague Counters, so
/// on the field a Buff places one on it and a Nerf removes one. Each is `true` when present.
fn validate_buff_targets(failures: &mut Vec<String>, at: &str, value: &Object) {
    let card_type = value.get("type").and_then(Json::as_str).unwrap_or_default();
    let text: String = ["base", "radiant"]
        .iter()
        .filter_map(|face| match value.get(*face) {
            Some(Json::Object(face)) => face.get("text").and_then(Json::as_str),
            _ => None,
        })
        .collect::<Vec<&str>>()
        .join("\n");
    let marks: [(&str, &[&str], &str); 2] = [
        ("dealsDamage", &["Spell", "Field Spell"], "damage"),
        ("usesPlague", &["Field Spell"], "Plague Counter"),
    ];
    for (field, types, named) in marks {
        let Some(mark) = value.get(field) else {
            continue;
        };
        if mark != &Json::Bool(true) {
            fail(
                failures,
                at,
                &format!("`{field}` is `true` when present (got {})", describe(Some(mark))),
            );
        }
        if !types.contains(&card_type) {
            fail(
                failures,
                at,
                &format!("`{field}` marks a {}, not a {card_type}", types.join(" or a ")),
            );
        }
        if !text.contains(named) {
            fail(
                failures,
                at,
                &format!("`{field}`, but no face's text says \"{named}\""),
            );
        }
    }
}

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
    check_catalog_against(catalog, sets())
}

/// `check_catalog` against these set expectations: `sets()`, or in a test those and a stand-in set
/// that does not ship, while the catalog holds none (R1420).
fn check_catalog_against(catalog: &Object, sets: Vec<SetExpectation>) -> CatalogReport {
    // R1420: the totals are the shipped catalog's.
    let expected_non_token: usize = sets
        .iter()
        .filter(|set| set.ships())
        .map(|set| set.cards + set.shared_cards.len())
        .sum();
    let expected_token: usize = sets
        .iter()
        .filter(|set| set.ships())
        .map(|set| set.card_defined_tokens.len() + set.shared_tokens.len())
        .sum();
    let expected_total = expected_non_token + expected_token;
    let (type_union, set_union, tag_union, rarity_union) = (card_types(), set_names(), tags(), rarities());
    let printed_union = printed_rarities();
    let flavor_union = flavors();
    let mut failures: Vec<String> = Vec::new();
    let entries = catalog.len();

    // 1. The census's entry total, R1420's unshipped sets aside. (TS checks the total here and again
    // after the loop, so a wrong total fails twice; kept.)
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

        // R1437000: every entry carries one since #1436's part 5.
        if !value
            .get("flavor")
            .and_then(Json::as_str)
            .is_some_and(|flavor| flavor_union.contains(&flavor))
        {
            fail(
                &mut failures,
                &at,
                &format!("`flavor` is not a Flavor (got {})", describe(value.get("flavor"))),
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

        // R1748001, R1748002: the Buff targets a card's data names.
        validate_buff_targets(&mut failures, &at, value);

        let known = [
            "id",
            "index",
            "name",
            "set",
            "type",
            "tags",
            "rarity",
            "printedRarity",
            "flavor",
            "token",
            "cost",
            "refs",
            "params",
            "dealsDamage",
            "usesPlague",
            // R1840000: a Resource's rarity and prices (`CardDef.resource`).
            "resource",
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

    // 1, 2. The totals: the census's sums over the sets that ship (R1420).
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
pub(crate) enum Quote {
    Plain,
    Raw(usize),
}

/// An identifier character, which a raw string's `r` may not follow.
/// A raw string opening at `i` (`r"`, `r#"`, `br"`, `cr##"` …): its length up to and including
/// the quote, and its `#` count.
pub(crate) fn raw_string_open(chars: &[char], i: usize) -> Option<(usize, usize)> {
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
pub(crate) fn char_literal_length(chars: &[char], i: usize) -> usize {
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
    code_lines(source).len()
}

/// The lines that hold code once comments are removed, each trimmed, above the file's first
/// `#[cfg(test)]`, its `use` declarations included: what `classify` compares for a Rust or
/// TypeScript file that is no card script, where an import can change which code a call reaches
/// (R2050000).
pub fn code_lines_with_uses(source: &str) -> Vec<String> {
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
    code_lines
        .into_iter()
        // A card's tests sit below its script (SURFACE §7.3) and are not its lines of code.
        .take_while(|line| !line.starts_with("#[cfg(test)]"))
        .filter(|line| !line.is_empty())
        .collect()
}

/// The lines `count_loc` counts, each trimmed: `code_lines_with_uses` without its `use`
/// declarations, what `classify` compares to tell a change to a card script's code from one to its
/// comments, tests, imports or layout (R1365000).
pub fn code_lines(source: &str) -> Vec<String> {
    let mut code = Vec::new();
    let mut in_use = false;
    for line in code_lines_with_uses(source) {
        if !in_use && starts_use(&line) {
            in_use = true;
        }
        if in_use {
            if line.ends_with(';') {
                in_use = false;
            }
            continue;
        }
        code.push(line);
    }
    code
}

/// A Luau long bracket's opening at `i` (`[[`, `[=[`, …): its length and its level, the `=`s.
fn long_bracket_open(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if chars.get(i) != Some(&'[') {
        return None;
    }
    let level = chars[i + 1..].iter().take_while(|c| **c == '=').count();
    (chars.get(i + 1 + level) == Some(&'[')).then_some((level + 2, level))
}

/// Whether a long bracket of `level` closes at `i` (`]]`, `]=]`, …).
fn long_bracket_closes(chars: &[char], i: usize, level: usize) -> bool {
    chars.get(i) == Some(&']')
        && chars.len() > i + level + 1
        && chars[i + 1..=i + level].iter().all(|c| *c == '=')
        && chars[i + 1 + level] == ']'
}

/// A module import, the Luau `use`: `local J = require("@jackioh")`.
fn is_require(code: &str) -> bool {
    code.strip_prefix("local ")
        .and_then(|rest| rest.split_once('='))
        .is_some_and(|(_, value)| value.trim_start().starts_with("require("))
}

/// SURFACE §7.5 for a Luau card script (#442): the lines that hold code once comments are removed
/// (`--` to the line's end, `--[[ … ]]` blocks of any level, the `--!strict` directive among them),
/// except its `require` imports, as `count_loc` leaves out `use` declarations. Strings (quoted,
/// interpolated and long) are read as such, so a `--` inside one is code. The file is the script
/// alone: its tests are in the `.rs` beside it.
pub fn count_luau_loc(source: &str) -> usize {
    #[derive(Clone, Copy)]
    enum In {
        Code,
        Comment(usize),
        LongString(usize),
        Quoted(char),
    }
    let mut state = In::Code;
    let mut count = 0;
    for line in source.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        let mut code = String::new();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            match state {
                In::Comment(level) => {
                    if long_bracket_closes(&chars, i, level) {
                        state = In::Code;
                        i += level + 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                In::LongString(level) => {
                    code.push(ch);
                    if long_bracket_closes(&chars, i, level) {
                        state = In::Code;
                        i += level + 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                In::Quoted(quote) => {
                    code.push(ch);
                    if ch == '\\' {
                        if let Some(next) = chars.get(i + 1) {
                            code.push(*next);
                        }
                        i += 2;
                        continue;
                    }
                    if ch == quote {
                        state = In::Code;
                    }
                    i += 1;
                    continue;
                }
                In::Code => {}
            }
            if ch == '-' && chars.get(i + 1) == Some(&'-') {
                match long_bracket_open(&chars, i + 2) {
                    Some((length, level)) => {
                        state = In::Comment(level);
                        i += 2 + length;
                        continue;
                    }
                    None => break,
                }
            }
            if let Some((length, level)) = long_bracket_open(&chars, i) {
                code.extend(&chars[i..i + length]);
                state = In::LongString(level);
                i += length;
                continue;
            }
            if matches!(ch, '"' | '\'' | '`') {
                state = In::Quoted(ch);
            }
            code.push(ch);
            i += 1;
        }
        // A quoted string ends with its line (a `\z` continuation aside, which no script uses).
        if matches!(state, In::Quoted(_)) {
            state = In::Code;
        }
        let code = code.trim();
        if !code.is_empty() && !is_require(code) {
            count += 1;
        }
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
        /// The card's `.rs` script file (a `.luau` script counts by Luau's comment rules).
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
            let loc = if path.extension().is_some_and(|ext| ext == "luau") {
                count_luau_loc(&source)
            } else {
                count_loc(&source)
            };
            println!("{loc}");
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
            expectation.cards + expectation.shared_cards.len(),
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
    use jackioh_cards::census::SetCensus;

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
        // The totals are the census's sums over the sets that ship (R1420, #2581).
        let shipped = || CENSUS.iter().filter(|census| census.ships());
        let cards: usize = shipped().map(SetCensus::card_count).sum();
        let tokens: usize = shipped().map(SetCensus::token_count).sum();
        assert_eq!(
            (report.entries, report.non_token_count, report.token_count),
            (cards + tokens, cards, tokens)
        );
    }

    /// An entry of `set` made from Core #2's (a Common Unit): its id, index, name and set its own.
    fn fixture_entry(set: &str, index: &str, id: &str) -> Json {
        let Json::Object(mut entry) = shipped_catalog()["core-002"].clone() else {
            panic!("core-002 is an object");
        };
        entry.insert("id".to_string(), Json::from(id));
        entry.insert("index".to_string(), Json::from(index));
        entry.insert("name".to_string(), Json::from(format!("{set} fixture {index}")));
        entry.insert("set".to_string(), Json::from(set));
        Json::Object(entry)
    }

    /// The shipped catalog with one field of one entry written over.
    fn with_field(id: &str, field: &str, value: Json) -> Object {
        let mut catalog = shipped_catalog();
        let Some(Json::Object(entry)) = catalog.get_mut(id) else {
            panic!("{id} is an object");
        };
        entry.insert(field.to_string(), value);
        catalog
    }

    #[test]
    fn r1178000_holds_the_meditative_plus_set_to_every_count_since_it_shipped() {
        // Since its release (#1187) its entries count toward the totals, and none is held apart: the
        // entries held apart are a held set's alone (R1857000's Stinkyscape, while it is held).
        let shipped = check_catalog(&shipped_catalog());
        assert_eq!(shipped.failures, Vec::<String>::new());
        let held = shipped_catalog()
            .values()
            .filter(|entry| {
                let set = entry
                    .as_object()
                    .and_then(|entry| entry.get("set"))
                    .and_then(Json::as_str);
                !SetName::ALL
                    .iter()
                    .any(|known| Some(known.as_str()) == set && set_ships(*known))
            })
            .count();
        assert_eq!(shipped.unshipped_count, held);
        // Stinkyscape ships, so leaving it out holds no more apart: only the held Training Grounds set
        // is (R2194000).
        assert_eq!(check_catalog(&without_stinkyscape()).unshipped_count, held);

        // Its census's counts, which the failures below name.
        let rarity = |name: &str| {
            CENSUS
                .iter()
                .find(|census| census.set == "Meditative+")
                .and_then(|census| census.rarities.iter().find(|(rarity, _)| *rarity == name))
                .map(|(_, count)| *count)
                .expect("the census counts Meditative+'s rarities")
        };
        let (common, epic) = (rarity("Common"), rarity("Epic"));
        let potions = tag_count(Tag::Potion);

        // So a card it lacks fails, as any shipped set's would: the totals and the index.
        let mut missing = shipped_catalog();
        missing.shift_remove("meditativeplus-002");
        let failures = check_catalog(&missing).failures;
        for expected in [
            format!(
                "catalog: expected {} entries, found {}",
                shipped.entries,
                shipped.entries - 1
            ),
            "Meditative+: index \"2\" is missing".to_string(),
            format!(
                "Meditative+: expected {epic} Epic non-token cards, found {}",
                epic - 1
            ),
        ] {
            assert!(failures.contains(&expected), "{expected} in {failures:#?}");
        }

        // Its rarities are held to their counts both ways: M+ #2, an Epic, made a sixth Common.
        assert_eq!(
            check_catalog(&with_field("meditativeplus-002", "rarity", Json::from("Common"))).failures,
            [
                format!(
                    "Meditative+: expected {common} Common non-token cards, found {}",
                    common + 1
                ),
                format!(
                    "Meditative+: expected {epic} Epic non-token cards, found {}",
                    epic - 1
                )
            ]
        );

        // Its tags are in the census: M+ #11 Living Mana Potion without its Potion tag.
        assert_eq!(
            check_catalog(&with_field("meditativeplus-011", "tags", Json::Array(Vec::new()))).failures,
            [format!(
                "catalog: expected {potions} entries tagged \"Potion\", found {}",
                potions - 1
            )]
        );

        // An index the brief does not list (#27, a sixth helper #1.6, #2521's #1.5 Star Chart being
        // the fifth), or an id that does not follow its index, still fails.
        let mut wrong = shipped_catalog();
        for (index, id) in [
            ("27", "meditativeplus-027"),
            ("1.6", "meditativeplus-001-6"),
            ("3", "meditativeplus-x"),
        ] {
            wrong.insert(id.to_string(), fixture_entry("Meditative+", index, id));
        }
        let failures = check_catalog(&wrong).failures.join("\n");
        assert!(failures.contains("unexpected index \"27\""), "{failures}");
        assert!(failures.contains("unexpected index \"1.6\""), "{failures}");
        assert!(
            failures.contains("implies id \"meditativeplus-003\""),
            "{failures}"
        );
    }

    /// A token entry of `set` made from the Sheep Token's: its id, index, name and set its own.
    fn fixture_token(set: &str, index: &str, id: &str) -> Json {
        let Json::Object(mut entry) = shipped_catalog()["core-t-sheep"].clone() else {
            panic!("core-t-sheep is an object");
        };
        entry.insert("id".to_string(), Json::from(id));
        entry.insert("index".to_string(), Json::from(index));
        entry.insert("name".to_string(), Json::from(format!("{set} fixture {index}")));
        entry.insert("set".to_string(), Json::from(set));
        Json::Object(entry)
    }

    /// The shipped catalog without any Stinkyscape entry: the catalog as #1857's foundation found it,
    /// whichever of the set's entries have landed since.
    fn without_stinkyscape() -> Object {
        let mut catalog = shipped_catalog();
        catalog.retain(|_, value| {
            value
                .as_object()
                .and_then(|entry| entry.get("set"))
                .and_then(Json::as_str)
                != Some("Stinkyscape")
        });
        catalog
    }

    #[test]
    fn r1857000_holds_the_stinkyscape_set_to_its_shape_alone_until_it_ships() {
        if set_ships(SetName::Stinkyscape) {
            // From its release (#1884) it is held to every count, as Meditative+ is (R1178000).
            return;
        }
        let expectation = sets()
            .into_iter()
            .find(|set| set.set == "Stinkyscape")
            .expect("the catalog holds the Stinkyscape set");
        // R1857001's shape: 177 cards (175 until #2521), the Dormant Starship, 30 tokens the cards
        // define and 22 shared.
        assert_eq!(
            (
                expectation.cards,
                expectation.shared_cards.len(),
                expectation.card_defined_tokens.len(),
                expectation.shared_tokens.len()
            ),
            (177, 1, 30, 22)
        );
        assert_eq!(expected_indices(&expectation).len(), 230);
        assert_eq!(
            id_for_index("stinkyscape", "T-dormant-starship"),
            jackioh_engine::DORMANT_STARSHIP_DEF_ID
        );
        assert_eq!(id_for_index("stinkyscape", "T-AI-1"), "stinkyscape-t-ai-01");
        assert_eq!(id_for_index("stinkyscape", "137.4"), "stinkyscape-137-4");

        // Whatever of it has landed, it counts toward no total and no tag.
        let shipped = check_catalog(&shipped_catalog());
        assert_eq!(shipped.failures, Vec::<String>::new());
        let bare = check_catalog(&without_stinkyscape());
        assert_eq!(bare.failures, Vec::<String>::new());
        let totals = |report: &CatalogReport| (report.entries, report.non_token_count, report.token_count);
        assert_eq!(totals(&shipped), totals(&bare));

        // A card, the Dormant Starship (a card at a T- index), a token a card defines and a shared
        // token, its tags among them: all pass, outside the totals and the census.
        let mut held = without_stinkyscape();
        let mut drone_core = fixture_entry("Stinkyscape", "27", "stinkyscape-027");
        if let Json::Object(entry) = &mut drone_core {
            entry.insert("tags".to_string(), Json::Array(vec![Json::from("Drone Core")]));
        }
        held.insert("stinkyscape-027".to_string(), drone_core);
        let mut hull = fixture_entry(
            "Stinkyscape",
            "T-dormant-starship",
            "stinkyscape-t-dormant-starship",
        );
        if let Json::Object(entry) = &mut hull {
            entry.insert("rarity".to_string(), Json::from("Legendary"));
        }
        held.insert("stinkyscape-t-dormant-starship".to_string(), hull);
        for (index, id) in [("137.1", "stinkyscape-137-1"), ("T-AI-1", "stinkyscape-t-ai-01")] {
            held.insert(id.to_string(), fixture_token("Stinkyscape", index, id));
        }
        let report = check_catalog(&held);
        assert_eq!(report.failures, Vec::<String>::new());
        assert_eq!(totals(&report), totals(&bare));
        assert_eq!(report.unshipped_count, bare.unshipped_count + 4);

        // No rarity passes its ceiling: a 42nd Common is refused while the set is built.
        let mut commons = without_stinkyscape();
        for n in 1..=42 {
            let id = format!("stinkyscape-{n:03}");
            commons.insert(id.clone(), fixture_entry("Stinkyscape", &n.to_string(), &id));
        }
        assert_eq!(
            check_catalog(&commons).failures,
            ["Stinkyscape: expected 41 Common non-token cards, found 42"]
        );

        // An index its brief does not list, or an id that does not follow its index, fails.
        let mut wrong = without_stinkyscape();
        for (index, id) in [
            ("178", "stinkyscape-178"),
            ("4.7", "stinkyscape-004-7"),
            ("T-starship", "stinkyscape-t-starship"),
            ("3", "stinkyscape-x"),
        ] {
            wrong.insert(id.to_string(), fixture_entry("Stinkyscape", index, id));
        }
        wrong.insert(
            "stinkyscape-t-ai-1".to_string(),
            fixture_token("Stinkyscape", "T-AI-1", "stinkyscape-t-ai-1"),
        );
        let failures = check_catalog(&wrong).failures.join("\n");
        for expected in [
            "Stinkyscape: unexpected index \"178\"",
            "Stinkyscape: unexpected index \"4.7\"",
            "Stinkyscape: unexpected index \"T-starship\"",
            "implies id \"stinkyscape-003\"",
            "implies id \"stinkyscape-t-ai-01\"",
        ] {
            assert!(failures.contains(expected), "{expected} in {failures}");
        }
    }

    /// R1420's stand-in for a set being built, since the catalog holds none once Meditative+ ships
    /// (R1178000): `sets()` and the reserved Boss set, which `SHIPPED_SETS` does not list, with a
    /// brief of three cards (one Common, two Rare) and the token #1.1.
    fn with_a_held_set() -> Vec<SetExpectation> {
        let mut sets = sets();
        sets.push(SetExpectation {
            set: "Boss",
            segment: "boss",
            cards: 3,
            shared_cards: Vec::new(),
            card_defined_tokens: texts(&["1.1"]),
            shared_tokens: Vec::new(),
            rarities: &[("Common", 1), ("Rare", 2)],
        });
        sets
    }

    #[test]
    fn r1420_holds_a_set_being_built_to_its_shape_alone_outside_the_totals() {
        assert!(!set_ships(SetName::Boss));
        let shipped = check_catalog_against(&shipped_catalog(), with_a_held_set());
        assert_eq!(shipped.failures, Vec::<String>::new());

        // Its brief's entries need not all be there, and those that are count toward no total.
        let mut held = shipped_catalog();
        held.insert("boss-002".to_string(), fixture_entry("Boss", "2", "boss-002"));
        let report = check_catalog_against(&held, with_a_held_set());
        assert_eq!(report.failures, Vec::<String>::new());
        assert_eq!(
            (
                report.entries,
                report.non_token_count,
                report.token_count,
                report.unshipped_count
            ),
            (
                shipped.entries,
                shipped.non_token_count,
                shipped.token_count,
                shipped.unshipped_count + 1
            )
        );

        // No rarity passes its count: a second Common is refused while the set is built.
        let mut commons = held.clone();
        commons.insert("boss-001".to_string(), fixture_entry("Boss", "1", "boss-001"));
        assert_eq!(
            check_catalog_against(&commons, with_a_held_set()).failures,
            ["Boss: expected 1 Common non-token cards, found 2"]
        );

        // An index its brief does not list, or an id that does not follow it, fails, as it does for
        // a set that ships (the Meditative set's, from v0.3.5).
        let mut wrong = held;
        wrong.insert("boss-004".to_string(), fixture_entry("Boss", "4", "boss-004"));
        wrong.insert("boss-x".to_string(), fixture_entry("Boss", "3", "boss-x"));
        wrong.insert(
            "meditative-150".to_string(),
            fixture_entry("Meditative", "150", "meditative-150"),
        );
        wrong.insert(
            "meditative-x".to_string(),
            fixture_entry("Meditative", "3", "meditative-x"),
        );
        let failures = check_catalog_against(&wrong, with_a_held_set())
            .failures
            .join("\n");
        assert!(failures.contains("Boss: unexpected index \"4\""), "{failures}");
        assert!(failures.contains("implies id \"boss-003\""), "{failures}");
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
        entry.insert("color".to_string(), Json::from("red"));
        catalog.insert("core-002".to_string(), Json::Object(entry));
        let failures = check_catalog(&catalog).failures;
        let at = "catalog[\"core-002\"]";
        for expected in [
            format!("{at}.base: Unit face must carry an integer `attack` (got undefined)"),
            format!("{at}.params[0]: amount: no face's text writes {{amount}}"),
            format!("{at}: unknown field(s) color"),
        ] {
            assert!(failures.contains(&expected), "{expected} in {failures:#?}");
        }
        assert_eq!(failures.len(), 3, "{failures:#?}");
    }

    #[test]
    fn r2280000_an_animated_face_may_print_no_stats_but_not_half_a_line() {
        // The shipped catalog's statless Animated faces (S #16 Rod of Ages, C+ #74) pass.
        assert_eq!(check_catalog(&shipped_catalog()).failures, Vec::<String>::new());
        let mut catalog = shipped_catalog();
        // C #5 Tesla, an Animated Field Trap, with its base face's attack gone and its health kept.
        let mut entry = catalog["classic-005"].as_object().expect("classic-005").clone();
        if let Some(Json::Object(base)) = entry.get_mut("base") {
            base.shift_remove("attack");
        }
        catalog.insert("classic-005".to_string(), Json::Object(entry));
        assert_eq!(
            check_catalog(&catalog).failures,
            [
                "catalog[\"classic-005\"].base: an Animated face prints both `attack` and `health` or neither (R2280000)"
            ]
        );
    }

    // R1437000: a flavor is one of the fifteen names, and every entry carries one since
    // #1436's part 5.
    #[test]
    fn r1437000_a_flavor_is_one_of_the_fifteen_and_every_entry_carries_one() {
        assert_eq!(check_catalog(&shipped_catalog()).failures, Vec::<String>::new());
        for flavor in Flavor::ALL {
            let catalog = with_field("core-002", "flavor", Json::from(flavor.as_str()));
            assert_eq!(check_catalog(&catalog).failures, Vec::<String>::new(), "{flavor}");
        }
        for name in ["Water", "fire"] {
            let catalog = with_field("core-002", "flavor", Json::from(name));
            assert_eq!(
                check_catalog(&catalog).failures,
                vec![format!(
                    "catalog[\"core-002\"]: `flavor` is not a Flavor (got \"{name}\")"
                )]
            );
        }
        // An entry with none fails, naming the entry and showing `undefined`.
        let mut catalog = shipped_catalog();
        match catalog.get_mut("core-002") {
            Some(Json::Object(entry)) => {
                entry.shift_remove("flavor");
            }
            _ => panic!("core-002 is an object"),
        }
        assert_eq!(
            check_catalog(&catalog).failures,
            ["catalog[\"core-002\"]: `flavor` is not a Flavor (got undefined)"]
        );
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
    fn r824000_a_number_only_the_radiant_face_writes_is_tuned_there_alone() {
        let marked = r#"[{"key":"discount","base":1,"radiant":1,"better":"up","tunedOn":"radiant"}]"#;
        let unmarked = r#"[{"key":"discount","base":1,"radiant":1,"better":"up"}]"#;
        let (plain, shining) = ("Discover a card.", "Discover a card. It costs ({discount}) less.");
        // Jewelosco Scarab's shape: the Radiant face alone writes the number, tuned there alone.
        assert_eq!(params_failures(marked, plain, shining), Vec::<String>::new());
        // Unmarked, it is refused.
        assert_eq!(
            params_failures(unmarked, plain, shining),
            [
                "x.params[0]: discount: only the Radiant face's text writes {discount}, so it is `tunedOn: \"radiant\"`"
            ]
        );
        // Both faces write it: unmarked passes, marked is refused.
        assert_eq!(params_failures(unmarked, shining, shining), Vec::<String>::new());
        assert_eq!(
            params_failures(marked, shining, shining),
            [
                "x.params[0]: discount: tuned on the Radiant face only, but the base face's text writes {discount}"
            ]
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
    fn r1748000_a_number_only_a_cry_writes_is_cry_and_a_cry_number_is_written_only_by_a_cry() {
        let marked = r#"[{"key":"draw","base":1,"radiant":2,"better":"up","cry":true}]"#;
        let unmarked = r#"[{"key":"draw","base":1,"radiant":2,"better":"up"}]"#;
        let (cry, cry_radiant) = ("Cry: Draw {draw}.", "Rush\\nCry: Draw {draw}.");
        let lasting = "Start of turn: Draw {draw}.";
        let both = "Cry: Draw {draw}.\\nStart of turn: Draw {draw}.";
        // Every line that writes it is a Cry: marked, it passes; unmarked, it is refused.
        assert_eq!(params_failures(marked, cry, cry_radiant), Vec::<String>::new());
        assert_eq!(
            params_failures(unmarked, cry, cry_radiant),
            ["x.params[0]: draw: every line that writes {draw} is a Cry, so it is `cry: true`"]
        );
        // A line that is no Cry writes it on each face, beside a Cry or not: unmarked, it passes;
        // marked, it is refused. (A Cry on one face beside a lasting text on the other is R2261001's.)
        assert_eq!(params_failures(unmarked, lasting, lasting), Vec::<String>::new());
        assert_eq!(params_failures(unmarked, both, both), Vec::<String>::new());
        assert_eq!(
            params_failures(marked, both, lasting),
            ["x.params[0]: draw: `cry: true`, but a line that writes {draw} is no Cry"]
        );
        // `cry` is `true`, a face (R2261001) or absent.
        assert_eq!(
            params_failures(
                r#"[{"key":"draw","base":1,"radiant":2,"better":"up","cry":false}]"#,
                cry,
                cry_radiant
            ),
            ["x.params[0]: draw: `cry` is `true`, \"base\" or \"radiant\" when present (got false)"]
        );
    }

    #[test]
    fn r2261001_a_number_only_one_face_s_cry_reads_names_that_face() {
        let on = |mark: &str| format!(r#"[{{"key":"lucky","base":1,"radiant":1,"better":"up"{mark}}}]"#);
        let cry = "Cry: Give it Lucky {lucky}.";
        let activate = "Activate: Give it Lucky {lucky}.";
        // Meditative #101 Gachaholic's shape: the base face's Cry and the Radiant face's Activate.
        assert_eq!(
            params_failures(&on(r#","cry":"base""#), cry, activate),
            Vec::<String>::new()
        );
        assert_eq!(
            params_failures(&on(r#","cry":"radiant""#), activate, cry),
            Vec::<String>::new()
        );
        assert_eq!(
            params_failures(&on(""), cry, activate),
            ["x.params[0]: lucky: only the base face's Cry reads {lucky}, so it is `cry: \"base\"`"]
        );
        assert_eq!(
            params_failures(&on(r#","cry":true"#), cry, activate),
            ["x.params[0]: lucky: only the base face's Cry reads {lucky}, so it is `cry: \"base\"`"]
        );
        // The other face, every face's Cry, or no face's Cry alone: not that face.
        assert_eq!(
            params_failures(&on(r#","cry":"radiant""#), cry, activate),
            ["x.params[0]: lucky: only the base face's Cry reads {lucky}, so it is `cry: \"base\"`"]
        );
        assert_eq!(
            params_failures(&on(r#","cry":"base""#), cry, cry),
            ["x.params[0]: lucky: every line that writes {lucky} is a Cry, so it is `cry: true`"]
        );
        assert_eq!(
            params_failures(&on(r#","cry":"radiant""#), activate, activate),
            [
                "x.params[0]: lucky: `cry: \"radiant\"`, but the Radiant face's Cry alone does not read {lucky}"
            ]
        );
    }

    #[test]
    fn r1748001_deals_damage_marks_only_a_spell_or_field_spell_that_deals_damage() {
        let failures = |id: &str, value: Json| check_catalog(&with_field(id, "dealsDamage", value)).failures;
        // The shipped catalog marks Lunar Eclipse, a Spell that deals damage, and passes.
        assert_eq!(failures("core-035", Json::Bool(true)), Vec::<String>::new());
        // Mana Well, a Field Spell that deals none.
        assert_eq!(
            failures("core-006", Json::Bool(true)),
            ["catalog[\"core-006\"]: `dealsDamage`, but no face's text says \"damage\""]
        );
        // Bigot, a Unit that deals none.
        assert_eq!(
            failures("core-002", Json::Bool(true)),
            [
                "catalog[\"core-002\"]: `dealsDamage` marks a Spell or a Field Spell, not a Unit",
                "catalog[\"core-002\"]: `dealsDamage`, but no face's text says \"damage\"",
            ]
        );
        assert_eq!(
            failures("core-035", Json::Bool(false)),
            ["catalog[\"core-035\"]: `dealsDamage` is `true` when present (got false)"]
        );
    }

    #[test]
    fn r1748002_uses_plague_marks_only_a_field_spell_that_names_plague_counters() {
        let failures = |id: &str| check_catalog(&with_field(id, "usesPlague", Json::Bool(true))).failures;
        // Corpse Plantation, marked in the shipped catalog, passes.
        assert_eq!(failures("classic-074"), Vec::<String>::new());
        // Living Bomb names Plague Counters, but on other permanents: still a Field Spell that names them.
        assert_eq!(failures("classic-062"), Vec::<String>::new());
        // Mana Well names none.
        assert_eq!(
            failures("core-006"),
            ["catalog[\"core-006\"]: `usesPlague`, but no face's text says \"Plague Counter\""]
        );
        // Outbreak is a Spell: it never stays on the field to carry them.
        assert_eq!(
            failures("classic-039"),
            ["catalog[\"classic-039\"]: `usesPlague` marks a Field Spell, not a Spell"]
        );
    }

    #[test]
    fn r1840000_catalog_check_admits_resource_data() {
        // The Coin, a (0) Spell token, made a Resource: the check admits the field.
        let resource = parse_object(
            r#"{"rarity":"Common","base":{"sellMin":1,"sellMax":3,"buy":3},"radiant":{"sellMin":10,"sellMax":20,"buy":20}}"#,
        );
        let catalog = with_field("core-t-coin", "resource", Json::Object(resource));
        assert_eq!(check_catalog(&catalog).failures, Vec::<String>::new());
        // A field it does not know still fails.
        assert_eq!(
            check_catalog(&with_field("core-t-coin", "resourceTray", Json::Bool(true))).failures,
            ["catalog[\"core-t-coin\"]: unknown field(s) resourceTray"]
        );
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
    fn r2050000_code_lines_with_uses_keeps_use_declarations_and_drops_comments_tests_and_blank_lines() {
        let source = "//! A module.\n\nuse crate::a::{\n    f,\n};\n\n/// It.\npub fn g() -> i32 {\n    f() // the call\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n";
        assert_eq!(
            code_lines_with_uses(source),
            ["use crate::a::{", "f,", "};", "pub fn g() -> i32 {", "f()", "}"]
        );
        assert_eq!(code_lines(source), ["pub fn g() -> i32 {", "f()", "}"]);
    }

    #[test]
    fn count_luau_loc_counts_code_without_comments_blank_lines_or_the_require() {
        let source = r##"--!strict
local J = require("@jackioh")

--[[ A block comment,
     two lines long. ]]
local function cry(ctx: J.EffectContext) -- a trailing comment
	return { J.draw({ count = ctx:param("draw") }) }
end
--[==[ a level-2 block ]] still the comment ]==]
local TEXT = "a -- inside a string"
local LONG = [[
-- a long string's line is code
]]

return { base = { cry = cry }, radiant = "base" }
"##;
        assert_eq!(count_luau_loc(source), 8);
    }

    #[test]
    fn catalog_version_is_the_newest_patch() {
        let path = repo_root().join("crates/cards/patches/patches.json");
        assert_eq!(newest_version(&path).unwrap(), jackioh_cards::catalog_version());
    }

    /// One catalog entry's `loc` against SPEC §5 and SURFACE §7.5: every entry carries one, and one
    /// written since the newest shipped snapshot (an entry the snapshot lacks, `shipped` `None`, or a
    /// `loc` other than the snapshot's) is its script's lines of code as `catalog loc` prints them,
    /// `counted`. A shipped `loc` is frozen: a refactor of its script leaves it as it is.
    fn loc_failure(
        id: &str,
        loc: Option<i32>,
        shipped: Option<Option<i32>>,
        counted: Option<usize>,
    ) -> Option<String> {
        let Some(loc) = loc else {
            return Some(format!("{id}: no `loc`"));
        };
        if shipped == Some(Some(loc)) {
            return None;
        }
        match counted {
            Some(counted) if usize::try_from(loc).ok() == Some(counted) => None,
            Some(counted) => Some(format!(
                "{id}: `loc` is {loc}, its script's lines of code {counted} (`cargo jackioh catalog loc`)"
            )),
            None => Some(format!(
                "{id}: no script under crates/cards/src/scripts/ has this `ID`"
            )),
        }
    }

    /// Every card script's lines of code (`count_loc`) by its `pub const ID`, read off
    /// `crates/cards/src/scripts/<set>/`.
    fn script_locs() -> IndexMap<String, usize> {
        let mut locs = IndexMap::new();
        let scripts = repo_root().join("crates/cards/src/scripts");
        for set in fs::read_dir(&scripts).expect("crates/cards/src/scripts reads") {
            let set = set.expect("a set directory").path();
            if !set.is_dir() {
                continue;
            }
            for file in fs::read_dir(&set).expect("a set directory reads") {
                let path = file.expect("a script").path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                    continue;
                }
                let source = fs::read_to_string(&path).expect("a script reads");
                if let Some(id) = crate::classify::script_id(&source) {
                    locs.insert(id, count_loc(&source));
                }
            }
        }
        locs
    }

    #[test]
    fn a_loc_written_since_the_newest_snapshot_is_its_scripts_count_and_a_shipped_one_is_frozen() {
        let wrong = |loc: i32, counted: usize| {
            Some(format!(
                "x-001: `loc` is {loc}, its script's lines of code {counted} (`cargo jackioh catalog loc`)"
            ))
        };
        // A new entry, which the snapshot lacks, is its script's count.
        assert_eq!(loc_failure("x-001", Some(7), None, Some(7)), None);
        assert_eq!(loc_failure("x-001", Some(8), None, Some(7)), wrong(8, 7));
        // A missing `loc` fails, new or shipped.
        assert_eq!(
            loc_failure("x-001", None, None, Some(7)),
            Some("x-001: no `loc`".to_string())
        );
        assert_eq!(
            loc_failure("x-001", None, Some(Some(7)), Some(7)),
            Some("x-001: no `loc`".to_string())
        );
        // A shipped `loc` is frozen, whatever its script counts now.
        assert_eq!(loc_failure("x-001", Some(10), Some(Some(10)), Some(23)), None);
        // A hand-edited `loc` is written again: it must be the count.
        assert_eq!(
            loc_failure("x-001", Some(11), Some(Some(10)), Some(23)),
            wrong(11, 23)
        );
        assert_eq!(loc_failure("x-001", Some(23), Some(Some(10)), Some(23)), None);
        // A `loc` the shipped entry lacked is written too.
        assert_eq!(loc_failure("x-001", Some(9), Some(None), Some(7)), wrong(9, 7));
        // A written entry no script declares.
        assert_eq!(
            loc_failure("x-001", Some(7), None, None),
            Some("x-001: no script under crates/cards/src/scripts/ has this `ID`".to_string())
        );
    }

    #[test]
    fn every_catalog_entry_carries_a_loc_and_each_written_one_is_its_scripts_count() {
        let shipped = jackioh_cards::catalog_defs_at(jackioh_cards::CATALOG_VERSION)
            .expect("the newest shipped catalog is carried");
        let locs = script_locs();
        for id in jackioh_cards::CATALOG.keys() {
            assert!(locs.contains_key(id), "{id}: its script was not read");
        }
        let failures: Vec<String> = jackioh_cards::CATALOG
            .iter()
            .filter_map(|(id, def)| {
                loc_failure(
                    id,
                    def.loc,
                    shipped.get(id).map(|old| old.loc),
                    locs.get(id).copied(),
                )
            })
            .collect();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
