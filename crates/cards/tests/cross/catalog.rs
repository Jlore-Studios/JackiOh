//! BUILD M4-T1 and M9-T1 acceptance: catalog.json is the diff against SPEC §8 and the brief.
//!
//! PROVENANCE OF THE FIXTURE TABLES BELOW. Each table is a transcription of a document, not of
//! catalog.json, so that a card whose data drifts from its source fails here. Each was produced
//! mechanically from its document's own lines and checked in as a literal; the test never reads a
//! document at run time (a test that re-parsed one would pass while it and the catalog drifted
//! together, and would put file I/O in a crate that must stay pure — CLAUDE.md rule 4).
//!
//!   CORE           — SPEC §8.1–§8.5, the 105 rows of the catalog table, columns "#", "Name",
//!                    "Rarity", "Cost", "Type, tags" and "Stats" (`A/B → C/D` is base → radiant;
//!                    `A/B` alone means the radiant form keeps those stats; an empty cell means the
//!                    card has no stats), and SPEC §7's six named tokens (`T-rush`, `T-sheep`,
//!                    `T-felinor`, `T-bread`, `T-coin`, the one §2.1's setup deals, R244, and
//!                    `T-ghoul`, a card of its own in patch v0.1.1, R353), whose `rarity: "Token"` and
//!                    Token tag follow §8's in-catalog token rows and §7's token rules. The costs of
//!                    #16 Hit Job (3), #17 Flood, #34 Collateral Damage, #43 Big Felinor, #49 Snom
//!                    Bunny Mind Control, #88 Twisting Nether (4) and #65 Masochism Mask (1) are
//!                    patch v0.2.0's (issue #40).
//!   CLASSIC        — docs/classic-sets.md B6, each entry's header line (`classic-NNN` · (cost)
//!                    type, tags · rarity · stats base → Radiant), with the decisions B9 and the
//!                    v0.2.0 plan adopted: #55 is Book of Wildfire, #72 Grand Counterspell (R381),
//!                    #86 Genn's Radiant 42/42.
//!   CLASSIC_PLUS   — docs/classic-sets.md B7's header lines and token bullets (B2.3's numbering;
//!                    the Losers typed Unit, Otherworldly Removal's three and the Grapes typed Spell
//!                    by their sections' own sentences, the Grapes tagged Fruit, every token tagged
//!                    Token and rated "Token", B2.5) and B8's table of the ten AI generated cards
//!                    (tagged AI). Buff Billy's "3X/3X → 7X/7X" prints 0/0 (its `xStats` carry X).
//!   radiant faces  — every entry has a Radiant face of its own (§5.2, R276), the Ghoul Token's
//!                    being R349's fallback.
//!   rarity         — SPEC §8's rarity paragraph for Core (32/40/16/7/5) and B2.5's table for
//!                    Classic (35/26/18/10/1) and Classic+ (13/24/25/13/3), as patch v0.2.9
//!                    (issue #44) left them; B2.1's totals, 268 cards and 50 tokens in 318
//!                    entries (Glitch, issue #170, the fiftieth).
//!   tag vocabulary — SPEC §5/§6 tags as BUILD M4-T1 lists them, R278's Jlockeed, B2.4's Book,
//!                    Pancake and AI, the mechanics patch's Plague (every card that uses
//!                    Plague Counters), and patch v0.2.Y's Catalyst, Prime and Acclaimed.
//!
//! Not asserted here: `keywords`, `text`, `params`, `refs` and `loc` (the per-card tests prove the
//! behaviour `keywords`, `text` and `params` describe, references.rs proves `refs`, and `loc` is
//! frozen data since v0.3.0, SURFACE §7.5), and id shape (`cargo jackioh catalog check` owns the
//! schema).
//!
//! Port of `packages/cards/test/catalog.test.ts`. TS's one `it` per fixture row is one test per set
//! here, which checks every row and names every row that fails.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use jackioh_cards::CATALOG;
use jackioh_engine::{
    CardCost, CardDef, CardFace, CardType, FaceKind, GLITCH_DEF_ID, KeywordKind, Rarity, SetName, Tag,
    fill_params, set_ships,
};
use serde_json::Value;

use CardType as T;
use Rarity as R;
use Tag as G;

/// `[attack, health]`, or `None` for a §8 Stats cell that is empty (a card with no stats).
type StatPair = (Option<i32>, Option<i32>);

struct SpecRow {
    index: &'static str,
    name: &'static str,
    cost: CardCost,
    type_: CardType,
    tags: &'static [Tag],
    rarity: Rarity,
    base: StatPair,
    radiant: StatPair,
}

#[allow(clippy::too_many_arguments)]
const fn row(
    index: &'static str,
    name: &'static str,
    cost: CardCost,
    type_: CardType,
    tags: &'static [Tag],
    rarity: Rarity,
    base: StatPair,
    radiant: StatPair,
) -> SpecRow {
    SpecRow {
        index,
        name,
        cost,
        type_,
        tags,
        rarity,
        base,
        radiant,
    }
}

/// A printed cost.
const fn c(n: i32) -> CardCost {
    CardCost::Fixed(n)
}

/// "X".
const X: CardCost = CardCost::X;

/// "A embiggen B".
const fn emb(base: i32, embiggen: i32) -> CardCost {
    CardCost::Embiggen { base, embiggen }
}

/// A Stats cell `A/B`.
const fn st(attack: i32, health: i32) -> StatPair {
    (Some(attack), Some(health))
}

/// An empty Stats cell.
const NO_STATS: StatPair = (None, None);

const CORE: &[SpecRow] = &[
    row(
        "1",
        "Big D-fender",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(0, 7),
        st(0, 14),
    ),
    row(
        "2",
        "Bigot",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 1),
        st(12, 2),
    ),
    row(
        "3",
        "Right-house defender",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "4",
        "Gary the Gambler",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "5",
        "Stockpile",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "6",
        "Mana Well",
        c(3),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "7",
        "Jewelosco Scarab",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "8",
        "Mr. Vanilla",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 4),
        st(12, 12),
    ),
    row(
        "9",
        "Moths to the Flame",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(1, 14),
        st(2, 28),
    ),
    row(
        "10",
        "Rapid Replenish",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Tempo Timmy",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "12",
        "Duplicating Felinors",
        c(2),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 4),
        st(6, 9),
    ),
    row(
        "13",
        "Jlockeed Shredder-10",
        c(3),
        T::Unit,
        &[G::Jlockeed],
        R::Common,
        st(8, 10),
        st(16, 20),
    ),
    row(
        "14",
        "Jlockeed's Weapons",
        c(4),
        T::FieldSpell,
        &[G::Jlockeed],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Me and Mr Token",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "16",
        "Hit Job",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row("17", "Flood", c(4), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "18",
        "Bread and Butter",
        c(1),
        T::FieldTrap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Midrange Menace",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "20",
        "Pointmaster",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(7, 1),
        st(14, 2),
    ),
    row("21", "Hinder", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "22",
        "Carnivorous Cube",
        c(3),
        T::Unit,
        &[],
        R::Epic,
        st(4, 6),
        st(8, 12),
    ),
    row(
        "23",
        "Reoccurring Dream",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Efficiency Dividend",
        X,
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "4-mana 7/7",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(7, 7),
        st(14, 14),
    ),
    row(
        "26",
        "Glowy Jelly Bean",
        c(3),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Blood Ridden Glowy Jelly Bean",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "28",
        "Knockoff Temu Glowy Jelly Bean",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "GIGA Glowy Jelly Bean",
        c(6),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Archivist",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "31",
        "KY's Math Equation",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32",
        "Prem Panther",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "33",
        "Unstable Clone Machine",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "Collateral Damage",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Lunar Eclipse",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "Magic Jammed",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "Gravedigger",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "38",
        "Quickstriker",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39",
        "Recycling Initiative",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "40",
        "Echoes of the Forgotten",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("41", "Sheepish", c(1), T::Trap, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "42",
        "Eugenics",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "Big Felinor",
        c(4),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 10),
        st(6, 20),
    ),
    row(
        "44",
        "True Strike",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Deft Duelist",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 3),
        st(8, 6),
    ),
    row(
        "46",
        "Suppressive Aura",
        emb(2, 4),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "Fig of Life",
        c(3),
        T::Spell,
        &[G::Fruit],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "48",
        "5pek Controller",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "49",
        "Snom Bunny Mind Control",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "50",
        "K-Pop Fanatic",
        c(1),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "51",
        "KY's Private Tutor",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51.1",
        "KY's Empty Notebook",
        c(1),
        T::Spell,
        &[G::Ky, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "52",
        "Silly Silas",
        c(3),
        T::Unit,
        &[G::Human],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "53",
        "Reno",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 6),
        st(8, 12),
    ),
    row(
        "54",
        "Straaza",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "55",
        "Lava Golem",
        c(3),
        T::Unit,
        &[],
        R::Rare,
        st(10, 5),
        st(20, 10),
    ),
    row("56", "Jilliax", c(2), T::Unit, &[], R::Common, st(3, 2), st(6, 4)),
    row(
        "57",
        "Conjure KY",
        c(2),
        T::Spell,
        &[G::Ky],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "58",
        "Rush Token Farm",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "Unbiased Immigration",
        emb(2, 4),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "60",
        "Bear Honeypot",
        c(1),
        T::Trap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "61",
        "Prejudiced Postdoc",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "62",
        "Friend of Felinors",
        c(1),
        T::Spell,
        &[G::Felinor],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Plastic Surgery",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Gifted Program",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65",
        "Masochism Mask",
        c(1),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.1",
        "Spikey Pillow",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 2),
        st(0, 4),
    ),
    row(
        "66",
        "The Rock",
        c(4),
        T::Unit,
        &[G::Human],
        R::Common,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "67",
        "Zoomerbin Oomen",
        c(1),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(1, 2),
        st(2, 4),
    ),
    row(
        "68",
        "Twisted Sorcerer",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "69",
        "Call to Arms",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "70",
        "Spiteful Stab",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71",
        "Intern Stimmy",
        c(1),
        T::FieldTrap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "72",
        "Reminisce",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73",
        "Anti-oneshot Armor",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("74", "Adaptive UI", X, T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "75",
        "Infinite Reserves",
        c(0),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Field of Dreams",
        c(3),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "77",
        "Professor Curvature",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "78",
        "/fullsend",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "79",
        "Twinspell",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "80",
        "Zao Gao",
        c(2),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "81",
        "Radiant Saintess",
        c(1),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "82",
        "KY's Trial",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "83",
        "Transmogulate",
        c(2),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "84",
        "Going Long",
        emb(2, 4),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "85",
        "Unlicensed Experimentation",
        c(2),
        T::Trap,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "86",
        "\"Miss\" Mrow",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "87",
        "Pocket Chaos",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "88",
        "Twisting Nether",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "89",
        "Corpse Eater",
        c(4),
        T::Unit,
        &[],
        R::Epic,
        st(2, 2),
        st(6, 6),
    ),
    row(
        "90",
        "CN-Viral Injection",
        c(2),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "90.1",
        "CN-Virus",
        c(1),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "91",
        "Fed Fauci",
        c(2),
        T::Unit,
        &[G::Human, G::Plague],
        R::Rare,
        st(1, 6),
        st(2, 12),
    ),
    row(
        "92",
        "Felinor Fiender",
        c(2),
        T::Unit,
        &[G::Human],
        R::Legendary,
        st(5, 7),
        st(10, 14),
    ),
    row(
        "93",
        "Combo-Index",
        c(2),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "93.1",
        "Combo-Fodder",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "94",
        "Genn's Greed",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "95",
        "Call to Chaos (Core Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "95.1",
        "Chaos Golem",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row("96", "My Pawn", c(1), T::Trap, &[], R::Mythic, NO_STATS, NO_STATS),
    row(
        "97",
        "Zephyrs",
        c(0),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "98",
        "Heroic Power",
        c(0),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "99",
        "Craft a Card",
        c(4),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "100",
        "Ceaseless Void",
        c(100),
        T::Unit,
        &[],
        R::Mythic,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "T-rush",
        "Rush Token",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "T-sheep",
        "Sheep Token",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "T-felinor",
        "Felinor Token",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "T-bread",
        "Bread Token",
        c(0),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 0),
        st(0, 0),
    ),
    row(
        "T-coin",
        "The Coin",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-ghoul",
        "Ghoul Token",
        c(0),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 0),
        st(0, 0),
    ),
];

const CLASSIC: &[SpecRow] = &[
    row(
        "1",
        "Curse of the Forgotten Classic",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "The Trickster",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "3",
        "Book of Heal",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4",
        "Palantir",
        c(1),
        T::FieldSpell,
        &[G::Jlockeed],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row("5", "Tesla", c(2), T::FieldTrap, &[], R::Epic, st(1, 4), st(2, 8)),
    row(
        "6",
        "Cloaked Toe Cracker",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "7",
        "InfiniScepter",
        c(1),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row("8", "Pickle", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "9",
        "Income Tax",
        c(2),
        T::Trap,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row("10", "Exile", c(1), T::Trap, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "11",
        "Mind Melt",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12",
        "Book of Blood",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "13",
        "Boots on the Ground",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "14",
        "Shadowstep",
        c(2),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Nose Hunter",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 1),
        st(6, 2),
    ),
    row(
        "16",
        "Book of Flame",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "17",
        "Counterspell",
        c(2),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18",
        "Glitch in the System",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Lizard's Breath",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20",
        "The Power to Punish",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "21",
        "Turtinator",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "22",
        "Mid Runner",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "23",
        "Devil's Pact",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Book of Knowledge",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Lag in the System",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "26",
        "Rapid Draw",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Pestilent Slime",
        c(0),
        T::Unit,
        &[G::Plague],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "28",
        "Second Wind",
        c(0),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "Book of Vital Kill",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row("30", "Recycle", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "31",
        "Cookie Guild",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "32",
        "Felinor Feelings",
        c(0),
        T::Spell,
        &[G::Felinor],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("33", "Joro", c(0), T::Unit, &[], R::Legendary, st(1, 1), st(2, 2)),
    row(
        "34",
        "Ancient Acquisition",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("35", "Prep", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row("36", "Burn", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "37",
        "Last Hurrah",
        c(1),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "38",
        "Jackiestan Auctioneer",
        c(2),
        T::FieldTrap,
        &[G::Human],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "39",
        "Outbreak",
        c(1),
        T::Spell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row("40", "MC Tech", c(1), T::Unit, &[], R::Rare, st(3, 3), st(6, 6)),
    row(
        "41",
        "State of the Game",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "42",
        "Transmutable Toxins",
        c(2),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "Plague Nuke",
        c(4),
        T::Spell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "44",
        "Back from the GY",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Nature Titan",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "46",
        "Divine Favor",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "Recurring Felinor",
        c(2),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "48",
        "Hired Shrimp",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(4, 3),
        st(8, 6),
    ),
    row(
        "49",
        "Anti-Greed Machine",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "50",
        "Voidwalker",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(6, 3),
        st(12, 6),
    ),
    row(
        "51",
        "Back Breaker",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "52",
        "Final Gambit",
        c(2),
        T::Trap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Plague Crawler",
        c(1),
        T::Unit,
        &[G::Plague],
        R::Common,
        st(2, 2),
        st(4, 4),
    ),
    row("54", "Rewind", c(1), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "55",
        "Book of Wildfire",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "Spell Tyrant",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row("57", "Echo", c(1), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "58",
        "Common Resources",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "Plague Doctor",
        c(1),
        T::Unit,
        &[G::Human, G::Plague],
        R::Common,
        st(2, 3),
        st(4, 6),
    ),
    row("60", "Pile On", c(5), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "61",
        "Plague Bringer Goliath",
        c(3),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(7, 7),
        st(14, 14),
    ),
    row(
        "62",
        "Living Bomb",
        c(1),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Crop Dusting",
        c(2),
        T::Trap,
        &[G::Plague],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Malzahar's Recycler",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65",
        "Ace in the Hole",
        c(2),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "66",
        "EU Striker",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "67",
        "Felinor Feeler",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "68",
        "Small Card Lobbyist",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(11, 13),
        st(22, 26),
    ),
    row(
        "69",
        "Plague Charger",
        c(2),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(4, 2),
        st(8, 4),
    ),
    row(
        "70",
        "Book of Plague",
        c(1),
        T::Spell,
        &[G::Book, G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71",
        "Lane Eater",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "72",
        "Grand Counterspell",
        c(2),
        T::Trap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73",
        "Nurse Cleaver",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(3, 6),
        st(6, 12),
    ),
    row(
        "74",
        "Corpse Plantation",
        c(2),
        T::FieldSpell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "75",
        "Argusland",
        c(3),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Plague Bringer",
        c(2),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "77",
        "Anti-Magic Monkey",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "78",
        "Mutate Spell",
        c(1),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "79",
        "Risky Die",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "80",
        "BOOM! Big Max",
        c(4),
        T::Unit,
        &[G::Acclaimed],
        R::Legendary,
        st(13, 8),
        st(26, 16),
    ),
    row(
        "81",
        "The Power to Thrive",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "82",
        "Sheeople",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "83",
        "Flame Lance",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "84",
        "Lockdown",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "85",
        "King Wagtoggle",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "86",
        "Genn",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(14, 14),
        st(42, 42),
    ),
    row(
        "87",
        "Plague Chalice",
        X,
        T::FieldSpell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "88",
        "Siphon Squad",
        c(2),
        T::FieldTrap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "89",
        "Paul Allen's Ghost",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(5, 6),
        st(10, 12),
    ),
    row(
        "90",
        "In Too Deep",
        c(1),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    // Issue #170, R674: Glitch, the hidden token R673's roll makes, blank on both faces.
    row(
        "T-glitch",
        "Glitch",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
];

const CLASSIC_PLUS: &[SpecRow] = &[
    row(
        "1",
        "Doom Shroom",
        c(3),
        T::Trap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "Groom Shroom",
        c(3),
        T::Trap,
        &[G::Felinor],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "3",
        "Second Amendment Snake",
        c(2),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(1, 6),
        st(2, 12),
    ),
    row(
        "4",
        "Juhan Biggest Bat",
        c(3),
        T::Unit,
        &[G::Cn],
        R::Common,
        st(9, 6),
        st(18, 12),
    ),
    row(
        "5",
        "Guy Att",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 8),
        st(12, 16),
    ),
    row(
        "6",
        "Wrong-House Attacker",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "7",
        "The House",
        c(3),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "8",
        "Withering Storm",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("9", "Silence", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "10",
        "New Wraps",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Anime Armor",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "12",
        "The Mother Pancake",
        c(3),
        T::Unit,
        &[G::Pancake],
        R::Legendary,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "12.1",
        "Devour",
        c(0),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.2",
        "Death Boil",
        c(1),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.3",
        "Fluffy Grip",
        c(1),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.4",
        "Powder Spray",
        c(1),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.5",
        "Anti-Waffle Shell",
        c(1),
        T::FieldSpell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.6",
        "Frozen Wastes",
        c(2),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.7",
        "Legion of the Hungry",
        c(2),
        T::FieldSpell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.8",
        "Frostspatula",
        c(2),
        T::FieldSpell,
        &[G::Pancake, G::Token],
        R::Token,
        st(10, 3),
        st(20, 6),
    ),
    row(
        "13",
        "Mommy Barker",
        c(1),
        T::Unit,
        &[G::Human, G::Pancake],
        R::Legendary,
        st(2, 2),
        st(4, 4),
    ),
    row("14", "Forever&", c(1), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "15",
        "Conjure Rush Token",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "16",
        "Conjure Rush Token+",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "17",
        "Conjure Rush Token++",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18",
        "Gullible Treatler",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(8, 9),
        st(16, 18),
    ),
    row(
        "19",
        "League of Losers",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19.1",
        "Top Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "19.2",
        "Jungle Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "19.3",
        "Mid Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "19.4",
        "Support Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 5),
        st(0, 10),
    ),
    row(
        "19.5",
        "Bot Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "20",
        "Mushroom Power",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "21",
        "Whirlwind",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "22",
        "Blood Moon",
        c(1),
        T::Trap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "23",
        "Dropshipping",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Crushing Walls",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Soul Shot",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "26",
        "Tommy Tempo",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "27",
        "Zephrys Zealotism",
        c(4),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "28",
        "Nuestro hogar, nuestras tumbas",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "29",
        "Portal to the Past",
        c(3),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Felinor Fuser",
        c(4),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "31",
        "Fusion Lab",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32",
        "Otherworldly Removal",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32.1",
        "Execute",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32.2",
        "Brawl",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32.3",
        "Blade Storm",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "33",
        "Ivory Tower",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "Memory Leak",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Rollback",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "Conjure Bones",
        emb(2, 4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36.1",
        "Bone Storm",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "Wardrum",
        c(5),
        T::Unit,
        &[G::Quickdraw, G::Acclaimed],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "38",
        "Solarius",
        c(2),
        T::Unit,
        &[G::Catalyst],
        R::Epic,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "38.1",
        "Solarius Prime",
        c(4),
        T::Unit,
        &[G::Prime, G::Token],
        R::Token,
        st(9, 5),
        st(18, 10),
    ),
    row(
        "39",
        "Book Worm",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 4),
        st(2, 8),
    ),
    row(
        "40",
        "Appropriations",
        X,
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "41",
        "KY's Constant",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "42",
        "KY's Test",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "42.1",
        "KY's Gift",
        c(4),
        T::FieldSpell,
        &[G::Ky, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "AI Slop",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "44",
        "Simplicity Audit",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Complexity Audit",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "46",
        "Felinor Flagbearer",
        c(2),
        T::Unit,
        &[G::Felinor, G::Catalyst],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "46.1",
        "Felinor Flagbearer Prime",
        c(2),
        T::Unit,
        &[G::Felinor, G::Prime, G::Token],
        R::Token,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "47",
        "Jogg's Box",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "48",
        "Jlockheed's Lobbyist",
        c(1),
        T::Unit,
        &[G::Jlockeed],
        R::Legendary,
        st(0, 3),
        st(0, 6),
    ),
    row(
        "49",
        "Jay Fungus",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(3, 6),
        st(6, 12),
    ),
    row(
        "50",
        "Adaptive Growth",
        c(1),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51",
        "Jlockheed's J15 Fighter",
        c(3),
        T::Unit,
        &[G::Jlockeed],
        R::Epic,
        st(7, 2),
        st(14, 4),
    ),
    row(
        "52",
        "Jlockheed's Permanent Defense Contract",
        c(2),
        T::Spell,
        &[G::Jlockeed],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Book of Tokens",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "54",
        "Book of Books",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "55",
        "Book of Greed",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "Book of Pain",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "57",
        "Book of Stats",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "58",
        "Fruit Basket",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "All Purpose Apple",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "60",
        "Doctors Orders",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "61",
        "Bauble Bubble",
        c(1),
        T::FieldSpell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "62",
        "KY's Papaya",
        c(1),
        T::Spell,
        &[G::Fruit, G::Ky],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Fruit Tree",
        c(2),
        T::FieldSpell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Mulch Muncher",
        c(10),
        T::Unit,
        &[],
        R::Rare,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "65",
        "Two Grapes",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.1",
        "Rotten Grape",
        c(1),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.2",
        "Normal Grape",
        c(1),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.3",
        "Large Grape",
        c(3),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.4",
        "Golden Grape",
        c(1),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.5",
        "Mythic Grape",
        c(0),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "66",
        "Vine of Grapes",
        c(3),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "67",
        "Pear",
        c(2),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "68",
        "Organic Produce",
        c(4),
        T::FieldSpell,
        &[G::Fruit],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "69",
        "Buff Billy",
        X,
        T::Unit,
        &[G::Human],
        R::Rare,
        st(0, 0),
        st(0, 0),
    ),
    row(
        "70",
        "Chaos Machine",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71",
        "Book of Buff",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "72",
        "Book of Nerf",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73",
        "Call to Chaos (Classic+ Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73.1",
        "Classic Golem",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "74",
        "Twice Forward One Step Backwards",
        c(2),
        T::FieldTrap,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "75",
        "J-lease J-Jungle EX-plorer",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "75.1",
        "J-lease J-Jungle EX-plorer Pack",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Brother Lar",
        c(1),
        T::Unit,
        &[G::Cn, G::Human],
        R::Rare,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "76.1",
        "Brother Ping",
        c(2),
        T::Unit,
        &[G::Cn, G::Human, G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "77",
        "Anti-Softlock",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "78",
        "Claude's Datacenter",
        c(2),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-1",
        "Helpful Assistant",
        c(1),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(1, 3),
        st(2, 6),
    ),
    row(
        "T-AI-2",
        "Scaling Law",
        c(2),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "T-AI-3",
        "Hallucination",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-4",
        "Chain of Thought",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-5",
        "Autocomplete",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-6",
        "Datacenter Fire",
        c(2),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-7",
        "Alignment Tax",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-8",
        "Rate Limit",
        c(1),
        T::Trap,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-9",
        "Refusal",
        c(1),
        T::Trap,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-10",
        "Fine-Tuning",
        c(2),
        T::FieldSpell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
];

/// Each set's fixture, in catalog order (B2.2).
/// R1420: the Meditative set's rows, transcribed from docs/meditative-set.md's card headers (M6), in
/// number order with each token after its card. The set does not ship yet: an entry it holds must
/// equal its row, and a row with no entry is a card still being built, until it ships.
const MEDITATIVE: &[SpecRow] = &[
    row(
        "1",
        "Disruptive Disruptor",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "Rampaging Rhino",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 9),
        st(11, 20),
    ),
    row(
        "3",
        "Jlockwork Machine",
        c(3),
        T::Unit,
        &[],
        R::Rare,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "4",
        "Juicy Kumquat Melon",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "5",
        "Death by 1000 cuts",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "6",
        "Me no Likey",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "7",
        "Introspection",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "8",
        "Reach the Summit",
        c(0),
        T::Spell,
        &[G::Quickdraw, G::Wincon],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "9",
        "Joint Filing",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10",
        "Double Counting",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Double Header",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12",
        "Fear Mongerer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(6, 8),
        st(12, 16),
    ),
    row(
        "13",
        "Gatling Pea",
        c(2),
        T::Unit,
        &[],
        R::Epic,
        st(2, 6),
        st(4, 12),
    ),
    row(
        "14",
        "Prime Time",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Smelly Steven",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "16",
        "Trenful Trickster",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(1, 5),
        st(2, 10),
    ),
    row(
        "17",
        "True Craft a Card",
        c(0),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18",
        "Expedition12",
        c(1),
        T::Spell,
        &[G::Quickdraw],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Expedition1234",
        c(1),
        T::Spell,
        &[G::Quickdraw],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19.1",
        "Temporal Rift",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20",
        "Aestheticize the Game",
        c(1),
        T::Spell,
        &[G::Quickdraw, G::Wincon],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "21",
        "API Key Fishing",
        emb(0, 1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "22",
        "Mind Games",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "22.1",
        "Fortify Mind",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "23",
        "Golly Bob Howdy",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 5),
        st(6, 10),
    ),
    row(
        "24",
        "Polymorph",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Blue-Eyes White Felinor",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(12, 9),
        st(24, 18),
    ),
    row(
        "26",
        "Alternate Fate",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Clip-Farming Lawyer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 5),
        st(12, 10),
    ),
    row(
        "28",
        "Shade-iris",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(7, 4),
        st(14, 8),
    ),
    row(
        "28.1",
        "Ancient Curse",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "Forbiddenous Factory",
        c(3),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Fickle E-Kitten",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "30.1",
        "Love Bomb",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "31",
        "The Conductor",
        c(3),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(7, 9),
        st(14, 18),
    ),
    row(
        "32",
        "Spiritually 中国",
        c(2),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "33",
        "First Day of 学校",
        c(0),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "高考",
        c(4),
        T::Spell,
        &[G::Cn, G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "RCTA (CN)",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "CN Peptides",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "CN in a bottle",
        X,
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "38",
        "H1B Printer",
        c(2),
        T::FieldSpell,
        &[G::Cn, G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39",
        "赌石 Addict",
        c(2),
        T::Unit,
        &[G::Cn],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "39.1",
        "Auspicious Rock",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.2",
        "Jade",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.3",
        "Dud",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.4",
        "Red Jade",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.5",
        "Jade Beauty",
        c(10),
        T::Unit,
        &[G::Cn, G::Token],
        R::Token,
        st(20, 20),
        st(40, 40),
    ),
    row(
        "40",
        "Feng Shui",
        c(2),
        T::FieldSpell,
        &[G::Cn],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "41",
        "CN Smuggler",
        c(2),
        T::Unit,
        &[G::Cn],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "42",
        "CN Flea Market",
        c(0),
        T::Spell,
        &[G::Cn],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "CN Jade Market",
        X,
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "44",
        "CN Jade Well",
        c(3),
        T::FieldSpell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Knowledge Breaker",
        c(1),
        T::Unit,
        &[G::Cn, G::Ky, G::Catalyst],
        R::Legendary,
        st(1, 3),
        st(2, 6),
    ),
    row(
        "45.1",
        "Knowledge Breaker Prime",
        c(4),
        T::Unit,
        &[G::Cn, G::Ky, G::Prime, G::Token],
        R::Token,
        st(6, 18),
        st(12, 36),
    ),
    row(
        "46",
        "Conjure Intellect",
        emb(2, 4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "饕餮",
        c(4),
        T::Unit,
        &[G::Cn],
        R::Legendary,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "48",
        "Tranquility",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "49",
        "YileGPT Tamed",
        c(2),
        T::Unit,
        &[G::Cn, G::Acclaimed],
        R::Mythic,
        st(2, 6),
        st(4, 12),
    ),
    row(
        "49.1",
        "YileGPT Unleashed",
        c(10),
        T::Unit,
        &[G::Cn, G::Acclaimed, G::Token],
        R::Token,
        st(8, 20),
        st(16, 40),
    ),
    row(
        "49.2",
        "Yile's Virus",
        c(2),
        T::Unit,
        &[G::Cn, G::Acclaimed, G::Token],
        R::Token,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "49.3",
        "AI Girlfriend",
        c(4),
        T::FieldSpell,
        &[G::Cn, G::Acclaimed, G::Token],
        R::Token,
        st(2, 30),
        st(4, 60),
    ),
    row(
        "50",
        "CN Tech",
        c(0),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51",
        "Devin Bot",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "52",
        "Economic Anxiety",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Prestige",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "54",
        "Money Machine",
        c(1),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "55",
        "Dragon Fruit",
        c(2),
        T::Spell,
        &[G::Cn, G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "House Party",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "57",
        "Clip-Farming Critikal",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(5, 1),
        st(10, 2),
    ),
    row(
        "58",
        "Permanent Underclassman",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "59",
        "Permanent Upperclassman",
        c(3),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "60",
        "Eschews",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "61",
        "Joon Jorker",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(7, 5),
        st(14, 10),
    ),
    row(
        "62",
        "Small Time Recruits",
        c(0),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Skull of J'Nari",
        c(3),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Traitorous Blood",
        c(1),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65",
        "Keymaster Keenus",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "66",
        "Fiery Waraxe",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "67",
        "Sentient Cat Ears",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "68",
        "Catnip",
        c(1),
        T::FieldTrap,
        &[G::Felinor],
        R::Rare,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "69",
        "The Maestro",
        c(4),
        T::Unit,
        &[G::Human, G::Plague],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "70",
        "I'M WILL BE YOUR DOOM",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(5, 8),
        st(10, 16),
    ),
    row(
        "70.1",
        "Ready… I'm",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "71",
        "Pareto Optimality",
        c(2),
        T::FieldSpell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71.1",
        "The Cane",
        c(2),
        T::FieldSpell,
        &[G::Cn, G::Token],
        R::Token,
        st(3, 1),
        st(6, 2),
    ),
    row(
        "72",
        "The Banisher",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "73",
        "Plate Packer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "74",
        "Montaña Giant",
        c(8),
        T::Unit,
        &[],
        R::Epic,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "75",
        "Ever Growing Tree",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Do or Die",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "77",
        "Bulk Booster",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "78",
        "Occidentless Mandate",
        c(4),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "79",
        "Touched by KY",
        c(3),
        T::Unit,
        &[G::Ky],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "80",
        "Aluneth",
        c(3),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "81",
        "Deadman's Hand",
        emb(0, 2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "82",
        "Medina Outfitter",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "83",
        "Medina Enforcer",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "84",
        "Volatility",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "85",
        "Playtester",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "86",
        "Mayor Medinamogger",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "87",
        "Tatches the Totem",
        c(1),
        T::Unit,
        &[G::Human, G::Felinor, G::Ky, G::Cn, G::Jlockeed],
        R::Legendary,
        st(0, 3),
        st(0, 6),
    ),
    row(
        "88",
        "The True Sheep",
        c(4),
        T::Unit,
        &[],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row("89", "Jlarna", c(0), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "90",
        "Spell Basket",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "91",
        "Windfast",
        c(1),
        T::Unit,
        &[G::Catalyst],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "91.1",
        "Windfurious Prime",
        c(3),
        T::Unit,
        &[G::Prime, G::Token],
        R::Token,
        st(5, 10),
        st(10, 20),
    ),
    row("92", "Unan", c(3), T::Unit, &[], R::Rare, st(3, 13), st(6, 26)),
    row(
        "93",
        "Growing Felinor",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "93.1",
        "Growing Felinor Sr",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "93.2",
        "Growing Felinor Sr Sr",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "93.3",
        "Growing Felinor Super Senior",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "94",
        "Shrinking Felinor",
        c(4),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(9, 11),
        st(18, 22),
    ),
    row(
        "95",
        "Call to Chaos (Meditative Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "95.1",
        "CN Golem",
        c(4),
        T::Unit,
        &[G::Cn, G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "96",
        "Meditative Journey",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "96.1",
        "Journey Complete",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "97",
        "Jlockheed's Evil Blueprints",
        c(0),
        T::Spell,
        &[G::Jlockeed],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "97.1",
        "Empty Plot",
        c(0),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 3),
        st(0, 8),
    ),
    row(
        "97.2",
        "Wishing Well",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "97.3",
        "School",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "97.4",
        "Mega Church",
        c(3),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 5),
        st(0, 10),
    ),
    row(
        "97.5",
        "Bunker",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(5, 10),
        st(10, 30),
    ),
    row(
        "97.6",
        "University",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 12),
        st(0, 24),
    ),
    row(
        "97.7",
        "The Great Wall",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 50),
        st(0, 100),
    ),
    row(
        "97.8",
        "Prison",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 16),
        st(0, 32),
    ),
    row(
        "97.9",
        "Jlockheed's Headquarters",
        c(4),
        T::Unit,
        &[G::Jlockeed, G::Token],
        R::Token,
        st(0, 20),
        st(0, 50),
    ),
    row("98", "Showdown", c(2), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "99",
        "Paranoia",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
];

const FIXTURES: &[(SetName, &[SpecRow])] = &[
    (SetName::Core, CORE),
    (SetName::Classic, CLASSIC),
    (SetName::ClassicPlus, CLASSIC_PLUS),
    (SetName::Meditative, MEDITATIVE),
];

/// BUILD M4-T1 and B2.4, plus the mechanics patch's Plague and patch v0.2.Y's Catalyst, Prime and
/// Acclaimed: the only tags any entry may carry.
const ALLOWED_TAGS: &[&str] = &[
    "Human",
    "Felinor",
    "KY",
    "CN",
    "Fruit",
    "Call to Chaos",
    "Quickdraw",
    "Jlockeed",
    "Book",
    "Pancake",
    "AI",
    "Plague",
    "Catalyst",
    "Prime",
    "Acclaimed",
    "Wincon",
    "Token",
];

/// SPEC §8's and B2.5's distributions. Tokens carry rarity "Token" and are counted apart.
const RARITY_COUNTS: &[(&str, &[(&str, usize)])] = &[
    (
        "Core",
        &[
            ("Common", 32),
            ("Rare", 40),
            ("Epic", 16),
            ("Legendary", 7),
            ("Mythic", 5),
        ],
    ),
    (
        "Classic",
        &[
            ("Common", 35),
            ("Rare", 26),
            ("Epic", 18),
            ("Legendary", 10),
            ("Mythic", 1),
        ],
    ),
    (
        "Classic+",
        &[
            ("Common", 13),
            ("Rare", 24),
            ("Epic", 25),
            ("Legendary", 13),
            ("Mythic", 3),
        ],
    ),
    // R1420: docs/meditative-set.md M2's distribution, a ceiling until the set ships.
    (
        "Meditative",
        &[
            ("Common", 26),
            ("Rare", 29),
            ("Epic", 22),
            ("Legendary", 16),
            ("Mythic", 6),
        ],
    ),
];

/// B2.1: cards and tokens per set.
struct SetSize {
    set: &'static str,
    cards: usize,
    tokens: usize,
}

const SET_SIZES: &[SetSize] = &[
    SetSize {
        set: "Core",
        cards: 100,
        tokens: 11,
    },
    SetSize {
        set: "Classic",
        cards: 90,
        tokens: 1,
    },
    SetSize {
        set: "Classic+",
        cards: 78,
        tokens: 38,
    },
    // R1420: docs/meditative-set.md M2, a ceiling until the set ships.
    SetSize {
        set: "Meditative",
        cards: 99,
        tokens: 30,
    },
];

/// R1420: whether a set ships. A set that does not may hold fewer entries than its fixture and its
/// counts, never more, and none of them counts toward the catalog's totals.
fn ships(set: &str) -> bool {
    SetName::ALL
        .iter()
        .find(|known| known.as_str() == set)
        .is_some_and(|known| set_ships(*known))
}

/// B2.1's totals, as SET_SIZES sums them over the sets that ship: the only count of the whole
/// catalog this file keeps.
fn total_cards() -> usize {
    SET_SIZES
        .iter()
        .filter(|size| ships(size.set))
        .map(|size| size.cards)
        .sum()
}

fn total_tokens() -> usize {
    SET_SIZES
        .iter()
        .filter(|size| ships(size.set))
        .map(|size| size.tokens)
        .sum()
}

/// The entries of the sets that ship (R1420).
fn shipped_entries() -> Vec<&'static CardDef> {
    entries()
        .into_iter()
        .filter(|entry| set_ships(entry.set))
        .collect()
}

/// A set's SET_SIZES row as one number (cards + tokens), for the fixture-row counts below.
fn set_size(set: &str) -> usize {
    SET_SIZES
        .iter()
        .find(|size| size.set == set)
        .map_or(0, |size| size.cards + size.tokens)
}

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

fn key_of(set: &str, index: &str) -> String {
    format!("{set} #{index}")
}

/// An index is unique only within its set (B2.2), so entries are found by `set` and `index`.
fn by_key() -> IndexMap<String, &'static CardDef> {
    let mut by_key: IndexMap<String, &'static CardDef> = IndexMap::new();
    for entry in entries() {
        let key = key_of(entry.set.as_str(), &entry.index);
        if by_key.contains_key(&key) {
            panic!("catalog has two entries with {key}");
        }
        by_key.insert(key, entry);
    }
    by_key
}

fn entry_for(by_key: &IndexMap<String, &'static CardDef>, set: SetName, index: &str) -> &'static CardDef {
    match by_key.get(&key_of(set.as_str(), index)) {
        Some(entry) => entry,
        None => panic!(
            "{} is in the fixture but missing from catalog.json",
            key_of(set.as_str(), index)
        ),
    }
}

/// A face's stat as the fixture spells it: a number, or `None` when the card has no stats.
fn stat_of(face: &CardFace, field: &str) -> Option<i32> {
    match field {
        "attack" => face.attack,
        _ => face.health,
    }
}

fn show(value: &Value) -> String {
    match value.as_str() {
        Some(text) => text.to_string(),
        None => value.to_string(),
    }
}

/// "core-043 rarity: expected Rare, got Epic" — the label every failure in this file carries.
fn label(entry: &CardDef, field: &str, expected: &Value, actual: &Value) -> String {
    format!(
        "{} (#{}) {field}: expected {}, got {}",
        entry.id,
        entry.index,
        show(expected),
        show(actual)
    )
}

fn json<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("catalog values serialise")
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Every problem one entry has against its fixture row (TS: the body of each row's `it`).
fn row_problems(set: SetName, row: &SpecRow, by_key: &IndexMap<String, &'static CardDef>) -> Vec<String> {
    let entry = entry_for(by_key, set, row.index);
    let mut problems: Vec<String> = Vec::new();

    if entry.name != row.name {
        problems.push(label(entry, "name", &json(row.name), &json(&entry.name)));
    }
    if entry.cost != row.cost {
        problems.push(label(entry, "cost", &json(row.cost), &json(entry.cost)));
    }
    if entry.type_ != row.type_ {
        problems.push(label(entry, "type", &json(row.type_), &json(entry.type_)));
    }
    if entry.rarity != row.rarity {
        problems.push(label(entry, "rarity", &json(row.rarity), &json(entry.rarity)));
    }

    // A cell's tag order and the catalog's may differ without either being wrong, so the
    // assertion is on the set.
    let mut expected_tags: Vec<&str> = row.tags.iter().map(|tag| tag.as_str()).collect();
    expected_tags.sort();
    let mut actual_tags: Vec<&str> = entry.tags.iter().map(|tag| tag.as_str()).collect();
    actual_tags.sort();
    if expected_tags != actual_tags {
        problems.push(label(
            entry,
            "tags (as a set)",
            &json(&expected_tags),
            &json(&actual_tags),
        ));
    }

    let stats: [(&str, &CardFace, StatPair); 2] = [
        ("base", &entry.base, row.base),
        ("radiant", &entry.radiant, row.radiant),
    ];
    for (face, actual, expected) in stats {
        if stat_of(actual, "attack") != expected.0 {
            problems.push(label(
                entry,
                &format!("{face}.attack"),
                &json(expected.0),
                &json(stat_of(actual, "attack")),
            ));
        }
        if stat_of(actual, "health") != expected.1 {
            problems.push(label(
                entry,
                &format!("{face}.health"),
                &json(expected.1),
                &json(stat_of(actual, "health")),
            ));
        }
    }
    problems
}

/// Every row of one set's fixture against its entry, as one report (TS: one `it` per row).
fn fixture_report(set: SetName) -> String {
    let by_key = by_key();
    let rows = FIXTURES
        .iter()
        .find(|(fixture_set, _)| *fixture_set == set)
        .map(|(_, rows)| *rows)
        .unwrap_or_default();
    rows.iter()
        // R1420: a row of a set that has not shipped, with no entry yet, is a card still being built.
        .filter(|row| set_ships(set) || by_key.contains_key(&key_of(set.as_str(), row.index)))
        .filter_map(|row| {
            let problems = row_problems(set, row, &by_key);
            if problems.is_empty() {
                None
            } else {
                Some(format!(
                    "{set} #{} {} vs its fixture:\n{}",
                    row.index,
                    row.name,
                    problems.join("\n")
                ))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `/^\d+$/`.
fn is_plain_index(index: &str) -> bool {
    !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit())
}

mod catalog_membership_build_m4_t1_b2_1 {
    use super::*;

    #[test]
    fn holds_b2_1_s_cards_and_tokens_every_entry_across_core_classic_and_classic_plus() {
        let cards = shipped_entries().into_iter().filter(|entry| !entry.token).count();
        let tokens = shipped_entries().into_iter().filter(|entry| entry.token).count();
        assert_eq!(cards, total_cards(), "entries with token: false");
        assert_eq!(tokens, total_tokens(), "entries with token: true");
        assert_eq!(
            shipped_entries().len(),
            total_cards() + total_tokens(),
            "shipped catalog entries"
        );
        for size in SET_SIZES {
            let in_set: Vec<&CardDef> = entries()
                .into_iter()
                .filter(|entry| entry.set.as_str() == size.set)
                .collect();
            let (cards, tokens) = (
                in_set.iter().filter(|entry| !entry.token).count(),
                in_set.iter().filter(|entry| entry.token).count(),
            );
            if ships(size.set) {
                assert_eq!(cards, size.cards, "{} cards", size.set);
                assert_eq!(tokens, size.tokens, "{} tokens", size.set);
            } else {
                // R1420: a set being built holds no more than its brief lists.
                assert!(
                    cards <= size.cards,
                    "{} cards: {cards} of at most {}",
                    size.set,
                    size.cards
                );
                assert!(
                    tokens <= size.tokens,
                    "{} tokens: {tokens} of at most {}",
                    size.set,
                    size.tokens
                );
            }
        }
    }

    #[test]
    fn has_each_set_s_indices_1_n_present_exactly_once_none_of_them_a_token() {
        let by_key = by_key();
        let mut missing: Vec<String> = Vec::new();
        for size in SET_SIZES {
            for n in 1..=size.cards {
                match by_key.get(&key_of(size.set, &n.to_string())) {
                    // R1420: a set that has not shipped need not hold every card yet.
                    None if !ships(size.set) => {}
                    None => missing.push(format!("{} #{n} missing", size.set)),
                    Some(entry) if entry.token => {
                        missing.push(format!("{} #{n} ({}) is flagged token: true", size.set, entry.id));
                    }
                    Some(_) => {}
                }
            }
            // by_key() is built with a duplicate guard, so one entry per index is already proved.
            let plain = entries()
                .into_iter()
                .filter(|entry| entry.set.as_str() == size.set && is_plain_index(&entry.index))
                .count();
            if ships(size.set) {
                assert_eq!(plain, size.cards, "{} plain numeric indices", size.set);
            } else {
                assert!(plain <= size.cards, "{} plain numeric indices", size.set);
            }
        }
        assert_eq!(missing, Vec::<String>::new(), "indices");
    }

    #[test]
    fn has_core_s_five_card_defined_tokens_the_four_shared_tokens_the_coin_and_the_ghoul_token() {
        let by_key = by_key();
        let expected = [
            "51.1",
            "65.1",
            "90.1",
            "93.1",
            "95.1",
            "T-rush",
            "T-sheep",
            "T-felinor",
            "T-bread",
            "T-coin",
            "T-ghoul",
        ];
        for index in expected {
            let entry = by_key.get(&key_of("Core", index));
            assert_eq!(
                entry.map(|entry| entry.index.as_str()),
                Some(index),
                "token index {index}"
            );
            assert_eq!(
                entry.map(|entry| entry.token),
                Some(true),
                "token index {index} token flag"
            );
        }
        let mut flagged: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::Core && entry.token)
            .map(|entry| entry.index.clone())
            .collect();
        flagged.sort();
        let mut expected_sorted = strings(&expected);
        expected_sorted.sort();
        assert_eq!(flagged, expected_sorted, "Core entries flagged token: true");
    }

    #[test]
    fn has_classic_plus_s_card_defined_tokens_and_the_ten_ai_generated_cards_b2_1_b2_3_b8() {
        let tokens: Vec<String> = CLASSIC_PLUS
            .iter()
            .filter(|row| row.rarity == Rarity::Token)
            .map(|row| row.index.to_string())
            .collect();
        assert_eq!(
            Some(tokens.len()),
            SET_SIZES
                .iter()
                .find(|size| size.set == "Classic+")
                .map(|size| size.tokens)
        );
        assert_eq!(
            tokens.iter().filter(|index| index.starts_with("T-AI-")).count(),
            10
        );
        let mut flagged: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::ClassicPlus && entry.token)
            .map(|entry| entry.index.clone())
            .collect();
        flagged.sort();
        let mut tokens_sorted = tokens.clone();
        tokens_sorted.sort();
        assert_eq!(flagged, tokens_sorted);
    }

    #[test]
    fn holds_no_entry_its_set_s_fixture_does_not_list() {
        let known: Vec<String> = FIXTURES
            .iter()
            .flat_map(|(set, rows)| rows.iter().map(|row| key_of(set.as_str(), row.index)))
            .collect();
        let extra: Vec<String> = entries()
            .into_iter()
            .filter(|entry| !known.contains(&key_of(entry.set.as_str(), &entry.index)))
            .map(|entry| format!("{} ({})", entry.id, key_of(entry.set.as_str(), &entry.index)))
            .collect();
        assert_eq!(extra, Vec::<String>::new(), "catalog entries with no fixture row");
        for (set, rows) in FIXTURES {
            assert_eq!(rows.len(), set_size(set.as_str()), "{set} fixture rows");
        }
    }
}

mod every_core_entry_equals_its_fixture_row_build_m4_t1_m9_t1 {
    use super::*;

    #[test]
    fn every_core_entry_equals_its_fixture_row() {
        assert_eq!(fixture_report(SetName::Core), "");
    }
}

mod every_classic_entry_equals_its_fixture_row_build_m4_t1_m9_t1 {
    use super::*;

    #[test]
    fn every_classic_entry_equals_its_fixture_row() {
        assert_eq!(fixture_report(SetName::Classic), "");
    }
}

mod every_classic_plus_entry_equals_its_fixture_row_build_m4_t1_m9_t1 {
    use super::*;

    #[test]
    fn every_classic_plus_entry_equals_its_fixture_row() {
        assert_eq!(fixture_report(SetName::ClassicPlus), "");
    }
}

mod r1420_every_meditative_entry_equals_its_fixture_row_while_the_set_is_built {
    use super::*;

    #[test]
    fn r1420_every_meditative_entry_equals_its_fixture_row_and_none_is_missing_once_it_ships() {
        assert_eq!(fixture_report(SetName::Meditative), "");
        if set_ships(SetName::Meditative) {
            let by_key = by_key();
            let missing: Vec<&str> = MEDITATIVE
                .iter()
                .filter(|row| !by_key.contains_key(&key_of("Meditative", row.index)))
                .map(|row| row.name)
                .collect();
            assert_eq!(
                missing,
                Vec::<&str>::new(),
                "a shipped set holds every card it lists"
            );
        }
    }
}

mod rarity_distribution_spec_8_b2_5_build_m4_t1 {
    use super::*;

    #[test]
    fn is_core_32_40_16_7_5_classic_35_26_18_10_1_and_classic_plus_13_24_25_13_3_common_rare_epic_legendary_mythic()
     {
        for (set, expected) in RARITY_COUNTS {
            let mut counted: BTreeMap<String, usize> = BTreeMap::new();
            for entry in entries()
                .into_iter()
                .filter(|entry| entry.set.as_str() == *set && !entry.token)
            {
                *counted.entry(entry.rarity.as_str().to_string()).or_insert(0) += 1;
            }
            let expected: BTreeMap<String, usize> = expected
                .iter()
                .map(|(rarity, n)| ((*rarity).to_string(), *n))
                .collect();
            if ships(set) {
                assert_eq!(counted, expected, "{set} rarity counts over its non-token cards");
            } else {
                // R1420: a set being built holds no more of a rarity than its brief lists.
                for (rarity, n) in &counted {
                    let most = expected.get(rarity).copied().unwrap_or(0);
                    assert!(*n <= most, "{set}: {n} {rarity} non-token cards, at most {most}");
                }
            }
        }
    }

    #[test]
    fn gives_every_token_rarity_token_and_no_card_rarity_token() {
        let wrong: Vec<String> = entries()
            .into_iter()
            .filter(|entry| (entry.rarity == Rarity::Token) != entry.token)
            .map(|entry| {
                label(
                    entry,
                    "rarity",
                    &json(if entry.token { "Token" } else { "not Token" }),
                    &json(entry.rarity),
                )
            })
            .collect();
        assert_eq!(wrong, Vec::<String>::new(), "token rarity");
    }

    #[test]
    fn r739_every_family_shares_one_rarity_issue_44() {
        let rarity_of = |id: &str| -> Rarity {
            match CATALOG.get(id) {
                Some(entry) => entry.rarity,
                None => panic!("R739 family member {id} is not in the catalog"),
            }
        };
        // [family, expected rarity, member ids]. The Jlockeed family is Core's two: the Classic+
        // Jlockheed cards spell it differently and differ in type and effect shape, so the
        // template does not reach them (R739). The House is no Right-house/Wrong-House card.
        let families: &[(&str, Rarity, &[&str])] = &[
            (
                "Book of ___",
                Rarity::Epic,
                &[
                    "classic-003",
                    "classic-012",
                    "classic-016",
                    "classic-024",
                    "classic-029",
                    "classic-055",
                    "classic-070",
                    "classicplus-053",
                    "classicplus-054",
                    "classicplus-055",
                    "classicplus-056",
                    "classicplus-057",
                    "classicplus-071",
                    "classicplus-072",
                ],
            ),
            (
                "Call to Chaos",
                Rarity::Legendary,
                &["core-095", "classicplus-073"],
            ),
            (
                "___ Glowy Jelly Bean",
                Rarity::Rare,
                &["core-026", "core-027", "core-028", "core-029"],
            ),
            ("The Power to ___", Rarity::Rare, &["classic-020", "classic-081"]),
            ("___ in the System", Rarity::Epic, &["classic-018", "classic-025"]),
            ("___ of the Forgotten", Rarity::Rare, &["core-040", "classic-001"]),
            ("Rapid ___", Rarity::Common, &["core-010", "classic-026"]),
            (
                "___ Shroom",
                Rarity::Epic,
                &["classicplus-001", "classicplus-002"],
            ),
            ("Plague Bringer", Rarity::Rare, &["classic-061", "classic-076"]),
            ("Jlockeed ___", Rarity::Common, &["core-013", "core-014"]),
            (
                "Right-house defender / Wrong-House Attacker",
                Rarity::Common,
                &["core-003", "classicplus-006"],
            ),
        ];
        for (family, rarity, members) in families {
            for id in *members {
                assert_eq!(rarity_of(id), *rarity, "R739 {family} member {id}");
            }
        }
    }

    #[test]
    fn r739_a_bigger_version_of_an_effect_is_never_a_lower_rarity_pile_on_over_call_to_arms() {
        let rank = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
        let position = |id: &str| -> Option<usize> {
            CATALOG
                .get(id)
                .and_then(|entry| rank.iter().position(|r| *r == entry.rarity.as_str()))
        };
        // TS `indexOf` reads a missing card as -1; `None` sorts below every `Some` the same way.
        assert!(position("classic-060") >= position("core-069"));
    }

    #[test]
    fn r739_one_name_names_one_card() {
        let mut seen: IndexMap<String, String> = IndexMap::new();
        let mut dupes: Vec<String> = Vec::new();
        for entry in entries().into_iter().filter(|entry| !entry.token) {
            match seen.get(&entry.name) {
                None => {
                    seen.insert(entry.name.clone(), entry.id.clone());
                }
                Some(first) => dupes.push(format!("{} ({first}, {})", entry.name, entry.id)),
            }
        }
        assert_eq!(dupes, Vec::<String>::new(), "cards sharing a name");
    }

    #[test]
    fn b2_5_prints_the_designer_s_rarity_on_the_classic_plus_tokens_he_rated_never_on_a_card_or_an_ai_generated_card()
     {
        let printed: BTreeMap<String, String> = entries()
            .into_iter()
            .filter_map(|entry| {
                entry
                    .printed_rarity
                    .map(|rarity| (entry.index.clone(), rarity.as_str().to_string()))
            })
            .collect();
        let mut legendary: Vec<String> = CLASSIC_PLUS
            .iter()
            .filter(|row| row.index.starts_with("12.") || row.index.starts_with("19."))
            .map(|row| row.index.to_string())
            .collect();
        legendary.extend(strings(&["42.1", "46.1", "73.1", "75.1"]));
        let mut expected: BTreeMap<String, String> = legendary
            .into_iter()
            .map(|index| (index, "Legendary".to_string()))
            .collect();
        for (index, rarity) in [
            ("28.1", "Common"),
            ("30.1", "Epic"),
            ("32.1", "Epic"),
            ("32.2", "Epic"),
            ("32.3", "Epic"),
            ("38.1", "Epic"),
            ("36.1", "Rare"),
            ("76.1", "Rare"),
            ("65.1", "Common"),
            ("65.2", "Common"),
            ("65.3", "Rare"),
            ("65.4", "Legendary"),
            ("65.5", "Mythic"),
        ] {
            expected.insert(index.to_string(), rarity.to_string());
        }
        assert_eq!(printed, expected);
        // §8.8 prints a designer rarity on two Meditative tokens too (M #28.1 Common, M #30.1
        // Epic); every other printed rarity stays a Classic+ token's.
        assert!(
            entries()
                .into_iter()
                .filter(|entry| entry.printed_rarity.is_some())
                .all(|entry| entry.token
                    && (entry.set == SetName::ClassicPlus || entry.set == SetName::Meditative))
        );
    }
}

mod faces_that_change_more_than_text_b2_7_e40 {
    use super::*;

    #[test]
    fn gives_blood_moon_s_radiant_face_a_type_of_its_own_a_field_trap_and_no_other_face_one() {
        let typed: Vec<String> = entries()
            .into_iter()
            .flat_map(|entry| {
                [(FaceKind::Base, &entry.base), (FaceKind::Radiant, &entry.radiant)]
                    .into_iter()
                    .filter_map(move |(face, printed)| {
                        printed.type_.map(|kind| format!("{} {face} {kind}", entry.id))
                    })
            })
            .collect();
        assert_eq!(typed, strings(&["classicplus-022 radiant Field Trap"]));
        assert_eq!(
            CATALOG.get("classicplus-022").map(|entry| entry.type_),
            Some(CardType::Trap)
        );
    }

    #[test]
    fn prints_buff_billy_s_3x_3x_and_7x_7x_as_x_multiples_on_a_0_0_face_and_no_other_face_has_them() {
        let with_x: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.base.x_stats.is_some() || entry.radiant.x_stats.is_some())
            .map(|entry| entry.id.clone())
            .collect();
        assert_eq!(with_x, strings(&["classicplus-069"]));
        let billy = CATALOG.get("classicplus-069");
        assert_eq!(billy.map(|entry| entry.cost), Some(CardCost::X));
        assert_eq!(
            billy.and_then(|entry| entry.base.x_stats).map(json),
            Some(serde_json::json!({ "attack": 3, "health": 3 }))
        );
        assert_eq!(
            billy.and_then(|entry| entry.radiant.x_stats).map(json),
            Some(serde_json::json!({ "attack": 7, "health": 7 }))
        );
    }
}

mod names_r381_b2_8 {
    use super::*;

    fn name_of(id: &str) -> Option<String> {
        CATALOG.get(id).map(|entry| entry.name.clone())
    }

    #[test]
    fn r381_gives_no_two_cards_one_name_classic_55_is_book_of_wildfire_and_72_grand_counterspell() {
        let mut names: IndexMap<String, Vec<String>> = IndexMap::new();
        for entry in entries() {
            names
                .entry(entry.name.clone())
                .or_default()
                .push(entry.id.clone());
        }
        let shared: Vec<(String, Vec<String>)> = names.into_iter().filter(|(_, ids)| ids.len() > 1).collect();
        assert_eq!(shared, Vec::<(String, Vec<String>)>::new());
        assert_eq!(name_of("classic-016").as_deref(), Some("Book of Flame"));
        assert_eq!(name_of("classic-055").as_deref(), Some("Book of Wildfire"));
        assert_eq!(name_of("classic-017").as_deref(), Some("Counterspell"));
        assert_eq!(name_of("classic-072").as_deref(), Some("Grand Counterspell"));
        // Book of Wildfire is its own card: the designer's text, not a copy of Book of Flame's.
        assert_eq!(
            CATALOG.get("classic-055").map(|entry| entry.rarity),
            Some(Rarity::Epic)
        );
        assert_eq!(
            CATALOG.get("classic-055").map(|entry| entry.tags.clone()),
            Some(vec![Tag::Book])
        );
    }

    #[test]
    fn r381_keeps_the_names_that_are_rules_words_exile_burn_echo_recycle_as_the_designer_named_them() {
        let names: Vec<Option<String>> = ["classic-010", "classic-036", "classic-057", "classic-030"]
            .into_iter()
            .map(name_of)
            .collect();
        assert_eq!(
            names,
            ["Exile", "Burn", "Echo", "Recycle"]
                .map(|name| Some(name.to_string()))
                .to_vec()
        );
    }
}

mod tag_vocabulary_build_m4_t1_b2_4 {
    use super::*;

    #[test]
    fn uses_only_human_felinor_ky_cn_fruit_call_to_chaos_quickdraw_jlockeed_book_pancake_ai_plague_catalyst_prime_acclaimed_and_token()
     {
        let mut wrong: Vec<String> = Vec::new();
        for entry in entries() {
            for tag in &entry.tags {
                if !ALLOWED_TAGS.contains(&tag.as_str()) {
                    wrong.push(label(
                        entry,
                        "tags",
                        &json(format!("one of {}", ALLOWED_TAGS.join(", "))),
                        &json(tag),
                    ));
                }
            }
        }
        assert_eq!(wrong, Vec::<String>::new(), "tags outside the allowed vocabulary");
    }
}

mod the_jlockeed_tag_spec_5_8_r278_b2_4 {
    use super::*;

    #[test]
    fn r278_tags_core_13_and_14_the_three_classic_plus_jlockheed_cards_48_51_and_52_and_classic_4_palantir_and_no_other_entry()
     {
        let mut tagged: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.tags.contains(&Tag::Jlockeed))
            .map(|entry| entry.id.clone())
            .collect();
        tagged.sort();
        assert_eq!(
            tagged,
            strings(&[
                "classic-004",
                "classicplus-048",
                "classicplus-051",
                "classicplus-052",
                "core-013",
                "core-014",
            ]),
            "entries tagged Jlockeed"
        );
        // The tag follows the name: every entry whose name says Jlockeed, or the Classic+ spelling
        // Jlockheed, carries it — one faction under two spellings (B2.4) — plus Classic #4 Palantir,
        // which the balance patch inducted without renaming it.
        let mut named: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.name.contains("Jlockeed") || entry.name.contains("Jlockheed"))
            .map(|entry| entry.id.clone())
            .collect();
        named.sort();
        let expected: Vec<String> = tagged.into_iter().filter(|id| id != "classic-004").collect();
        assert_eq!(named, expected, "entries whose name names Jlockeed or Jlockheed");
    }
}

mod the_catalyst_prime_and_acclaimed_tags_spec_5_7_8_6_8_7_patch_v0_2_y {
    use super::*;

    fn tagged(tag: Tag) -> Vec<String> {
        let mut ids: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .map(|entry| entry.id.clone())
            .collect();
        ids.sort();
        ids
    }

    /// `/-Prime\b/`.
    fn hyphenated_prime(text: &str) -> bool {
        text.match_indices("-Prime").any(|(at, found)| {
            !text[at + found.len()..]
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    }

    #[test]
    fn tags_classic_plus_38_solarius_and_46_felinor_flagbearer_catalyst_and_no_other_entry() {
        assert_eq!(
            tagged(Tag::Catalyst),
            strings(&["classicplus-038", "classicplus-046"])
        );
    }

    #[test]
    fn tags_the_two_prime_tokens_prime_each_named_its_catalyst_s_name_and_a_spaced_prime_and_no_other_entry()
    {
        assert_eq!(
            tagged(Tag::Prime),
            strings(&["classicplus-038-1", "classicplus-046-1"])
        );
        // Every entry named "<card> Prime" carries the tag, and each is the token its Catalyst defines.
        let mut named: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.name.ends_with(" Prime"))
            .map(|entry| entry.id.clone())
            .collect();
        named.sort();
        assert_eq!(named, tagged(Tag::Prime), "entries named <card> Prime");
        for id in tagged(Tag::Prime) {
            let prime = CATALOG.get(id.as_str());
            let catalyst = CATALOG.get(id.strip_suffix("-1").unwrap_or(id.as_str()));
            assert_eq!(prime.map(|entry| entry.token), Some(true), "{id}");
            assert!(
                catalyst.is_some_and(|entry| entry.tags.contains(&Tag::Catalyst)),
                "{id}"
            );
            assert_eq!(
                prime.map(|entry| entry.name.clone()),
                Some(format!(
                    "{} Prime",
                    catalyst.map_or("?", |entry| entry.name.as_str())
                )),
                "{id}"
            );
        }
        // #322 spaces both: no name or face text writes a hyphenated Prime.
        let hyphenated: Vec<String> = entries()
            .into_iter()
            .filter(|entry| {
                [&entry.name, &entry.base.text, &entry.radiant.text]
                    .iter()
                    .any(|text| hyphenated_prime(text))
            })
            .map(|entry| entry.id.clone())
            .collect();
        assert_eq!(hyphenated, Vec::<String>::new());
    }

    #[test]
    fn tags_classic_80_boom_big_max_and_classic_plus_37_wardrum_acclaimed_and_no_other_entry() {
        assert_eq!(
            tagged(Tag::Acclaimed),
            strings(&["classic-080", "classicplus-037"])
        );
    }
}

mod every_entry_has_a_radiant_face_of_its_own_spec_5_2_r276 {
    use super::*;

    #[test]
    fn gives_no_entry_a_radiant_face_identical_to_its_base_face_the_five_that_had_none_included() {
        // R349: a card that prints no Radiant form has the fallback as its Radiant face, which doubles
        // its stats — for an X/X token, the X it is summoned with (`layers::worn_stats_override`) — so
        // it is the one entry whose printed faces may read alike. It is marked as such, and only a Unit.
        // A face's text is read with its own `params` values filled in (B3.4 rule 5): "Heal a target
        // {heal}." prints 9 on one face and 18 on the other.
        let printed = |entry: &CardDef, face: FaceKind| -> Value {
            let mut printed = json(entry.face(face));
            if let Some(object) = printed.as_object_mut() {
                object.insert("text".to_string(), Value::from(fill_params(entry, face, None)));
            }
            printed
        };
        // R674: Glitch is blank on both faces, the one entry exempt; its client face corrupts whatever it shows.
        let same: Vec<String> = entries()
            .into_iter()
            .filter(|entry| {
                entry.radiant_fallback != Some(true)
                    && entry.id != GLITCH_DEF_ID
                    && printed(entry, FaceKind::Radiant) == printed(entry, FaceKind::Base)
            })
            .map(|entry| format!("{} (#{}) radiant is a copy of base", entry.id, entry.index))
            .collect();
        assert_eq!(
            same,
            Vec::<String>::new(),
            "entries whose radiant face is identical to base"
        );
        let fallbacks: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.radiant_fallback == Some(true))
            .map(|entry| entry.id.clone())
            .collect();
        assert_eq!(fallbacks, strings(&["core-t-ghoul"]));
    }
}

mod no_face_prints_taunt_beside_indestructible_spec_6_1_r347 {
    use super::*;

    // R347 keeps Taunt off an Indestructible unit whatever prints it, so a face printing both would
    // print a Taunt it never has. Patch v0.1.1 dropped Indestructible from the two faces that did, #55
    // and #56 radiant (issue #27), and none may print both again — except Keymaster Keenus (M #65),
    // whose list prints every keyword and whose Taunt R347 drops while it is Indestructible (R1084).
    #[test]
    fn r347_r1084_no_unit_face_prints_both_but_keenus() {
        let both: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.type_ == CardType::Unit)
            .flat_map(|entry| {
                [(FaceKind::Base, &entry.base), (FaceKind::Radiant, &entry.radiant)]
                    .into_iter()
                    .filter_map(move |(face, printed)| {
                        let kinds: Vec<KeywordKind> =
                            printed.keywords.iter().map(|keyword| keyword.kind()).collect();
                        (kinds.contains(&KeywordKind::Taunt) && kinds.contains(&KeywordKind::Indestructible))
                            .then(|| format!("{} {face}", entry.id))
                    })
            })
            .collect();
        assert_eq!(both, ["meditative-065 base", "meditative-065 radiant"]);
    }
}
