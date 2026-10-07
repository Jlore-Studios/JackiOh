//! Shared builders for the loadout tests (SPEC §9.4 L1–L6, BUILD M6-T3).
//!
//! The catalog is synthetic, but the five cards the §9.4 messages name keep their real ids and
//! names, so the message assertions read like the spec. Every size comes from DECK_SIZE and
//! LOADOUT_DECKS: nothing here spells 20 or 3.
//!
//! Each injection below breaks exactly one rule. That is the point of the file: a careless
//! mutation trips several rules at once (a second copy of a card is also a copy you may not own),
//! so the collection grants 2 of the two ids the L3-copies and L4 injections duplicate, and grants
//! the token, the banned card and the uncatalogued id so those injections do not also trip L5.
//!
//! Port of `packages/validator/test/fixtures/loadouts.ts` (SURFACE §4.1). The rule tests
//! (`validator.rs`, `validator_drafts.rs`) reach it as `crate::rules::fixtures::validator_loadouts`.

use std::sync::LazyLock;

use indexmap::IndexSet;
use serde_json::{Value, json};

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::validator::{
    CardId, CatalogSnapshot, Collection, LOADOUT_DECKS, LoadoutDeck, LoadoutInput, LoadoutRule,
};
use jackioh_engine::wire::{CardDef, CardDefs};

/// The snapshot version the L6 "no such card" message quotes.
pub const CATALOG_VERSION: &str = "2026-09-01";

/// In Deck 1 of the legal loadout, owned twice: the L4 injection puts it in a second deck.
pub const HIT_JOB: &str = "core-012";
/// In Deck 3 of the legal loadout, owned once: the L5 injection zeroes what the profile owns.
pub const ARCHIVIST: &str = "core-030";
/// In Deck 2 of the legal loadout, owned twice: the L3-copies injection duplicates it there.
pub const JELLY_BEAN: &str = "core-051";
/// Catalogued and owned, but Token-tagged, so L3 refuses it in a deck.
pub const SHEEP_TOKEN: &str = "core-051.1";
/// Catalogued and owned, but on the snapshot's banned list, so L6 refuses it.
pub const CEASELESS_VOID: &str = "core-100";
/// Owned, and absent from the snapshot, so L6 refuses it by id alone.
pub const NOT_IN_CATALOG: &str = "core-999";

/// `DECK_SIZE` as a length.
const DECK_LEN: usize = DECK_SIZE as usize;
/// `LOADOUT_DECKS` as a count of decks.
#[allow(clippy::unnecessary_cast)]
const LOADOUT_LEN: usize = LOADOUT_DECKS as usize;

/// Spares beyond the three decks: a whole deck's worth, so a fourth deck can be a legal one.
const SPARE_COUNT: usize = DECK_LEN;
const POOL_SIZE: usize = LOADOUT_LEN * DECK_LEN + SPARE_COUNT;

/// TS's `NAMED` record: the three pool cards the messages name, by id.
const NAMED: &[(&str, &str)] = &[
    (HIT_JOB, "Hit Job"),
    (ARCHIVIST, "Archivist"),
    (JELLY_BEAN, "Glowy Jelly Bean"),
];

fn named(id: &str) -> Option<&'static str> {
    NAMED.iter().find(|(key, _)| *key == id).map(|(_, name)| *name)
}

fn pool_id(n: usize) -> CardId {
    format!("core-{n:03}")
}

/// TS's `vanillaDef` object literal, as JSON.
fn vanilla_json(id: &str, index: &str, name: &str) -> Value {
    json!({
        "id": id,
        "index": index,
        "name": name,
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": "vanilla" },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "vanilla" },
    })
}

fn vanilla_def(id: &str, index: &str, name: &str) -> CardDef {
    serde_json::from_value(vanilla_json(id, index, name)).expect("fixture error: a vanilla def is a CardDef")
}

fn token_def() -> CardDef {
    let mut value = vanilla_json(SHEEP_TOKEN, "51.1", "Sheep Token");
    value["tags"] = json!(["Token"]);
    value["rarity"] = json!("Token");
    value["token"] = json!(true);
    serde_json::from_value(value).expect("fixture error: the token def is a CardDef")
}

/// Every deckable id in the catalog: the decks draw from these, the rest are spares.
pub static POOL_IDS: LazyLock<Vec<CardId>> = LazyLock::new(|| (1..=POOL_SIZE).map(pool_id).collect());

pub fn catalog() -> CatalogSnapshot {
    let mut cards: CardDefs = CardDefs::new();
    for id in POOL_IDS.iter() {
        let index = &id["core-".len()..];
        let name = match named(id) {
            Some(name) => name.to_string(),
            None => format!("Fixture Card {index}"),
        };
        cards.insert(id.clone(), vanilla_def(id, index, &name));
    }
    let token = token_def();
    cards.insert(token.id.clone(), token);
    cards.insert(
        CEASELESS_VOID.to_string(),
        vanilla_def(CEASELESS_VOID, "100", "Ceaseless Void"),
    );
    CatalogSnapshot {
        version: CATALOG_VERSION.to_string(),
        cards,
        banned: Some(vec![CEASELESS_VOID.to_string()]),
    }
}

/// One of every deckable card, two of the pair the L3-copies and L4 injections duplicate, and one
/// each of the token, the banned card and the uncatalogued id, so only their own rule fires.
pub fn collection() -> Collection {
    let mut owned: Collection = Collection::new();
    for id in POOL_IDS.iter() {
        owned.insert(id.clone(), 1);
    }
    owned.insert(HIT_JOB.to_string(), 2);
    owned.insert(JELLY_BEAN.to_string(), 2);
    owned.insert(SHEEP_TOKEN.to_string(), 1);
    owned.insert(CEASELESS_VOID.to_string(), 1);
    owned.insert(NOT_IN_CATALOG.to_string(), 1);
    owned
}

// Deck 2 takes the third block of the pool (core-041…), which is the block holding Glowy Jelly
// Bean (core-051), and Deck 3 takes the second: that puts Hit Job in Deck 1, the Jelly Bean in
// Deck 2 and the Archivist in Deck 3, exactly as the §9.4 messages read.
const BLOCK_ORDER: &[usize] = &[0, 2, 1];

/// The legal loadout's decks: LOADOUT_DECKS disjoint blocks of DECK_SIZE distinct owned ids.
pub fn legal_decks() -> Vec<Vec<CardId>> {
    (0..LOADOUT_LEN)
        .map(|i| {
            let block = BLOCK_ORDER.get(i).copied().unwrap_or(i);
            let from = (block * DECK_LEN).min(POOL_IDS.len());
            let to = ((block + 1) * DECK_LEN).min(POOL_IDS.len());
            POOL_IDS[from..to].to_vec()
        })
        .collect()
}

/// A loadout that breaks none of L1–L6.
pub fn legal_loadout() -> LoadoutInput {
    LoadoutInput {
        decks: legal_decks()
            .into_iter()
            .map(|cards| LoadoutDeck { name: None, cards })
            .collect(),
        catalog: catalog(),
        collection: collection(),
    }
}

// --- injections ------------------------------------------------------------
// Each one takes any legal loadout — the fixture's or a generated one — and breaks one rule.

fn with_decks(input: &LoadoutInput, decks: Vec<LoadoutDeck>) -> LoadoutInput {
    LoadoutInput {
        decks,
        catalog: input.catalog.clone(),
        collection: input.collection.clone(),
    }
}

fn map_deck(input: &LoadoutInput, index: usize, f: impl Fn(&[CardId]) -> Vec<CardId>) -> LoadoutInput {
    with_decks(
        input,
        input
            .decks
            .iter()
            .enumerate()
            .map(|(i, deck)| {
                if i == index {
                    LoadoutDeck {
                        name: deck.name.clone(),
                        cards: f(&deck.cards),
                    }
                } else {
                    deck.clone()
                }
            })
            .collect(),
    )
}

/// Swaps `replacement` in for the first card that is not already `replacement`.
fn substitute(cards: &[CardId], replacement: &str) -> Vec<CardId> {
    let at = cards.iter().position(|id| id != replacement);
    cards
        .iter()
        .enumerate()
        .map(|(i, id)| {
            if Some(i) == at {
                replacement.to_string()
            } else {
                id.clone()
            }
        })
        .collect()
}

fn deck_holding(input: &LoadoutInput, card_id: &str) -> usize {
    match input
        .decks
        .iter()
        .position(|deck| deck.cards.iter().any(|id| id == card_id))
    {
        Some(index) => index,
        None => panic!("fixture error: no deck holds {card_id}"),
    }
}

/// Owned, catalogued, deckable ids no deck of this loadout uses.
fn unused_ids(input: &LoadoutInput) -> Vec<CardId> {
    let used: IndexSet<&CardId> = input.decks.iter().flat_map(|deck| deck.cards.iter()).collect();
    POOL_IDS.iter().filter(|id| !used.contains(id)).cloned().collect()
}

/// L1: one deck short. Dropping a deck cannot break another rule — its cards simply go unused.
pub fn drop_deck(input: &LoadoutInput) -> LoadoutInput {
    let keep = input.decks.len().saturating_sub(1);
    with_decks(input, input.decks[..keep].to_vec())
}

/// L1: one deck too many, and a legal one, so only the deck count is wrong.
pub fn add_deck(input: &LoadoutInput) -> LoadoutInput {
    let spares: Vec<CardId> = unused_ids(input).into_iter().take(DECK_LEN).collect();
    if spares.len() < DECK_LEN {
        panic!("fixture error: not enough spares for one more deck");
    }
    let mut decks = input.decks.clone();
    decks.push(LoadoutDeck {
        name: None,
        cards: spares,
    });
    with_decks(input, decks)
}

/// L2: one card short. Removing a card cannot break another rule.
pub fn drop_card(input: &LoadoutInput) -> LoadoutInput {
    map_deck(input, 0, |cards| cards.iter().skip(1).cloned().collect())
}

/// L2: one card too many. Appending an id already in the loadout would break L3, L4 and L5 too,
/// so this appends a spare the profile owns and no deck uses.
pub fn add_spare_card(input: &LoadoutInput) -> LoadoutInput {
    let Some(spare) = unused_ids(input).into_iter().next() else {
        panic!("fixture error: the loadout uses every pool card");
    };
    map_deck(input, 0, |cards| {
        let mut out = cards.to_vec();
        out.push(spare.clone());
        out
    })
}

/// L3: a second copy of the Jelly Bean in its own deck. The profile owns 2, so L5 stays quiet.
pub fn duplicate_in_deck(input: &LoadoutInput) -> LoadoutInput {
    map_deck(input, deck_holding(input, JELLY_BEAN), |cards| {
        substitute(cards, JELLY_BEAN)
    })
}

/// L3: a Token card in the last deck. It is catalogued and owned, so only the token rule fires.
pub fn insert_token(input: &LoadoutInput) -> LoadoutInput {
    match input.decks.len().checked_sub(1) {
        Some(last) => map_deck(input, last, |cards| substitute(cards, SHEEP_TOKEN)),
        // TS's `decks.length - 1` is -1 here, which no deck's index matches.
        None => with_decks(input, input.decks.clone()),
    }
}

/// L4: Hit Job in a second deck while it stays in its first. That is 2 copies across the loadout,
/// which is why the profile owns 2 of it.
pub fn cross_deck(input: &LoadoutInput) -> LoadoutInput {
    let home = deck_holding(input, HIT_JOB);
    let other = (home + 1) % input.decks.len();
    map_deck(input, other, |cards| substitute(cards, HIT_JOB))
}

/// L5: the profile no longer owns the Archivist its loadout uses.
pub fn unown_card(input: &LoadoutInput) -> LoadoutInput {
    deck_holding(input, ARCHIVIST); // fixture guard: the card must be in play for L5 to fire
    let mut collection = input.collection.clone();
    collection.insert(ARCHIVIST.to_string(), 0);
    LoadoutInput {
        decks: input.decks.clone(),
        catalog: input.catalog.clone(),
        collection,
    }
}

/// L6: an id this snapshot does not know. The profile "owns" it, so L5 stays quiet.
pub fn unknown_card(input: &LoadoutInput) -> LoadoutInput {
    map_deck(input, 0, |cards| substitute(cards, NOT_IN_CATALOG))
}

/// L6: a banned card. Catalogued and owned, so only the ban fires.
pub fn banned_card(input: &LoadoutInput) -> LoadoutInput {
    map_deck(input, 0, |cards| substitute(cards, CEASELESS_VOID))
}

pub struct Injection {
    pub rule: LoadoutRule,
    pub label: &'static str,
    pub apply: fn(&LoadoutInput) -> LoadoutInput,
}

/// Every single-rule injection, for the no-cascade table and the property test.
pub const INJECTIONS: &[Injection] = &[
    Injection {
        rule: LoadoutRule::L1,
        label: "a missing deck",
        apply: drop_deck,
    },
    Injection {
        rule: LoadoutRule::L1,
        label: "a fourth deck",
        apply: add_deck,
    },
    Injection {
        rule: LoadoutRule::L2,
        label: "a deck one card short",
        apply: drop_card,
    },
    Injection {
        rule: LoadoutRule::L2,
        label: "a deck one card over",
        apply: add_spare_card,
    },
    Injection {
        rule: LoadoutRule::L3,
        label: "a second copy in one deck",
        apply: duplicate_in_deck,
    },
    Injection {
        rule: LoadoutRule::L3,
        label: "a Token card in a deck",
        apply: insert_token,
    },
    Injection {
        rule: LoadoutRule::L4,
        label: "a card in two decks",
        apply: cross_deck,
    },
    Injection {
        rule: LoadoutRule::L5,
        label: "a card the profile does not own",
        apply: unown_card,
    },
    Injection {
        rule: LoadoutRule::L6,
        label: "a card missing from the catalog",
        apply: unknown_card,
    },
    Injection {
        rule: LoadoutRule::L6,
        label: "a banned card",
        apply: banned_card,
    },
];
