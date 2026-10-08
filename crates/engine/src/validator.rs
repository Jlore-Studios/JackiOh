//! The loadout rules L1–L6, as SPEC §9.4 states them, and the draft rules D1–D5 and T1–T3 that a
//! saved deck and a saved trio obey (R250, R252, R641).
//!
//! Since R250 a player keeps up to ten named decks and builds up to five trios from them. A trio is
//! what §9.4 first called a loadout: three decks with no card in common, and L1–L6 are its rules
//! (`validate_loadout`, also exported as `validate_trio`). A Best-of-1 deck is checked by the rules
//! that are about one deck — L2, L3, L5 and L6 (`validate_deck`, R253). Neither is checked at save:
//! a saved deck or trio is a draft, which may be incomplete, hold cards the player does not own, or
//! share cards with another deck of its trio, and it is judged when it is queued. What a save does
//! check is structure (`check_deck_draft`, `check_trio_draft`): a name, at most `DECK_SIZE` deckable
//! cards, at most `MAX_COPIES` of each, and three trio slots that name three different decks.
//!
//! §9.4 asks for "one validator module shared by client and server, at save and again at queue",
//! and this is that module. Three callers need the same answer: the deckbuilder in `apps/web`, so a
//! player sees why a deck is illegal before saving; `saveLoadout` in the server, which is the
//! only authority; and the enqueue path, which re-checks because the collection or the catalog may
//! have moved since the save. A second copy of these rules would drift, and drift here means a deck
//! the builder accepts and the server rejects — or worse, the reverse. So there is one copy, and
//! the client's verdict is UX while the server's is law (§9.3). (Since v0.3.0 the web calls this
//! very module through the WASM bindings, SURFACE §10.1's `validator`.)
//!
//! That is also why §9.4's last line lands on the message: "a queue-time failure names the deck and
//! the card". Every error here carries a human sentence naming both, so the same string can be shown
//! in the builder at save time and returned from the queue endpoint later.
//!
//! Pure and I/O-free, so both sides can run it: no clock, no randomness, no catalog lookup of its
//! own. The caller passes the catalog snapshot and the collection in.
//!
//! Port of `packages/validator/src/index.ts` (part 5). TS's regular expressions over Unicode
//! properties (`\p{Cc}`, `\p{Cf}`, `\p{White_Space}`, `\p{Default_Ignorable_Code_Point}`, `\s` and
//! `String.prototype.trim`) are the range tables at the bottom of this file, read off the engine the
//! TypeScript ran on (Node 22, Unicode 16.0).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::catalog::set_is_open;
use crate::config::{DECK_SIZE, MAX_COPIES};
use crate::wire::{CardDef, CardDefs, Tag, is_js_space, string_union};

/// §9.4: exactly 3 decks per loadout. Not in engine config, so it lives here.
pub const LOADOUT_DECKS: usize = 3;

/// R252: a trio is §9.4's loadout of three decks, so it holds `LOADOUT_DECKS` of them.
pub const TRIO_DECKS: usize = LOADOUT_DECKS;

pub type CardId = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LoadoutDeck {
    /// Optional builder label; messages fall back to `Deck <n>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    pub cards: Vec<CardId>,
}

/// The static, versioned catalog snapshot (§9.4) a loadout is checked against.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub version: String,
    pub cards: CardDefs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub banned: Option<Vec<CardId>>,
}

/// The profile's entitlements projected to quantities; an absent id means none owned.
pub type Collection = IndexMap<CardId, i32>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LoadoutInput {
    pub decks: Vec<LoadoutDeck>,
    pub catalog: CatalogSnapshot,
    pub collection: Collection,
}

string_union! {
    pub enum LoadoutRule {
        L1 = "L1",
        L2 = "L2",
        L3 = "L3",
        L4 = "L4",
        L5 = "L5",
        L6 = "L6",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LoadoutError {
    pub rule: LoadoutRule,
    pub message: String,
    /// 1-based deck index; absent on loadout-wide failures (L1, L4, L5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub deck: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card_id: Option<CardId>,
}

/// TS `{ ok: true } | { ok: false; errors: readonly LoadoutError[] }`: `ok` with no `errors` key,
/// or not `ok` with at least one error. Serialised exactly so.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LoadoutResult {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<LoadoutError>>", optional))]
    pub errors: Vec<LoadoutError>,
}

// --- message helpers: each string exists exactly once -----------------------

/// §9.4: a failure names the deck. The builder's own label wins, else the 1-based position.
fn deck_label(deck: &LoadoutDeck, index: usize) -> String {
    match &deck.name {
        Some(name) => name.clone(),
        None => format!("Deck {}", index + 1),
    }
}

/// §9.4: a failure names the card. `"Name" (id)` when catalogued, else the bare id.
fn card_label(card_id: &str, cards: &CardDefs) -> String {
    match cards.get(card_id) {
        None => format!("\"{card_id}\""),
        Some(def) => format!("\"{}\" ({card_id})", def.name),
    }
}

fn copy_word(count: i32) -> &'static str {
    if count == 1 { "copy" } else { "copies" }
}

/// A deckbuilder reaches "1 card" by deleting, so L2's count is pluralised too.
fn card_word(count: usize) -> &'static str {
    if count == 1 { "card" } else { "cards" }
}

/// `Deck 1`, `Deck 1 and Deck 2`, `Deck 1, Deck 2 and Deck 3`.
fn join_labels(labels: &[String]) -> String {
    if labels.len() <= 1 {
        return labels.first().cloned().unwrap_or_default();
    }
    let (last, rest) = labels.split_last().expect("at least two labels");
    format!("{} and {last}", rest.join(", "))
}

/// Mirrors the engine catalog's private token test (§5.1): the printed flag or the tag.
/// L3 bans both spellings from a deck.
fn is_token(def: &CardDef) -> bool {
    def.token || def.tags.contains(&Tag::Token)
}

// --- L1–L6 (§9.4) ----------------------------------------------------------

/// Checks every loadout rule and collects every failure, so the deckbuilder can show all of
/// them at once. Pure and total: bad-but-typed input returns errors, never an exception.
///
/// Catalog staleness ("update required") is not one of L1–L6; the endpoint compares versions
/// (BUILD M6-T2) and this module only checks membership in the snapshot it was handed.
pub fn validate_loadout(input: &LoadoutInput) -> LoadoutResult {
    check(&input.decks, &input.catalog, &input.collection, Scope::Trio)
}

/// R253: a Best-of-3 trio is §9.4's loadout, so its rules are L1–L6 exactly.
pub fn validate_trio(input: &LoadoutInput) -> LoadoutResult {
    validate_loadout(input)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DeckInput {
    pub deck: LoadoutDeck,
    pub catalog: CatalogSnapshot,
    pub collection: Collection,
}

/// R253: the rules a Best-of-1 deck must pass to be queued — the four of L1–L6 that are about one
/// deck: L2 (exactly `DECK_SIZE` cards), L3 (copies and Tokens), L5 (owned) and L6 (in the catalog,
/// not banned). L1 and L4 are about three decks together and cannot apply to one. Every error names
/// the deck, as 1, so a message and a `deck` field read the same way they do for a trio.
pub fn validate_deck(input: &DeckInput) -> LoadoutResult {
    check(
        std::slice::from_ref(&input.deck),
        &input.catalog,
        &input.collection,
        Scope::Deck,
    )
}

/// `check`'s scope: TS `"trio" | "deck"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Trio,
    Deck,
}

/// One pass over the decks for either scope. `Trio` is L1–L6 over three decks; `Deck` is one
/// deck, where L1 and L4 do not apply and L5's sentence names the deck rather than the trio.
fn check(
    decks: &[LoadoutDeck],
    catalog: &CatalogSnapshot,
    collection: &Collection,
    scope: Scope,
) -> LoadoutResult {
    let mut errors: Vec<LoadoutError> = Vec::new();
    let banned: IndexSet<&str> = catalog.banned.iter().flatten().map(String::as_str).collect();
    let deck_labels: Vec<String> = decks
        .iter()
        .enumerate()
        .map(|(index, deck)| deck_label(deck, index))
        .collect();
    let label_at = |index: usize| -> String {
        deck_labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Deck {}", index + 1))
    };
    let label = |card_id: &str| -> String { card_label(card_id, &catalog.cards) };

    // L1 — exactly LOADOUT_DECKS decks. Loadout-wide, so no `deck` field.
    if scope == Scope::Trio && decks.len() != LOADOUT_DECKS {
        errors.push(LoadoutError {
            rule: LoadoutRule::L1,
            message: format!(
                "A trio needs exactly {LOADOUT_DECKS} decks; this one has {}.",
                decks.len()
            ),
            deck: None,
            card_id: None,
        });
    }

    // Distinct ids in first-appearance order across the whole loadout, for L4 and L5.
    let mut order: Vec<CardId> = Vec::new();
    // Copies of each id across the whole loadout (L5).
    let mut totals: IndexMap<CardId, i32> = IndexMap::new();
    // Which decks hold each id, in deck order and without repeats (L4).
    let mut decks_holding: IndexMap<CardId, Vec<usize>> = IndexMap::new();

    // L1 does not stop the per-deck rules: every supplied deck is checked, even a fourth.
    for (index, deck) in decks.iter().enumerate() {
        let deck_number = index + 1;

        // L2 — exactly DECK_SIZE cards.
        if deck.cards.len() as i32 != DECK_SIZE {
            errors.push(LoadoutError {
                rule: LoadoutRule::L2,
                message: format!(
                    "{} has {} {}; every deck needs exactly {DECK_SIZE}.",
                    label_at(index),
                    deck.cards.len(),
                    card_word(deck.cards.len())
                ),
                deck: Some(deck_number),
                card_id: None,
            });
        }

        let mut deck_order: Vec<CardId> = Vec::new();
        let mut deck_counts: IndexMap<CardId, i32> = IndexMap::new();
        for card_id in &deck.cards {
            let seen = deck_counts.get(card_id).copied();
            if seen.is_none() {
                deck_order.push(card_id.clone());
            }
            deck_counts.insert(card_id.clone(), seen.unwrap_or(0) + 1);

            // A card missing from the snapshot still counts towards L2, L3, L4 and L5.
            let total = totals.get(card_id).copied();
            if total.is_none() {
                order.push(card_id.clone());
            }
            totals.insert(card_id.clone(), total.unwrap_or(0) + 1);

            match decks_holding.get_mut(card_id) {
                None => {
                    decks_holding.insert(card_id.clone(), vec![index]);
                }
                Some(holders) => {
                    if !holders.contains(&index) {
                        holders.push(index);
                    }
                }
            }
        }

        for card_id in &deck_order {
            let count = deck_counts.get(card_id).copied().unwrap_or(0);
            let def = catalog.cards.get(card_id);

            // L3 (tokens) — skipped for an unknown id: there is no def to read. L6 reports that id.
            if let Some(def) = def
                && is_token(def)
            {
                errors.push(LoadoutError {
                    rule: LoadoutRule::L3,
                    message: format!(
                        "{} cannot contain {}: Token cards are never deckable.",
                        label_at(index),
                        label(card_id)
                    ),
                    deck: Some(deck_number),
                    card_id: Some(card_id.clone()),
                });
            } else if let Some(def) = def
                && !set_is_open(def.set)
            {
                // L3 (R1420): a card of a set that has not shipped is in the catalog and in no deck.
                errors.push(LoadoutError {
                    rule: LoadoutRule::L3,
                    message: format!(
                        "{} cannot contain {}: the {} set has not shipped yet.",
                        label_at(index),
                        label(card_id),
                        def.set.as_str()
                    ),
                    deck: Some(deck_number),
                    card_id: Some(card_id.clone()),
                });
            }

            // L3 (copies) — at most MAX_COPIES of a card per deck.
            if count > MAX_COPIES {
                errors.push(LoadoutError {
                    rule: LoadoutRule::L3,
                    message: format!(
                        "{} has {count} {} of {}; at most {MAX_COPIES} {} of a card is allowed per deck.",
                        label_at(index),
                        copy_word(count),
                        label(card_id),
                        copy_word(MAX_COPIES)
                    ),
                    deck: Some(deck_number),
                    card_id: Some(card_id.clone()),
                });
            }

            // L6 — the card exists in this snapshot and is not banned.
            if def.is_none() {
                errors.push(LoadoutError {
                    rule: LoadoutRule::L6,
                    message: format!(
                        "{} cannot contain {}: no such card in catalog version {}.",
                        label_at(index),
                        label(card_id),
                        catalog.version
                    ),
                    deck: Some(deck_number),
                    card_id: Some(card_id.clone()),
                });
            }
            if banned.contains(card_id.as_str()) {
                errors.push(LoadoutError {
                    rule: LoadoutRule::L6,
                    message: format!(
                        "{} cannot contain {}: that card is banned.",
                        label_at(index),
                        label(card_id)
                    ),
                    deck: Some(deck_number),
                    card_id: Some(card_id.clone()),
                });
            }
        }
    }

    // L4 — a card id appears in at most one deck of the loadout. One deck has nothing to share with.
    if scope == Scope::Trio {
        for card_id in &order {
            let holders = decks_holding.get(card_id).cloned().unwrap_or_default();
            if holders.len() < 2 {
                continue;
            }
            let labels: Vec<String> = holders.iter().map(|&holder| label_at(holder)).collect();
            errors.push(LoadoutError {
                rule: LoadoutRule::L4,
                message: format!(
                    "{} appears in {}; a card may be in only one deck of a trio.",
                    label(card_id),
                    join_labels(&labels)
                ),
                deck: None,
                card_id: Some(card_id.clone()),
            });
        }
    }

    // L5 — copies across the loadout never exceed the quantity owned; an absent id is 0 owned.
    for card_id in &order {
        let used = totals.get(card_id).copied().unwrap_or(0);
        let owned = collection.get(card_id).copied().unwrap_or(0);
        if used <= owned {
            continue;
        }
        errors.push(LoadoutError {
            rule: LoadoutRule::L5,
            message: if scope == Scope::Deck {
                format!(
                    "{} uses {used} {} of {} but you own {owned}.",
                    label_at(0),
                    copy_word(used),
                    label(card_id)
                )
            } else {
                format!(
                    "Your trio uses {used} {} of {} but you own {owned}.",
                    copy_word(used),
                    label(card_id)
                )
            },
            deck: if scope == Scope::Deck { Some(1) } else { None },
            card_id: Some(card_id.clone()),
        });
    }

    if errors.is_empty() {
        LoadoutResult {
            ok: true,
            errors: Vec::new(),
        }
    } else {
        LoadoutResult { ok: false, errors }
    }
}

// --- Trio conflicts (R251, R252) -----------------------------------------------------------

/// One card that two or more decks of a trio hold, and which decks (0-based, in trio order).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TrioConflict {
    pub card_id: CardId,
    pub decks: Vec<usize>,
}

/// Every card that more than one of `decks` holds, in first-appearance order: the builder's
/// "unavailable, used in <deck>" highlight and L4's input are the same fact. R251: a card is its
/// catalog id — Radiant is a flag on a card in play (§5.2), never a second id, so there is no Radiant
/// copy to tell apart. No rule is decided here and no sentence is written: L4 words it.
///
/// TS took any `{ cards }[]`; a `LoadoutDeck`'s name is optional, so `[{ cards }]` reads as one.
pub fn trio_conflicts(decks: &[LoadoutDeck]) -> Vec<TrioConflict> {
    let mut holders: IndexMap<CardId, Vec<usize>> = IndexMap::new();
    let mut order: Vec<CardId> = Vec::new();
    for (index, deck) in decks.iter().enumerate() {
        for card_id in &deck.cards {
            match holders.get_mut(card_id) {
                None => {
                    holders.insert(card_id.clone(), vec![index]);
                    order.push(card_id.clone());
                }
                Some(held) => {
                    if !held.contains(&index) {
                        held.push(index);
                    }
                }
            }
        }
    }
    order
        .into_iter()
        .map(|card_id| TrioConflict {
            decks: holders.get(&card_id).cloned().unwrap_or_default(),
            card_id,
        })
        .filter(|conflict| conflict.decks.len() > 1)
        .collect()
}

// --- Draft rules: what a save checks (R250, R252) ------------------------------------------

string_union! {
    /// R250: a saved deck's structural rules. They are all a save checks, because a saved deck is a
    /// draft: incomplete, unowned or conflicting cards are allowed and judged at queue (R253).
    ///
    ///  D1 — a name of 1 to `name_max_length` characters once trimmed, with no control characters;
    ///  D2 — at most `DECK_SIZE` cards;
    ///  D3 — every card a deckable card of the catalog (it exists and is not a Token);
    ///  D4 — at most `MAX_COPIES` copies of a card;
    ///  D5 — `portrait` is `null` or a known portrait id (R641).
    pub enum DraftRule {
        D1 = "D1",
        D2 = "D2",
        D3 = "D3",
        D4 = "D4",
        D5 = "D5",
    }
}

string_union! {
    /// R252: a saved trio's rules. T1 a name as D1; T2 exactly `TRIO_DECKS` slots; T3 no deck twice.
    pub enum TrioDraftRule {
        T1 = "T1",
        T2 = "T2",
        T3 = "T3",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DraftIssue<Rule = DraftRule> {
    pub rule: Rule,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card_id: Option<CardId>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct NameLimits {
    /// The longest name, in characters, after trimming. The caller's config states the number.
    pub name_max_length: usize,
}

/// Control characters: a name is shown in lists, buttons and messages, and none of them has a place
/// there. That is the C0 and C1 controls and DEL (`Cc`), and the invisible format characters (`Cf`):
/// the bidirectional controls make a name display as text it does not hold (a right-to-left override
/// between "Aggro" and "orez" shows "Aggrozero"), and a deck code carries its name to other players
/// (R255); the zero-width ones hide characters inside a name. Three kinds of `Cf` are how real text
/// is written and stay legal: the zero-width non-joiner and joiner (Persian and the Indic scripts;
/// every emoji family) and the tag characters of a subdivision flag. (TS:
/// `/‌|‍|[\u{e0020}-\u{e007f}]/gu`, taken out before `/[\p{Cc}\p{Cf}]/u` is asked.)
fn is_writing_format_character(character: char) -> bool {
    matches!(character, '\u{200C}' | '\u{200D}' | '\u{E0020}'..='\u{E007F}')
}

fn is_control_character(character: char) -> bool {
    in_ranges(CC, character) || in_ranges(CF, character)
}

/// Something a player can see: a name made only of spaces, joiners and variation selectors draws
/// nothing, so it is no name at all. (TS `/[^\p{White_Space}\p{Default_Ignorable_Code_Point}]/u`.)
fn is_visible_character(character: char) -> bool {
    !in_ranges(WHITE_SPACE, character) && !in_ranges(DEFAULT_IGNORABLE, character)
}

fn has_control_character(name: &str) -> bool {
    name.chars()
        .filter(|&character| !is_writing_format_character(character))
        .any(is_control_character)
}

/// A name as it is stored: trimmed, and every run of whitespace inside it one space.
pub fn normalize_name(raw: &str) -> String {
    let trimmed = raw.trim_matches(is_js_space);
    let mut out = String::with_capacity(trimmed.len());
    let mut in_space = false;
    for character in trimmed.chars() {
        if is_js_space(character) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(character);
            in_space = false;
        }
    }
    out
}

/// The length a name limit counts: code points, so an emoji is one character, as a player sees it.
fn name_length(name: &str) -> usize {
    name.chars().count()
}

fn name_issue<Rule>(rule: Rule, what: &str, raw: &str, limits: NameLimits) -> Option<DraftIssue<Rule>> {
    let name = normalize_name(raw);
    if !name.chars().any(is_visible_character) {
        return Some(DraftIssue {
            rule,
            message: format!("A {what} needs a name."),
            card_id: None,
        });
    }
    if has_control_character(&name) {
        return Some(DraftIssue {
            rule,
            message: format!("A {what} name cannot contain control characters."),
            card_id: None,
        });
    }
    if name_length(&name) > limits.name_max_length {
        return Some(DraftIssue {
            rule,
            message: format!(
                "A {what} name can be at most {} characters.",
                limits.name_max_length
            ),
            card_id: None,
        });
    }
    None
}

/// TS `DeckDraftInput`: the draft and two predicates the caller answers, so it is not wire data.
pub struct DeckDraftInput<'a> {
    pub name: String,
    pub cards: Vec<CardId>,
    /// Whether an id is a deckable card: in the current catalog and not a Token. A predicate rather
    /// than a snapshot, because the server holds its catalog as ids and flags and the client holds
    /// definitions; both answer the same question.
    pub is_deckable: &'a dyn Fn(&str) -> bool,
    /// R641's D5: the deck's hero portrait, `None` (the default, `vanilla`; TS `null` or absent) or a
    /// known id.
    pub portrait: Option<String>,
    /// Whether an id is a known portrait, answered like `is_deckable` by the caller against
    /// `PORTRAIT_IDS`: the validator names no roster of its own. Absent, D5 has nothing to check
    /// against and the field is taken as given.
    pub is_portrait: Option<&'a dyn Fn(&str) -> bool>,
    /// `NameLimits.name_max_length` (TS `& NameLimits`).
    pub name_max_length: usize,
}

/// R250's D1–D4 and R641's D5, every failure at once. Empty when the draft may be saved.
pub fn check_deck_draft(input: &DeckDraftInput) -> Vec<DraftIssue> {
    let mut issues: Vec<DraftIssue> = Vec::new();
    let limits = NameLimits {
        name_max_length: input.name_max_length,
    };
    if let Some(named) = name_issue(DraftRule::D1, "deck", &input.name, limits) {
        issues.push(named);
    }

    if input.cards.len() as i32 > DECK_SIZE {
        issues.push(DraftIssue {
            rule: DraftRule::D2,
            message: format!(
                "A deck holds at most {DECK_SIZE} cards; this one has {}.",
                input.cards.len()
            ),
            card_id: None,
        });
    }

    let mut counts: IndexMap<&str, i32> = IndexMap::new();
    for card_id in &input.cards {
        *counts.entry(card_id.as_str()).or_insert(0) += 1;
    }
    for (card_id, count) in counts {
        if !(input.is_deckable)(card_id) {
            issues.push(DraftIssue {
                rule: DraftRule::D3,
                message: format!("\"{card_id}\" is not a card a deck can hold."),
                card_id: Some(card_id.to_string()),
            });
        }
        if count > MAX_COPIES {
            issues.push(DraftIssue {
                rule: DraftRule::D4,
                message: format!(
                    "A deck may hold at most {MAX_COPIES} {} of \"{card_id}\"; this one has {count}.",
                    copy_word(MAX_COPIES)
                ),
                card_id: Some(card_id.to_string()),
            });
        }
    }

    // D5 (R641): the portrait is cosmetic and a draft may carry none (`null`, which every deck
    // saved before portraits reads back as `vanilla`) or one of the roster the caller knows.
    if let (Some(portrait), Some(is_portrait)) = (input.portrait.as_deref(), input.is_portrait)
        && !is_portrait(portrait)
    {
        issues.push(DraftIssue {
            rule: DraftRule::D5,
            message: "\"portrait\" is not a known portrait id.".to_string(),
            card_id: None,
        });
    }
    issues
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TrioDraftInput {
    pub name: String,
    /// Deck ids by slot; `None` is an empty slot, which a draft may have (R252).
    pub deck_ids: Vec<Option<String>>,
    /// `NameLimits.name_max_length` (TS `& NameLimits`).
    pub name_max_length: usize,
}

/// R252's T1–T3, every failure at once. Empty when the trio may be saved.
pub fn check_trio_draft(input: &TrioDraftInput) -> Vec<DraftIssue<TrioDraftRule>> {
    let mut issues: Vec<DraftIssue<TrioDraftRule>> = Vec::new();
    let limits = NameLimits {
        name_max_length: input.name_max_length,
    };
    if let Some(named) = name_issue(TrioDraftRule::T1, "trio", &input.name, limits) {
        issues.push(named);
    }
    if input.deck_ids.len() != TRIO_DECKS {
        issues.push(DraftIssue {
            rule: TrioDraftRule::T2,
            message: format!(
                "A trio has exactly {TRIO_DECKS} slots; this one has {}.",
                input.deck_ids.len()
            ),
            card_id: None,
        });
    }
    let filled: Vec<&str> = input.deck_ids.iter().flatten().map(String::as_str).collect();
    let distinct: IndexSet<&str> = filled.iter().copied().collect();
    if distinct.len() != filled.len() {
        issues.push(DraftIssue {
            rule: TrioDraftRule::T3,
            message: "A trio cannot hold the same deck twice.".to_string(),
            card_id: None,
        });
    }
    issues
}

// --- Room for an imported trio (R340) -------------------------------------------------------

/// TS's inline `{ decks: number; trios: number }` of `ImportRoomInput`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SlotCounts {
    pub decks: i32,
    pub trios: i32,
}

/// R340: what a trio import would add to a profile, against what it has and the caps it lives
/// under. The caps are the caller's config (`MAX_SAVED_DECKS`, `MAX_SAVED_TRIOS`), handed in like
/// `name_max_length`, so this module states no number of its own.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ImportRoomInput {
    /// What the profile has saved now.
    pub saved: SlotCounts,
    /// The most it may keep.
    pub limits: SlotCounts,
    /// What the import would make: the code's decks, and the trio.
    pub adding: SlotCounts,
}

/// `ok`, or exactly how far short the profile is: how many decks and trios it must delete, and one
/// sentence saying so that the workshop and the server both show (R340: an import that would pass a
/// cap says how many slots it needs and makes nothing). TS `{ ok: true } | { ok: false; decksShort;
/// triosShort; message }`: the three are `Some` exactly when not `ok`, and absent from the JSON
/// otherwise.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ImportRoom {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub decks_short: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trios_short: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub message: Option<String>,
}

fn counted(count: i32, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn free(count: i32, what: &str) -> String {
    if count <= 0 {
        format!("no free {what} slot")
    } else {
        counted(count, &format!("free {what} slot"), &format!("free {what} slots"))
    }
}

/// R340: whether an import fits under both caps, and the sentence when it does not.
pub fn check_import_room(input: &ImportRoomInput) -> ImportRoom {
    let ImportRoomInput {
        saved,
        limits,
        adding,
    } = *input;
    let free_decks = 0_i32.max(limits.decks - saved.decks);
    let free_trios = 0_i32.max(limits.trios - saved.trios);
    let decks_short = 0_i32.max(adding.decks - free_decks);
    let trios_short = 0_i32.max(adding.trios - free_trios);
    if decks_short == 0 && trios_short == 0 {
        return ImportRoom {
            ok: true,
            decks_short: None,
            trios_short: None,
            message: None,
        };
    }

    let mut needs: Vec<String> = Vec::new();
    let mut has: Vec<String> = Vec::new();
    let mut remove: Vec<String> = Vec::new();
    if decks_short > 0 {
        needs.push(counted(adding.decks, "free deck slot", "free deck slots"));
        has.push(free(free_decks, "deck"));
        remove.push(counted(decks_short, "deck", "decks"));
    }
    if trios_short > 0 {
        needs.push(counted(adding.trios, "free trio slot", "free trio slots"));
        has.push(free(free_trios, "trio"));
        remove.push(counted(trios_short, "trio", "trios"));
    }
    let message = format!(
        "Importing this trio needs {}, and you have {}. Delete {}, then import it again.",
        needs.join(" and "),
        has.join(" and "),
        remove.join(" and ")
    );
    ImportRoom {
        ok: false,
        decks_short: Some(decks_short),
        trios_short: Some(trios_short),
        message: Some(message),
    }
}

// --- The Unicode properties the TS regular expressions named --------------------------------

/// Whether `character` falls in one of `ranges` (sorted, inclusive, non-overlapping).
fn in_ranges(ranges: &[(u32, u32)], character: char) -> bool {
    let code = character as u32;
    ranges
        .binary_search_by(|&(first, last)| {
            if last < code {
                std::cmp::Ordering::Less
            } else if first > code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// `\p{Cc}`.
const CC: &[(u32, u32)] = &[(0x0, 0x1f), (0x7f, 0x9f)];

/// `\p{Cf}` (Unicode 16.0).
#[rustfmt::skip]
const CF: &[(u32, u32)] = &[
    (0xad, 0xad), (0x600, 0x605), (0x61c, 0x61c), (0x6dd, 0x6dd), (0x70f, 0x70f), (0x890, 0x891),
    (0x8e2, 0x8e2), (0x180e, 0x180e), (0x200b, 0x200f), (0x202a, 0x202e), (0x2060, 0x2064), (0x2066, 0x206f),
    (0xfeff, 0xfeff), (0xfff9, 0xfffb), (0x110bd, 0x110bd), (0x110cd, 0x110cd), (0x13430, 0x1343f),
    (0x1bca0, 0x1bca3), (0x1d173, 0x1d17a), (0xe0001, 0xe0001), (0xe0020, 0xe007f),
];

/// `\p{White_Space}`.
#[rustfmt::skip]
const WHITE_SPACE: &[(u32, u32)] = &[
    (0x9, 0xd), (0x20, 0x20), (0x85, 0x85), (0xa0, 0xa0), (0x1680, 0x1680), (0x2000, 0x200a),
    (0x2028, 0x2029), (0x202f, 0x202f), (0x205f, 0x205f), (0x3000, 0x3000),
];

/// `\p{Default_Ignorable_Code_Point}` (Unicode 16.0).
#[rustfmt::skip]
const DEFAULT_IGNORABLE: &[(u32, u32)] = &[
    (0xad, 0xad), (0x34f, 0x34f), (0x61c, 0x61c), (0x115f, 0x1160), (0x17b4, 0x17b5), (0x180b, 0x180f),
    (0x200b, 0x200f), (0x202a, 0x202e), (0x2060, 0x206f), (0x3164, 0x3164), (0xfe00, 0xfe0f),
    (0xfeff, 0xfeff), (0xffa0, 0xffa0), (0xfff0, 0xfff8), (0x1bca0, 0x1bca3), (0x1d173, 0x1d17a),
    (0xe0000, 0xe0fff),
];
